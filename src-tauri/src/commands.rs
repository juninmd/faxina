use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::dupes::{self, DupGroup, DupProgress};
use crate::empty::{self, EmptyItem};
use crate::history::{self, Growth};
use crate::junk::{self, CleanReport, JunkItem};
use crate::model::{BigFile, Node, Suggestion, ViewNode};
use crate::scan::{self, Progress};
use crate::{guard, remove, view};

const MAX_DUP_GROUPS: usize = 500;

struct ScanData {
    root: PathBuf,
    tree: Node,
}

#[derive(Default)]
pub struct AppState {
    scan: Mutex<Option<ScanData>>,
    progress: Arc<Progress>,
    dup_root: Mutex<Option<PathBuf>>,
    dup_cancel: Arc<AtomicBool>,
    empty_root: Mutex<Option<PathBuf>>,
    growth: Mutex<Option<Growth>>,
}

#[derive(Clone, Serialize)]
struct ScanTick {
    files: u64,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    path: String,
    error: String,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    freed: u64,
    deleted: Vec<String>,
    failed: Vec<Failure>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // A panic inside a scan must not brick the app; the data is still consistent.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn view_of(data: &ScanData, path: &Path, depth: u8) -> Result<ViewNode, String> {
    let segs = view::relative(&data.root, path).ok_or("fora da área analisada")?;
    let node = view::find(&data.tree, &segs).ok_or("pasta não encontrada")?;
    Ok(view::build(
        node,
        path.to_path_buf(),
        depth.min(8),
        node.size / 3000,
    ))
}

#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    depth: u8,
) -> Result<ViewNode, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("pasta não encontrada".into());
    }
    let progress = state.progress.clone();
    progress.files.store(0, Ordering::Relaxed);
    progress.bytes.store(0, Ordering::Relaxed);
    progress.cancel.store(false, Ordering::Relaxed);

    let done = Arc::new(AtomicBool::new(false));
    let (ticker_done, ticker_progress) = (done.clone(), progress.clone());
    std::thread::spawn(move || {
        while !ticker_done.load(Ordering::Relaxed) {
            let tick = ScanTick {
                files: ticker_progress.files.load(Ordering::Relaxed),
                bytes: ticker_progress.bytes.load(Ordering::Relaxed),
            };
            let _ = app.emit("scan-progress", tick);
            std::thread::sleep(Duration::from_millis(120));
        }
    });

    let scan_root = root.clone();
    let tree = tauri::async_runtime::spawn_blocking(move || scan::scan(&scan_root, &progress))
        .await
        .map_err(|e| e.to_string());
    done.store(true, Ordering::Relaxed);
    let tree = tree?;
    if state.progress.cancel.load(Ordering::Relaxed) {
        return Err("análise cancelada".into());
    }

    let data = ScanData {
        root: root.clone(),
        tree,
    };
    let v = view_of(&data, &root, depth)?;
    let snap = history::snapshot(&data.tree, &root, now());
    let growth = tauri::async_runtime::spawn_blocking(move || {
        let mut store = history::load();
        let growth = history::record(&mut store, snap);
        history::save(&store);
        growth
    })
    .await
    .map_err(|e| e.to_string())?;
    *lock(&state.growth) = growth;
    *lock(&state.scan) = Some(data);
    Ok(v)
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.progress.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_view(state: State<'_, AppState>, path: String, depth: u8) -> Result<ViewNode, String> {
    let guard = lock(&state.scan);
    let data = guard.as_ref().ok_or("nenhuma análise")?;
    view_of(data, Path::new(&path), depth)
}

#[tauri::command]
pub fn get_suggestions(state: State<'_, AppState>) -> Vec<Suggestion> {
    lock(&state.scan).as_ref().map_or_else(Vec::new, |d| {
        let mut s = view::suggestions(&d.tree, &d.root, now(), 24);
        // A stale hiberfil.sys is huge and old, and still never ours to offer.
        s.retain(|x| !guard::is_protected(Path::new(&x.path)));
        s.truncate(12);
        s
    })
}

/// `node_modules` of projects untouched for `months`, biggest first. Empty until a scan has run.
#[tauri::command]
pub fn get_stale_deps(state: State<'_, AppState>, months: u32) -> Vec<Suggestion> {
    let min_secs = u64::from(months.clamp(1, 120)) * 30 * 24 * 3600;
    lock(&state.scan).as_ref().map_or_else(Vec::new, |d| {
        let mut s = view::stale_deps(&d.tree, &d.root, now(), min_secs, 50);
        s.retain(|x| !guard::is_protected(Path::new(&x.path)));
        s
    })
}

/// What grew since the previous scan of the same folder. `None` on the first scan of a folder.
#[tauri::command]
pub fn get_growth(state: State<'_, AppState>) -> Option<Growth> {
    lock(&state.growth).clone()
}

/// Biggest files of the current scan, largest first. Empty until a scan has run.
#[tauri::command]
pub fn largest_files(state: State<'_, AppState>, limit: usize) -> Vec<BigFile> {
    lock(&state.scan).as_ref().map_or_else(Vec::new, |d| {
        // Same rule as the suggestions: OS-owned entries are never ours to offer. The filter runs
        // before the cut, so a whole-drive scan still fills the list, and the protection lookups
        // are computed once for the whole walk.
        let protection = guard::Protection::new();
        view::biggest_files(&d.tree, &d.root, limit.clamp(1, 1000), |p| {
            !protection.is_protected(p)
        })
    })
}

