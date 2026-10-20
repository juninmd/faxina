use std::fs::Metadata;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use rayon::prelude::*;

use crate::category::{
    classify_dir, classify_file, hints_for, is_app_data_name, under_app_data, DirHints,
};
use crate::listing::{self, Links};
use crate::model::{Kind, Node};

/// Files below this size are folded into one "small files" bucket per directory,
/// which keeps a multi-million-file scan within a few hundred MB of RAM.
pub const SMALL_FILE: u64 = 512 * 1024;

/// Hard cap on how deep a scan descends. A pathological tree (or a link that slipped past the
/// reparse check) must not blow the stack.
pub const MAX_DEPTH: usize = 128;

#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub cancel: AtomicBool,
}

/// What every directory of one scan shares.
struct Ctx<'a> {
    ids: bool,
    /// The hard-link set belongs to one scan: keeping it on `Progress` made a second scan in the
    /// same session treat every file as already counted and report an empty tree.
    links: Links,
    progress: &'a Progress,
}

pub fn scan(root: &Path, progress: &Progress) -> Node {
    let name = root.to_string_lossy().into_owned();
    let ctx = Ctx {
        ids: listing::ids_supported(root),
        links: Links::default(),
        progress,
    };
    let run = || scan_dir(root, name, Kind::Other, under_app_data(root), &ctx, 0);
    // Directory listing waits on the disk, not the CPU: on a spinning drive a pool sized to the
    // core count keeps too few requests in flight (a cold 15k-file tree took 16 s).
    let mut node = match rayon::ThreadPoolBuilder::new()
        .num_threads(io_threads())
        .build()
    {
        Ok(pool) => pool.install(run),
        Err(_) => run(),
    };
    node.kind = dominant_kind(&node.children);
    node
}

fn io_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(4, usize::from);
    (cores * 4).clamp(16, 64)
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

fn scan_dir(
    path: &Path,
    name: String,
    forced: Kind,
    in_app_data: bool,
    ctx: &Ctx,
    depth: usize,
) -> Node {
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
    if depth > MAX_DEPTH || ctx.progress.cancel.load(Ordering::Relaxed) {
        return node;
    }
    let Ok(entries) = listing::list(path, ctx.ids) else {
        return node;
    };

    let hints = DirHints {
        under_app_data: in_app_data,
        // Unity is told apart by its two sibling folders, so directories take part too.
        ..hints_for(entries.iter().map(|e| e.name.as_str()))
    };
    let mut dirs = Vec::new();
    let (mut small_size, mut small_count, mut small_mtime) = (0u64, 0u64, 0u64);
    // One atomic update per directory: a fetch_add per file made every core fight for the counter.
    let (mut seen_files, mut seen_bytes) = (0u64, 0u64);

    for entry in entries {
        let entry_name = entry.name;
        if entry.is_dir {
            dirs.push((path.join(&entry_name), entry_name));
            continue;
        }
        // Another link to this file was already counted: it takes no extra space.
        if entry.id.is_some_and(|id| !ctx.links.first(id)) {
            continue;
        }
        let (size, modified) = (entry.size, entry.modified);
        seen_files += 1;
        seen_bytes += size;
        if size >= SMALL_FILE {
            let kind = if forced == Kind::Other {
                classify_file(&entry_name)
            } else {
                forced
            };
            node.children
                .push(Node::file(entry_name, size, modified, kind));
        } else {
            small_size += size;
            small_count += 1;
            small_mtime = small_mtime.max(modified);
        }
    }
    ctx.progress.files.fetch_add(seen_files, Ordering::Relaxed);
    ctx.progress.bytes.fetch_add(seen_bytes, Ordering::Relaxed);

    let subdirs: Vec<Node> = dirs
        .into_par_iter()
        .map(|(p, n)| {
            let special = classify_dir(&n, hints);
            let inherited = if forced == Kind::Other {
                special.unwrap_or(Kind::Other)
            } else {
                forced
            };
            let child_in_app_data = in_app_data || is_app_data_name(&n);
            let mut child = scan_dir(&p, n, inherited, child_in_app_data, ctx, depth + 1);
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
    fn maven_target_is_reclaimable_only_next_to_its_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        for proj in ["maven", "plain"] {
            create_dir_all(tmp.path().join(proj).join("target")).unwrap();
        }
        write(tmp.path().join("maven/pom.xml"), "<project/>").unwrap();

        let tree = scan(tmp.path(), &Progress::default());

        assert!(
            tree.child("maven")
                .unwrap()
                .child("target")
                .unwrap()
                .reclaimable
        );
        assert!(
            !tree
                .child("plain")
                .unwrap()
                .child("target")
                .unwrap()
                .reclaimable
        );
    }

    #[test]
    fn generic_cache_names_are_trusted_below_an_app_data_folder() {
        // The flag travels down one level per directory instead of being re-derived per path.
        let tmp = tempfile::tempdir().unwrap();
        create_dir_all(tmp.path().join("AppData/Local/Temp")).unwrap();
        write(
            tmp.path().join("AppData/Local/Temp/x.bin"),
            vec![0u8; SMALL_FILE as usize],
        )
        .unwrap();

        let tree = scan(tmp.path(), &Progress::default());

        let temp = tree
            .child("AppData")
            .unwrap()
            .child("Local")
            .unwrap()
            .child("Temp")
            .unwrap();
        assert_eq!(temp.kind, Kind::Cache);
        assert!(temp.reclaimable);
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
    fn hard_links_are_counted_once() {
        // WinSxS and System32 share files this way; counting each link inflated C: by GBs.
        let tmp = tempfile::tempdir().unwrap();
        create_dir_all(tmp.path().join("a")).unwrap();
        create_dir_all(tmp.path().join("b")).unwrap();
        let big = SMALL_FILE as usize * 2;
        write(tmp.path().join("a/lib.dll"), vec![0u8; big]).unwrap();
        std::fs::hard_link(tmp.path().join("a/lib.dll"), tmp.path().join("b/lib.dll")).unwrap();

        let p = Progress::default();
        let tree = scan(tmp.path(), &p);
        assert_eq!(tree.size, big as u64);
        assert_eq!(p.bytes.load(Ordering::Relaxed), big as u64);
    }

    #[test]
    fn rescanning_with_the_same_progress_is_stable() {
        // The hard-link set is per scan: on NTFS every file has an id, and on Unix only linked
        // files do, so the test creates a hard link to reproduce the leak on every platform.
        let tmp = tempfile::tempdir().unwrap();
        let big = SMALL_FILE as usize * 2;
        write(tmp.path().join("a.bin"), vec![0u8; big]).unwrap();
        std::fs::hard_link(tmp.path().join("a.bin"), tmp.path().join("b.bin")).unwrap();

        let p = Progress::default();
        let first = scan(tmp.path(), &p);
        let second = scan(tmp.path(), &p);
        assert_eq!(first.size, big as u64, "the pair counts once");
        assert_eq!(second.size, first.size);
        assert_eq!(second.files, first.files);
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
