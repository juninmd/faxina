use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rayon::prelude::*;
use serde::Serialize;

use crate::junk_defs::{definitions, Base, JunkDef, Rule, Scope};
use crate::junk_projects::Projects;
use crate::listing;
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
    pub caution: Option<&'static str>,
}

#[derive(Default)]
pub struct Snapshot {
    targets: Vec<Target>,
}

struct Target {
    id: &'static str,
    path: PathBuf,
    canonical: PathBuf,
    rule: Rule,
    min_age: Duration,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub freed: u64,
    pub removed: u64,
    /// Entries left alone because they are younger than the rule's minimum age.
    pub skipped: u64,
    /// Entries that could not be removed at all (permissions, bin quota, in use).
    pub failed: u64,
    /// First failure, so the UI can explain a zero-freed clean instead of guessing "in use".
    pub reason: Option<String>,
}

pub fn base_dir(base: Base) -> Option<PathBuf> {
    match base {
        Base::Home => dirs::home_dir(),
        Base::Local => dirs::data_local_dir(),
        Base::Roaming => dirs::data_dir(),
        Base::Cache => dirs::cache_dir(),
        Base::Temp => Some(std::env::temp_dir()),
        Base::Config => dirs::config_dir(),
        Base::NodeProjects | Base::RustProjects | Base::PythonProjects => None,
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
fn resolve(def: &JunkDef, rule: &Rule, projects: &Projects) -> Vec<PathBuf> {
    let mut out = if let Some(base) = base_dir(def.base) {
        expand(&base, rule.path)
    } else {
        let protection = crate::guard::Protection::new();
        projects
            .roots(def.base)
            .iter()
            .flat_map(|base| expand(base, rule.path))
            .filter(|p| !protection.is_protected(p))
            .collect()
    };
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

/// Size and file count of what a rule would remove from `dir`, before touching anything.
pub fn measure(dir: &Path, rule: &Rule) -> (u64, u64) {
    measure_tree(dir, rule, cfg!(windows), 0)
}

fn measure_tree(dir: &Path, rule: &Rule, ci: bool, depth: usize) -> (u64, u64) {
    if depth > MAX_DEPTH {
        return (0, 0);
    }
    // Use the same bulk Windows directory reader as the disk map; unsupported filesystems
    // fall back to the portable reader. Its entries already exclude links and reparse points.
    let Ok(entries) = listing::list(dir, cfg!(windows)) else {
        return (0, 0);
    };
    // Large dependency trees need directory-level parallelism too, rather than leaving a
    // single worker to measure millions of entries while the other rule workers are idle.
    entries
        .into_par_iter()
        .map(|entry| {
            if excluded(rule, &entry.name, ci) {
                return (0, 0);
            }
            if entry.is_dir {
                if rule.scope == Scope::Contents || rule.recurse {
                    return measure_tree(&dir.join(&entry.name), rule, ci, depth + 1);
                }
            } else if rule.scope == Scope::Files {
                if rule.patterns.iter().any(|p| glob_match(p, &entry.name, ci)) {
                    return (entry.size, 1);
                }
            } else {
                return (entry.size, 1);
            }
            (0, 0)
        })
        .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
}

pub fn scan_all() -> (Vec<JunkItem>, Snapshot) {
    #[cfg(test)]
    let started = std::time::Instant::now();
    let projects = Projects::discover();
    #[cfg(test)]
    eprintln!(
        "Mounted-volume project discovery completed in {:?}",
        started.elapsed()
    );
    scan_snapshot(&definitions(), &projects)
}

#[cfg(test)]
fn scan_definitions(defs: &[JunkDef], projects: &Projects) -> Vec<JunkItem> {
    scan_snapshot(defs, projects).0
}

fn scan_snapshot(defs: &[JunkDef], projects: &Projects) -> (Vec<JunkItem>, Snapshot) {
    let results: Vec<_> = defs
        .par_iter()
        .filter_map(|def| {
            let mut paths = Vec::new();
            for rule in &def.rules {
                for p in resolve(def, rule, projects) {
                    if let Ok(canonical) = fs::canonicalize(&p) {
                        paths.push(Target {
                            id: def.id,
                            path: p,
                            canonical,
                            rule: *rule,
                            min_age: def.min_age,
                        });
                    }
                }
            }
            let (size, files) = paths
                .par_iter()
                .map(|target| measure(&target.path, &target.rule))
                .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
            let mut displayed: Vec<_> = paths
                .iter()
                .map(|t| t.path.to_string_lossy().into_owned())
                .collect();
            displayed.sort();
            displayed.dedup();
            (size > 0).then_some({
                (
                    JunkItem {
                        id: def.id,
                        label: def.label,
                        group: def.group,
                        size,
                        files,
                        paths: displayed,
                        caution: def.caution,
                    },
                    paths,
                )
            })
        })
        .collect();
    let (mut items, targets): (Vec<_>, Vec<_>) = results.into_iter().unzip();
    items.sort_unstable_by_key(|n| std::cmp::Reverse(n.size));
    (
        items,
        Snapshot {
            targets: targets.into_iter().flatten().collect(),
        },
    )
}

/// Deletes what a rule points at inside `dir`; the folder itself stays so apps keep working.
/// `permanent` false sends every entry to the OS Recycle Bin instead.
pub fn clean_dir(dir: &Path, rule: &Rule, min_age: Duration, permanent: bool) -> CleanReport {
    clean_tree(dir, rule, min_age, cfg!(windows), permanent, 0)
}

fn failed(reason: &str) -> CleanReport {
    CleanReport {
        failed: 1,
        reason: Some(reason.to_string()),
        ..CleanReport::default()
    }
}

fn clean_tree(
    dir: &Path,
    rule: &Rule,
    min_age: Duration,
    ci: bool,
    permanent: bool,
    depth: usize,
) -> CleanReport {
    if depth > MAX_DEPTH {
        return CleanReport::default();
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return CleanReport::default();
    };

    // Plan first, remove second: the trash is one shell operation per directory, not per entry
    // (each `trash::delete` costs ~20 ms on Windows and froze the cleaner on real temp folders).
    let mut report = CleanReport::default();
    let mut whole: Vec<PathBuf> = Vec::new();
    let mut files: Vec<(PathBuf, u64)> = Vec::new();
    let mut recurse: Vec<PathBuf> = Vec::new();
    for (path, meta) in rd
        .flatten()
        .filter_map(|e| e.metadata().ok().map(|m| (e.path(), m)))
    {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if excluded(rule, &name, ci) {
            continue;
        }
        let is_tree = meta.is_dir() && !is_link(&meta);
        if is_tree {
            match rule.scope {
                Scope::Contents if rule.excludes.is_empty() => {
                    if old_enough(&meta, min_age) {
                        whole.push(path);
                    } else {
                        report.skipped += 1;
                    }
                }
                Scope::Contents | Scope::Files if rule.recurse => recurse.push(path),
                _ => {}
            }
        } else {
            let matched = rule.scope == Scope::Contents
                || rule.patterns.iter().any(|p| glob_match(p, &name, ci));
            if !matched {
                continue;
            }
            if old_enough(&meta, min_age) {
                files.push((path, meta.len()));
            } else {
                report.skipped += 1;
            }
        }
    }

    // Measure before removing; whole subtrees are measured in parallel.
    let mut targets: Vec<(PathBuf, u64, u64)> = whole
        .into_par_iter()
        .map(|p| {
            let (size, count) = measure_tree(&p, rule, ci, depth + 1);
            (p, size, count)
        })
        .collect();
    targets.extend(files.into_iter().map(|(p, size)| (p, size, 1)));
    report = merge(report, remove_entries(&targets, permanent));

    let nested: CleanReport = recurse
        .into_par_iter()
        .map(|p| clean_tree(&p, rule, min_age, ci, permanent, depth + 1))
        .reduce(CleanReport::default, merge);
    merge(report, nested)
}

/// Removes one directory's worth of entries, already measured: one shell batch for the trash, or
/// parallel permanent deletes.
fn remove_entries(items: &[(PathBuf, u64, u64)], permanent: bool) -> CleanReport {
    if items.is_empty() {
        return CleanReport::default();
    }
    if permanent {
        return items
            .par_iter()
            .map(|(p, size, files)| {
                match crate::remove::remove_permanent(p).map_err(|e| e.to_string()) {
                    Ok(()) => CleanReport {
                        freed: *size,
                        removed: *files,
                        ..CleanReport::default()
                    },
                    Err(reason) => failed(&reason),
                }
            })
            .reduce(CleanReport::default, merge);
    }
    let paths: Vec<PathBuf> = items.iter().map(|(p, _, _)| p.clone()).collect();
    let results = crate::remove::trash_paths(&paths);
    items
        .iter()
        .zip(results)
        .map(|((_, size, files), result)| match result {
            Ok(()) => CleanReport {
                freed: *size,
                removed: *files,
                ..CleanReport::default()
            },
            Err(reason) => failed(&reason),
        })
        .fold(CleanReport::default(), merge)
}

fn merge(a: CleanReport, b: CleanReport) -> CleanReport {
    CleanReport {
        freed: a.freed + b.freed,
        removed: a.removed + b.removed,
        skipped: a.skipped + b.skipped,
        failed: a.failed + b.failed,
        reason: a.reason.or(b.reason),
    }
}

pub fn clean(snapshot: Snapshot, ids: &[String], permanent: bool) -> CleanReport {
    snapshot
        .targets
        .into_iter()
        .filter(|t| ids.iter().any(|id| id == t.id))
        .map(|target| {
            if !real_dir(&target.path)
                || fs::canonicalize(&target.path).ok().as_ref() != Some(&target.canonical)
            {
                return failed("O local analisado mudou. Execute uma nova análise.");
            }
            let Some(parent) = target.canonical.parent() else {
                return failed("caminho inválido");
            };
            match crate::guard::check(&target.path, &[parent.to_path_buf()]) {
                Ok(path) => clean_dir(&path, &target.rule, target.min_age, permanent),
                Err(reason) => failed(&reason),
            }
        })
        .fold(CleanReport::default(), merge)
}

#[cfg(test)]
fn clean_with_projects(
    defs: &[JunkDef],
    ids: &[String],
    permanent: bool,
    projects: &Projects,
) -> CleanReport {
    clean(scan_snapshot(defs, projects).1, ids, permanent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junk_defs::{matching, whole};

    #[test]
    fn cleanup_uses_reviewed_locations_not_newly_created_projects() {
        let volume = tempfile::tempdir().unwrap();
        let add_project = |name: &str| {
            let project = volume.path().join(name);
            fs::create_dir_all(project.join(".next/cache")).unwrap();
            fs::write(project.join("package.json"), "{}").unwrap();
            fs::write(project.join(".next/cache/data"), "data").unwrap();
            project
        };
        let reviewed = add_project("reviewed");
        let defs: Vec<_> = definitions()
            .into_iter()
            .filter(|d| d.id == "project-node-caches")
            .collect();
        let projects = Projects::from_roots(&[volume.path().into()]);
        let (items, snapshot) = scan_snapshot(&defs, &projects);
        assert_eq!(items[0].paths.len(), 1);
        let unreviewed = add_project("new-project");
        let result = clean(snapshot, &["project-node-caches".into()], true);
        assert_eq!((result.removed, result.failed), (1, 0));
        assert!(!reviewed.join(".next/cache/data").exists());
        assert!(unreviewed.join(".next/cache/data").exists());
    }

    /// Manual smoke for the real mounted-volume discovery; never deletes anything.
    #[test]
    #[ignore = "reads all mounted volumes; run explicitly with --ignored --nocapture"]
    fn mounted_volume_junk_scan_read_only() {
        let disks = crate::disks::list_disks();
        assert!(!disks.is_empty());
        let (items, _) = scan_all();
        for disk in disks {
            let locations = items
                .iter()
                .flat_map(|i| &i.paths)
                .filter(|p| Path::new(p).starts_with(&disk.mount))
                .count();
            println!("mount={} cleanup_locations={locations}", disk.mount);
        }
        for item in items.iter().filter(|i| i.id.starts_with("project-")) {
            println!(
                "{}: bytes={} files={} locations={}",
                item.id,
                item.size,
                item.files,
                item.paths.len()
            );
        }
    }

    #[test]
    fn reviews_and_cleans_caches_on_both_volumes_preserving_sources_and_unselected_deps() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for root in [first.path(), second.path()] {
            for path in [
                "app/.next/cache",
                "app/node_modules",
                "personal/cache",
                "personal/target",
            ] {
                fs::create_dir_all(root.join(path)).unwrap();
                fs::write(root.join(path).join("keep-or-clean"), "data").unwrap();
            }
            fs::write(root.join("app/package.json"), "{}").unwrap();
            fs::write(root.join("app/source.ts"), "source").unwrap();
        }
        let projects = Projects::from_roots(&[first.path().into(), second.path().into()]);
        let defs: Vec<_> = definitions()
            .into_iter()
            .filter(|d| {
                matches!(
                    d.base,
                    Base::NodeProjects | Base::RustProjects | Base::PythonProjects
                )
            })
            .collect();
        let items = scan_definitions(&defs, &projects);
        let caches = items
            .iter()
            .find(|i| i.id == "project-node-caches")
            .unwrap();
        assert_eq!((caches.size, caches.files, caches.paths.len()), (8, 2, 2));
        let deps = items.iter().find(|i| i.id == "project-node-deps").unwrap();
        assert!(deps.caution.is_some());
        assert_eq!(deps.paths.len(), 2);
        let report = clean_with_projects(&defs, &["project-node-caches".into()], true, &projects);
        assert_eq!((report.freed, report.removed, report.failed), (8, 2, 0));
        for root in [first.path(), second.path()] {
            assert!(!root.join("app/.next/cache/keep-or-clean").exists());
            for path in [
                "app/source.ts",
                "app/package.json",
                "app/node_modules/keep-or-clean",
                "personal/cache/keep-or-clean",
                "personal/target/keep-or-clean",
            ] {
                assert!(root.join(path).is_file(), "{path}");
            }
        }
        let after = scan_definitions(&defs, &projects);
        assert!(!after.iter().any(|i| i.id == "project-node-caches"));
    }

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

    #[cfg(windows)]
    #[test]
    fn dir_size_does_not_count_through_junctions() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("precious");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("big.bin"), [0u8; 1000]).unwrap();
        let cache = tmp.path().join("cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("a.bin"), [0u8; 10]).unwrap();
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(cache.join("Evil"))
            .arg(&outside)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "mklink /J needs no admin rights");
        assert_eq!(dir_size(&cache).0, 10);
        assert_eq!(measure(&cache, &whole("")), (10, 1));
    }

    #[test]
    fn clean_keeps_folder_and_removes_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");
        fs::create_dir_all(cache.join("sub")).unwrap();
        fs::write(cache.join("a.bin"), [0u8; 10]).unwrap();
        fs::write(cache.join("sub/b.bin"), [0u8; 5]).unwrap();
        let r = clean_dir(&cache, &whole(""), Duration::ZERO, true);
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
        let r = clean_dir(tmp.path(), &non_recursive, Duration::ZERO, true);
        assert_eq!((r.freed, r.removed), (4, 1));
        assert!(tmp.path().join("keep.db").exists());
        assert!(tmp.path().join("sub/nested.log").exists(), "no recursion");

        let recursive = matching("", &["*.log"], true);
        let r = clean_dir(tmp.path(), &recursive, Duration::ZERO, true);
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
        let r = clean_dir(tmp.path(), &rule, Duration::ZERO, true);
        assert_eq!((r.freed, r.removed), (2, 1));
        assert!(tmp.path().join("keep.tmp").exists());
        assert!(!tmp.path().join("remove.tmp").exists());
    }

    #[test]
    fn fresh_files_survive_min_age() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("new.tmp"), "x").unwrap();
        let r = clean_dir(tmp.path(), &whole(""), Duration::from_secs(3600), true);
        assert_eq!((r.removed, r.skipped), (0, 1));
        assert!(tmp.path().join("new.tmp").exists());
    }

    #[test]
    fn unknown_ids_clean_nothing() {
        let r = clean(Snapshot::default(), &["../../etc".into()], true);
        assert_eq!(r.removed, 0);
    }

    #[cfg(target_os = "linux")]
    struct EnvGuard {
        previous: Option<std::ffi::OsString>,
    }

    /// Restores `XDG_DATA_HOME` even when the test panics. `set_var` is process-global, so the
    /// window is kept as short as possible and no other test reads this variable.
    #[cfg(target_os = "linux")]
    impl EnvGuard {
        fn set(value: &Path) -> Self {
            let previous = std::env::var_os("XDG_DATA_HOME");
            std::env::set_var("XDG_DATA_HOME", value);
            Self { previous }
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn trash_mode_sends_entries_to_the_bin_and_keeps_the_folder() {
        // Isolation: on Linux the `trash` crate puts files under $XDG_DATA_HOME/Trash, so a
        // temporary data home keeps the test from touching the real Recycle Bin. Both the cache
        // and the bin live in the same temp dir, which `trash` requires (same filesystem).
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        fs::create_dir_all(&data).unwrap();
        let cache = tmp.path().join("cache");
        fs::create_dir_all(cache.join("sub")).unwrap();
        fs::write(cache.join("a.bin"), [0u8; 10]).unwrap();
        fs::write(cache.join("sub/b.bin"), [0u8; 5]).unwrap();

        let _env = EnvGuard::set(&data);
        let r = clean_dir(&cache, &whole(""), Duration::ZERO, false);

        assert!(cache.is_dir(), "the cache folder itself must survive");
        assert_eq!(
            (r.freed, r.removed, r.skipped, r.failed),
            (15, 2, 0, 0),
            "both entries left: {r:?}"
        );
        assert_eq!(cache.read_dir().unwrap().count(), 0);
        let bin = data.join("Trash").join("files");
        assert_eq!(
            fs::read_dir(&bin).unwrap().count(),
            2,
            "both entries really landed in the bin, not deleted for good"
        );
    }

    #[test]
    fn permanent_mode_does_not_touch_the_bin() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.tmp"), b"xx").unwrap();
        let r = clean_dir(tmp.path(), &whole(""), Duration::ZERO, true);
        assert_eq!((r.freed, r.removed, r.skipped), (2, 1, 0));
        assert_eq!(tmp.path().read_dir().unwrap().count(), 0);
    }
}
