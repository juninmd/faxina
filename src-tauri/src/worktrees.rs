//! Linked Git worktrees, with a verdict that comes from git itself and never from a folder name:
//! clean tree, no commit outside a remote-tracking ref and no lock. Discovery only follows a
//! `.git` *file* that points into `<repo>/.git/worktrees/<id>` (submodules use `modules/`, main
//! checkouts use a `.git` directory).
//!
//! Not every reason to hold a checkout back is the same, so the screen offers two explicit
//! opt-ins instead of a single yes/no. Local work — uncommitted files, commits outside the remote,
//! a stash — is *recoverable*: the checkout goes to the trash and the commits stay in the shared
//! repository. A detached HEAD with commits outside the remote, initialized submodules or a
//! repository whose linked worktrees would lose their metadata need the *definitive* opt-in,
//! because that work exists nowhere else. A lock, a repository-declared filter and metadata that
//! does not belong to the expected repository are refused for good: no opt-in passes them.
//!
//! Git runs with a cleared environment and `core.fsmonitor` off, and a repository that declares a
//! local clean/process filter is refused instead of inspected: running `git status` there would
//! execute commands the repository chose (CWE-94). The admin directory is re-validated against
//! the repository's own common dir before it leaves the disk, and it travels through the same
//! trash as the checkout (never `remove_dir_all`), so a swapped path cannot take anything else.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
#[cfg(windows)]
use std::path::{Component, Prefix};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::guard::Protection;
use crate::scan::{is_link, MAX_DEPTH};

const MAX_ITEMS: usize = 500;
/// A `.git` pointer file is a few dozen bytes; anything huge is not git's.
const GIT_FILE_LIMIT: u64 = 4096;
/// A hostile checkout (pipe-shaped config, pathologically slow index) must not hang a search.
const GIT_TIMEOUT: Duration = Duration::from_secs(30);
/// Each inspection spawns several git processes; a handful at a time keeps the machine responsive.
const INSPECT_THREADS: usize = 4;
/// Windows flag: git is a console program and would otherwise flash a terminal per call.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Generated trees never hold a checkout of their own; walking them only costs time.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".next",
    ".angular",
    ".parcel-cache",
    ".turbo",
    ".gradle",
    ".dart_tool",
    ".terraform",
    ".stack-work",
    "dist-newstyle",
    ".zig-cache",
    "zig-out",
    // System trees are protected on `C:` by `Protection`; on a secondary volume only the name
    // identifies them, and a worktree below one is never something the user put there.
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "system volume information",
    "$recycle.bin",
    "recovery",
    "perflogs",
    "config.msi",
];

/// What a checkout needs before it may leave the disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Allowance {
    /// Git confirms every commit is on a remote-tracking ref: removable right away.
    Synced,
    /// Removable with an explicit opt-in: the files land in the trash and the commits stay in the
    /// shared repository.
    Recoverable,
    /// Removable only with the "definitive loss" opt-in: something exists nowhere else.
    Definitive,
    /// No opt-in takes it: git or the layout refuses the removal.
    Never,
}

/// How much the user allowed when asking for a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OptIn {
    /// Only what git confirms.
    Synced,
    /// Also local work that stays recoverable.
    Local,
    /// Also work that exists nowhere else.
    Definitive,
}

impl Allowance {
    /// The least opt-in that permits this checkout; `None` when none does.
    fn requires(self) -> Option<OptIn> {
        match self {
            Self::Synced => Some(OptIn::Synced),
            Self::Recoverable => Some(OptIn::Local),
            Self::Definitive => Some(OptIn::Definitive),
            Self::Never => None,
        }
    }
}

