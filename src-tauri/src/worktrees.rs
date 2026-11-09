//! Linked Git worktrees whose work already lives on the remote. Discovery only follows a `.git`
//! *file* that points into `<repo>/.git/worktrees/<id>` (submodules use `modules/`, main checkouts
//! use a `.git` directory); the "safe to remove" verdict comes from git itself, never from a
//! folder name: clean tree, no commit outside a remote-tracking ref and no lock.
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
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use serde::Serialize;

use crate::guard::Protection;
use crate::scan::{is_link, MAX_DEPTH};

const MAX_ITEMS: usize = 500;
/// A `.git` pointer file is a few dozen bytes; anything huge is not git's.
const GIT_FILE_LIMIT: u64 = 4096;
/// A hostile checkout (pipe-shaped config, pathologically slow index) must not hang a search.
const GIT_TIMEOUT: Duration = Duration::from_secs(30);

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    pub branch: String,
    pub head: String,
    /// Last commit, seconds since the epoch.
    pub modified: u64,
    pub size: u64,
    pub files: u64,
    /// Ignored entries that would leave with the checkout (build output, dependencies, `.env`).
    /// They are not part of the remote history, but they are what the trash is for.
    pub ignored: u64,
    /// Every commit of the checkout is on a remote-tracking ref, nothing tracked would be lost and
    /// the worktree is not locked: it can be removed.
    pub synced: bool,
    /// Why it cannot be removed; `None` when `synced`.
    pub reason: Option<String>,
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

/// Linked worktrees under `root`, safe-to-remove first, then biggest. `root` itself is never
/// listed: the screen only offers folders below the one the user chose to search.
pub fn find(root: &Path) -> Vec<Worktree> {
    let protection = Protection::new();
    let mut paths = walk(root, &protection, 0);
    let root_canon = fs::canonicalize(root).ok();
    paths.retain(|p| fs::canonicalize(p).ok() != root_canon);
    paths.sort_unstable();
    paths.dedup();
    paths.truncate(MAX_ITEMS);
    let mut trees: Vec<Worktree> = paths.par_iter().map(|p| inspect(p)).collect();
    trees.sort_unstable_by(|a, b| {
        b.synced
            .cmp(&a.synced)
            .then_with(|| b.size.cmp(&a.size))
            .then_with(|| a.path.cmp(&b.path))
    });
    trees
}

fn walk(dir: &Path, protection: &Protection, depth: usize) -> Vec<PathBuf> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let entries: Vec<fs::DirEntry> = entries.flatten().collect();
    // One checkout is one unit: nested repositories inside it are not separate worktrees here.
    if entries.iter().any(|e| e.file_name() == OsStr::new(".git"))
        && linked_worktree_admin(dir).is_some()
    {
        return vec![dir.to_path_buf()];
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
    subdirs
        .par_iter()
        .flat_map_iter(|sub| walk(sub, protection, depth + 1))
        .collect()
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
    /// Why the checkout may not be removed; `None` when every check passed.
    blocked: Option<String>,
}

fn inspect(path: &Path) -> Worktree {
    let admin = linked_worktree_admin(path);
    let state = match admin.as_deref() {
        Some(admin) => read_state(path, admin),
        None => State {
            blocked: Some("não é mais uma worktree vinculada".into()),
            ..State::default()
        },
    };
    // The lock is the one blocker git will not undo on its own; keep it first so the reason tells
    // the user exactly what to run.
    let reason = match admin.as_deref() {
        Some(admin) if admin.join("locked").exists() => {
            Some("worktree travada (use `git worktree unlock`)".to_string())
        }
        _ => state.blocked,
    };
    let (size, files) = crate::junk::dir_size(path);
    Worktree {
        path: path.to_string_lossy().into_owned(),
        branch: state.branch,
        head: state.head,
        modified: state.modified,
        size,
        files,
        ignored: state.ignored,
        synced: reason.is_none(),
        reason,
    }
}

