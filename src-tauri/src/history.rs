//! Remembers the size of the big folders after every scan, so the next scan of the same root can
//! say what grew in between. Paths and sizes only; nothing here ever deletes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::Node;

/// Folders smaller than this are not saved, which keeps the history file small.
const MIN_FOLDER: u64 = 64 << 20;
/// Growth below this is noise (logs, browser sessions) and is not reported.
const MIN_GROWTH: u64 = 100 << 20;
/// A parent whose growth is mostly explained by one deeper folder is hidden behind it.
const EXPLAINED_SHARE: f64 = 0.8;
const MAX_FOLDERS: usize = 3000;
const SHOWN: usize = 15;
/// Scans kept per root.
const KEEP: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub root: String,
    pub taken_at: u64,
    pub total: u64,
    pub folders: Vec<Folder>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderGrowth {
    pub path: String,
    pub before: u64,
    /// False when the folder was too small to be saved last time: `before` is then an upper bound.
    pub before_known: bool,
    pub after: u64,
    pub growth: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Growth {
    pub root: String,
    pub previous_at: u64,
    /// Total size now minus total size then; negative when the root shrank.
    pub delta: i64,
    pub grown: Vec<FolderGrowth>,
}

pub fn snapshot(tree: &Node, root: &Path, taken_at: u64) -> Snapshot {
    let mut folders = Vec::new();
    collect(tree, root.to_path_buf(), &mut folders);
    folders.sort_unstable_by_key(|f| std::cmp::Reverse(f.size));
    folders.truncate(MAX_FOLDERS);
    Snapshot {
        root: root.to_string_lossy().into_owned(),
        taken_at,
        total: tree.size,
        folders,
    }
}

fn collect(node: &Node, path: PathBuf, out: &mut Vec<Folder>) {
    if !node.is_dir || node.grouped || node.size < MIN_FOLDER {
        return;
    }
    out.push(Folder {
        path: path.to_string_lossy().into_owned(),
        size: node.size,
    });
    for c in &node.children {
        collect(c, path.join(&c.name), out);
    }
}

pub fn compare(previous: &Snapshot, current: &Snapshot) -> Growth {
    let before: HashMap<&str, u64> = previous
        .folders
        .iter()
        .map(|f| (f.path.as_str(), f.size))
        .collect();
    let growing: Vec<FolderGrowth> = current
        .folders
        .iter()
        .filter_map(|f| {
            let saved = before.get(f.path.as_str()).copied();
            // A folder missing from the old scan was smaller than the saving threshold.
            let old = saved.unwrap_or(MIN_FOLDER);
            let growth = f.size.saturating_sub(old);
            (growth >= MIN_GROWTH).then(|| FolderGrowth {
                path: f.path.clone(),
                before: old,
                before_known: saved.is_some(),
                after: f.size,
                growth,
            })
        })
        .collect();
    let mut grown: Vec<FolderGrowth> = growing
        .iter()
        .filter(|c| !explained_by_deeper(c, &growing))
        .cloned()
        .collect();
    grown.sort_unstable_by_key(|g| std::cmp::Reverse(g.growth));
    grown.truncate(SHOWN);
    Growth {
        root: current.root.clone(),
        previous_at: previous.taken_at,
        delta: i128::from(current.total)
            .saturating_sub(i128::from(previous.total))
            .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
        grown,
    }
}

fn explained_by_deeper(candidate: &FolderGrowth, all: &[FolderGrowth]) -> bool {
    let base = Path::new(&candidate.path);
    all.iter().any(|o| {
        let p = Path::new(&o.path);
        p != base
            && p.starts_with(base)
            && o.growth as f64 >= candidate.growth as f64 * EXPLAINED_SHARE
    })
}

/// Adds `current` to the history and returns what changed since the last scan of the same root.
pub fn record(store: &mut Vec<Snapshot>, current: Snapshot) -> Option<Growth> {
    let growth = store
        .iter()
        .rfind(|s| s.root == current.root)
        .map(|prev| compare(prev, &current));
    let root = current.root.clone();
    store.push(current);
    let mut same = store.iter().filter(|s| s.root == root).count();
    store.retain(|s| {
        if s.root != root || same <= KEEP {
            return true;
        }
        same -= 1;
        false
    });
    growth
}

fn store_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("faxina").join("history.json"))
}