/// Refuses the removal unless the user allowed this much; `reason` is the text shown on screen.
fn permitted(allowance: Allowance, opt_in: OptIn, reason: Option<&str>) -> Result<(), String> {
    match allowance.requires() {
        Some(needed) if opt_in >= needed => Ok(()),
        _ => Err(reason
            .unwrap_or("esta worktree não pode ser removida")
            .to_string()),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    /// `"worktree"` (linked checkout) or `"repository"` (a main checkout with its own `.git`).
    pub kind: &'static str,
    pub branch: String,
    pub head: String,
    /// Last commit, seconds since the epoch.
    pub modified: u64,
    pub size: u64,
    pub files: u64,
    /// Ignored entries that would leave with the checkout (build output, dependencies, `.env`).
    /// They are not part of the remote history, but they are what the trash is for.
    pub ignored: u64,
    /// How far an explicit opt-in can go; `reason` says why when it is not `synced`.
    pub allowance: Allowance,
    /// Why the removal is held back; `None` when the checkout is synced.
    pub reason: Option<String>,
    /// Commits outside every remote-tracking ref.
    pub ahead: u64,
    /// Entries with uncommitted changes (tracked or untracked).
    pub changes: u64,
}

/// Search progress: `walk` counts folders visited, `check` counts worktrees verified by git.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tick {
    pub phase: &'static str,
    pub done: u64,
    pub total: u64,
    pub found: u64,
    pub current: String,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Removed {
    pub freed: u64,
    pub removed: Vec<String>,
    pub failed: Vec<Failure>,
    /// Cleanup that did not stop the removal, e.g. git metadata left behind for `worktree prune`.
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub path: String,
    pub error: String,
}

/// What `remove` did with one worktree.
#[derive(Debug)]
pub struct Outcome {
    pub freed: u64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Worktree,
    Repository,
}

/// Linked worktrees and main repositories under `root`, removable-first, then biggest. `root`
/// itself is never listed: the screen only offers folders below the one the user chose to search.
pub fn find(root: &Path, progress: &(dyn Fn(Tick) + Sync)) -> Vec<Worktree> {
    let protection = Protection::new();
    let visited = AtomicU64::new(0);
    let mut paths = walk(root, &protection, 0, false, &visited, progress);
    progress(Tick {
        phase: "walk",
        done: visited.load(Ordering::Relaxed),
        total: 0,
        found: paths.len() as u64,
        current: String::new(),
    });
    let root_canon = fs::canonicalize(root).ok();
    paths.retain(|(p, _)| fs::canonicalize(p).ok() != root_canon);
    paths.sort_unstable();
    paths.dedup();
    paths.truncate(MAX_ITEMS);
    let total = paths.len() as u64;
    let done = AtomicU64::new(0);
    let check = |(p, _): &(PathBuf, Kind)| {
        let item = inspect(p);
        progress(Tick {
            phase: "check",
            done: done.fetch_add(1, Ordering::Relaxed) + 1,
            total,
            found: total,
            current: item.path.clone(),
        });
        item
    };
    // A bounded pool: the global one would start a git process per core, times several calls each.
    let mut trees: Vec<Worktree> = match rayon::ThreadPoolBuilder::new()
        .num_threads(INSPECT_THREADS)
        .build()
    {
        Ok(pool) => pool.install(|| paths.par_iter().map(check).collect()),
        Err(_) => paths.iter().map(check).collect(),
    };
    trees.sort_unstable_by(|a, b| {
        a.allowance
            .cmp(&b.allowance)
            .then_with(|| b.size.cmp(&a.size))
            .then_with(|| a.path.cmp(&b.path))
    });
    trees
}

fn walk(
    dir: &Path,
    protection: &Protection,
    depth: usize,
    in_repo: bool,
    visited: &AtomicU64,
    progress: &(dyn Fn(Tick) + Sync),
) -> Vec<(PathBuf, Kind)> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let n = visited.fetch_add(1, Ordering::Relaxed) + 1;
    if n.is_multiple_of(200) {
        progress(Tick {
            phase: "walk",
            done: n,
            total: 0,
            found: 0,
            current: dir.to_string_lossy().into_owned(),
        });
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let entries: Vec<fs::DirEntry> = entries.flatten().collect();
    let has_git = entries.iter().any(|e| e.file_name() == OsStr::new(".git"));
    // One checkout is one unit: nested repositories inside it are not separate worktrees here.
    if has_git && linked_worktree_admin(dir).is_some() {
        return vec![(dir.to_path_buf(), Kind::Worktree)];
    }
    let mut found = Vec::new();
    let mut inside = in_repo;
    // A main checkout is listed once; below it only linked worktrees are still collected (a
    // repository nested in another one is that repository's own business).
    if has_git && !in_repo && main_git_dir(dir).is_some() {
        found.push((dir.to_path_buf(), Kind::Repository));
        inside = true;
    }
    let subdirs: Vec<PathBuf> = entries
        .iter()
        .filter_map(|entry| {
            // DirEntry metadata never follows a link; junctions and symlinks are skipped.
            let meta = entry.metadata().ok()?;
            if !meta.is_dir() || is_link(&meta) {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if SKIP_DIRS.contains(&name.as_str()) || protection.is_system(&entry.path()) {
                return None;
            }
            Some(entry.path())
        })
        .collect();
    found.extend(
        subdirs
            .par_iter()
            .flat_map_iter(|sub| walk(sub, protection, depth + 1, inside, visited, progress))
            .collect::<Vec<_>>(),
    );
    found
}

/// `<dir>/.git` when it is a real directory (a main checkout), never a link or pointer file.
fn main_git_dir(dir: &Path) -> Option<PathBuf> {
    let git_dir = dir.join(".git");
    let meta = fs::symlink_metadata(&git_dir).ok()?;
    (meta.is_dir() && !is_link(&meta)).then_some(git_dir)
}

/// Admin directory of a linked worktree (`<repo>/.git/worktrees/<id>`, or the same layout under a
/// bare repo), or `None` for a main checkout, a submodule (`.../.git/modules/<id>`) or anything
/// whose back-reference disagrees.
fn linked_worktree_admin(worktree: &Path) -> Option<PathBuf> {
    let git_file = worktree.join(".git");
    let text = read_small_file(&git_file)?;
    let raw = resolve_local(worktree, text.trim().strip_prefix("gitdir:")?.trim())?;
    let meta = fs::symlink_metadata(&raw).ok()?;
    if !meta.is_dir() || is_link(&meta) {
        return None;
    }
    // Git ≥ 2.48 writes `..` with `worktree.useRelativePaths`; canonicalizing here, before the
    // shape check, keeps such checkouts working while a crafted `..` still lands outside
    // `worktrees/<id>` and is refused.
    let admin = fs::canonicalize(&raw).ok()?;
    if admin.parent()?.file_name()? != OsStr::new("worktrees") {
        return None;
    }
    // `git worktree add` writes the checkout path back into the admin dir. Requiring the two to
    // agree keeps a crafted `.git` file from naming metadata we would then remove.
    let back = resolve_local(&admin, read_small_file(&admin.join("gitdir"))?.trim())?;
    same_path(&back, &git_file).then_some(admin)
}

/// Resolves a path written by git. Non-local (UNC/device) prefixes are refused; `..` is allowed
/// because git itself writes it, and every caller canonicalizes before acting on the result.
fn resolve_local(base: &Path, raw: &str) -> Option<PathBuf> {
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    let path = if path.is_absolute() {
        path
    } else {
        base.join(path)
    };
    if !path.is_absolute() {
        return None;
    }
    #[cfg(windows)]
    match path.components().next() {
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => {}
        _ => return None,
    }
    Some(path)
}

fn read_small_file(path: &Path) -> Option<String> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || is_link(&meta) {
        return None;
    }
    // Read from the handle, bounded: a path swapped after the check cannot turn this into an
    // unbounded read.
    let file = fs::File::open(path).ok()?;
    let handle = file.metadata().ok()?;
    if !handle.is_file() || handle.len() > GIT_FILE_LIMIT {
        return None;
    }
    let mut buf = Vec::new();
    (&file)
        .take(GIT_FILE_LIMIT + 1)
        .read_to_end(&mut buf)
        .ok()?;
    if buf.len() as u64 > GIT_FILE_LIMIT {
        return None;
    }
    String::from_utf8(buf).ok()
}

pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// The repository's common dir, as git itself reports it.
fn common_dir(worktree: &Path) -> Option<PathBuf> {
    let raw = git(worktree, &["rev-parse", "--git-common-dir"]).ok()?;
    let path = resolve_local(worktree, raw.trim())?;
    fs::canonicalize(path).ok()
}

/// True only for a real `<common>/worktrees/<id>`: the admin dir must sit directly below the
/// repository's own common dir, so no crafted path can make us move anything else.
fn admin_under_common(admin: &Path, common: &Path) -> bool {
    let (Ok(admin), Ok(common)) = (fs::canonicalize(admin), fs::canonicalize(common)) else {
        return false;
    };
    admin.parent().and_then(Path::parent) == Some(common.as_path())
        && admin.parent().and_then(Path::file_name) == Some(OsStr::new("worktrees"))
}

#[derive(Default)]
struct State {
    branch: String,
    head: String,
    modified: u64,
    ignored: u64,
    /// Commits outside every remote-tracking ref.
    ahead: u64,
    /// Entries git reports as changed (tracked or untracked); ignored ones are counted apart.
    changes: u64,
    /// The checkout sits on a detached HEAD, so only its admin dir names those commits.
    detached: bool,
    /// Why the checkout may not be removed; `None` when every check passed.
    block: Option<Block>,
}

/// One reason a checkout stays on the disk, with how far an explicit opt-in may go against it.
enum Block {
    /// Nothing on this screen overrides it: git or the layout refuses the removal.
    Never(String),
    /// The checkout goes to the trash and the commits stay in the shared repository.
    Recoverable(String),
    /// Something exists nowhere else (a detached HEAD's commits, the submodule history kept in the
    /// admin dir): only the "definitive loss" opt-in may take it.
    Definitive(String),
}

impl Block {
    fn text(&self) -> &str {
        match self {
            Self::Never(text) | Self::Recoverable(text) | Self::Definitive(text) => text,
        }
    }

    fn allowance(&self) -> Allowance {
        match self {
            Self::Never(_) => Allowance::Never,
            Self::Recoverable(_) => Allowance::Recoverable,
            Self::Definitive(_) => Allowance::Definitive,
        }
    }

    /// A harder reason always wins the one line shown on screen.
    fn severity(&self) -> u8 {
        match self {
            Self::Recoverable(_) => 1,
            Self::Definitive(_) => 2,
            Self::Never(_) => 3,
        }
    }
}

impl State {
    /// Keeps the hardest reason; an equal one loses, so the checks below decide the rest of the
    /// order — the most immediate thing to fix comes first.
    fn hold(&mut self, block: Block) {
        let harder = self
            .block
            .as_ref()
            .is_none_or(|current| block.severity() > current.severity());
        if harder {
            self.block = Some(block);
        }
    }

    fn allowance(&self) -> Allowance {
        self.block
            .as_ref()
            .map_or(Allowance::Synced, Block::allowance)
    }
}

/// A checkout whose state could not be established at all: no opt-in may guess it.
fn refused(reason: String) -> State {
    State {
        block: Some(Block::Never(reason)),
        ..State::default()
    }
}

fn inspect(path: &Path) -> Worktree {
    let admin = linked_worktree_admin(path);
    let main = admin.is_none() && main_git_dir(path).is_some();
    let mut state = match (admin.as_deref(), main) {
        (Some(admin), _) => read_state(path, Some(admin)),
        (None, true) => read_state(path, None),
        (None, false) => refused("não é mais um checkout Git".into()),
    };
    // The lock is the one blocker git will not undo on its own; it outranks everything so the
    // reason tells the user exactly what to run.
    if let Some(admin) = admin.as_deref() {
        if admin.join("locked").exists() {
            state.block = Some(Block::Never(
                "worktree travada (use `git worktree unlock`)".to_string(),
            ));
        }
    }
    let allowance = state.allowance();
    let reason = state.block.as_ref().map(|block| block.text().to_string());
    let (size, files) = crate::junk::dir_size(path);
    Worktree {
        path: path.to_string_lossy().into_owned(),
        kind: if main { "repository" } else { "worktree" },
        branch: state.branch,
        head: state.head,
        modified: state.modified,
        size,
        files,
        ignored: state.ignored,
        allowance,
        reason,
        ahead: state.ahead,
        changes: state.changes,
    }
}

/// Removes one worktree the user allowed this much (`opt_in`), taking the checkout and its git
/// metadata to the same destination so a restore from the Recycle Bin keeps the pair usable. A
/// forced removal is still a trash move: the files sit in the Recycle Bin and, for a checkout on a
/// branch, the commits stay in the shared repository. This screen has no permanent-delete option.
pub fn remove(path: &Path, opt_in: OptIn) -> Result<Outcome, String> {
    let Some(admin) = linked_worktree_admin(path) else {
        return remove_repository(path, opt_in);
    };
    if admin.join("locked").exists() {
        return Err("worktree travada: rode `git worktree unlock` antes".into());
    }
    let common =
        common_dir(path).ok_or("não foi possível localizar o repositório desta worktree")?;
    if !admin_under_common(&admin, &common) {
        return Err("os metadados desta worktree não estão no repositório esperado".into());
    }
    // Re-checked here, not only at search time: a commit or a local edit may have appeared since,
    // and the opt-in only travels as far as the user actually granted.
    let state = read_state(path, Some(&admin));
    permitted(
        state.allowance(),
        opt_in,
        state.block.as_ref().map(Block::text),
    )?;
    let (size, _) = crate::junk::dir_size(path);
    let targets = [path.to_path_buf()];
    let removal = crate::remove::remove_all(&targets, false)
        .pop()
        .ok_or("nada para remover")?;
    removal.result?;

    // The checkout is gone, so git can no longer describe the admin dir; the path was validated
    // above and is checked once more (cheaply) before it moves.
    let mut warnings = Vec::new();
    if !admin_under_common(&admin, &common) {
        warnings.push(format!(
            "checkout removido, mas os metadados do git ficaram em {}; rode `git worktree prune`",
            admin.display()
        ));
        return Ok(Outcome {
            freed: size,
            warnings,
        });
    }
    let admin_targets = [admin.clone()];
    let admin_removal = crate::remove::remove_all(&admin_targets, false)
        .pop()
        .ok_or("nada para remover")?;
    if let Err(error) = admin_removal.result {
        warnings.push(format!(
            "checkout removido, mas os metadados do git ficaram em {} ({error}); rode `git worktree prune`",
            admin.display()
        ));
    }
    Ok(Outcome {
        freed: size,
        warnings,
    })
}

/// Removes a main checkout (its `.git` included) to the trash once the user allowed what git
/// reports, again, would be lost. The whole folder moves together, so branches, tags and the stash
/// come back with a restore of the Recycle Bin.
fn remove_repository(path: &Path, opt_in: OptIn) -> Result<Outcome, String> {
    main_git_dir(path).ok_or("não é mais um checkout Git")?;
    let state = read_state(path, None);
    permitted(
        state.allowance(),
        opt_in,
        state.block.as_ref().map(Block::text),
    )?;
    let (size, _) = crate::junk::dir_size(path);
    let targets = [path.to_path_buf()];
    crate::remove::remove_all(&targets, false)
        .pop()
        .ok_or("nada para remover")?
        .result?;
    Ok(Outcome {
        freed: size,
        warnings: Vec::new(),
    })
}

/// Everything git has to confirm before a checkout may leave the disk. The reasons are shown in
/// the UI, so they say what to do next; branch, head and the counters are filled even when a check
/// holds the removal back, so a kept worktree is still identified on screen.
fn read_state(path: &Path, admin: Option<&Path>) -> State {
    let top = match git(path, &["rev-parse", "--show-toplevel"]) {
        Ok(top) => top,
        Err(reason) => return refused(reason),
    };
    let mut state = State::default();
    if !same_path(Path::new(top.trim()), path) {
        return refused("o .git aponta para outro checkout".into());
    }
    let Some(common) = common_dir(path) else {
        return refused("não foi possível localizar o repositório desta worktree".into());
    };
    let consistent = match admin {
        Some(admin) => admin_under_common(admin, &common),
        None => same_path(&common, &path.join(".git")),
    };
    if !consistent {
        return refused("os metadados deste checkout não estão no repositório esperado".into());
    }
    // A filter declared by the repository would run as a command during `git status`; refuse
    // before any command that converts file contents.
    match git(path, &["config", "--list", "--show-scope"]) {
        Ok(list) if declares_filter(&list) => {
            return refused(
                "o repositório define filtros Git (filter.*): confira à mão antes".into(),
            );
        }
        Ok(_) => {}
        Err(reason) => return refused(reason),
    }
    let status = match git(
        path,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=normal",
            "--ignored=traditional",
        ],
    ) {
        Ok(status) => status,
        Err(reason) => return refused(reason),
    };
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            state.detached = value == "(detached)";
            state.branch = if state.detached {
                "HEAD destacado".to_string()
            } else {
                value.to_string()
            };
        } else if let Some(value) = line.strip_prefix("# branch.oid ") {
            if value != "(initial)" {
                state.head = value.chars().take(8).collect();
            }
        } else if line.starts_with('!') {
            // Porcelain v2 marks ignored entries with `! <path>` (`!!` is v1).
            state.ignored += 1;
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            state.changes += 1;
        }
    }
    if let Ok(last) = git(path, &["log", "-1", "--format=%ct"]) {
        state.modified = last.trim().parse().unwrap_or(0);
    }
    // Uncommitted files come first: they only exist in this checkout until someone commits them,
    // and the trash is where they end up.
    if state.changes > 0 {
        state.hold(Block::Recoverable(if state.changes == 1 {
            "tem 1 alteração local (não commitada)".to_string()
        } else {
            format!("tem {} alterações locais (não commitadas)", state.changes)
        }));
    }
    // For a linked worktree, submodule git dirs live below its admin dir; moving that dir takes
    // commits that only exist there. A main checkout carries its own `.git`, so a restore of the
    // trash brings the whole thing back.
    let modules = admin.unwrap_or(&common).join("modules");
    if modules.is_dir() {
        state.hold(if admin.is_some() {
            Block::Definitive(
                "tem submódulos inicializados: o histórico deles existe só nos metadados desta worktree"
                    .to_string(),
            )
        } else {
            Block::Recoverable(
                "tem submódulos inicializados: o histórico deles vai junto para a Lixeira".to_string(),
            )
        });
    }
    if admin.is_none() {
        let linked = fs::read_dir(common.join("worktrees")).is_ok_and(|mut d| d.next().is_some());
        if linked {
            state.hold(Block::Definitive(
                "tem worktrees vinculadas: elas perdem os metadados (`git worktree list`)".into(),
            ));
        }
        if git(path, &["rev-parse", "--verify", "--quiet", "refs/stash"]).is_ok() {
            state.hold(Block::Recoverable(
                "tem alterações guardadas no stash (saem junto com o .git, para a Lixeira)".into(),
            ));
        }
    }
    match git(
        path,
        &[
            "for-each-ref",
            "--count=1",
            "--format=%(refname)",
            "refs/remotes",
        ],
    ) {
        Ok(remotes) if remotes.trim().is_empty() => state.hold(Block::Recoverable(
            "sem referência remota: nenhum fetch foi feito".into(),
        )),
        Ok(_) => {}
        Err(reason) => state.hold(Block::Never(reason)),
    }
    if state.head.is_empty() {
        state.hold(Block::Recoverable(
            "o repositório ainda não tem commits".into(),
        ));
        return state;
    }
    // A main checkout owns every branch and tag, so all of them must be on the remote; a linked
    // worktree only answers for its own HEAD (the branches stay in the main repository).
    let scope = if admin.is_none() { "--all" } else { "HEAD" };
    match git(path, &["rev-list", "--count", scope, "--not", "--remotes"]) {
        Ok(ahead) => match ahead.trim().parse::<u64>() {
            Ok(count) => {
                state.ahead = count;
                if count > 0 {
                    // Without a branch, the admin dir is the only thing naming these commits: once
                    // it leaves, nothing local reaches them again until git collects them.
                    state.hold(if state.detached {
                        Block::Definitive(format!(
                            "HEAD destacado: {count} commit(s) fora do remoto sem branch apontando para eles"
                        ))
                    } else if count == 1 {
                        Block::Recoverable("1 commit local fora do remoto".into())
                    } else {
                        Block::Recoverable(format!("{count} commits locais fora do remoto"))
                    });
                }
            }
            Err(_) => state.hold(Block::Never("git devolveu uma contagem inesperada".into())),
        },
        Err(reason) => state.hold(Block::Never(reason)),
    }
    state
}

