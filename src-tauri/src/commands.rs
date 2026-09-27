use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::dupes::{self, DupGroup, DupProgress};
use crate::junk::{self, CleanReport, JunkItem};
use crate::model::{Node, Suggestion, ViewNode};
use crate::scan::{self, Progress};
use crate::{guard, view};

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

fn remove_one(path: &Path, permanent: bool) -> Result<(), String> {
    let result = if !permanent {
        trash::delete(path).map_err(|e| {
            format!("{e} (se este disco não tem Lixeira, marque \"Excluir permanentemente\")")
        })
    } else if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    };
    result.map_err(|e| format!("não foi possível remover: {e}"))
}

#[tauri::command]
pub async fn delete_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
    permanent: bool,
) -> Result<DeleteReport, String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(d) = lock(&state.scan).as_ref() {
        roots.push(d.root.clone());
    }
    if let Some(r) = lock(&state.dup_root).as_ref() {
        roots.push(r.clone());
    }
    let outcomes = tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|raw| {
                let target = PathBuf::from(&raw);
                let size = std::fs::symlink_metadata(&target)
                    .map(|m| {
                        if m.is_dir() {
                            junk::dir_size(&target).0
                        } else {
                            m.len()
                        }
                    })
                    .unwrap_or(0);
                let result = guard::check(&target, &roots).and_then(|p| remove_one(&p, permanent));
                (raw, size, result)
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
pub async fn junk_clean(ids: Vec<String>) -> Result<CleanReport, String> {
    tauri::async_runtime::spawn_blocking(move || junk::clean(&ids))
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
pub fn cancel_duplicates(state: State<'_, AppState>) {
    state.dup_cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}
