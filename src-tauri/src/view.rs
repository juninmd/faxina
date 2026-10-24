use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::path::{Component, Path, PathBuf};

use crate::model::{BigFile, Kind, Node, Suggestion, ViewNode};

const MAX_CHILDREN: usize = 60;
const SUGGEST_MIN: u64 = 50 * 1024 * 1024;
const STALE_BIG_FILE: u64 = 500 * 1024 * 1024;
const STALE_SECS: u64 = 180 * 24 * 3600;

/// Path segments of `target` relative to `root`, or None when it lies outside the scan.
pub fn relative(root: &Path, target: &Path) -> Option<Vec<String>> {
    let rest = target.strip_prefix(root).ok()?;
    rest.components()
        .map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

pub fn find<'a>(tree: &'a Node, segments: &[String]) -> Option<&'a Node> {
    segments.iter().try_fold(tree, |node, seg| node.child(seg))
}

/// Prunes the tree for the UI: bounded depth and child count, tiny nodes dropped.
pub fn build(node: &Node, path: PathBuf, depth: u8, min_size: u64) -> ViewNode {
    let children = if depth == 0 || !node.is_dir {
        Vec::new()
    } else {
        node.children
            .iter()
            .filter(|c| c.size >= min_size && c.size > 0)
            .take(MAX_CHILDREN)
            .map(|c| build(c, path.join(&c.name), depth - 1, min_size))
            .collect()
    };
    ViewNode {
        name: node.name.clone(),
        path: path.to_string_lossy().into_owned(),
        size: node.size,
        files: node.files,
        modified: node.modified,
        kind: node.kind,
        is_dir: node.is_dir,
        grouped: node.grouped,
        reclaimable: node.reclaimable && !node.grouped,
        children,
    }
}

pub fn suggestions(tree: &Node, root: &Path, now: u64, limit: usize) -> Vec<Suggestion> {
    let mut out = Vec::new();
    collect(tree, root.to_path_buf(), now, &mut out, true);
    out.sort_unstable_by_key(|n| std::cmp::Reverse(n.size));
    out.truncate(limit);
    out
}

fn collect(node: &Node, path: PathBuf, now: u64, out: &mut Vec<Suggestion>, is_root: bool) {
    if !is_root && !node.grouped {
        if let Some(reason) = reason_for(node, now) {
            out.push(Suggestion {
                path: path.to_string_lossy().into_owned(),
                name: node.name.clone(),
                size: node.size,
                kind: node.kind,
                reason,
            });
            return;
        }
    }
    for c in node.children.iter().filter(|c| c.size >= SUGGEST_MIN) {
        collect(c, path.join(&c.name), now, out, false);
    }
}

/// Reclaimable build folders (`node_modules`, `target`, `.venv`, `.next`, ...) of projects that
/// have not been touched for `min_secs`, biggest first. A folder's own mtime only says when it was
/// generated, so the age comes from its siblings (the project's sources), ignoring other
/// reclaimable folders. Caches are left to the suggestions: only project output counts here.
pub fn stale_deps(
    tree: &Node,
    root: &Path,
    now: u64,
    min_secs: u64,
    limit: usize,
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    collect_stale(tree, root.to_path_buf(), now, min_secs, &mut out);
    out.sort_unstable_by_key(|n| Reverse(n.size));
    out.truncate(limit);
    out
}

fn collect_stale(node: &Node, path: PathBuf, now: u64, min_secs: u64, out: &mut Vec<Suggestion>) {
    let project_modified = node
        .children
        .iter()
        .filter(|c| !c.reclaimable)
        .map(|c| c.modified)
        .max()
        .unwrap_or(0);
    for c in &node.children {
        if !c.is_dir || c.grouped {
            continue;
        }
        let child_path = path.join(&c.name);
        if c.reclaimable && c.kind == Kind::Build {
            // No siblings with a known mtime: nothing proves the project is idle.
            if c.size >= SUGGEST_MIN && project_modified > 0 {
                let idle = now.saturating_sub(project_modified);
                if idle >= min_secs {
                    out.push(Suggestion {
                        path: child_path.to_string_lossy().into_owned(),
                        name: format!("{}/{}", node.name, c.name),
                        size: c.size,
                        kind: c.kind,
                        reason: format!("projeto parado há {} meses", idle / (30 * 24 * 3600)),
                    });
                }
            }
        } else if !c.reclaimable && c.size >= SUGGEST_MIN {
            collect_stale(c, child_path, now, min_secs, out);
        }
    }
}