/// True when the repository itself (local or worktree scope) defines a filter command. Global
/// config is not considered: it belongs to the machine, not to the scanned checkout.
fn declares_filter(config: &str) -> bool {
    config.lines().any(|line| {
        let Some((scope, rest)) = line.split_once('\t') else {
            return false;
        };
        if !matches!(scope, "local" | "worktree") {
            return false;
        }
        let key = rest
            .split('=')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        key.starts_with("filter.")
            && (key.ends_with(".clean") || key.ends_with(".process") || key.ends_with(".smudge"))
    })
}

/// Read-only git call with a minimal environment: inherited `GIT_*` variables could otherwise
/// point the checks at another repository, and `core.fsmonitor` would run a command from the
/// scanned checkout. `GIT_OPTIONAL_LOCKS=0` keeps a search from writing the index.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.env_clear();
    for key in [
        "PATH",
        "SystemRoot",
        "windir",
        "ComSpec",
        "PATHEXT",
        "TEMP",
        "TMP",
        // Machine-level config stays in play so `core.autocrlf` and `filter.lfs` behave as in the
        // user's own git; a repository can only *name* a filter, never provide the program.
        "HOME",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "XDG_CONFIG_HOME",
    ] {
        if let Some(value) = env::var_os(key) {
            command.env(key, value);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .arg("--no-pager")
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-c")
        .arg("core.useBuiltinFSMonitor=false")
        // Signature display would make `git log` spawn the repository's `gpg.program`.
        .arg("-c")
        .arg("log.showSignature=false")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "git não encontrado no PATH".to_string()
            } else {
                format!("git não executou: {e}")
            }
        })?;
    // Drain both pipes while the child runs: a full pipe would block it and the deadline would
    // fire on a process that is merely talking. The channel bounds the wait even when a
    // grandchild keeps the pipe open after the child is killed.
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out = drain(stdout);
    let err = drain(stderr);
    let deadline = Instant::now() + GIT_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("git não respondeu a tempo".to_string());
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("git não executou: {e}"));
            }
        }
    };
    let stdout = out.recv_timeout(Duration::from_secs(1)).unwrap_or_default();
    let stderr = err.recv_timeout(Duration::from_secs(1)).unwrap_or_default();
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr);
        let first = stderr.lines().next().unwrap_or("").trim();
        return Err(if first.is_empty() {
            "git recusou o repositório".to_string()
        } else if first.contains("dubious ownership") {
            "git recusou o repositório: dono da pasta diferente do usuário atual".to_string()
        } else {
            format!("git recusou o repositório: {first}")
        });
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

