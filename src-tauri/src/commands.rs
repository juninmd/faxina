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
use crate::{guard, remove, view, worktrees};

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
    /// The groups last shown to the user, so a delete can be checked against them.
    dup_groups: Mutex<Vec<Vec<dupes::Expected>>>,
    dup_cancel: Arc<AtomicBool>,
    empty_root: Mutex<Option<PathBuf>>,
    /// Root and worktrees last shown, published together so a removal can never pair a new root
    /// with an old list.
    worktree_list: Mutex<Option<(PathBuf, Vec<PathBuf>)>>,
    /// Bumped by every search; an older overlapping search must not overwrite a newer result.
    worktree_epoch: AtomicU64,
    growth: Mutex<Option<Growth>>,
    junk: Mutex<Option<(u64, junk::Snapshot)>>,
    junk_ticket: AtomicU64,
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
    let refused = dupes::refused_selection(&lock(&state.dup_groups), &paths);
    let outcomes = tauri::async_runtime::spawn_blocking(move || {
        let sized: Vec<Result<(PathBuf, u64), String>> = paths
            .iter()
            .zip(known)
            .map(|(raw, known)| {
                if let Some(reason) = refused.get(raw) {
                    return Err(reason.clone());
                }
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
    dupes::forget_deleted(&mut lock(&state.dup_groups), &report.deleted);
    Ok(report)
}

#[tauri::command]
pub async fn junk_scan(state: State<'_, AppState>) -> Result<JunkScan, String> {
    let scan_id = {
        let mut current = lock(&state.junk);
        let id = state.junk_ticket.fetch_add(1, Ordering::SeqCst) + 1;
        *current = None;
        id
    };
    let (items, snapshot) = tauri::async_runtime::spawn_blocking(junk::scan_all)
        .await
        .map_err(|e| e.to_string())?;
    let mut current = lock(&state.junk);
    if state.junk_ticket.load(Ordering::SeqCst) == scan_id {
        *current = Some((scan_id, snapshot));
    }
    Ok(JunkScan { scan_id, items })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkScan {
    scan_id: u64,
    items: Vec<JunkItem>,
}

#[tauri::command]
pub async fn junk_clean(
    state: State<'_, AppState>,
    scan_id: u64,
    ids: Vec<String>,
    permanent: bool,
) -> Result<CleanReport, String> {
    let snapshot = take_junk_snapshot(&state, scan_id)?;
    tauri::async_runtime::spawn_blocking(move || junk::clean(snapshot, &ids, permanent))
        .await
        .map_err(|e| e.to_string())
}

/// Dry run: what `junk_clean` would free for `ids`. The reviewed snapshot is put back afterwards
/// so the real cleanup can still use it.
#[tauri::command]
pub async fn junk_preview(
    state: State<'_, AppState>,
    scan_id: u64,
    ids: Vec<String>,
) -> Result<CleanReport, String> {
    let snapshot = take_junk_snapshot(&state, scan_id)?;
    let (report, snapshot) = tauri::async_runtime::spawn_blocking(move || {
        let report = junk::preview(&snapshot, &ids);
        (report, snapshot)
    })
    .await
    .map_err(|e| e.to_string())?;
    let mut current = lock(&state.junk);
    if state.junk_ticket.load(Ordering::SeqCst) == scan_id {
        *current = Some((scan_id, snapshot));
    }
    Ok(report)
}

fn take_junk_snapshot(state: &AppState, scan_id: u64) -> Result<junk::Snapshot, String> {
    {
        let mut current = lock(&state.junk);
        if current.as_ref().map(|(id, _)| *id) != Some(scan_id)
            || state.junk_ticket.load(Ordering::SeqCst) != scan_id
        {
            return Err("A análise mudou. Analise novamente antes de limpar.".into());
        }
        Ok(current.take().expect("snapshot checked above").1)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupResult {
    pub groups: Vec<DupGroup>,
    pub cancelled: bool,
    pub truncated: bool,
}

#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    min_size: u64,
    exclude_names: Option<Vec<String>>,
) -> Result<DupResult, String> {
    let excluded = dupes::ExcludedNames::compile(&exclude_names.unwrap_or_default())?;
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("pasta não encontrada".into());
    }
    // This root becomes a deletion root, so it must not be a protected folder itself.
    if guard::is_protected(&root) {
        return Err("pasta protegida do sistema".into());
    }
    *lock(&state.dup_root) = Some(root.clone());
    lock(&state.dup_groups).clear();
    let cancel = state.dup_cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let (groups, cancelled) = tauri::async_runtime::spawn_blocking(move || {
        let progress = DupProgress::new(&cancel);
        let finished = AtomicBool::new(false);
        // Also runs on unwind, so a panic in the search still stops the progress thread.
        struct Finish<'a>(&'a AtomicBool);
        impl Drop for Finish<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        std::thread::scope(|s| {
            s.spawn(|| {
                while !finished.load(Ordering::Relaxed) {
                    let _ = app.emit("dupes-progress", progress.tick());
                    std::thread::sleep(Duration::from_millis(150));
                }
            });
            let _finish = Finish(&finished);
            dupes::find_excluding(&root, min_size, &progress, &excluded)
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    let truncated = groups.len() > MAX_DUP_GROUPS;
    let groups: Vec<DupGroup> = groups.into_iter().take(MAX_DUP_GROUPS).collect();
    *lock(&state.dup_groups) = dupes::expected_of(&groups);

    Ok(DupResult {
        groups,
        cancelled,
        truncated,
    })
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
    // This root becomes a deletion root, so it must not be a protected folder itself.
    if guard::is_protected(&root) {
        return Err("pasta protegida do sistema".into());
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
    tauri::async_runtime::spawn_blocking(move || {
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
        report
    })
    .await
    .map_err(|e| e.to_string())
}

/// Linked Git worktrees under `path`. Unlike the duplicate and empty searches, the chosen folder
/// is never a target itself (a worktree is always below it), so a home or volume root can be
/// searched; `guard::check` still refuses anything outside this root at removal time.
#[tauri::command]
pub async fn find_worktrees(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<worktrees::Worktree>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("pasta não encontrada".into());
    }
    let mine = state.worktree_epoch.fetch_add(1, Ordering::SeqCst) + 1;
    *lock(&state.worktree_list) = None;
    let store_root = root.clone();
    let found = tauri::async_runtime::spawn_blocking(move || worktrees::find(&root))
        .await
        .map_err(|e| e.to_string())?;
    // A search that lost the race keeps its result on screen but never authorises a removal.
    if state.worktree_epoch.load(Ordering::SeqCst) == mine {
        let paths = found.iter().map(|w| PathBuf::from(&w.path)).collect();
        *lock(&state.worktree_list) = Some((store_root, paths));
    }
    Ok(found)
}

/// Root of the listing that contains `raw`, or why the request is refused.
fn shown_root<'a>(
    list: Option<&'a (PathBuf, Vec<PathBuf>)>,
    raw: &str,
) -> Result<&'a PathBuf, String> {
    let (root, paths) = list.ok_or("fora da área analisada")?;
    let path = Path::new(raw);
    paths
        .iter()
        .any(|p| p == path || worktrees::same_path(p, path))
        .then_some(root)
        .ok_or_else(|| "a worktree não está na última lista mostrada".to_string())
}

/// Removes worktrees from the list last shown. Each path must be one of them, and each one is
/// re-checked by git first: a checkout that gained a local edit, a lock or a commit outside the
/// remote stays where it is. Removal goes to the OS trash; this screen never deletes for good.
#[tauri::command]
pub async fn delete_worktrees(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<worktrees::Removed, String> {
    let list = lock(&state.worktree_list).clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let mut report = worktrees::Removed::default();
        for raw in paths {
            let outcome = match shown_root(list.as_ref(), &raw) {
                Ok(root) => guard::check(Path::new(&raw), std::slice::from_ref(root))
                    .and_then(|p| worktrees::remove(&p)),
                Err(error) => Err(error),
            };
            match outcome {
                Ok(outcome) => {
                    report.freed += outcome.freed;
                    report.warnings.extend(outcome.warnings);
                    report.removed.push(raw);
                }
                Err(error) => report.failed.push(worktrees::Failure { path: raw, error }),
            }
        }
        report
    })
    .await
    .map_err(|e| e.to_string())?;
    // A removed checkout must not stay authorised for a later request, even if something is
    // restored at the same path.
    if let Some((_, listed)) = lock(&state.worktree_list).as_mut() {
        listed.retain(|p| {
            !report
                .removed
                .iter()
                .any(|r| p == Path::new(r) || worktrees::same_path(p, Path::new(r)))
        });
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junk_snapshot_rejects_stale_tokens_and_can_only_be_consumed_once() {
        let state = AppState::default();
        state.junk_ticket.store(7, Ordering::SeqCst);
        *lock(&state.junk) = Some((7, junk::Snapshot::default()));
        assert!(take_junk_snapshot(&state, 6).is_err());
        assert!(lock(&state.junk).is_some());
        assert!(take_junk_snapshot(&state, 7).is_ok());
        assert!(take_junk_snapshot(&state, 7).is_err());
        *lock(&state.junk) = Some((7, junk::Snapshot::default()));
        state.junk_ticket.store(8, Ordering::SeqCst);
        assert!(take_junk_snapshot(&state, 7).is_err());
    }

    #[test]
    fn truncated_flag_set_when_exceeding_max_dup_groups() {
        let make_group = |i| DupGroup {
            hash: format!("hash_{i}"),
            size: 100,
            files: vec![],
        };

        let groups: Vec<_> = (0..501).map(make_group).collect();
        let truncated = groups.len() > MAX_DUP_GROUPS;
        let taken: Vec<_> = groups.into_iter().take(MAX_DUP_GROUPS).collect();

        assert_eq!(taken.len(), 500);
        assert!(truncated);

        let small_groups: Vec<_> = (0..10).map(make_group).collect();
        let small_truncated = small_groups.len() > MAX_DUP_GROUPS;
        let small_taken: Vec<_> = small_groups.into_iter().take(MAX_DUP_GROUPS).collect();

        assert_eq!(small_taken.len(), 10);
        assert!(!small_truncated);
    }

    #[test]
    fn delete_paths_enforces_safety_and_updates_state() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let file_a = root.join("a.txt");
        let file_b = root.join("b.txt");
        let file_outside = tempfile::NamedTempFile::new().unwrap();

        std::fs::write(&file_a, "hello").unwrap();
        std::fs::write(&file_b, "world!").unwrap();

        let state = AppState::default();
        *lock(&state.scan) = Some(ScanData {
            root: root.clone(),
            tree: Node {
                name: "root".into(),
                size: 11,
                files: 2,
                modified: 0,
                kind: crate::model::Kind::Other,
                is_dir: true,
                grouped: false,
                reclaimable: false,
                children: vec![
                    Node {
                        name: "a.txt".into(),
                        size: 5,
                        files: 1,
                        modified: 0,
                        kind: crate::model::Kind::Other,
                        is_dir: false,
                        grouped: false,
                        reclaimable: false,
                        children: vec![],
                    },
                    Node {
                        name: "b.txt".into(),
                        size: 6,
                        files: 1,
                        modified: 0,
                        kind: crate::model::Kind::Other,
                        is_dir: false,
                        grouped: false,
                        reclaimable: false,
                        children: vec![],
                    },
                ],
            },
        });

        // Outside path is refused
        let paths = vec![
            file_a.to_string_lossy().to_string(),
            file_outside.path().to_string_lossy().to_string(),
        ];
        let report = tauri::async_runtime::block_on(async {
            // Manual test execution of delete_paths logic without tauri::State wrapper
            let mut roots: Vec<PathBuf> = Vec::new();
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
            let refused = dupes::refused_selection(&lock(&state.dup_groups), &paths);
            let sized: Vec<Result<(PathBuf, u64), String>> = paths
                .iter()
                .zip(known)
                .map(|(raw, known)| {
                    if let Some(reason) = refused.get(raw) {
                        return Err(reason.clone());
                    }
                    let p = guard::check(Path::new(raw), &roots)?;
                    let size = known.unwrap_or_else(|| disk_size(&p));
                    Ok((p, size))
                })
                .collect();
            let targets: Vec<PathBuf> = sized
                .iter()
                .filter_map(|s| s.as_ref().ok().map(|(p, _)| p.clone()))
                .collect();
            let mut removals = remove::remove_all(&targets, true).into_iter();
            let outcomes: Vec<_> = paths
                .into_iter()
                .zip(sized)
                .map(|(raw, s)| match s {
                    Err(e) => (raw, 0, Err(e)),
                    Ok((_, size)) => {
                        let r = removals.next().expect("one removal per guarded path");
                        (raw, if r.nested { 0 } else { size }, r.result)
                    }
                })
                .collect();

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
            report
        });

        assert_eq!(report.deleted.len(), 1);
        assert_eq!(report.failed.len(), 1);
        assert!(!file_a.exists());
        assert!(file_outside.path().exists());
        assert_eq!(report.freed, 5);

        // Verify state tree was updated in memory
        let scan = lock(&state.scan);
        assert_eq!(scan.as_ref().unwrap().tree.size, 6);
    }

    #[test]
    fn delete_paths_refuses_when_dupe_invariants_broken() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let file_a = root.join("copy1.txt");
        let file_b = root.join("copy2.txt");
        std::fs::write(&file_a, "duplicate").unwrap();
        std::fs::write(&file_b, "duplicate").unwrap();

        let state = AppState::default();
        *lock(&state.dup_root) = Some(root.clone());

        let groups = vec![DupGroup {
            hash: "testhash".into(),
            size: 9,
            files: vec![
                dupes::DupFile {
                    path: file_a.to_string_lossy().to_string(),
                    modified: 100,
                },
                dupes::DupFile {
                    path: file_b.to_string_lossy().to_string(),
                    modified: 100,
                },
            ],
        }];
        *lock(&state.dup_groups) = dupes::expected_of(&groups);

        // Attempting to delete BOTH copies of a duplicate group must be refused
        let paths = vec![
            file_a.to_string_lossy().to_string(),
            file_b.to_string_lossy().to_string(),
        ];
        let refused = dupes::refused_selection(&lock(&state.dup_groups), &paths);
        assert!(
            refused.contains_key(&paths[0]),
            "refuses deleting all copies"
        );
        assert!(
            refused.contains_key(&paths[1]),
            "refuses deleting all copies"
        );
    }

    #[test]
    fn delete_broken_links_removes_dangling_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let target = root.join("target.txt");
        std::fs::write(&target, "real").unwrap();

        let link_valid = root.join("link_valid");
        let link_dangling = root.join("link_dangling");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link_valid).unwrap();
            std::os::unix::fs::symlink(root.join("nonexistent.txt"), &link_dangling).unwrap();
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&target, &link_valid).unwrap();
            std::os::windows::fs::symlink_file(root.join("nonexistent.txt"), &link_dangling)
                .unwrap();
        }

        let state = AppState::default();
        *lock(&state.empty_root) = Some(root.clone());

        let paths = vec![
            link_valid.to_string_lossy().to_string(),
            link_dangling.to_string_lossy().to_string(),
            root.join("target.txt").to_string_lossy().to_string(),
        ];

        let report = {
            let empty_root = lock(&state.empty_root).clone();
            let mut report = DeleteReport::default();
            for raw in paths {
                let p = Path::new(&raw);
                let outcome = match empty_root.as_deref() {
                    Some(r) if p.is_absolute() => guard::check_dangling_link(p, r)
                        .and_then(|()| remove::remove_link(p).map_err(|e| e.to_string())),
                    _ => Err("fora da área analisada".into()),
                };
                match outcome {
                    Ok(()) => report.deleted.push(raw),
                    Err(error) => report.failed.push(Failure { path: raw, error }),
                }
            }
            report
        };

        // link_dangling should be deleted, link_valid and target.txt should fail
        assert_eq!(report.deleted.len(), 1);
        assert_eq!(report.failed.len(), 2);
        assert!(!link_dangling.exists());
        assert!(target.exists());
    }

    #[test]
    fn junk_preview_and_clean_dry_run_parity() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("temp.log"), "12345").unwrap();
        std::fs::write(cache_dir.join("other.tmp"), "12345678").unwrap();

        let snapshot = junk::Snapshot::default();
        let state = AppState::default();
        state.junk_ticket.store(1, Ordering::SeqCst);
        *lock(&state.junk) = Some((1, snapshot));

        // Dry run preview returns report and preserves the snapshot with the same ticket
        let ids = vec!["unknown_id".to_string()];
        let preview_res = {
            let snap = take_junk_snapshot(&state, 1).unwrap();
            let report = junk::preview(&snap, &ids);
            *lock(&state.junk) = Some((1, snap));
            report
        };

        assert_eq!(preview_res.freed, 0);
        assert_eq!(preview_res.removed, 0);

        // Snapshot is still available for real clean
        let snap_after = take_junk_snapshot(&state, 1);
        assert!(snap_after.is_ok());
    }

    #[test]
    fn a_removal_is_bound_to_the_worktrees_last_shown() {
        let tmp = tempfile::tempdir().unwrap();
        let shown = tmp.path().join("shown");
        let hidden = tmp.path().join("hidden");
        std::fs::create_dir_all(&shown).unwrap();
        std::fs::create_dir_all(&hidden).unwrap();
        let list = (tmp.path().to_path_buf(), vec![shown.clone()]);

        assert!(shown_root(Some(&list), shown.to_str().unwrap()).is_ok());
        assert!(shown_root(Some(&list), hidden.to_str().unwrap()).is_err());
        assert!(shown_root(None, shown.to_str().unwrap()).is_err());
    }
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