fn reason_for(node: &Node, now: u64) -> Option<String> {
    if node.size < SUGGEST_MIN {
        return None;
    }
    if node.is_dir && node.reclaimable {
        let why = match node.name.to_lowercase().as_str() {
            "node_modules" => "dependências reinstaláveis",
            "target" | "dist" | "build" | "out" | "obj" | "bin" => "saída de build",
            ".next" | ".nuxt" | ".turbo" | ".svelte-kit" | ".angular" => "cache de framework",
            _ if node.kind == Kind::Build => "artefato de build",
            _ => "cache regenerável",
        };
        return Some(why.into());
    }
    if !node.is_dir
        && node.size >= STALE_BIG_FILE
        && node.modified > 0
        && now.saturating_sub(node.modified) > STALE_SECS
    {
        let months = now.saturating_sub(node.modified) / (30 * 24 * 3600);
        return Some(format!("arquivo grande parado há {months} meses"));
    }
    None
}

/// Biggest individually tracked files, largest first. Folded "small files" buckets are synthetic
/// and not addressable, so they never appear here. Files rejected by `keep` (OS-owned ones) are
/// dropped before the cut, or a whole-drive scan could spend its whole limit on them.
pub fn biggest_files(
    tree: &Node,
    root: &Path,
    limit: usize,
    keep: impl Fn(&Path) -> bool,
) -> Vec<BigFile> {
    if limit == 0 {
        return Vec::new();
    }
    // Bounded min-heap via Reverse: memory stays O(limit) even on a multi-million-file scan, and
    // the scan mutex is not held while an unbounded vector is built and sorted.
    let mut top: BinaryHeap<Reverse<Found>> = BinaryHeap::with_capacity(limit + 1);
    collect_files(tree, root.to_path_buf(), limit, &keep, &mut top);
    // `into_sorted_vec` is ascending for the heap order, and `Reverse` flips it: largest first.
    top.into_sorted_vec()
        .into_iter()
        .map(|r| r.0.file)
        .collect()
}

struct Found {
    size: u64,
    file: BigFile,
}

impl PartialEq for Found {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
    }
}
impl Eq for Found {}
impl PartialOrd for Found {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Found {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size.cmp(&other.size)
    }
}

fn collect_files(
    node: &Node,
    path: PathBuf,
    limit: usize,
    keep: &dyn Fn(&Path) -> bool,
    top: &mut BinaryHeap<Reverse<Found>>,
) {
    if node.grouped {
        return;
    }
    if !node.is_dir {
        let worth_it = top.len() < limit || node.size > top.peek().map_or(0, |r| r.0.size);
        if worth_it && keep(&path) {
            top.push(Reverse(Found {
                size: node.size,
                file: BigFile {
                    path: path.to_string_lossy().into_owned(),
                    name: node.name.clone(),
                    size: node.size,
                    modified: node.modified,
                    kind: node.kind,
                },
            }));
            if top.len() > limit {
                top.pop();
            }
        }
        return;
    }
    for c in &node.children {
        collect_files(c, path.join(&c.name), limit, keep, top);
    }
}