/// Reads a child pipe on its own thread; the receiver stops waiting even if a descendant keeps
/// the pipe open.
fn drain(mut pipe: impl Read + Send + 'static) -> mpsc::Receiver<Vec<u8>> {
    let (send, recv) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        let _ = send.send(buf);
    });
    recv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    /// Runs git with an identity of its own: CI runners have no global user configured.
    fn git_ok(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=Faxina Test",
                "-c",
                "user.email=faxina@example.com",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn repo_with_remote(root: &Path) -> PathBuf {
        let origin = root.join("origin.git");
        git_ok(root, &["init", "--bare", origin.to_str().unwrap()]);
        let repo = root.join("repo");
        git_ok(root, &["init", repo.to_str().unwrap()]);
        fs::write(repo.join("README.md"), "main\n").unwrap();
        git_ok(&repo, &["add", "."]);
        git_ok(&repo, &["commit", "-m", "initial"]);
        git_ok(
            &repo,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        git_ok(&repo, &["push", "-u", "origin", "main"]);
        repo
    }

    fn worktree(repo: &Path, path: &Path, branch: &str) {
        git_ok(
            repo,
            &["worktree", "add", "-b", branch, path.to_str().unwrap()],
        );
    }

    fn by_name<'a>(found: &'a [Worktree], name: &str) -> &'a Worktree {
        found
            .iter()
            .find(|w| Path::new(&w.path).file_name() == Some(OsStr::new(name)))
            .unwrap_or_else(|| panic!("{name} not found in {found:?}"))
    }

    #[test]
    fn each_reason_to_hold_a_checkout_back_gets_its_own_opt_in() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());

        let synced = volume.path().join("synced");
        worktree(&repo, &synced, "feature/synced");
        fs::write(synced.join("work.txt"), "done\n").unwrap();
        fs::write(synced.join(".gitignore"), ".env\n").unwrap();
        git_ok(&synced, &["add", "."]);
        git_ok(&synced, &["commit", "-m", "work"]);
        git_ok(&synced, &["push", "-u", "origin", "feature/synced"]);
        // Ignored content is not tracked work: it leaves with the checkout, in the trash.
        fs::write(synced.join(".env"), "SECRET=1\n").unwrap();

        let pending = volume.path().join("pending");
        worktree(&repo, &pending, "feature/pending");
        fs::write(pending.join("work.txt"), "later\n").unwrap();
        git_ok(&pending, &["add", "."]);
        git_ok(&pending, &["commit", "-m", "not pushed"]);

        let dirty = volume.path().join("dirty");
        worktree(&repo, &dirty, "feature/dirty");
        fs::write(dirty.join("scratch.txt"), "unsaved\n").unwrap();

        let stashed = volume.path().join("stashed");
        worktree(&repo, &stashed, "feature/stashed");
        fs::write(stashed.join("README.md"), "changed\n").unwrap();
        git_ok(&stashed, &["stash", "push", "-m", "wip"]);

        // The commit is made *while* detached, so only the admin dir names it.
        let detached = volume.path().join("detached");
        worktree(&repo, &detached, "feature/detached");
        git_ok(&detached, &["checkout", "--detach"]);
        fs::write(detached.join("loose.txt"), "nowhere else\n").unwrap();
        git_ok(&detached, &["add", "."]);
        git_ok(&detached, &["commit", "-m", "detached work"]);

        // A repo with no remote at all must not look synced just because `--not --remotes` is 0.
        let local = volume.path().join("local");
        git_ok(volume.path(), &["init", local.to_str().unwrap()]);
        fs::write(local.join("a.txt"), "local\n").unwrap();
        git_ok(&local, &["add", "."]);
        git_ok(&local, &["commit", "-m", "local"]);
        let lonely = volume.path().join("lonely");
        worktree(&local, &lonely, "feature/lonely");

        let found = find(volume.path(), &|_| {});
        let ok = by_name(&found, "synced");
        assert_eq!(ok.allowance, Allowance::Synced, "{:?}", ok.reason);
        assert!(ok.reason.is_none());
        assert_eq!(ok.branch, "feature/synced");
        assert!(!ok.head.is_empty() && ok.modified > 0);
        assert!(ok.size > 0 && ok.files > 0);
        assert_eq!(ok.ignored, 1, "the ignored .env is reported");
        assert_eq!((ok.ahead, ok.changes), (0, 0));

        // Stashes live in the common git dir and survive the removal: they do not hold it back.
        assert_eq!(by_name(&found, "stashed").allowance, Allowance::Synced);

        let blocked = by_name(&found, "lonely");
        assert_eq!(blocked.allowance, Allowance::Recoverable);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("sem referência remota"));

        let blocked = by_name(&found, "pending");
        assert_eq!(blocked.allowance, Allowance::Recoverable);
        assert_eq!(blocked.ahead, 1);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("fora do remoto"));
        // A held-back checkout is still identified on screen.
        assert_eq!(blocked.branch, "feature/pending");
        assert!(!blocked.head.is_empty());

        let blocked = by_name(&found, "dirty");
        assert_eq!(blocked.allowance, Allowance::Recoverable);
        assert_eq!(blocked.changes, 1);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("alteração local"));

        let blocked = by_name(&found, "detached");
        assert_eq!(blocked.allowance, Allowance::Definitive);
        assert_eq!(blocked.ahead, 1);
        assert!(blocked.reason.as_deref().unwrap().contains("destacado"));

        // The main checkout is listed as a repository, and its linked worktrees hold it back.
        let main = by_name(&found, "repo");
        assert_eq!(main.kind, "repository");
        assert_eq!(main.allowance, Allowance::Definitive);
        assert!(main
            .reason
            .as_deref()
            .unwrap()
            .contains("worktrees vinculadas"));
        // Searching from inside a worktree never offers that same folder for deletion.
        assert!(find(&synced, &|_| {}).is_empty());
    }

    #[test]
    fn a_local_opt_in_takes_unpushed_work_and_keeps_the_branch_in_the_repository() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let pending = volume.path().join("pending");
        worktree(&repo, &pending, "feature/pending");
        fs::write(pending.join("work.txt"), "later\n").unwrap();
        git_ok(&pending, &["add", "."]);
        git_ok(&pending, &["commit", "-m", "not pushed"]);
        let admin = linked_worktree_admin(&pending).unwrap();

        let refused = remove(&pending, OptIn::Synced).unwrap_err();
        assert!(refused.contains("fora do remoto"), "{refused}");
        assert!(
            pending.exists() && admin.exists(),
            "a refusal removes nothing"
        );

        let outcome = remove(&pending, OptIn::Local).unwrap();
        assert!(outcome.freed > 0);
        assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
        assert!(!pending.exists() && !admin.exists());
        // The branch and its commit stay in the shared repository: only the checkout left.
        let kept = git_ok(&repo, &["log", "-1", "--format=%s", "feature/pending"]);
        assert_eq!(kept.trim(), "not pushed");
    }

    #[test]
    fn uncommitted_files_leave_only_with_the_opt_in() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("dirty");
        worktree(&repo, &checkout, "feature/dirty");
        fs::write(checkout.join("scratch.txt"), "unsaved\n").unwrap();

        let refused = remove(&checkout, OptIn::Synced).unwrap_err();
        assert!(refused.contains("alteração local"), "{refused}");
        assert!(checkout.exists(), "a refusal removes nothing");

        assert!(remove(&checkout, OptIn::Local).unwrap().freed > 0);
        assert!(!checkout.exists());
    }

    #[test]
    fn work_that_exists_nowhere_else_needs_the_definitive_opt_in() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("detached");
        worktree(&repo, &checkout, "feature/detached");
        git_ok(&checkout, &["checkout", "--detach"]);
        fs::write(checkout.join("loose.txt"), "nowhere else\n").unwrap();
        git_ok(&checkout, &["add", "."]);
        git_ok(&checkout, &["commit", "-m", "detached work"]);

        assert_eq!(
            by_name(&find(volume.path(), &|_| {}), "detached").allowance,
            Allowance::Definitive
        );
        let refused = remove(&checkout, OptIn::Local).unwrap_err();
        assert!(refused.contains("destacado"), "{refused}");
        assert!(checkout.exists(), "a refusal removes nothing");

        assert!(remove(&checkout, OptIn::Definitive).unwrap().freed > 0);
        assert!(!checkout.exists());
    }

    #[test]
    fn removal_refuses_changes_made_after_the_search_then_cleans_the_git_metadata() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("checkout");
        worktree(&repo, &checkout, "feature/remove");
        fs::write(checkout.join("work.txt"), "done\n").unwrap();
        git_ok(&checkout, &["add", "."]);
        git_ok(&checkout, &["commit", "-m", "work"]);
        git_ok(&checkout, &["push", "-u", "origin", "feature/remove"]);

        let admin = linked_worktree_admin(&checkout).unwrap();
        assert!(admin.is_dir());

        // A new local file after the scan is exactly what the second check must catch: the opt-in
        // travels with the request, it never becomes a blanket permission.
        fs::write(checkout.join("late.txt"), "not committed\n").unwrap();
        let refused = remove(&checkout, OptIn::Synced).unwrap_err();
        assert!(refused.contains("alteração local"), "{refused}");
        assert!(checkout.exists(), "nothing may be removed on a refusal");

        fs::remove_file(checkout.join("late.txt")).unwrap();
        let outcome = remove(&checkout, OptIn::Synced).unwrap();
        assert!(outcome.freed > 0);
        assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
        assert!(!checkout.exists());
        assert!(
            !admin.exists(),
            "the worktree entry must not stay as a ghost"
        );
        let list = git_ok(&repo, &["worktree", "list", "--porcelain"]);
        assert!(!list.contains("checkout"), "{list}");
    }

    #[test]
    fn a_locked_worktree_is_never_removed() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("locked");
        worktree(&repo, &checkout, "feature/locked");
        git_ok(&repo, &["worktree", "lock", checkout.to_str().unwrap()]);

        let found = find(volume.path(), &|_| {});
        let item = by_name(&found, "locked");
        assert_eq!(item.allowance, Allowance::Never);
        assert!(item.reason.as_deref().unwrap().contains("travada"));
        // A lock is git's own refusal: no opt-in passes it.
        assert!(remove(&checkout, OptIn::Definitive).is_err());
        assert!(checkout.exists());
    }

    #[test]
    fn worktrees_with_initialized_submodules_need_the_definitive_opt_in() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("with-submodule");
        worktree(&repo, &checkout, "feature/submodule");
        let admin = linked_worktree_admin(&checkout).unwrap();
        // A linked worktree keeps its submodule git dirs below the admin dir; moving it would
        // destroy commits that exist nowhere else.
        fs::create_dir_all(admin.join("modules/lib")).unwrap();

        let found = find(volume.path(), &|_| {});
        let item = by_name(&found, "with-submodule");
        assert_eq!(item.allowance, Allowance::Definitive);
        assert!(item.reason.as_deref().unwrap().contains("submódulos"));
        let refused = remove(&checkout, OptIn::Local).unwrap_err();
        assert!(refused.contains("submódulos"), "{refused}");
        assert!(checkout.exists() && admin.join("modules/lib").exists());

        assert!(remove(&checkout, OptIn::Definitive).unwrap().freed > 0);
        assert!(!checkout.exists());
    }

    #[test]
    fn repository_declared_filters_are_refused_without_running_them() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("filtered");
        worktree(&repo, &checkout, "feature/filtered");
        // If the check ever runs git status here, this command would leave a marker behind.
        git_ok(
            &checkout,
            &["config", "filter.evil.clean", "echo ran > filter-ran.txt"],
        );
        // A filter only runs for a file whose attributes select it; without this file the
        // assertion below would prove nothing.
        fs::write(checkout.join(".gitattributes"), "* filter=evil\n").unwrap();
        fs::write(checkout.join("tracked.txt"), "content\n").unwrap();
        git_ok(&checkout, &["add", ".gitattributes", "tracked.txt"]);
        git_ok(&checkout, &["commit", "-m", "attributes"]);
        let marker = checkout.join("filter-ran.txt");
        // Rewriting the file makes git convert it again, which is when the filter would run.
        fs::write(checkout.join("tracked.txt"), "changed content\n").unwrap();

        // `git add`/`commit` already ran the filter once: start the control from a clean slate.
        let _ = fs::remove_file(&marker);
        // Control: with the guard out of the way, `git status` really does run the filter.
        let status = Command::new("git")
            .arg("-C")
            .arg(&checkout)
            .args(["-c", "core.fsmonitor=false", "status"])
            .output()
            .unwrap();
        assert!(status.status.success());
        assert!(
            marker.exists(),
            "the filter must run when git status converts"
        );
        fs::remove_file(&marker).unwrap();

        let found = find(volume.path(), &|_| {});
        let item = by_name(&found, "filtered");
        assert_eq!(item.allowance, Allowance::Never);
        assert!(item.reason.as_deref().unwrap().contains("filtros"));
        // The filter would run during the removal check, so no opt-in may reach it.
        assert!(remove(&checkout, OptIn::Definitive).is_err());
        assert!(!marker.exists(), "the filter command must never run");
    }

    #[test]
    fn removal_goes_to_the_trash_by_default() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("trash-me");
        worktree(&repo, &checkout, "feature/trash");
        let admin = linked_worktree_admin(&checkout).unwrap();

        let outcome = remove(&checkout, OptIn::Synced).unwrap();
        assert!(outcome.freed > 0);
        assert!(!checkout.exists(), "the checkout must leave the disk");
        assert!(!admin.exists(), "the git entry must not stay as a ghost");
    }

    #[test]
    fn a_crafted_parent_dir_pointer_is_not_a_worktree() {
        let volume = tempfile::tempdir().unwrap();
        let checkout = volume.path().join("wt");
        fs::create_dir_all(&checkout).unwrap();
        // `..` would make the admin path resolve to the scanned folder itself.
        fs::write(checkout.join(".git"), "gitdir: ../worktrees/..\n").unwrap();
        fs::create_dir_all(volume.path().join("worktrees")).unwrap();
        fs::write(
            volume.path().join("worktrees/gitdir"),
            format!("{}\n", checkout.join(".git").display()),
        )
        .unwrap();

        assert!(linked_worktree_admin(&checkout).is_none());
        assert!(find(volume.path(), &|_| {}).is_empty());
        assert!(checkout.exists());
    }

    #[test]
    fn relative_gitdir_pointers_are_still_worktrees() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let repo = repo_with_remote(volume.path());
        let checkout = volume.path().join("relative");
        // Git ≥ 2.48 can write `gitdir: ../../repo/.git/worktrees/<id>`; older versions ignore
        // the setting, and the test then has nothing to check.
        let output = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args([
                "-c",
                "worktree.useRelativePaths=true",
                "worktree",
                "add",
                "-b",
                "feature/relative",
                checkout.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        if !output.status.success() {
            return;
        }
        let pointer = fs::read_to_string(checkout.join(".git")).unwrap();
        if !pointer.contains("..") {
            return;
        }

        let found = find(volume.path(), &|_| {});
        let item = by_name(&found, "relative");
        assert_eq!(item.branch, "feature/relative");
    }

    #[cfg(windows)]
    #[test]
    fn unc_and_device_paths_are_not_resolved() {
        let base = Path::new(r"C:\repo");
        assert!(resolve_local(base, r"\\attacker\share\worktrees\x").is_none());
        assert!(resolve_local(base, r"\\?\UNC\attacker\share\worktrees\x").is_none());
        assert!(resolve_local(base, r"C:\repo\.git\worktrees\x").is_some());
    }

    #[test]
    fn submodules_and_crafted_pointers_are_not_worktrees() {
        let volume = tempfile::tempdir().unwrap();
        // A submodule stores its git dir under `.git/modules/`, never `.git/worktrees/`.
        let admin = volume.path().join("super/.git/modules/lib");
        fs::create_dir_all(&admin).unwrap();
        let checkout = volume.path().join("super/lib");
        fs::create_dir_all(&checkout).unwrap();
        let git_file = checkout.join(".git");
        fs::write(&git_file, format!("gitdir: {}\n", admin.display())).unwrap();
        fs::write(admin.join("gitdir"), format!("{}\n", git_file.display())).unwrap();

        // A `.git` file pointing into `worktrees/` whose admin dir points somewhere else is not
        // trusted: removing that metadata would hit another checkout.
        let other_admin = volume.path().join("other/.git/worktrees/fake");
        fs::create_dir_all(&other_admin).unwrap();
        let crafted = volume.path().join("crafted");
        fs::create_dir_all(&crafted).unwrap();
        fs::write(
            crafted.join(".git"),
            format!("gitdir: {}\n", other_admin.display()),
        )
        .unwrap();
        fs::write(
            other_admin.join("gitdir"),
            format!("{}\n", checkout.join(".git").display()),
        )
        .unwrap();

        let found = find(volume.path(), &|_| {});
        assert!(found.iter().all(|w| w.kind != "worktree"), "{found:?}");
        assert!(
            found.iter().all(|w| w.allowance != Allowance::Synced),
            "{found:?}"
        );
    }

    #[test]
    fn a_repository_is_offered_with_an_opt_in_when_a_branch_is_not_on_the_remote() {
        if !available() {
            return;
        }
        let volume = tempfile::tempdir().unwrap();
        let work = volume.path().join("work");
        fs::create_dir_all(&work).unwrap();
        let repo = repo_with_remote(&work);
        // `repo_with_remote` also made the bare origin next to it; keep only repos in the search.
        let found = find(&work, &|_| {});
        let listed = by_name(&found, "repo");
        assert_eq!(listed.kind, "repository");
        assert_eq!(listed.allowance, Allowance::Synced, "{listed:?}");

        // A local branch that was never pushed: the whole `.git` goes to the trash on removal, so
        // a restore brings the branch back.
        git_ok(&repo, &["checkout", "-b", "wip"]);
        fs::write(repo.join("wip.txt"), "x\n").unwrap();
        git_ok(&repo, &["add", "."]);
        git_ok(&repo, &["commit", "-m", "wip"]);
        git_ok(&repo, &["checkout", "main"]);
        let found = find(&work, &|_| {});
        let listed = by_name(&found, "repo");
        assert_eq!(listed.allowance, Allowance::Recoverable);
        assert_eq!(listed.ahead, 1);
        assert!(listed.reason.as_deref().unwrap().contains("fora do remoto"));
        assert!(remove(&repo, OptIn::Synced).is_err());
        assert!(repo.exists());

        // Untracked files are the first thing the reason mentions, and are still recoverable.
        fs::write(repo.join("scratch.txt"), "x\n").unwrap();
        let found = find(&work, &|_| {});
        assert!(by_name(&found, "repo")
            .reason
            .as_deref()
            .unwrap()
            .contains("alteração local"));
        assert!(remove(&repo, OptIn::Synced).is_err());
        fs::remove_file(repo.join("scratch.txt")).unwrap();

        let outcome = remove(&repo, OptIn::Local).unwrap();
        assert!(outcome.freed > 0);
        assert!(!repo.exists(), "the repository must leave the disk");
    }
}
