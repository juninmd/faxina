use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::listing::{self, Links};

/// Bytes read for the cheap prefix/suffix probes before committing to a full hash.
const PARTIAL: usize = 64 * 1024;
/// `update_mmap_rayon` is only worth its overhead above this size (blake3 docs put the
/// crossover around 128 KiB; files reaching a full hash are already past it).
const MMAP_MIN: u64 = 256 * 1024;
/// Upper bound on persisted hashes, so re-scanning many different folders cannot grow the cache
/// file forever. Past this, the file is left as-is instead of being rewritten.
const CACHE_CAP: usize = 300_000;

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

/// Parallel directory walk. Hard links share storage, so each file id is kept once; deleting a
/// second link would free nothing.
fn walk(dir: &Path, w: &Walk) -> Vec<Candidate> {
    if w.cancel.load(Ordering::Relaxed) {
        return Vec::new();
    }
    let Ok(entries) = listing::list(dir, w.ids) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for e in entries {
        let path = dir.join(&e.name);
        if e.is_dir {
            dirs.push(path);
        } else if e.size >= w.min_size && e.id.is_none_or(|id| w.links.first(id)) {
            files.push((path, e.size, e.modified));
        }
    }
    let deeper: Vec<Vec<Candidate>> = dirs.into_par_iter().map(|d| walk(&d, w)).collect();
    for d in deeper {
        files.extend(d);
    }
    files
}

#[derive(Clone, Copy)]
enum Mode {
    /// Hash of the first `n` bytes.
    Prefix(usize),
    /// Hash of the last `n` bytes.
    Tail(usize),
    /// Hash of the whole file.
    Full,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    size: u64,
    modified: u64,
    hash: String,
}

/// Hashes survive across runs as long as a file's size and mtime are unchanged, so re-scanning a
/// big media library does not re-read every multi-GB file.
struct HashCache {
    path: Option<PathBuf>,
    map: Mutex<HashMap<String, CacheEntry>>,
    dirty: AtomicBool,
}

impl HashCache {
    fn disabled() -> Self {
        Self {
            path: None,
            map: Mutex::new(HashMap::new()),
            dirty: AtomicBool::new(false),
        }
    }

    fn open() -> Self {
        // Tests must not touch the user's real cache directory.
        if cfg!(test) {
            return Self::disabled();
        }
        match dirs::cache_dir() {
            Some(dir) => Self::at(dir.join("faxina").join("dupes-cache.json")),
            None => Self::disabled(),
        }
    }