fn disk_size(path: &Path) -> u64 {
    std::fs::symlink_metadata(path)
        .map(|m| {
            if m.is_dir() {
                junk::dir_size(path).0
            } else {
                m.len()
            }
        })
        .unwrap_or(0)
}

#[tauri::command]
pub async fn delete_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
    permanent: bool,
) -> Result<DeleteReport, String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    // The scan already knows each folder's size; walking it again before deleting doubled the work.
    let known: Vec<Option<u64>> = {
        let scan = lock(&state.scan);
        if let Some(d) = scan.as_ref() {
            roots.push(d.root.clone());
        }
        paths
            .iter()
            .map(|raw| {
                let d = scan.as_ref()?;
                let segs = view::relative(&d.root, Path::new(raw))?;
                view::find(&d.tree, &segs).map(|n| n.size)
            })
            .collect()
    };
    if let Some(r) = lock(&state.dup_root).as_ref() {
        roots.push(r.clone());
    }
    if let Some(r) = lock(&state.empty_root).as_ref() {
        roots.push(r.clone());
    }
    let outcomes = tauri::async_runtime::spawn_blocking(move || {
        let sized: Vec<Result<(PathBuf, u64), String>> = paths
            .iter()
            .zip(known)
            .map(|(raw, known)| {
                let p = guard::check(Path::new(raw), &roots)?;
                let size = known.unwrap_or_else(|| disk_size(&p));
                Ok((p, size))
            })
            .collect();
        let targets: Vec<PathBuf> = sized
            .iter()
            .filter_map(|s| s.as_ref().ok().map(|(p, _)| p.clone()))
            .collect();
        let mut removals = remove::remove_all(&targets, permanent).into_iter();
        paths
            .into_iter()
            .zip(sized)
            .map(|(raw, s)| match s {
                Err(e) => (raw, 0, Err(e)),
                Ok((_, size)) => {
                    let r = removals.next().expect("one removal per guarded path");
                    (raw, if r.nested { 0 } else { size }, r.result)
                }
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut report = DeleteReport::default();
    let mut scan = lock(&state.scan);
    for (raw, size, result) in outcomes {
        match result {
            Ok(()) => {
                if let Some(d) = scan.as_mut() {
                    if let Some(segs) = view::relative(&d.root, Path::new(&raw)) {
                        view::remove(&mut d.tree, &segs);
                    }
                }
                report.freed += size;
                report.deleted.push(raw);
            }
            Err(error) => report.failed.push(Failure { path: raw, error }),
        }
    }
    Ok(report)
}

#[tauri::command]
pub async fn junk_scan() -> Result<Vec<JunkItem>, String> {
    tauri::async_runtime::spawn_blocking(junk::scan_all)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn junk_clean(ids: Vec<String>, permanent: bool) -> Result<CleanReport, String> {
    tauri::async_runtime::spawn_blocking(move || junk::clean(&ids, permanent))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    min_size: u64,
) -> Result<Vec<DupGroup>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("pasta não encontrada".into());
    }
    *lock(&state.dup_root) = Some(root.clone());
    let cancel = state.dup_cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let groups = tauri::async_runtime::spawn_blocking(move || {
        let progress = DupProgress {
            done: AtomicU64::new(0),
            cancel: &cancel,
        };
        let finished = AtomicBool::new(false);
        std::thread::scope(|s| {
            s.spawn(|| {
                while !finished.load(Ordering::Relaxed) {
                    let _ = app.emit("dupes-progress", progress.done.load(Ordering::Relaxed));
                    std::thread::sleep(Duration::from_millis(150));
                }
            });
            let found = dupes::find(&root, min_size, &progress);
            finished.store(true, Ordering::Relaxed);
            found
        })
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(groups.into_iter().take(MAX_DUP_GROUPS).collect())
}

#[tauri::command]
pub async fn find_empty(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<EmptyItem>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("pasta não encontrada".into());
    }
    *lock(&state.empty_root) = Some(root.clone());
    tauri::async_runtime::spawn_blocking(move || empty::find(&root))
        .await
        .map_err(|e| e.to_string())
}

/// Removes dangling links found by `find_empty`. `delete_paths` refuses every link on purpose;
/// here each one is re-checked (inside the searched root, still a link, still dangling) and only
/// the link itself is removed, never what it points to.
#[tauri::command]
pub async fn delete_broken_links(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<DeleteReport, String> {
    let root = lock(&state.empty_root).clone();
    let mut report = DeleteReport::default();
    for raw in paths {
        let p = Path::new(&raw);
        let outcome = match root.as_deref() {
            Some(r) if p.is_absolute() => guard::check_dangling_link(p, r)
                .and_then(|()| remove::remove_link(p).map_err(|e| e.to_string())),
            _ => Err("fora da área analisada".into()),
        };
        match outcome {
            Ok(()) => report.deleted.push(raw),
            Err(error) => report.failed.push(Failure { path: raw, error }),
        }
    }
    Ok(report)
}

#[tauri::command]
pub fn cancel_duplicates(state: State<'_, AppState>) {
    state.dup_cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}
