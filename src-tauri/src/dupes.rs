use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::guard::Protection;
use crate::listing::{self, Links};
use crate::scan::mtime;

/// Bytes read for the cheap prefix/suffix probes before committing to a full hash.
const PARTIAL: usize = 64 * 1024;
/// `update_mmap_rayon` is only worth its overhead above this size (blake3 docs put the
/// crossover around 128 KiB; files reaching a full hash are already past it).
const MMAP_MIN: u64 = 256 * 1024;
/// Upper bound on persisted hashes, so re-scanning many different folders cannot grow the cache
/// file forever. Past this, the oldest entries are evicted.
const CACHE_CAP: usize = 300_000;

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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

/// What the search found for one file, kept so a later delete can be checked against it:
/// path, size and modification time.
pub type Expected = (String, u64, u64);

pub fn expected_of(groups: &[DupGroup]) -> Vec<Vec<Expected>> {
    groups
        .iter()
        .map(|g| {
            g.files
                .iter()
                .map(|f| (f.path.clone(), g.size, f.modified))
                .collect()
        })
        .collect()
}

/// Paths of a delete request that must be refused, with the reason. A request may never take
/// every copy of a group (the webview could be wrong or compromised), and a file that changed
/// since the search is no longer the file the user reviewed. Paths outside every group are not
/// this check's business.
pub fn refused_selection(
    groups: &[Vec<Expected>],
    requested: &[String],
) -> HashMap<String, String> {
    let wanted: std::collections::HashSet<&str> = requested.iter().map(String::as_str).collect();
    let mut refused = HashMap::new();
    for group in groups {
        let all_taken = group
            .iter()
            .all(|(path, _, _)| wanted.contains(path.as_str()));
        for (path, size, modified) in group {
            if !wanted.contains(path.as_str()) {
                continue;
            }
            if all_taken {
                refused.insert(
                    path.clone(),
                    "Uma cópia de cada grupo de duplicatas precisa ficar.".to_string(),
                );
            } else if !unchanged(Path::new(path), *size, *modified) {
                refused.insert(
                    path.clone(),
                    "O arquivo mudou desde a busca. Procure as duplicatas de novo.".to_string(),
                );
            }
        }
    }
    refused
}

/// Forgets deleted copies; a group left with one file is no longer a duplicate.
pub fn forget_deleted(groups: &mut Vec<Vec<Expected>>, deleted: &[String]) {
    let gone: std::collections::HashSet<&str> = deleted.iter().map(String::as_str).collect();
    for group in groups.iter_mut() {
        group.retain(|(path, _, _)| !gone.contains(path.as_str()));
    }
    groups.retain(|g| g.len() > 1);
}

/// One progress sample: which hashing stage runs, and how far it has got.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct DupTick {
    pub stage: u64,
    pub done: u64,
    pub total: u64,
}

pub struct DupProgress<'a> {
    stage: AtomicU64,
    done: AtomicU64,
    total: AtomicU64,
    pub cancel: &'a AtomicBool,
}

impl<'a> DupProgress<'a> {
    pub fn new(cancel: &'a AtomicBool) -> Self {
        Self {
            stage: AtomicU64::new(0),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            cancel,
        }
    }

    pub fn tick(&self) -> DupTick {
        DupTick {
            stage: self.stage.load(Ordering::Relaxed),
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }

    /// Each stage hashes a different set of files, so its total is only known when it starts.
    /// A stage with nothing left to hash is not a stage, so it leaves the counter alone.
    fn begin_stage(&self, total: usize) {
        if total == 0 {
            return;
        }
        self.stage.fetch_add(1, Ordering::Relaxed);
        self.done.store(0, Ordering::Relaxed);
        self.total.store(total as u64, Ordering::Relaxed);
    }
}

type Candidate = (PathBuf, u64, u64);

struct Walk<'a> {
    min_size: u64,
    cancel: &'a AtomicBool,
    /// Off on volumes whose file ids are not trustworthy (FAT, exFAT).
    ids: bool,
    links: Links,
    protection: Protection,
}

/// Names that are OS bookkeeping rather than user content, never offered as duplicates.
fn is_noise_file(name: &str) -> bool {
    matches!(name, ".DS_Store" | "Thumbs.db" | "desktop.ini")
}