/// Drops the node at `segments` and subtracts its weight from every ancestor.
pub fn remove(tree: &mut Node, segments: &[String]) -> Option<(u64, u64)> {
    let (first, rest) = segments.split_first()?;
    let idx = tree
        .children
        .iter()
        .position(|c| &c.name == first && !c.grouped)?;
    let removed = if rest.is_empty() {
        let n = tree.children.remove(idx);
        (n.size, n.files)
    } else {
        remove(&mut tree.children[idx], rest)?
    };
    tree.size = tree.size.saturating_sub(removed.0);
    tree.files = tree.files.saturating_sub(removed.1);
    Some(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, kind: Kind, children: Vec<Node>) -> Node {
        let size = children.iter().map(|c| c.size).sum();
        let files = children.iter().map(|c| c.files).sum();
        Node {
            name: name.into(),
            size,
            files,
            modified: 0,
            kind,
            is_dir: true,
            grouped: false,
            reclaimable: kind.reclaimable(),
            children,
        }
    }

    fn sample() -> Node {
        let nm = dir(
            "node_modules",
            Kind::Build,
            vec![Node::file("big.bin".into(), 80 << 20, 0, Kind::Build)],
        );
        let web = dir(
            "web",
            Kind::Other,
            vec![nm, Node::file("a.ts".into(), 1 << 20, 0, Kind::Code)],
        );
        dir("/root", Kind::Other, vec![web])
    }

    #[test]
    fn remove_updates_ancestors() {
        let mut t = sample();
        let before = t.size;
        let (freed, files) = remove(&mut t, &["web".into(), "node_modules".into()]).unwrap();
        assert_eq!(freed, 80 << 20);
        assert_eq!(files, 1);
        assert_eq!(t.size, before - freed);
        assert_eq!(t.children[0].size, 1 << 20);
        assert!(remove(&mut t, &["web".into(), "nope".into()]).is_none());
    }

    #[test]
    fn suggestions_stop_at_reclaimable_root() {
        let t = sample();
        let s = suggestions(&t, Path::new("/root"), 0, 10);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "node_modules");
        assert_eq!(s[0].reason, "dependências reinstaláveis");
    }

    #[test]
    fn stale_deps_use_project_sources_age_not_install_time() {
        const DAY: u64 = 24 * 3600;
        let now = 1000 * DAY;
        let project = |name: &str, src_age_days: u64, nm_age_days: u64| {
            let mut nm = dir(
                "node_modules",
                Kind::Build,
                vec![Node::file("big.bin".into(), 80 << 20, 0, Kind::Build)],
            );
            nm.modified = now - nm_age_days * DAY;
            let src = Node::file("a.ts".into(), 1 << 20, now - src_age_days * DAY, Kind::Code);
            dir(name, Kind::Other, vec![nm, src])
        };
        // "old" installed yesterday but nobody touched the sources for a year.
        let root = dir(
            "/root",
            Kind::Other,
            vec![project("old", 400, 1), project("fresh", 5, 5)],
        );

        let s = stale_deps(&root, Path::new("/root"), now, 180 * DAY, 10);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "old/node_modules");
        assert_eq!(s[0].reason, "projeto parado há 13 meses");
        assert!(Path::new(&s[0].path).ends_with("old/node_modules"));
    }

    #[test]
    fn stale_projects_list_every_build_folder_but_not_caches() {
        const DAY: u64 = 24 * 3600;
        let now = 1000 * DAY;
        let big = |kind| vec![Node::file("big.bin".into(), 80 << 20, 0, kind)];
        let src = Node::file("main.rs".into(), 1 << 20, now - 400 * DAY, Kind::Code);
        let project = dir(
            "old",
            Kind::Other,
            vec![
                dir("target", Kind::Build, big(Kind::Build)),
                dir(".venv", Kind::Build, big(Kind::Build)),
                dir(".cache", Kind::Cache, big(Kind::Cache)),
                src,
            ],
        );
        let root = dir("/root", Kind::Other, vec![project]);

        let s = stale_deps(&root, Path::new("/root"), now, 180 * DAY, 10);
        let mut names: Vec<_> = s.iter().map(|x| x.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["old/.venv", "old/target"]);
    }

    #[test]
    fn folder_colored_as_build_is_never_suggested_whole() {
        // "Projetos" is mostly target/, so it is painted Build, but it holds source code.
        let target = dir(
            "target",
            Kind::Build,
            vec![Node::file("big".into(), 900 << 20, 0, Kind::Build)],
        );
        let src = Node::file("main.rs".into(), 1 << 20, 0, Kind::Code);
        let mut projetos = dir("Projetos", Kind::Other, vec![target, src]);
        projetos.kind = Kind::Build;
        let root = dir("/root", Kind::Other, vec![projetos]);

        let s = suggestions(&root, Path::new("/root"), 0, 10);
        assert_eq!(
            s.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            ["target"]
        );
        let v = build(&root, PathBuf::from("/root"), 2, 0);
        assert!(!v.children[0].reclaimable);
    }

    #[test]
    fn relative_rejects_outside_paths() {
        let root = Path::new("/home/u");
        assert_eq!(
            relative(root, Path::new("/home/u/a/b")),
            Some(vec!["a".into(), "b".into()])
        );
        assert_eq!(relative(root, Path::new("/etc")), None);
    }

    #[test]
    fn biggest_files_skips_grouped_buckets_and_sorts_by_size() {
        let mut t = sample();
        t.children
            .push(Node::file("huge.iso".into(), 900 << 20, 0, Kind::Archives));
        let mut grouped = Node::file("3 arquivos pequenos".into(), 300 << 20, 0, Kind::Other);
        grouped.grouped = true;
        t.children.push(grouped);

        let found = biggest_files(&t, Path::new("/root"), 10, |_| true);
        assert_eq!(
            found.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["huge.iso", "big.bin", "a.ts"],
            "the grouped bucket is not addressable and never shows up"
        );
        assert_eq!(
            found[0].path,
            Path::new("/root").join("huge.iso").to_string_lossy()
        );
        assert_eq!(biggest_files(&t, Path::new("/root"), 1, |_| true).len(), 1);
        assert!(biggest_files(&t, Path::new("/root"), 0, |_| true).is_empty());
    }

    #[test]
    fn biggest_files_filters_before_the_cut() {
        // Regression: filtering after `truncate` emptied the list when the biggest entries were
        // OS-owned (a whole-drive scan), because the limit was spent on paths then discarded.
        let mut t = sample();
        t.children
            .push(Node::file("pagefile.sys".into(), 900 << 20, 0, Kind::Other));
        t.children
            .push(Node::file("movie.mkv".into(), 700 << 20, 0, Kind::Media));

        let found = biggest_files(&t, Path::new("/root"), 2, |p| !p.ends_with("pagefile.sys"));
        assert_eq!(
            found.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["movie.mkv", "big.bin"]
        );
    }

    #[test]
    fn build_respects_depth() {
        let v = build(&sample(), PathBuf::from("/root"), 1, 0);
        assert_eq!(v.children.len(), 1);
        assert!(v.children[0].children.is_empty());
    }
}
