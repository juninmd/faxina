use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rayon::prelude::*;
use serde::Serialize;

use crate::junk_defs::{definitions, Base, JunkDef};
use crate::scan::is_link;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkItem {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub size: u64,
    pub files: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub freed: u64,
    pub removed: u64,
    /// Entries in use by a running program (typical in %TEMP%) are left alone.
    pub skipped: u64,
}

pub fn base_dir(base: Base) -> Option<PathBuf> {
    match base {
        Base::Home => dirs::home_dir(),
        Base::Local => dirs::data_local_dir(),
        Base::Roaming => dirs::data_dir(),
        Base::Cache => dirs::cache_dir(),
        Base::Temp => Some(std::env::temp_dir()),
    }
}

/// A directory we may step into: never a symlink or junction, which could point anywhere.
fn real_dir(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !is_link(&m))
}

/// Expands `*` segments (e.g. `User Data/*/Cache`) into the directories that exist.
pub fn expand(base: &Path, rel: &str) -> Vec<PathBuf> {
    if !real_dir(base) {
        return Vec::new();
    }
    let mut current = vec![base.to_path_buf()];
    for seg in rel.split('/').filter(|s| !s.is_empty()) {
        current = current
            .into_iter()
            .flat_map(|dir| -> Vec<PathBuf> {
                if seg == "*" {
                    fs::read_dir(&dir)
                        .map(|rd| {
                            rd.flatten()
                                .map(|e| e.path())
                                .filter(|p| real_dir(p))
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    let next = dir.join(seg);
                    if real_dir(&next) {
                        vec![next]
                    } else {
                        Vec::new()
                    }
                }
            })
            .collect();
    }
    current
}

pub fn resolve(def: &JunkDef) -> Vec<PathBuf> {
    let Some(base) = base_dir(def.base) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = def.rels.iter().flat_map(|rel| expand(&base, rel)).collect();
    out.sort();
    out.dedup();
    out
}

pub fn dir_size(path: &Path) -> (u64, u64) {
    let Ok(rd) = fs::read_dir(path) else {
        return (0, 0);
    };
    rd.flatten()
        .filter_map(|e| e.metadata().ok().map(|m| (e.path(), m)))
        .filter(|(_, m)| !is_link(m))
        .map(|(p, m)| {
            if m.is_dir() {
                dir_size(&p)
            } else {
                (m.len(), 1)
            }
        })
        .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
}

fn old_enough(meta: &fs::Metadata, min_age: Duration) -> bool {
    min_age.is_zero()
        || meta
            .modified()
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age >= min_age)
}

pub fn scan_all() -> Vec<JunkItem> {
    let mut items: Vec<JunkItem> = definitions()
        .par_iter()
        .filter_map(|def| {
            let paths = resolve(def);
            let (size, files) = paths
                .iter()
                .map(|p| dir_size(p))
                .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
            (size > 0).then(|| JunkItem {
                id: def.id,
                label: def.label,
                group: def.group,
                size,
                files,
                paths: paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
            })
        })
        .collect();
    items.sort_unstable_by_key(|n| std::cmp::Reverse(n.size));
    items
}

/// Deletes the *contents* of each location; the folder itself stays so apps keep working.
pub fn clean_dir(dir: &Path, min_age: Duration) -> CleanReport {
    let mut report = CleanReport::default();
    let Ok(rd) = fs::read_dir(dir) else {
        return report;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !old_enough(&meta, min_age) {
            report.skipped += 1;
            continue;
        }
        let (size, files) = if meta.is_dir() && !is_link(&meta) {
            dir_size(&path)
        } else {
            (meta.len(), 1)
        };
        let result = if meta.is_dir() && !is_link(&meta) {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match result {
            Ok(()) => {
                report.freed += size;
                report.removed += files;
            }
            Err(_) => report.skipped += 1,
        }
    }
    report
}

pub fn clean(ids: &[String]) -> CleanReport {
    definitions()
        .iter()
        .filter(|d| ids.iter().any(|id| id == d.id))
        .flat_map(|d| resolve(d).into_iter().map(move |p| (p, d.min_age)))
        .map(|(p, age)| clean_dir(&p, age))
        .fold(CleanReport::default(), |a, b| CleanReport {
            freed: a.freed + b.freed,
            removed: a.removed + b.removed,
            skipped: a.skipped + b.skipped,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_matches_every_profile() {
        let tmp = tempfile::tempdir().unwrap();
        for p in ["Default", "Profile 1"] {
            fs::create_dir_all(tmp.path().join("User Data").join(p).join("Cache")).unwrap();
        }
        fs::create_dir_all(tmp.path().join("User Data/System Profile")).unwrap();
        let found = expand(tmp.path(), "User Data/*/Cache");
        assert_eq!(
            found.len(),
            2,
            "profiles without a Cache folder are ignored"
        );
    }

    #[cfg(unix)]
    #[test]
    fn expand_never_follows_links_out_of_the_cache_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("precious/Cache");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.txt"), "data").unwrap();
        fs::create_dir_all(tmp.path().join("User Data")).unwrap();
        std::os::unix::fs::symlink(
            tmp.path().join("precious"),
            tmp.path().join("User Data/Evil"),
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside, tmp.path().join("User Data/Cache")).unwrap();

        assert!(expand(tmp.path(), "User Data/*/Cache").is_empty());
        assert!(expand(tmp.path(), "User Data/Cache").is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn expand_skips_windows_junctions() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("precious");
        fs::create_dir_all(outside.join("Cache")).unwrap();
        fs::create_dir_all(tmp.path().join("User Data")).unwrap();
        let link = tmp.path().join("User Data").join("Evil");
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "mklink /J needs no admin rights");
        assert!(expand(tmp.path(), "User Data/*/Cache").is_empty());
        assert!(crate::guard::check(&link, &[tmp.path().to_path_buf()]).is_err());
    }

    #[test]
    fn clean_keeps_folder_and_removes_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");
        fs::create_dir_all(cache.join("sub")).unwrap();
        fs::write(cache.join("a.bin"), [0u8; 10]).unwrap();
        fs::write(cache.join("sub/b.bin"), [0u8; 5]).unwrap();
        let r = clean_dir(&cache, Duration::ZERO);
        assert_eq!((r.freed, r.removed, r.skipped), (15, 2, 0));
        assert!(cache.is_dir());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    }

    #[test]
    fn fresh_files_survive_min_age() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("new.tmp"), "x").unwrap();
        let r = clean_dir(tmp.path(), Duration::from_secs(3600));
        assert_eq!((r.removed, r.skipped), (0, 1));
        assert!(tmp.path().join("new.tmp").exists());
    }

    #[test]
    fn unknown_ids_clean_nothing() {
        let r = clean(&["../../etc".into()]);
        assert_eq!(r.removed, 0);
    }
}
