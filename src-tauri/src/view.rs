use std::path::{Component, Path, PathBuf};

use crate::model::{Kind, Node, Suggestion, ViewNode};

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
    fn build_respects_depth() {
        let v = build(&sample(), PathBuf::from("/root"), 1, 0);
        assert_eq!(v.children.len(), 1);
        assert!(v.children[0].children.is_empty());
    }
}