/// Folders below the scan root that hold tool metadata (`.git`, `.cache`, ...) or OS and
/// credential data. Their files are not the user's to deduplicate: deleting one corrupts a
/// repository or fails at the guard later, so they are not walked at all. The root itself is
/// always scanned, even when the user picked a dot-folder on purpose.
fn skip_dir(name: &str, path: &Path, w: &Walk) -> bool {
    name.starts_with('.') || w.protection.is_system(path)
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
        if e.is_dir {
            let path = dir.join(&e.name);
            if !skip_dir(&e.name, &path, w) {
                dirs.push(path);
            }
        } else if e.size >= w.min_size
            && !is_noise_file(&e.name)
            && e.id.is_none_or(|id| w.links.first(id))
        {
            files.push((dir.join(&e.name), e.size, e.modified));
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
    #[serde(default)]
    last_used: u64,
}

/// Hashes survive across runs as long as a file's size and mtime are unchanged, so re-scanning a
/// big media library does not re-read every multi-GB file.
struct HashCache {
    path: Option<PathBuf>,
    cap: usize,
    map: Mutex<HashMap<String, CacheEntry>>,
    dirty: AtomicBool,
}

impl HashCache {
    fn disabled() -> Self {
        Self {
            path: None,
            cap: CACHE_CAP,
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
        Self::with_cap(path, CACHE_CAP)
    }

    fn with_cap(path: PathBuf, cap: usize) -> Self {
        let map = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            path: Some(path),
            cap,
            map: Mutex::new(map),
            dirty: AtomicBool::new(false),
        }
    }

    fn get(&self, path: &Path, size: u64, modified: u64) -> Option<String> {
        let mut guard = self.map.lock().unwrap_or_else(|e| e.into_inner());
        let key = path.to_string_lossy();
        if let Some(e) = guard.get_mut(key.as_ref()) {
            if e.size == size && e.modified == modified {
                let now = current_timestamp();
                if e.last_used != now {
                    e.last_used = now;
                    self.dirty.store(true, Ordering::Relaxed);
                }
                return Some(e.hash.clone());
            }
        }
        None
    }

    fn insert(&self, path: &Path, size: u64, modified: u64, hash: String) {
        let now = current_timestamp();
        self.map.lock().unwrap_or_else(|e| e.into_inner()).insert(
            path.to_string_lossy().into_owned(),
            CacheEntry {
                size,
                modified,
                hash,
                last_used: now,
            },
        );
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn prune(&self, root: &Path, seen: &HashMap<PathBuf, (u64, u64)>) {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        let before_len = map.len();
        map.retain(|key, entry| {
            let path = Path::new(key);
            if path.starts_with(root) {
                if let Some(&(s, m)) = seen.get(path) {
                    entry.size == s && entry.modified == m
                } else {
                    false
                }
            } else {
                true
            }
        });
        if map.len() != before_len {
            self.dirty.store(true, Ordering::Relaxed);
        }
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        if !self.dirty.load(Ordering::Relaxed) {
            return;
        }
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() > self.cap {
            prune_to_cap(&mut map, self.cap);
        }
        let json = match serde_json::to_string(&*map) {
            Ok(j) => j,
            Err(err) => {
                eprintln!("Failed to serialize hash cache: {err}");
                return;
            }
        };
        if let Some(dir) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(dir) {
                eprintln!("Failed to create cache directory {}: {err}", dir.display());
                return;
            }
        }
        let tmp = path.with_extension("json.tmp");
        if let Err(err) = std::fs::write(&tmp, &json) {
            eprintln!(
                "Failed to write temporary hash cache file {}: {err}",
                tmp.display()
            );
            return;
        }
        if let Err(err) = std::fs::rename(&tmp, path) {
            eprintln!(
                "Failed to rename temporary hash cache file {} to {}: {err}",
                tmp.display(),
                path.display()
            );
        }
    }
}

/// Entries for deleted files can never hit again, so they go first. Any excess then goes
/// least-recently-used first, so a hash that keeps being reused survives.
fn prune_to_cap(map: &mut HashMap<String, CacheEntry>, cap: usize) {
    map.retain(|p, _| Path::new(p).exists());
    if map.len() <= cap {
        return;
    }
    let mut by_age: Vec<(String, u64)> =
        map.iter().map(|(k, v)| (k.clone(), v.last_used)).collect();
    by_age.sort_by(|(k1, lu1), (k2, lu2)| lu1.cmp(lu2).then_with(|| k1.cmp(k2)));
    for (k, _) in by_age.into_iter().take(map.len() - cap) {
        map.remove(&k);
    }
}

/// A file that changed or was replaced while the search ran must not be grouped with its old
/// content: the hash describes bytes that are gone. Size and mtime are re-read after hashing.
fn unchanged(path: &Path, size: u64, modified: u64) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_file() && m.len() == size && mtime(&m) == modified)
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
            // A file shorter than the probe needs only its own bytes: zeroing the full probe per small file was wasted work.
            let mut buf = vec![0u8; n.min(size as usize)];
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
    p.begin_stage(buckets.iter().map(Vec::len).sum());
    buckets
        .into_par_iter()
        .flat_map(|bucket| {
            let mut by_hash: HashMap<String, Vec<Candidate>> = HashMap::new();
            for c in bucket {
                if p.cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(h) = hash_file(&c.0, mode, c.1, c.2, cache) {
                    if unchanged(&c.0, c.1, c.2) {
                        by_hash.entry(h).or_default().push(c);
                    }
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

pub fn find(root: &Path, min_size: u64, p: &DupProgress) -> (Vec<DupGroup>, bool) {
    let cache = HashCache::open();
    find_impl(root, min_size, p, &cache)
}

fn find_impl(
    root: &Path,
    min_size: u64,
    p: &DupProgress,
    cache: &HashCache,
) -> (Vec<DupGroup>, bool) {
    let w = Walk {
        min_size: min_size.max(1),
        cancel: p.cancel,
        ids: listing::ids_supported(root),
        links: Links::default(),
        protection: Protection::new(),
    };
    let files = walk(root, &w);
    let seen: HashMap<PathBuf, (u64, u64)> = files
        .iter()
        .map(|(p, s, m)| (p.clone(), (*s, *m)))
        .collect();

    let mut by_size: HashMap<u64, Vec<Candidate>> = HashMap::new();
    for f in files {
        by_size.entry(f.1).or_default().push(f);
    }
    let same_size: Vec<Vec<Candidate>> = by_size.into_values().filter(|v| v.len() > 1).collect();

    // Size -> prefix hash -> suffix hash -> full hash (fclones-style), the cheap probes first.
    let prefix = refine(same_size, Mode::Prefix(PARTIAL), p, cache);
    let (small, big): (Vec<_>, Vec<_>) = prefix
        .into_iter()
        .partition(|(_, v)| v[0].1 <= PARTIAL as u64);

    let suffix = refine(
        big.into_iter().map(|(_, v)| v).collect(),
        Mode::Tail(PARTIAL),
        p,
        cache,
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
        cache,
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
    cache.prune(root, &seen);
    cache.save();
    let cancelled = p.cancel.load(Ordering::Relaxed);
    (out, cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn run(root: &Path, min: u64) -> Vec<DupGroup> {
        let cancel = AtomicBool::new(false);
        find(root, min, &DupProgress::new(&cancel)).0
    }

    #[test]
    fn progress_reports_the_stage_it_is_in_with_its_own_total() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::write(r.join("a.jpg"), b"same bytes").unwrap();
        fs::write(r.join("b.jpg"), b"same bytes").unwrap();
        let cancel = AtomicBool::new(false);
        let p = DupProgress::new(&cancel);
        assert_eq!(p.tick().stage, 0, "no stage before the search starts");
        let g = find(r, 1, &p).0;
        assert_eq!(g.len(), 1);
        let t = p.tick();
        assert!(t.stage >= 1, "at least one hashing stage ran");
        assert!(t.total >= 2, "the stage reports the files it had to hash");
        assert_eq!(
            t.done, t.total,
            "a finished stage has hashed everything it counted"
        );
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
    fn tool_metadata_folders_and_os_noise_are_not_offered() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::create_dir_all(r.join(".git/objects")).unwrap();
        fs::create_dir(r.join("docs")).unwrap();
        fs::write(r.join("a.txt"), b"same bytes").unwrap();
        fs::write(r.join(".git/objects/copy.txt"), b"same bytes").unwrap();
        fs::write(r.join("docs/.DS_Store"), b"same bytes").unwrap();
        fs::write(r.join("docs/Thumbs.db"), b"same bytes").unwrap();
        assert!(run(r, 1).is_empty());
        fs::write(r.join("docs/b.txt"), b"same bytes").unwrap();
        assert_eq!(run(r, 1)[0].files.len(), 2);
    }

    #[test]
    fn a_dot_folder_chosen_as_the_root_is_still_scanned() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join(".archive");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.txt"), b"same bytes").unwrap();
        fs::write(root.join("b.txt"), b"same bytes").unwrap();
        assert_eq!(run(&root, 1).len(), 1);
    }

    #[test]
    fn a_file_changed_after_the_walk_is_not_grouped() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a.bin");
        fs::write(&path, b"12345").unwrap();
        let meta = fs::symlink_metadata(&path).unwrap();
        assert!(unchanged(&path, 5, mtime(&meta)));
        assert!(!unchanged(&path, 6, mtime(&meta)), "size differs");
        assert!(!unchanged(&path, 5, mtime(&meta) + 1), "mtime differs");
        assert!(!unchanged(&tmp.path().join("gone"), 5, 0), "file vanished");
    }

    #[test]
    fn a_request_can_never_take_every_copy_of_a_group() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::write(r.join("a.txt"), b"same bytes").unwrap();
        fs::write(r.join("b.txt"), b"same bytes").unwrap();
        fs::write(r.join("c.txt"), b"same bytes").unwrap();
        let groups = expected_of(&run(r, 1));
        let path = |n: &str| r.join(n).to_string_lossy().into_owned();

        assert!(refused_selection(&groups, &[path("b.txt"), path("c.txt")]).is_empty());
        let all = [path("a.txt"), path("b.txt"), path("c.txt")];
        assert_eq!(refused_selection(&groups, &all).len(), 3);
        // Unrelated paths are left to the regular guard.
        assert!(refused_selection(&groups, &[path("elsewhere")]).is_empty());
    }

    #[test]
    fn a_file_changed_since_the_search_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::write(r.join("a.txt"), b"same bytes").unwrap();
        fs::write(r.join("b.txt"), b"same bytes").unwrap();
        let groups = expected_of(&run(r, 1));
        let b = r.join("b.txt").to_string_lossy().into_owned();
        assert!(refused_selection(&groups, std::slice::from_ref(&b)).is_empty());
        fs::write(r.join("b.txt"), b"edited since the search").unwrap();
        let refused = refused_selection(&groups, std::slice::from_ref(&b));
        assert!(refused[&b].contains("mudou"));
    }

    #[test]
    fn forgetting_deleted_copies_keeps_the_one_survivor_rule_honest() {
        let mut groups = vec![vec![
            ("a".to_string(), 1, 1),
            ("b".to_string(), 1, 1),
            ("c".to_string(), 1, 1),
        ]];
        forget_deleted(&mut groups, &["b".into(), "c".into()]);
        assert!(groups.is_empty(), "a lone survivor is not a group anymore");
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
    fn pre_cancelled_find_returns_cancelled_true() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::write(r.join("a.jpg"), b"same bytes").unwrap();
        fs::write(r.join("b.jpg"), b"same bytes").unwrap();

        let cancel = AtomicBool::new(true);
        let (_, cancelled) = find(r, 1, &DupProgress::new(&cancel));
        assert!(cancelled);

        let cancel_false = AtomicBool::new(false);
        let (_, cancelled_false) = find(r, 1, &DupProgress::new(&cancel_false));
        assert!(!cancelled_false);
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

    #[test]
    fn prune_to_cap_drops_missing_files_then_trims_to_the_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let mut map = HashMap::new();
        for i in 0..3 {
            let live = tmp.path().join(format!("live-{i}.bin"));
            fs::write(&live, b"x").unwrap();
            map.insert(
                live.to_string_lossy().into_owned(),
                CacheEntry {
                    size: 1,
                    modified: 2,
                    hash: "h".into(),
                    last_used: 0,
                },
            );
        }
        for i in 0..5 {
            let gone = tmp.path().join(format!("gone-{i}.bin"));
            map.insert(
                gone.to_string_lossy().into_owned(),
                CacheEntry {
                    size: 1,
                    modified: 2,
                    hash: "h".into(),
                    last_used: 0,
                },
            );
        }
        prune_to_cap(&mut map, 2);
        assert_eq!(map.len(), 2, "trimmed to the cap");
        assert!(
            map.keys().all(|p| Path::new(p).exists()),
            "only live files survive"
        );
    }

    #[test]
    fn save_prunes_entries_for_deleted_files_when_over_the_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("live.bin");
        fs::write(&live, b"x").unwrap();
        let cache_file = tmp.path().join("cache.json");
        let c = HashCache::at(cache_file.clone());
        c.insert(&live, 1, 2, "keep".into());
        for i in 0..CACHE_CAP {
            c.insert(
                &tmp.path().join(format!("gone-{i}.bin")),
                1,
                2,
                "stale".into(),
            );
        }
        c.save();
        let reloaded = HashCache::at(cache_file);
        assert_eq!(reloaded.get(&live, 1, 2).as_deref(), Some("keep"));
        assert!(reloaded.map.lock().unwrap().len() <= CACHE_CAP);
    }

    #[test]
    fn hash_cache_evicts_oldest_entries_when_exceeding_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("cache.json");
        let c = HashCache::with_cap(file.clone(), 3);
        // Real files: entries for deleted files are pruned before the age-based trim.
        let paths: Vec<PathBuf> = (1..=4)
            .map(|i| tmp.path().join(format!("{i}.bin")))
            .collect();
        for p in &paths {
            fs::write(p, b"x").unwrap();
            c.insert(p, 1, 2, "hash".into());
        }

        {
            let mut guard = c.map.lock().unwrap();
            for (p, used) in paths.iter().zip([100u64, 200, 300, 400]) {
                guard
                    .get_mut(p.to_string_lossy().as_ref())
                    .unwrap()
                    .last_used = used;
            }
        }

        c.save();

        let loaded = HashCache::at(file);
        let guard = loaded.map.lock().unwrap();
        assert_eq!(guard.len(), 3);
        assert!(
            !guard.contains_key(paths[0].to_string_lossy().as_ref()),
            "oldest entry should be evicted"
        );
        assert!(guard.contains_key(paths[1].to_string_lossy().as_ref()));
        assert!(guard.contains_key(paths[2].to_string_lossy().as_ref()));
        assert!(guard.contains_key(paths[3].to_string_lossy().as_ref()));
    }

    #[test]
    fn find_prunes_deleted_or_modified_files_from_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        let cache_file = r.join("dupes-cache.json");

        let content = vec![5u8; 2 * PARTIAL + 100];
        fs::write(r.join("a.bin"), &content).unwrap();
        fs::write(r.join("b.bin"), &content).unwrap();

        let cancel = AtomicBool::new(false);
        let progress = DupProgress::new(&cancel);

        let cache = HashCache::at(cache_file.clone());
        find_impl(r, 1, &progress, &cache);

        {
            let loaded = HashCache::at(cache_file.clone());
            let guard = loaded.map.lock().unwrap();
            assert_eq!(guard.len(), 2);
        }

        fs::remove_file(r.join("b.bin")).unwrap();

        find_impl(r, 1, &progress, &cache);

        {
            let loaded = HashCache::at(cache_file);
            let guard = loaded.map.lock().unwrap();
            assert_eq!(guard.len(), 1);
            let key = r.join("b.bin").to_string_lossy().into_owned();
            assert!(
                !guard.contains_key(&key),
                "deleted file should be pruned from cache"
            );
        }
    }

    #[test]
    fn hash_cache_reads_old_json_without_last_used() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("old_cache.json");
        let old_json = r#"{
            "/x/y": {
                "size": 10,
                "modified": 20,
                "hash": "abc"
            }
        }"#;
        fs::write(&file, old_json).unwrap();

        let c = HashCache::at(file);
        assert_eq!(c.get(Path::new("/x/y"), 10, 20).as_deref(), Some("abc"));
    }
}
