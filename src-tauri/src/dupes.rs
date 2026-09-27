use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rayon::prelude::*;
use serde::Serialize;

use crate::listing::{self, Links};

const PARTIAL: usize = 64 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupFile {
    pub path: String,
    pub modified: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    pub hash: String,
    pub size: u64,
    /// Oldest first: the natural "original" to keep.
    pub files: Vec<DupFile>,
}

impl DupGroup {
    pub fn wasted(&self) -> u64 {
        self.size * (self.files.len() as u64 - 1)
    }
}

pub struct DupProgress<'a> {
    pub done: AtomicU64,
    pub cancel: &'a AtomicBool,
}

type Candidate = (PathBuf, u64, u64);

struct Walk<'a> {
    min_size: u64,
    cancel: &'a AtomicBool,
    /// Off on volumes whose file ids are not trustworthy (FAT, exFAT).
    ids: bool,
    links: Links,
}

fn walk(dir: &Path, w: &Walk, out: &mut Vec<Candidate>) {
    if w.cancel.load(Ordering::Relaxed) {
        return;
    }
    let Ok(entries) = listing::list(dir, w.ids) else {
        return;
    };
    for e in entries {
        let path = dir.join(&e.name);
        if e.is_dir {
            walk(&path, w, out);
        // Hard links share storage, so deleting one frees nothing: count each file once.
        } else if e.size >= w.min_size && e.id.is_none_or(|id| w.links.first(id)) {
            out.push((path, e.size, e.modified));
        }
    }
}

fn hash_file(path: &Path, limit: Option<usize>) -> io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut f = File::open(path)?;
    match limit {
        Some(n) => {
            let mut buf = vec![0u8; n];
            let read = f.read(&mut buf)?;
            hasher.update(&buf[..read]);
        }
        None => {
            hasher.update_reader(f)?;
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Splits each bucket by hash; buckets that end up with a single file are not duplicates.
fn refine(
    buckets: Vec<Vec<Candidate>>,
    limit: Option<usize>,
    p: &DupProgress,
) -> Vec<(String, Vec<Candidate>)> {
    buckets
        .into_par_iter()
        .flat_map(|bucket| {
            let mut by_hash: HashMap<String, Vec<Candidate>> = HashMap::new();
            for c in bucket {
                if p.cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(h) = hash_file(&c.0, limit) {
                    by_hash.entry(h).or_default().push(c);
                }
                p.done.fetch_add(1, Ordering::Relaxed);
            }
            by_hash
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn find(root: &Path, min_size: u64, p: &DupProgress) -> Vec<DupGroup> {
    let mut files = Vec::new();
    let w = Walk {
        min_size: min_size.max(1),
        cancel: p.cancel,
        ids: listing::ids_supported(root),
        links: Links::default(),
    };
    walk(root, &w, &mut files);

    let mut by_size: HashMap<u64, Vec<Candidate>> = HashMap::new();
    for f in files {
        by_size.entry(f.1).or_default().push(f);
    }
    let same_size: Vec<Vec<Candidate>> = by_size.into_values().filter(|v| v.len() > 1).collect();

    let partial = refine(same_size, Some(PARTIAL), p);
    // Small files are fully covered by the partial hash already.
    let (small, big): (Vec<_>, Vec<_>) = partial
        .into_iter()
        .partition(|(_, v)| v[0].1 <= PARTIAL as u64);
    let mut groups = small;
    groups.extend(refine(big.into_iter().map(|(_, v)| v).collect(), None, p));

    let mut out: Vec<DupGroup> = groups
        .into_iter()
        .map(|(hash, mut v)| {
            // Ties (same mtime) favor the shorter path: "natal.png" over "natal (1).png".
            v.sort_by_key(|c| (c.2, c.0.as_os_str().len()));
            DupGroup {
                hash,
                size: v[0].1,
                files: v
                    .into_iter()
                    .map(|c| DupFile {
                        path: c.0.to_string_lossy().into_owned(),
                        modified: c.2,
                    })
                    .collect(),
            }
        })
        .collect();
    out.sort_unstable_by_key(|g| std::cmp::Reverse(g.wasted()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn run(root: &Path, min: u64) -> Vec<DupGroup> {
        let cancel = AtomicBool::new(false);
        find(
            root,
            min,
            &DupProgress {
                done: AtomicU64::new(0),
                cancel: &cancel,
            },
        )
    }

    #[test]
    fn finds_identical_content_across_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::create_dir(r.join("sub")).unwrap();
        fs::write(r.join("a.jpg"), b"same bytes").unwrap();
        fs::write(r.join("sub/copy of a.jpg"), b"same bytes").unwrap();
        fs::write(r.join("other.jpg"), b"diff bytes").unwrap(); // same size, other content

        let g = run(r, 1);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].files.len(), 2);
        assert_eq!(g[0].wasted(), 10);
        // Written first (older or same second, shorter path): kept as the original.
        assert!(g[0].files[0].path.ends_with("a.jpg") && !g[0].files[0].path.contains("copy"));
    }

    #[test]
    fn hard_links_are_not_duplicates() {
        // Deleting one link frees nothing, so offering it would be a lie.
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::write(r.join("a.bin"), b"linked bytes").unwrap();
        fs::hard_link(r.join("a.bin"), r.join("b.bin")).unwrap();
        assert!(run(r, 1).is_empty());
    }

    #[test]
    fn same_prefix_different_tail_is_not_a_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let mut a = vec![7u8; PARTIAL + 10];
        fs::write(tmp.path().join("a.bin"), &a).unwrap();
        *a.last_mut().unwrap() = 8;
        fs::write(tmp.path().join("b.bin"), &a).unwrap();
        assert!(run(tmp.path(), 1).is_empty());
    }

    #[test]
    fn min_size_filters_and_empty_files_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("x"), b"").unwrap();
        fs::write(tmp.path().join("y"), b"").unwrap();
        fs::write(tmp.path().join("s1"), b"abc").unwrap();
        fs::write(tmp.path().join("s2"), b"abc").unwrap();
        assert!(
            run(tmp.path(), 0).len() == 1,
            "empty files are never reported"
        );
        assert!(run(tmp.path(), 100).is_empty());
    }
}