/// A missing or damaged file just means "no history yet".
pub fn load() -> Vec<Snapshot> {
    store_path().map(|p| load_from(&p)).unwrap_or_default()
}

fn load_from(path: &Path) -> Vec<Snapshot> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    match serde_json::from_slice(&bytes) {
        Ok(store) => store,
        Err(_) => {
            // Keep the damaged file: the next save would otherwise overwrite it without a trace.
            let _ = std::fs::rename(path, path.with_extension("json.corrupt"));
            Vec::new()
        }
    }
}

pub fn save(store: &[Snapshot]) {
    let Some(path) = store_path() else { return };
    let Some(dir) = path.parent() else { return };
    let Ok(json) = serde_json::to_vec(store) else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    // Write beside the target and rename, so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() && std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1 << 20;

    fn snap(root: &str, at: u64, total: u64, folders: &[(&str, u64)]) -> Snapshot {
        Snapshot {
            root: root.into(),
            taken_at: at,
            total,
            folders: folders
                .iter()
                .map(|(p, s)| Folder {
                    path: (*p).into(),
                    size: s * MB,
                })
                .collect(),
        }
    }

    #[test]
    fn reports_growth_and_hides_parents_explained_by_a_child() {
        let old = snap("/r", 1, 1000 * MB, &[("/r/a", 500), ("/r/a/b", 400)]);
        let new = snap("/r", 2, 1700 * MB, &[("/r/a", 1200), ("/r/a/b", 1100)]);
        let g = compare(&old, &new);
        assert_eq!(g.delta, (700 * MB) as i64);
        assert_eq!(g.grown.len(), 1);
        assert_eq!(g.grown[0].path, "/r/a/b");
        assert_eq!(g.grown[0].growth, 700 * MB);
    }

    #[test]
    fn keeps_a_parent_when_growth_is_spread_over_children() {
        let old = snap(
            "/r",
            1,
            0,
            &[("/r/a", 500), ("/r/a/b", 200), ("/r/a/c", 200)],
        );
        let new = snap(
            "/r",
            2,
            0,
            &[("/r/a", 1000), ("/r/a/b", 400), ("/r/a/c", 400)],
        );
        let g = compare(&old, &new);
        let paths: Vec<_> = g.grown.iter().map(|x| x.path.as_str()).collect();
        assert!(paths.contains(&"/r/a"));
    }

    #[test]
    fn small_growth_is_noise_and_unknown_folders_count_from_the_threshold() {
        let old = snap("/r", 1, 0, &[("/r/a", 500)]);
        let new = snap("/r", 2, 0, &[("/r/a", 520), ("/r/new", 400)]);
        let g = compare(&old, &new);
        assert_eq!(g.grown.len(), 1);
        assert_eq!(g.grown[0].path, "/r/new");
        assert!(!g.grown[0].before_known);
        assert_eq!(g.grown[0].growth, 400 * MB - MIN_FOLDER);
    }

    #[test]
    fn shrinking_gives_a_negative_delta() {
        let old = snap("/r", 1, 900 * MB, &[]);
        let new = snap("/r", 2, 400 * MB, &[]);
        assert_eq!(compare(&old, &new).delta, -((500 * MB) as i64));
    }

    #[test]
    fn record_compares_against_the_same_root_only_and_trims() {
        let mut store = vec![snap("/other", 1, 1, &[])];
        assert!(record(&mut store, snap("/r", 2, 1, &[])).is_none());
        for i in 0..20 {
            record(&mut store, snap("/r", 3 + i, 1, &[]));
        }
        assert_eq!(store.iter().filter(|s| s.root == "/r").count(), KEEP);
        assert_eq!(store.iter().filter(|s| s.root == "/other").count(), 1);
        let last = record(&mut store, snap("/r", 99, 1, &[])).expect("previous exists");
        assert_eq!(last.previous_at, 22);
    }

    #[test]
    fn a_damaged_store_is_kept_aside_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.json");
        std::fs::write(&path, b"{ not json").unwrap();
        assert!(load_from(&path).is_empty());
        assert!(!path.exists(), "the damaged file is moved away");
        assert_eq!(
            std::fs::read(path.with_extension("json.corrupt")).unwrap(),
            b"{ not json"
        );
    }

    #[test]
    fn a_missing_store_is_empty_and_leaves_nothing_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.json");
        assert!(load_from(&path).is_empty());
        assert!(!path.with_extension("json.corrupt").exists());
    }
}
