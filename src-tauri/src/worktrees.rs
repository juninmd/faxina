//! Linked Git worktrees whose work already lives on the remote. Discovery only follows a `.git`
//! *file* that points into `<repo>/.git/worktrees/<id>` (submodules use `modules/`, main checkouts
//! use a `.git` directory); the "safe to remove" verdict comes from git itself, never from a
//! folder name: clean tree, no commit outside a remote-tracking ref and no lock.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rayon::prelude::*;
use serde::Serialize;

use crate::guard::Protection;
use crate::scan::{is_link, MAX_DEPTH};

const MAX_ITEMS: usize = 500;
/// A `.git` pointer file is a few dozen bytes; anything huge is not git's.
const GIT_FILE_LIMIT: u64 = 4096;

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
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "$recycle.bin",
    "system volume information",
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
    /// Every commit of the checkout is on a remote-tracking ref, nothing local would be lost and
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub path: String,
    pub error: String,
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
    let meta = fs::symlink_metadata(&git_file).ok()?;
    if !meta.is_file() || meta.len() > GIT_FILE_LIMIT {
        return None;
    }
    let text = fs::read_to_string(&git_file).ok()?;
    let admin = resolve(worktree, text.trim().strip_prefix("gitdir:")?.trim())?;
    if admin.parent()?.file_name()? != OsStr::new("worktrees") {
        return None;
    }
    // `git worktree add` writes the checkout path back into the admin dir. Requiring the two to
    // agree keeps a crafted `.git` file from naming metadata we would then delete.
    let back = resolve(
        &admin,
        fs::read_to_string(admin.join("gitdir")).ok()?.trim(),
    )?;
    same_path(&back, &git_file).then_some(admin)
}

fn resolve(base: &Path, raw: &str) -> Option<PathBuf> {
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    Some(if path.is_absolute() {
        path
    } else {
        base.join(path)
    })
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[derive(Default)]
struct State {
    branch: String,
    head: String,
    modified: u64,
    /// Why the checkout may not be removed; `None` when every check passed.
    blocked: Option<String>,
}

fn inspect(path: &Path) -> Worktree {
    let admin = linked_worktree_admin(path);
    let state = read_state(path);
    // The lock is the one blocker git will not undo on its own; keep it first so the reason tells
    // the user exactly what to run.
    let reason = match admin {
        None => Some("não é mais uma worktree vinculada".to_string()),
        Some(admin) if admin.join("locked").exists() => {
            Some("worktree travada (use `git worktree unlock`)".to_string())
        }
        Some(_) => state.blocked,
    };
    let (size, files) = crate::junk::dir_size(path);
    Worktree {
        path: path.to_string_lossy().into_owned(),
        branch: state.branch,
        head: state.head,
        modified: state.modified,
        size,
        files,
        synced: reason.is_none(),
        reason,
    }
}

/// Removes one worktree that still passes every check, sends it to the Recycle Bin (or deletes it
/// for good) and drops its now-empty admin directory so `git worktree list` keeps no ghost entry.
pub fn remove(path: &Path, permanent: bool) -> Result<u64, String> {
    let admin = linked_worktree_admin(path).ok_or("não é mais uma worktree vinculada")?;
    if admin.join("locked").exists() {
        return Err("worktree travada: rode `git worktree unlock` antes".into());
    }
    // Re-checked here, not only at search time: a commit or a local edit may have appeared since.
    if let Some(reason) = read_state(path).blocked {
        return Err(reason);
    }
    let (size, _) = crate::junk::dir_size(path);
    let targets = [path.to_path_buf()];
    let removal = crate::remove::remove_all(&targets, permanent)
        .pop()
        .ok_or("nada para remover")?;
    removal.result?;
    let _ = fs::remove_dir_all(&admin);
    Ok(size)
}

/// Everything git has to confirm before a checkout may leave the disk. The reasons are shown in
/// the UI, so they say what to do next; branch and head are filled even when a check blocks the
/// removal, so a kept worktree is still identified on screen.
fn read_state(path: &Path) -> State {
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
    let status = match git(
        path,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=normal",
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

/// Read-only git call. `GIT_OPTIONAL_LOCKS=0` keeps a search from writing the index of a checkout
/// the user did not ask to touch.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "git não encontrado no PATH".to_string()
            } else {
                format!("git não executou: {e}")
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().next().unwrap_or("").trim();
        return Err(if first.is_empty() {
            "git recusou o repositório".to_string()
        } else {
            format!("git recusou o repositório: {first}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
        git_ok(&synced, &["add", "."]);
        git_ok(&synced, &["commit", "-m", "work"]);
        git_ok(&synced, &["push", "-u", "origin", "feature/synced"]);

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
        let by_name = |name: &str| {
            found
                .iter()
                .find(|w| Path::new(&w.path).file_name() == Some(OsStr::new(name)))
                .unwrap_or_else(|| panic!("{name} not found in {found:?}"))
        };

        let ok = by_name("synced");
        assert!(ok.synced, "{}", ok.reason.as_deref().unwrap_or(""));
        assert_eq!(ok.branch, "feature/synced");
        assert!(!ok.head.is_empty() && ok.modified > 0);
        assert!(ok.size > 0 && ok.files > 0);

        // Stashes live in the common git dir and survive the removal: they do not block it.
        assert!(by_name("stashed").synced);

        let blocked = by_name("lonely");
        assert!(!blocked.synced);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("sem referência remota"));

        let blocked = by_name("pending");
        assert!(!blocked.synced);
        assert!(blocked
            .reason
            .as_deref()
            .unwrap()
            .contains("fora do remoto"));
        // A blocked checkout is still identified on screen.
        assert_eq!(blocked.branch, "feature/pending");
        assert!(!blocked.head.is_empty());

        let blocked = by_name("dirty");
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
        let refused = remove(&checkout, true).unwrap_err();
        assert!(refused.contains("alterações locais"), "{refused}");
        assert!(checkout.exists(), "nothing may be removed on a refusal");

        fs::remove_file(checkout.join("late.txt")).unwrap();
        let freed = remove(&checkout, true).unwrap();
        assert!(freed > 0);
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
        let item = found
            .iter()
            .find(|w| Path::new(&w.path).file_name() == Some(OsStr::new("locked")))
            .unwrap();
        assert!(!item.synced);
        assert!(item.reason.as_deref().unwrap().contains("travada"));
        assert!(remove(&checkout, true).is_err());
        assert!(checkout.exists());
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
        // trusted: deleting that metadata would hit another checkout.
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
