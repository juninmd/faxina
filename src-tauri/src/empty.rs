use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::Serialize;

use crate::scan::is_link;

/// Folders whose content is the tool's own business: never descended, so never reported empty.
const SKIP_DIRS: &[&str] = &[".git", "node_modules"];
const MAX_ITEMS: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmptyKind {
    Folder,
    Link,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyItem {
    pub path: String,
    pub kind: EmptyKind,
}

/// Link whose target no longer exists. Re-checked right before removal: it may have been fixed.
pub fn is_dangling(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| is_link(&m))
        && fs::metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
}

/// Folders holding no file at all (only other empty folders) and dangling links under `root`.
/// Only the topmost empty folder of a chain is reported: removing it removes the rest.
pub fn find(root: &Path) -> Vec<EmptyItem> {
    let mut found = Vec::new();
    walk(root, &mut found);
    found.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    found.truncate(MAX_ITEMS);
    found
}

/// True when `dir` is empty in the above sense; its empty subfolders are then left for the caller
/// to report as part of it. Anything unreadable counts as not empty.
fn walk(dir: &Path, out: &mut Vec<EmptyItem>) -> bool {
    let Ok(rd) = fs::read_dir(dir) else {
        return false;
    };
    let mut keeps = false;
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else {
            keeps = true;
            continue;
        };
        let path = e.path();
        if is_link(&m) {
            keeps = true;
            if is_dangling(&path) {
                out.push(EmptyItem {
                    path: path.to_string_lossy().into_owned(),
                    kind: EmptyKind::Link,
                });
            }
        } else if m.is_dir() && !SKIP_DIRS.iter().any(|s| e.file_name() == **s) {
            subdirs.push(path);
        } else {
            keeps = true;
        }
    }
    let results: Vec<(PathBuf, bool, Vec<EmptyItem>)> = subdirs
        .into_par_iter()
        .map(|d| {
            let mut sub = Vec::new();
            let empty = walk(&d, &mut sub);
            (d, empty, sub)
        })
        .collect();
    let all_empty = !keeps && results.iter().all(|(_, empty, _)| *empty);
    for (d, empty, sub) in results {
        out.extend(sub);
        if empty && !all_empty {
            out.push(EmptyItem {
                path: d.to_string_lossy().into_owned(),
                kind: EmptyKind::Folder,
            });
        }
    }
    all_empty
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(items: &[EmptyItem], kind: EmptyKind) -> Vec<String> {
        items
            .iter()
            .filter(|i| i.kind == kind)
            .map(|i| {
                i.path
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn reports_only_the_topmost_empty_folder() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("a/b/c")).unwrap();
        fs::create_dir_all(t.path().join("keep/empty")).unwrap();
        fs::write(t.path().join("keep/file.txt"), "x").unwrap();
        let found = find(t.path());
        assert_eq!(paths(&found, EmptyKind::Folder), ["a", "empty"]);
    }

    #[test]
    fn skips_git_and_node_modules() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("p/.git/refs/heads")).unwrap();
        fs::create_dir_all(t.path().join("p/node_modules/x")).unwrap();
        assert!(find(t.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn finds_dangling_links_only() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("real"), "x").unwrap();
        std::os::unix::fs::symlink(t.path().join("real"), t.path().join("ok")).unwrap();
        std::os::unix::fs::symlink(t.path().join("gone"), t.path().join("broken")).unwrap();
        let found = find(t.path());
        assert_eq!(paths(&found, EmptyKind::Link), ["broken"]);
        assert!(paths(&found, EmptyKind::Folder).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_target_is_not_reported_as_dangling() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();
        let locked = tmp.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("target"), b"x").unwrap();
        let link = tmp.path().join("link");
        symlink(locked.join("target"), &link).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let dangling = is_dangling(&link);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            !dangling,
            "a link whose target exists must never be deleted"
        );
    }
}
