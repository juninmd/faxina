use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::scan::is_link;

pub struct Removal {
    pub result: Result<(), String>,
    /// Inside another target of the same request: it went with that one and frees nothing itself.
    pub nested: bool,
}

/// Removes guarded paths: one Recycle Bin batch, or a parallel permanent delete.
pub fn remove_all(targets: &[PathBuf], permanent: bool) -> Vec<Removal> {
    let parents = parents(targets);
    let tops: Vec<&Path> = targets
        .iter()
        .zip(&parents)
        .filter(|(_, p)| p.is_none())
        .map(|(t, _)| t.as_path())
        .collect();
    let results = if permanent {
        tops.par_iter()
            .map(|p| remove_permanent(p).map_err(|e| e.to_string()))
            .collect()
    } else {
        trash_batch(
            &tops,
            |b| trash::delete_all(b).map_err(|e| e.to_string()),
            |p| {
                trash::delete(p).map_err(|e| {
                    format!(
                        "{e} (se este disco não tem Lixeira, marque \"Excluir permanentemente\")"
                    )
                })
            },
        )
    };
    let mut own: Vec<Option<Result<(), String>>> = vec![None; targets.len()];
    let top_idx = parents.iter().enumerate().filter(|(_, p)| p.is_none());
    for ((i, _), r) in top_idx.zip(results) {
        own[i] = Some(r.map_err(|e| format!("não foi possível remover: {e}")));
    }
    parents
        .iter()
        .enumerate()
        .map(|(i, p)| Removal {
            result: own[p.unwrap_or(i)]
                .clone()
                .expect("every top-level target has a result"),
            nested: p.is_some(),
        })
        .collect()
}

/// Index of the target that already contains each one; duplicates count as nested in the first.
fn parents(targets: &[PathBuf]) -> Vec<Option<usize>> {
    let mut order: Vec<usize> = (0..targets.len()).collect();
    // Component-wise order puts every descendant right after its ancestor.
    order.sort_by(|&a, &b| targets[a].cmp(&targets[b]));
    let mut out = vec![None; targets.len()];
    let mut top: Option<usize> = None;
    for i in order {
        match top {
            Some(t) if targets[i].starts_with(&targets[t]) => out[i] = Some(t),
            _ => top = Some(i),
        }
    }
    out
}

/// One shell operation for the whole batch (one per path costs ~20 ms each on Windows).
fn trash_batch(
    paths: &[&Path],
    batch: impl FnOnce(&[&Path]) -> Result<(), String>,
    single: impl Fn(&Path) -> Result<(), String>,
) -> Vec<Result<(), String>> {
    if paths.is_empty() {
        return Vec::new();
    }
    // The batch error names no path, so whatever is still on disk is retried alone for its own error.
    let _ = batch(paths);
    paths
        .iter()
        .map(|p| if gone(p) { Ok(()) } else { single(p) })
        .collect()
}

fn gone(p: &Path) -> bool {
    matches!(fs::symlink_metadata(p), Err(e) if e.kind() == io::ErrorKind::NotFound)
}

fn remove_permanent(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !is_link(&meta) {
        remove_tree(path)
    } else if meta.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
}

/// Fans out one level so a single huge folder (node_modules) still uses every core.
pub fn remove_tree(dir: &Path) -> io::Result<()> {
    let entries: Vec<fs::DirEntry> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.par_iter().try_for_each(|e| {
        let path = e.path();
        if e.file_type()?.is_dir() {
            // std never follows links or junctions below this point.
            fs::remove_dir_all(path)
        } else {
            // A directory link needs remove_dir; neither call ever recurses.
            fs::remove_file(&path).or_else(|_| fs::remove_dir(&path))
        }
    })?;
    fs::remove_dir(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn tree(root: &Path) {
        for d in ["a/x", "a/y", "b"] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        for f in ["a/x/1.bin", "a/y/2.bin", "a/3.bin", "b/4.bin", "5.bin"] {
            fs::write(root.join(f), b"data").unwrap();
        }
    }

    #[test]
    fn permanent_removes_folders_files_and_nested_picks_once() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let t = |p: &str| tmp.path().join(p);
        let targets = [t("a/x"), t("a"), t("5.bin"), t("a"), t("b")];
        let out = remove_all(&targets, true);
        assert!(out.iter().all(|r| r.result.is_ok()));
        // a/x and the repeated a ride on the first a: counting them again would inflate "freed".
        assert_eq!(
            out.iter().map(|r| r.nested).collect::<Vec<_>>(),
            [true, false, false, true, false]
        );
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    #[test]
    fn nested_pick_shares_its_parent_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("gone");
        let out = remove_all(&[missing.join("child"), missing], true);
        assert!(out[0].nested);
        assert!(out[0].result.is_err() && out[1].result.is_err());
    }

    #[test]
    fn sibling_with_shared_prefix_is_not_nested() {
        let targets = [
            PathBuf::from("/r/ab"),
            PathBuf::from("/r/a"),
            PathBuf::from("/r/a/b"),
        ];
        assert_eq!(parents(&targets), [None, None, Some(1)]);
    }

    #[test]
    fn failed_batch_retries_only_what_is_still_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let (done, stuck) = (tmp.path().join("done"), tmp.path().join("stuck"));
        fs::write(&stuck, b"x").unwrap();
        let retried = RefCell::new(Vec::new());
        let out = trash_batch(
            &[done.as_path(), stuck.as_path()],
            |_| Err("Some operations were aborted".into()),
            |p| {
                retried.borrow_mut().push(p.to_path_buf());
                Err("in use".into())
            },
        );
        assert_eq!(out, [Ok(()), Err("in use".into())]);
        assert_eq!(retried.into_inner(), [stuck]);
    }

    #[test]
    fn successful_batch_still_reports_what_the_shell_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let kept = tmp.path().join("kept");
        fs::write(&kept, b"x").unwrap();
        let out = trash_batch(&[kept.as_path()], |_| Ok(()), |_| Err("skipped".into()));
        assert_eq!(out, [Err("skipped".into())]);
    }

    #[test]
    fn remove_tree_drops_links_without_touching_their_target() {
        let tmp = tempfile::tempdir().unwrap();
        let (outside, dir) = (tmp.path().join("outside"), tmp.path().join("dir"));
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"x").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, dir.join("link")).unwrap();
            std::os::unix::fs::symlink(&outside, dir.join("sub/link")).unwrap();
        }
        #[cfg(windows)]
        for link in [dir.join("link"), dir.join("sub").join("link")] {
            // Junctions need no admin rights, unlike symlinks.
            let ok = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&link)
                .arg(&outside)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok);
        }
        remove_tree(&dir).unwrap();
        assert!(!dir.exists());
        assert!(outside.join("keep.txt").exists());
    }
}
