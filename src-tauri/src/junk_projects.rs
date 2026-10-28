//! Manifest-backed project locations on every mounted volume. Only backend-listed volume roots
//! may be mount-point reparse directories; links below those roots are never followed.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::{guard::Protection, junk_defs::Base, scan::is_link};

#[derive(Default)]
pub struct Projects {
    node: Vec<PathBuf>,
    rust: Vec<PathBuf>,
    python: Vec<PathBuf>,
}

struct Discovery<'a> {
    protection: Protection,
    mounts: &'a [PathBuf],
    excluded: &'a HashSet<PathBuf>,
}

impl Projects {
    pub fn discover() -> Self {
        let roots = crate::disks::list_disks()
            .into_iter()
            .map(|d| PathBuf::from(d.mount))
            .collect::<Vec<_>>();
        let excluded = crate::junk_defs::definitions()
            .iter()
            .flat_map(|d| {
                crate::junk::base_dir(d.base)
                    .into_iter()
                    .flat_map(move |base| {
                        d.rules
                            .iter()
                            .flat_map(move |r| crate::junk::expand(&base, r.path))
                    })
            })
            .collect();
        Self::from_roots_excluding(&roots, &excluded)
    }

    #[cfg(test)]
    pub(crate) fn from_roots(roots: &[PathBuf]) -> Self {
        Self::from_roots_excluding(roots, &HashSet::new())
    }

    fn from_roots_excluding(roots: &[PathBuf], excluded: &HashSet<PathBuf>) -> Self {
        let ctx = Discovery {
            protection: Protection::new(),
            mounts: roots,
            excluded,
        };
        let mut projects = roots
            .par_iter()
            .map(|root| {
                let mut found = Self::default();
                found.walk(root, false, 0, &ctx);
                found
            })
            .reduce(Self::default, |mut a, b| {
                a.node.extend(b.node);
                a.rust.extend(b.rust);
                a.python.extend(b.python);
                a
            });
        for paths in [&mut projects.node, &mut projects.rust, &mut projects.python] {
            paths.sort();
            paths.dedup();
        }
        projects
    }

    pub fn roots(&self, base: Base) -> &[PathBuf] {
        match base {
            Base::NodeProjects => &self.node,
            Base::RustProjects => &self.rust,
            Base::PythonProjects => &self.python,
            _ => &[],
        }
    }

    fn walk(&mut self, dir: &Path, python: bool, depth: usize, ctx: &Discovery<'_>) {
        if depth > crate::scan::MAX_DEPTH
            || (depth == 0 && !fs::metadata(dir).is_ok_and(|m| m.is_dir()))
        {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let entries: Vec<_> = entries.flatten().collect();
        let has_file = |name: &str| {
            entries.iter().any(|e| {
                e.file_name() == name && e.metadata().is_ok_and(|m| m.is_file() && !is_link(&m))
            })
        };
        if has_file("package.json") {
            self.node.push(dir.to_path_buf());
        }
        if has_file("Cargo.toml") {
            self.rust.push(dir.to_path_buf());
        }
        let python = python
            || has_file("pyproject.toml")
            || has_file("setup.py")
            || has_file("requirements.txt");
        if python {
            self.python.push(dir.to_path_buf());
        }

        let nested = entries
            .into_par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_lowercase();
                // Do not descend into generated trees, installed software, OS data, or credentials.
                // Generic cache/tmp/temp directories remain ordinary user folders.
                if matches!(
                    name.as_str(),
                    ".git"
                        | "node_modules"
                        | "target"
                        | ".venv"
                        | "venv"
                        | "__pycache__"
                        | ".pytest_cache"
                        | ".mypy_cache"
                        | ".ruff_cache"
                        | ".next"
                        | ".angular"
                        | ".parcel-cache"
                        | ".turbo"
                        | "windows"
                        | "program files"
                        | "program files (x86)"
                        | "programdata"
                        | "$recycle.bin"
                        | "system volume information"
                        | ".ssh"
                        | ".aws"
                        | ".azure"
                ) || (cfg!(unix)
                    && depth == 0
                    && matches!(name.as_str(), "proc" | "sys" | "dev" | "run"))
                    || ctx.mounts.iter().any(|m| m == &path)
                    || ctx.protection.is_system(&path)
                    || ctx.excluded.contains(&path)
                {
                    return None;
                }
                // Known personal folders are protected as delete targets, but may contain projects.
                // Check protection on targets in the resolver, rather than pruning those ancestors.
                // On Windows DirEntry caches the listing's attributes, including all reparse tags.
                // Avoid reopening each child for a separate metadata query during the volume walk.
                if entry.metadata().is_ok_and(|m| m.is_dir() && !is_link(&m)) {
                    let mut found = Self::default();
                    found.walk(&path, python, depth + 1, ctx);
                    Some(found)
                } else {
                    None
                }
            })
            .reduce(Self::default, |mut a, b| {
                a.node.extend(b.node);
                a.rust.extend(b.rust);
                a.python.extend(b.python);
                a
            });
        self.node.extend(nested.node);
        self.rust.extend(nested.rust);
        self.python.extend(nested.python);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_locations_already_covered_by_app_rules_and_visits_nested_mounts_once() {
        let volume = tempfile::tempdir().unwrap();
        let nested = volume.path().join("mounted");
        let covered = volume.path().join("app-cache");
        for dir in [&nested, &covered] {
            fs::create_dir_all(dir).unwrap();
            fs::write(dir.join("package.json"), "{}").unwrap();
        }
        let projects = Projects::from_roots_excluding(
            &[volume.path().into(), nested.clone()],
            &HashSet::from([covered]),
        );
        assert_eq!(projects.node, [nested]);
    }

    #[test]
    fn discovers_projects_on_multiple_volumes_without_treating_generic_folders_as_junk() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for (root, marker, cache) in [
            (first.path(), "package.json", "node_modules"),
            (second.path(), "Cargo.toml", "target"),
            (second.path(), "pyproject.toml", "nested/__pycache__"),
        ] {
            let project = root.join(marker.replace('.', "-"));
            fs::create_dir_all(project.join(cache)).unwrap();
            fs::write(project.join(marker), "").unwrap();
        }
        fs::create_dir_all(second.path().join("personal/target")).unwrap();
        fs::create_dir_all(first.path().join("deps/node_modules/hidden")).unwrap();
        fs::write(first.path().join("deps/node_modules/hidden/Cargo.toml"), "").unwrap();
        let projects = Projects::from_roots(&[first.path().into(), second.path().into()]);
        assert_eq!(projects.node, [first.path().join("package-json")]);
        assert_eq!(projects.rust, [second.path().join("Cargo-toml")]);
        assert!(projects
            .python
            .contains(&second.path().join("pyproject-toml/nested")));
    }

    #[cfg(unix)]
    #[test]
    fn ignores_linked_projects() {
        let volume = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("Cargo.toml"), "").unwrap();
        std::os::unix::fs::symlink(outside.path(), volume.path().join("linked")).unwrap();
        assert!(Projects::from_roots(&[volume.path().into()])
            .rust
            .is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn cached_directory_attributes_still_reject_junctions() {
        let volume = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("Cargo.toml"), "").unwrap();
        let result = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(volume.path().join("linked"))
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(Projects::from_roots(&[volume.path().into()])
            .rust
            .is_empty());
        // A volume directory supplied as an explicit backend root must still be scanned.
        assert_eq!(
            Projects::from_roots(&[volume.path().join("linked")]).rust,
            [volume.path().join("linked")]
        );
    }
}