    fn at(path: PathBuf) -> Self {
        let map = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            path: Some(path),
            map: Mutex::new(map),
            dirty: AtomicBool::new(false),
        }
    }

    fn get(&self, path: &Path, size: u64, modified: u64) -> Option<String> {
        let guard = self.map.lock().unwrap_or_else(|e| e.into_inner());
        let key = path.to_string_lossy();
        match guard.get(key.as_ref()) {
            Some(e) if e.size == size && e.modified == modified => Some(e.hash.clone()),
            _ => None,
        }
    }

    fn insert(&self, path: &Path, size: u64, modified: u64, hash: String) {
        self.map.lock().unwrap_or_else(|e| e.into_inner()).insert(
            path.to_string_lossy().into_owned(),
            CacheEntry {
                size,
                modified,
                hash,
            },
        );
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        if !self.dirty.load(Ordering::Relaxed) {
            return;
        }
        let map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() > CACHE_CAP {
            return;
        }
        let Ok(json) = serde_json::to_string(&*map) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

/// Hash of a file under `mode`, reusing the cache for full hashes.
fn hash_file(
    path: &Path,
    mode: Mode,
    size: u64,
    modified: u64,
    cache: &HashCache,
) -> io::Result<String> {
    if matches!(mode, Mode::Full) {
        if let Some(hash) = cache.get(path, size, modified) {
            return Ok(hash);
        }
    }
    let mut hasher = blake3::Hasher::new();
    match mode {
        Mode::Full if size >= MMAP_MIN => {
            if hasher.update_mmap_rayon(path).is_err() {
                hasher = blake3::Hasher::new();
                hasher.update_reader(File::open(path)?)?;
            }
        }
        Mode::Full => {
            hasher.update_reader(File::open(path)?)?;
        }
        Mode::Prefix(n) => {
            let mut file = File::open(path)?;
            let mut buf = vec![0u8; n];
            let read = file.read(&mut buf)?;
            hasher.update(&buf[..read]);
        }
        Mode::Tail(n) => {
            let mut file = File::open(path)?;
            file.seek(SeekFrom::Start(size.saturating_sub(n as u64)))?;
            hasher.update_reader(file)?;
        }
    }
    let hash = hasher.finalize().to_hex().to_string();
    if matches!(mode, Mode::Full) {
        cache.insert(path, size, modified, hash.clone());
    }
    Ok(hash)
}

/// Splits each bucket by hash under `mode`; buckets that end up with a single file are not
/// duplicates.
fn refine(
    buckets: Vec<Vec<Candidate>>,
    mode: Mode,
    p: &DupProgress,
    cache: &HashCache,
) -> Vec<(String, Vec<Candidate>)> {
    buckets
        .into_par_iter()
        .flat_map(|bucket| {
            let mut by_hash: HashMap<String, Vec<Candidate>> = HashMap::new();
            for c in bucket {
                if p.cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(h) = hash_file(&c.0, mode, c.1, c.2, cache) {
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
    let cache = HashCache::open();
    let w = Walk {
        min_size: min_size.max(1),
        cancel: p.cancel,
        ids: listing::ids_supported(root),
        links: Links::default(),
    };
    let files = walk(root, &w);

    let mut by_size: HashMap<u64, Vec<Candidate>> = HashMap::new();
    for f in files {
        by_size.entry(f.1).or_default().push(f);
    }
    let same_size: Vec<Vec<Candidate>> = by_size.into_values().filter(|v| v.len() > 1).collect();

    // Size -> prefix hash -> suffix hash -> full hash (fclones-style), the cheap probes first.
    let prefix = refine(same_size, Mode::Prefix(PARTIAL), p, &cache);
    let (small, big): (Vec<_>, Vec<_>) = prefix
        .into_iter()
        .partition(|(_, v)| v[0].1 <= PARTIAL as u64);

    let suffix = refine(
        big.into_iter().map(|(_, v)| v).collect(),
        Mode::Tail(PARTIAL),
        p,
        &cache,
    );
    // Prefix + suffix cover the whole file once size <= 2*PARTIAL, so no full hash is needed.
    let (covered, rest): (Vec<_>, Vec<_>) = suffix
        .into_iter()
        .partition(|(_, v)| v[0].1 <= (2 * PARTIAL) as u64);

    let mut groups = small;
    groups.extend(covered);
    groups.extend(refine(
        rest.into_iter().map(|(_, v)| v).collect(),
        Mode::Full,
        p,
        &cache,
    ));

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
    cache.save();
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
    fn same_prefix_and_tail_different_middle_needs_the_full_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let size = 2 * PARTIAL + 100; // middle gap larger than zero
        let mut a = vec![7u8; size];
        fs::write(tmp.path().join("a.bin"), &a).unwrap();
        a[PARTIAL] = 9; // after the prefix, before the suffix
        fs::write(tmp.path().join("b.bin"), &a).unwrap();
        assert!(run(tmp.path(), 1).is_empty());
    }

    #[test]
    fn identical_big_files_are_found() {
        let tmp = tempfile::tempdir().unwrap();
        let big = vec![3u8; MMAP_MIN as usize + 4096];
        fs::write(tmp.path().join("a.bin"), &big).unwrap();
        fs::write(tmp.path().join("b.bin"), &big).unwrap();
        let g = run(tmp.path(), 1);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].files.len(), 2);
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

    #[test]
    fn hash_cache_round_trips_invalidating_on_change() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("map.json");
        {
            let c = HashCache::at(file.clone());
            c.insert(Path::new("/x/y"), 10, 20, "abc".into());
            c.save();
        }
        let c = HashCache::at(file);
        assert_eq!(c.get(Path::new("/x/y"), 10, 20).as_deref(), Some("abc"));
        assert_eq!(c.get(Path::new("/x/y"), 11, 20), None, "size changed");
        assert_eq!(c.get(Path::new("/x/y"), 10, 21), None, "mtime changed");
    }
}
