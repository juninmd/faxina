use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rayon::prelude::*;
use serde::Serialize;

use crate::junk_defs::{definitions, Base, JunkDef, Rule, Scope};
use crate::scan::is_link;

/// How deep we may descend into one cache tree. Guards against pathological nesting and links
/// that somehow survive `real_dir`.
const MAX_DEPTH: usize = 128;

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

#[derive(Clone, Debug, Default, Serialize)]
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

/// Matches one path segment or file name against a glob with `*` and `?`.
/// Winapp2-style partial globs (`Chrome*`, `*Cache*`) are the whole point.
fn glob_match(pattern: &str, name: &str, case_insensitive: bool) -> bool {
    let fold = |s: &str| -> Vec<char> {
        if case_insensitive {
            s.chars().flat_map(char::to_lowercase).collect()
        } else {
            s.chars().collect()
        }
    };
    let pat = fold(pattern);
    let txt = fold(name);
    let (mut p, mut t) = (0usize, 0usize);
    let (mut star, mut mark) = (usize::MAX, 0usize);
    while t < txt.len() {
        if p < pat.len() && (pat[p] == '?' || pat[p] == txt[t]) {
            p += 1;
            t += 1;
        } else if p < pat.len() && pat[p] == '*' {
            star = p;
            p += 1;
            mark = t;
        } else if star != usize::MAX {
            p = star + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < pat.len() && pat[p] == '*' {
        p += 1;
    }
    p == pat.len()
}

fn excluded(rule: &Rule, name: &str, ci: bool) -> bool {
    rule.excludes.iter().any(|e| glob_match(e, name, ci))
}

/// Expands `rel` segments (globs included, e.g. `Chrome*/User Data/*Cache*`) into existing dirs.
pub fn expand(base: &Path, rel: &str) -> Vec<PathBuf> {
    if !real_dir(base) {
        return Vec::new();
    }
    let ci = cfg!(windows);
    let mut current = vec![base.to_path_buf()];
    for seg in rel.split('/').filter(|s| !s.is_empty()) {
        current = current
            .into_iter()
            .flat_map(|dir| -> Vec<PathBuf> {
                if seg.contains(['*', '?']) {
                    fs::read_dir(&dir)
                        .map(|rd| {
                            rd.flatten()
                                .map(|e| e.path())
                                .filter(|p| {
                                    let name = p
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_default();
                                    glob_match(seg, &name, ci) && real_dir(p)
                                })
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

/// Directories one rule applies to, deduplicated.
pub fn resolve(def: &JunkDef, rule: &Rule) -> Vec<PathBuf> {
    let Some(base) = base_dir(def.base) else {
        return Vec::new();
    };
    let mut out = expand(&base, rule.path);
    out.sort();
    out.dedup();
    out
}

pub fn dir_size(path: &Path) -> (u64, u64) {
    let Ok(rd) = fs::read_dir(path) else {
        return (0, 0);
    };
    rd.flatten()
        .filter_map(|e| fs::symlink_metadata(e.path()).ok().map(|m| (e.path(), m)))
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

/// Size and file count of what a rule would remove from `dir`, before touching anything.
pub fn measure(dir: &Path, rule: &Rule) -> (u64, u64) {
    measure_tree(dir, rule, cfg!(windows), 0)
}

fn measure_tree(dir: &Path, rule: &Rule, ci: bool, depth: usize) -> (u64, u64) {
    if depth > MAX_DEPTH {
        return (0, 0);
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return (0, 0);
    };
    let (mut size, mut files) = (0u64, 0u64);
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if excluded(rule, &name, ci) {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        let is_tree = meta.is_dir() && !is_link(&meta);
        if is_tree {
            if rule.scope == Scope::Contents || rule.recurse {
                let (s, f) = measure_tree(&entry.path(), rule, ci, depth + 1);
                size += s;
                files += f;
            }
        } else if rule.scope == Scope::Files {
            if rule.patterns.iter().any(|p| glob_match(p, &name, ci)) {
                size += meta.len();
                files += 1;
            }
        } else {
            size += meta.len();
            files += 1;
        }
    }
    (size, files)
}

pub fn scan_all() -> Vec<JunkItem> {
    let mut items: Vec<JunkItem> = definitions()
        .par_iter()
        .filter_map(|def| {
            let mut paths = Vec::new();
            let (mut size, mut files) = (0u64, 0u64);
            for rule in &def.rules {
                for p in resolve(def, rule) {
                    let (s, f) = measure(&p, rule);
                    size += s;
                    files += f;
                    paths.push(p);
                }
            }
            paths.sort();
            paths.dedup();
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

/// Deletes what a rule points at inside `dir`; the folder itself stays so apps keep working.
pub fn clean_dir(dir: &Path, rule: &Rule, min_age: Duration) -> CleanReport {
    clean_tree(dir, rule, min_age, cfg!(windows), 0)
}

fn skipped() -> CleanReport {
    CleanReport {
        skipped: 1,
        ..CleanReport::default()
    }
}

fn clean_tree(dir: &Path, rule: &Rule, min_age: Duration, ci: bool, depth: usize) -> CleanReport {
    if depth > MAX_DEPTH {
        return CleanReport::default();
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return CleanReport::default();
    };
    let entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries
        .par_iter()
        .filter_map(|path| fs::symlink_metadata(path).ok().map(|m| (path, m)))
        .map(|(path, meta)| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if excluded(rule, &name, ci) {
                return CleanReport::default();
            }
            let is_tree = meta.is_dir() && !is_link(&meta);
            if is_tree {
                match rule.scope {
                    Scope::Contents if rule.excludes.is_empty() => {
                        if !old_enough(&meta, min_age) {
                            return skipped();
                        }
                        let (size, files) = measure_tree(path, rule, ci, depth + 1);
                        match crate::remove::remove_tree(path) {
                            Ok(()) => CleanReport {
                                freed: size,
                                removed: files,
                                skipped: 0,
                            },
                            Err(_) => skipped(),
                        }
                    }
                    Scope::Contents | Scope::Files if rule.recurse => {
                        clean_tree(path, rule, min_age, ci, depth + 1)
                    }
                    _ => CleanReport::default(),
                }
            } else {
                let matched = rule.scope == Scope::Contents
                    || rule.patterns.iter().any(|p| glob_match(p, &name, ci));
                if !matched {
                    return CleanReport::default();
                }
                if !old_enough(&meta, min_age) {
                    return skipped();
                }
                let size = meta.len();
                match fs::remove_file(path) {
                    Ok(()) => CleanReport {
                        freed: size,
                        removed: 1,
                        skipped: 0,
                    },
                    Err(_) => skipped(),
                }
            }
        })
        .reduce(CleanReport::default, merge)
}

fn merge(a: CleanReport, b: CleanReport) -> CleanReport {
    CleanReport {
        freed: a.freed + b.freed,
        removed: a.removed + b.removed,
        skipped: a.skipped + b.skipped,
    }
}

pub fn clean(ids: &[String]) -> CleanReport {
    definitions()
        .iter()
        .filter(|d| ids.iter().any(|id| id == d.id))
        .flat_map(|d| d.rules.iter().map(move |r| (d, r)))
        .flat_map(|(d, r)| resolve(d, r).into_iter().map(move |p| (p, r, d.min_age)))
        .map(|(p, rule, age)| clean_dir(&p, rule, age))
        .fold(CleanReport::default(), merge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junk_defs::{matching, whole};

    #[test]
    fn glob_matches_partial_segments() {
        assert!(glob_match("Chrome*", "Chrome Beta", false));
        assert!(glob_match("*Cache*", "GrShaderCache", false));
        assert!(glob_match("*cache*", "cache2", false));
        assert!(glob_match("*Cache*", "Cache_Data", false));
        assert!(!glob_match("*Cache*", "Network", false));
        assert!(glob_match("Profile ?", "Profile 1", false));
        assert!(glob_match("Cache", "Cache", false));
        assert!(!glob_match("Cache", "Cache2", false));
        assert!(glob_match("Chrome*", "chrome beta", true));
        assert!(!glob_match("Chrome*", "chrome beta", false));
    }

    #[test]
    fn expand_matches_partial_globs_and_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        for p in ["Chrome", "Chrome Beta"] {
            fs::create_dir_all(tmp.path().join(p).join("User Data/Default/Cache")).unwrap();
        }
        fs::create_dir_all(tmp.path().join("Chrome/User Data/GrShaderCache")).unwrap();
        let found = expand(tmp.path(), "Chrome*/User Data/*Cache*");
        let names: Vec<_> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["GrShaderCache"]);
        let nested = expand(tmp.path(), "Chrome*/User Data/*/*Cache*");
        assert_eq!(nested.len(), 2, "Default/Cache under each channel");
    }

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
        let r = clean_dir(&cache, &whole(""), Duration::ZERO);
        assert_eq!((r.freed, r.removed, r.skipped), (15, 2, 0));
        assert!(cache.is_dir());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    }

    #[test]
    fn files_rule_deletes_only_matching_names() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("keep.db"), b"keep").unwrap();
        fs::write(tmp.path().join("old.log"), b"log!").unwrap();
        fs::write(tmp.path().join("sub/nested.log"), b"deep").unwrap();

        let non_recursive = matching("", &["*.log"], false);
        let r = clean_dir(tmp.path(), &non_recursive, Duration::ZERO);
        assert_eq!((r.freed, r.removed), (4, 1));
        assert!(tmp.path().join("keep.db").exists());
        assert!(tmp.path().join("sub/nested.log").exists(), "no recursion");

        let recursive = matching("", &["*.log"], true);
        let r = clean_dir(tmp.path(), &recursive, Duration::ZERO);
        assert_eq!((r.freed, r.removed), (4, 1));
        assert!(tmp.path().join("keep.db").exists());
    }

    #[test]
    fn excludes_preserve_entries() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("remove.tmp"), b"xx").unwrap();
        fs::write(tmp.path().join("keep.tmp"), b"yy").unwrap();
        let rule = crate::junk_defs::excluding(whole(""), &["keep.tmp"]);
        let (size, files) = measure(tmp.path(), &rule);
        assert_eq!((size, files), (2, 1));
        let r = clean_dir(tmp.path(), &rule, Duration::ZERO);
        assert_eq!((r.freed, r.removed), (2, 1));
        assert!(tmp.path().join("keep.tmp").exists());
        assert!(!tmp.path().join("remove.tmp").exists());
    }

    #[test]
    fn fresh_files_survive_min_age() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("new.tmp"), "x").unwrap();
        let r = clean_dir(tmp.path(), &whole(""), Duration::from_secs(3600));
        assert_eq!((r.removed, r.skipped), (0, 1));
        assert!(tmp.path().join("new.tmp").exists());
    }

    #[test]
    fn unknown_ids_clean_nothing() {
        let r = clean(&["../../etc".into()]);
        assert_eq!(r.removed, 0);
    }
}
