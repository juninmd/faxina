use std::fs::{self, Metadata};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use rayon::prelude::*;

use crate::category::{classify_dir, classify_file, hints_for};
use crate::model::{Kind, Node};

/// Files below this size are folded into one "small files" bucket per directory,
/// which keeps a multi-million-file scan within a few hundred MB of RAM.
pub const SMALL_FILE: u64 = 512 * 1024;

#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub cancel: AtomicBool,
}

pub fn scan(root: &Path, progress: &Progress) -> Node {
    let name = root.to_string_lossy().into_owned();
    let mut node = scan_dir(root, name, Kind::Other, progress);
    node.kind = dominant_kind(&node.children);
    node
}

pub fn mtime(meta: &Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

/// Symlinks and Windows reparse points (junctions, OneDrive placeholders) are skipped so
/// nothing is counted twice and a scan never escapes the chosen root.
pub fn is_link(meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

fn scan_dir(path: &Path, name: String, forced: Kind, progress: &Progress) -> Node {
    let mut node = Node {
        name,
        size: 0,
        files: 0,
        modified: 0,
        kind: forced,
        is_dir: true,
        grouped: false,
        reclaimable: forced.reclaimable(),
        children: Vec::new(),
    };
    if progress.cancel.load(Ordering::Relaxed) {
        return node;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return node;
    };

    let mut dirs = Vec::new();
    let mut file_names = Vec::new();
    let (mut small_size, mut small_count, mut small_mtime) = (0u64, 0u64, 0u64);

    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if is_link(&meta) {
            continue;
        }
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        if meta.is_dir() {
            dirs.push((entry.path(), entry_name));
            continue;
        }
        let size = meta.len();
        let modified = mtime(&meta);
        progress.files.fetch_add(1, Ordering::Relaxed);
        progress.bytes.fetch_add(size, Ordering::Relaxed);
        if size >= SMALL_FILE {
            let kind = if forced == Kind::Other {
                classify_file(&entry_name)
            } else {
                forced
            };
            node.children
                .push(Node::file(entry_name.clone(), size, modified, kind));
        } else {
            small_size += size;
            small_count += 1;
            small_mtime = small_mtime.max(modified);
        }
        file_names.push(entry_name);
    }

    let hints = hints_for(&file_names);
    let subdirs: Vec<Node> = dirs
        .into_par_iter()
        .map(|(p, n)| {
            let special = classify_dir(&n, hints);
            let inherited = if forced == Kind::Other {
                special.unwrap_or(Kind::Other)
            } else {
                forced
            };
            let mut child = scan_dir(&p, n, inherited, progress);
            if inherited == Kind::Other {
                child.kind = dominant_kind(&child.children);
            }
            child
        })
        .collect();
    node.children.extend(subdirs);

    if small_count > 0 {
        node.children.push(Node {
            name: format!("{small_count} arquivos pequenos"),
            size: small_size,
            files: small_count,
            modified: small_mtime,
            kind: if forced == Kind::Other {
                Kind::Other
            } else {
                forced
            },
            is_dir: false,
            grouped: true,
            reclaimable: forced.reclaimable(),
            children: Vec::new(),
        });
    }

    node.children
        .sort_unstable_by_key(|n| std::cmp::Reverse(n.size));
    for c in &node.children {
        node.size += c.size;
        node.files += c.files;
        node.modified = node.modified.max(c.modified);
    }
    node
}

/// A plain folder takes the color of whatever fills most of it.
fn dominant_kind(children: &[Node]) -> Kind {
    let mut totals: Vec<(Kind, u64)> = Vec::new();
    for c in children {
        match totals.iter_mut().find(|(k, _)| *k == c.kind) {
            Some((_, s)) => *s += c.size,
            None => totals.push((c.kind, c.size)),
        }
    }
    totals
        .into_iter()
        .max_by_key(|(_, s)| *s)
        .map_or(Kind::Other, |(k, _)| k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};

    #[test]
    fn sizes_roll_up_and_build_output_is_tagged() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root.join("Cargo.toml"), "[package]").unwrap();
        create_dir_all(root.join("target/debug")).unwrap();
        write(
            root.join("target/debug/app.bin"),
            vec![0u8; SMALL_FILE as usize],
        )
        .unwrap();
        write(root.join("notes.txt"), "hello").unwrap();

        let tree = scan(root, &Progress::default());

        assert_eq!(
            tree.size,
            SMALL_FILE + "hello".len() as u64 + "[package]".len() as u64
        );
        assert_eq!(tree.files, 3);
        let target = tree.child("target").unwrap();
        assert_eq!(target.kind, Kind::Build);
        // Everything under a reclaimable folder inherits its kind.
        assert_eq!(target.child("debug").unwrap().kind, Kind::Build);
        assert!(tree.children.iter().any(|c| c.grouped && c.files == 2));
    }

    #[test]
    fn dominant_color_does_not_make_a_folder_reclaimable() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        create_dir_all(proj.join("target")).unwrap();
        write(proj.join("Cargo.toml"), "x").unwrap();
        write(
            proj.join("target/big.bin"),
            vec![0u8; SMALL_FILE as usize * 2],
        )
        .unwrap();

        let tree = scan(tmp.path(), &Progress::default());
        let p = tree.child("proj").unwrap();
        assert_eq!(p.kind, Kind::Build, "painted by what fills it");
        assert!(!p.reclaimable, "but never deletable as a whole");
        assert!(p.child("target").unwrap().reclaimable);
    }

    #[test]
    fn cancel_stops_early() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path().join("a.txt"), "x").unwrap();
        let p = Progress::default();
        p.cancel.store(true, Ordering::Relaxed);
        assert_eq!(scan(tmp.path(), &p).files, 0);
    }

    #[test]
    fn missing_root_is_empty_not_a_panic() {
        let tree = scan(
            Path::new("/definitely/not/here/faxina"),
            &Progress::default(),
        );
        assert_eq!(tree.size, 0);
    }
}