/// Removes one worktree that still passes every check and takes its now-empty admin directory to
/// the same destination, so a restore from the Recycle Bin keeps the pair usable. This screen has
/// no permanent-delete option: both halves always go to the trash.
pub fn remove(path: &Path) -> Result<Outcome, String> {
    let admin = linked_worktree_admin(path).ok_or("não é mais uma worktree vinculada")?;
    if admin.join("locked").exists() {
        return Err("worktree travada: rode `git worktree unlock` antes".into());
    }
    let common =
        common_dir(path).ok_or("não foi possível localizar o repositório desta worktree")?;
    if !admin_under_common(&admin, &common) {
        return Err("os metadados desta worktree não estão no repositório esperado".into());
    }
    // Re-checked here, not only at search time: a commit or a local edit may have appeared since.
    if let Some(reason) = read_state(path, &admin).blocked {
        return Err(reason);
    }
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

/// Everything git has to confirm before a checkout may leave the disk. The reasons are shown in
/// the UI, so they say what to do next; branch and head are filled even when a check blocks the
/// removal, so a kept worktree is still identified on screen.
fn read_state(path: &Path, admin: &Path) -> State {
    let top = match git(path, &["rev-parse", "--show-toplevel"]) {
        Ok(top) => top,
        Err(reason) => {
            return State {
                blocked: Some(reason),
                ..State::default()
            }
        }
    };
    let mut state = State::default();
    if !same_path(Path::new(top.trim()), path) {
        state.blocked = Some("o .git aponta para outro checkout".into());
        return state;
    }
    let Some(common) = common_dir(path) else {
        state.blocked = Some("não foi possível localizar o repositório desta worktree".into());
        return state;
    };
    if !admin_under_common(admin, &common) {
        state.blocked =
            Some("os metadados desta worktree não estão no repositório esperado".into());
        return state;
    }
    // A filter declared by the repository would run as a command during `git status`; refuse
    // before any command that converts file contents.
    match git(path, &["config", "--list", "--show-scope"]) {
        Ok(list) if declares_filter(&list) => {
            state.blocked =
                Some("o repositório define filtros Git (filter.*): confira à mão antes".into());
            return state;
        }
        Ok(_) => {}
        Err(reason) => {
            state.blocked = Some(reason);
            return state;
        }
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
        Err(reason) => {
            state.blocked = Some(reason);
            return state;
        }
    };
    let mut dirty = false;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            state.branch = value.to_string();
        } else if let Some(value) = line.strip_prefix("# branch.oid ") {
            if value != "(initial)" {
                state.head = value.chars().take(8).collect();
            }
        } else if line.starts_with('!') {
            // Porcelain v2 marks ignored entries with `! <path>` (`!!` is v1).
            state.ignored += 1;
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            dirty = true;
        }
    }
    if let Ok(last) = git(path, &["log", "-1", "--format=%ct"]) {
        state.modified = last.trim().parse().unwrap_or(0);
    }
    if dirty {
        state.blocked = Some("tem alterações locais (não commitadas)".into());
        return state;
    }
    if state.head.is_empty() {
        state.blocked = Some("o repositório ainda não tem commits".into());
        return state;
    }
    // For a linked worktree, submodule git dirs live below its admin dir; moving that dir would
    // destroy commits that only exist there (git itself refuses such checkouts too).
    if admin.join("modules").is_dir() {
        state.blocked =
            Some("tem submódulos inicializados: confira com `git status` no checkout".into());
        return state;
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
        Ok(remotes) if remotes.trim().is_empty() => {
            state.blocked = Some("sem referência remota: nenhum fetch foi feito".into());
        }
        Ok(_) => {}
        Err(reason) => state.blocked = Some(reason),
    }
    if state.blocked.is_some() {
        return state;
    }
    match git(path, &["rev-list", "--count", "HEAD", "--not", "--remotes"]) {
        Ok(ahead) => match ahead.trim().parse::<u64>() {
            Ok(0) => {}
            Ok(1) => state.blocked = Some("1 commit local fora do remoto".into()),
            Ok(n) => state.blocked = Some(format!("{n} commits locais fora do remoto")),
            Err(_) => state.blocked = Some("git devolveu uma contagem inesperada".into()),
        },
        Err(reason) => state.blocked = Some(reason),
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
    fn only_checkouts_with_every_commit_on_the_remote_are_synced() {
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

        // A repo with no remote at all must not look synced just because `--not --remotes` is 0.
        let local = volume.path().join("local");
        git_ok(volume.path(), &["init", local.to_str().unwrap()]);
        fs::write(local.join("a.txt"), "local\n").unwrap();
        git_ok(&local, &["add", "."]);
        git_ok(&local, &["commit", "-m", "local"]);
        let lonely = volume.path().join("lonely");
        worktree(&local, &lonely, "feature/lonely");

        let found = find(volume.path());
        let ok = by_name(&found, "synced");
        assert!(ok.synced, "{}", ok.reason.as_deref().unwrap_or(""));
        assert_eq!(ok.branch, "feature/synced");
        assert!(!ok.head.is_empty() && ok.modified > 0);
        assert!(ok.size > 0 && ok.files > 0);
        assert_eq!(ok.ignored, 1, "the ignored .env is reported");

        // Stashes live in the common git dir and survive the removal: they do not block it.
        assert!(by_name(&found, "stashed").synced);

        let blocked = by_name(&found, "lonely");
        assert!(!blocked.synced);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("sem referência remota"));

        let blocked = by_name(&found, "pending");
        assert!(!blocked.synced);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("fora do remoto"));
        // A blocked checkout is still identified on screen.
        assert_eq!(blocked.branch, "feature/pending");
        assert!(!blocked.head.is_empty());

        let blocked = by_name(&found, "dirty");
        assert!(!blocked.synced);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("alterações locais"));

        // The main checkout is a `.git` directory, not a linked worktree.
        assert!(!found.iter().any(|w| w.path.ends_with("repo")));
        // Searching from inside a worktree never offers that same folder for deletion.
        assert!(find(&synced).is_empty());
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

        // A new local file after the scan is exactly what the second check must catch.
        fs::write(checkout.join("late.txt"), "not committed\n").unwrap();
        let refused = remove(&checkout).unwrap_err();
        assert!(refused.contains("alterações locais"), "{refused}");
        assert!(checkout.exists(), "nothing may be removed on a refusal");

        fs::remove_file(checkout.join("late.txt")).unwrap();
        let outcome = remove(&checkout).unwrap();
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

        let found = find(volume.path());
        let item = by_name(&found, "locked");
        assert!(!item.synced);
        assert!(item.reason.as_deref().unwrap().contains("travada"));
        assert!(remove(&checkout).is_err());
        assert!(checkout.exists());
    }

    #[test]
    fn worktrees_with_initialized_submodules_are_kept() {
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

        let found = find(volume.path());
        let item = by_name(&found, "with-submodule");
        assert!(!item.synced);
        assert!(item.reason.as_deref().unwrap().contains("submódulos"));
        assert!(remove(&checkout).is_err());
        assert!(checkout.exists() && admin.join("modules/lib").exists());
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

        let found = find(volume.path());
        let item = by_name(&found, "filtered");
        assert!(!item.synced);
        assert!(item.reason.as_deref().unwrap().contains("filtros"));
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

        let outcome = remove(&checkout).unwrap();
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
        assert!(find(volume.path()).is_empty());
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

        let found = find(volume.path());
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

        let found = find(volume.path());
        assert!(found.is_empty(), "{found:?}");
    }
}
