//! Manifest-backed project locations on every mounted volume. Only backend-listed volume roots
//! may be mount-point reparse directories; links below those roots are never followed.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::{guard::Protection, junk_defs::Base, scan::is_link};

/// Ecosystems recognised by a manifest file in the project root. Each owns a few folders that
/// the toolchain recreates (see `junk_optin_defs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Marker {
    Gradle,
    Maven,
    Flutter,
    Composer,
    Terraform,
    Dotnet,
    Haskell,
    Elixir,
    Zig,
}

/// Manifest names are matched exactly (case-sensitive); .NET project files by extension.
fn markers_for(name: &str) -> &'static [Marker] {
    match name {
        "build.gradle" | "build.gradle.kts" => &[Marker::Gradle],
        "pom.xml" => &[Marker::Maven],
        "pubspec.yaml" => &[Marker::Flutter],
        "composer.json" => &[Marker::Composer],
        ".terraform.lock.hcl" => &[Marker::Terraform],
        "stack.yaml" | "cabal.project" => &[Marker::Haskell],
        "mix.exs" => &[Marker::Elixir],
        "build.zig" => &[Marker::Zig],
        _ => {
            let lower = name.to_lowercase();
            if [".csproj", ".fsproj", ".vbproj"]
                .iter()
                .any(|ext| lower.ends_with(ext))
            {
                &[Marker::Dotnet]
            } else {
                &[]
            }
        }
    }
}

#[derive(Default)]
pub struct Projects {
    node: Vec<PathBuf>,
    rust: Vec<PathBuf>,
    python: Vec<PathBuf>,
    marked: HashMap<Marker, Vec<PathBuf>>,
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
                a.merge(b);
                a
            });
        for paths in [&mut projects.node, &mut projects.rust, &mut projects.python]
            .into_iter()
            .chain(projects.marked.values_mut())
        {
            paths.sort();
            paths.dedup();
        }
        projects
    }

    fn merge(&mut self, other: Self) {
        self.node.extend(other.node);
        self.rust.extend(other.rust);
        self.python.extend(other.python);
        for (marker, paths) in other.marked {
            self.marked.entry(marker).or_default().extend(paths);
        }
    }

    pub fn roots(&self, base: Base) -> &[PathBuf] {
        match base {
            Base::NodeProjects => &self.node,
            Base::RustProjects => &self.rust,
            Base::PythonProjects => &self.python,
            Base::Marked(marker) => self.marked.get(&marker).map_or(&[], Vec::as_slice),
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
        // One metadata pass over the folder: regular files only, never links.
        let files: Vec<String> = entries
            .iter()
            .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && !is_link(&m)))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        let has_file = |name: &str| files.iter().any(|f| f == name);
        for marker in files.iter().flat_map(|f| markers_for(f)) {
            let found = self.marked.entry(*marker).or_default();
            if found.last().map(PathBuf::as_path) != Some(dir) {
                found.push(dir.to_path_buf());
            }
        }
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
                        | ".gradle"
                        | ".dart_tool"
                        | ".terraform"
                        | ".stack-work"
                        | "dist-newstyle"
                        | ".zig-cache"
                        | "zig-out"
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
                a.merge(b);
                a
            });
        self.merge(nested);
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

    #[test]
    fn discovers_projects_by_manifest_and_ignores_unmarked_folders() {
        let volume = tempfile::tempdir().unwrap();
        for (dir, manifest) in [
            ("gradle", "build.gradle.kts"),
            ("maven", "pom.xml"),
            ("flutter", "pubspec.yaml"),
            ("php", "composer.json"),
            ("infra", ".terraform.lock.hcl"),
            ("dotnet", "App.CSPROJ"),
            ("hs", "stack.yaml"),
            ("ex", "mix.exs"),
            ("zig", "build.zig"),
        ] {
            fs::create_dir_all(volume.path().join(dir)).unwrap();
            fs::write(volume.path().join(dir).join(manifest), "").unwrap();
        }
        fs::create_dir_all(volume.path().join("plain/build")).unwrap();
        // A generated tree is never searched for nested projects.
        fs::create_dir_all(volume.path().join("gradle/.gradle/inner")).unwrap();
        fs::write(volume.path().join("gradle/.gradle/inner/pom.xml"), "").unwrap();
        let projects = Projects::from_roots(&[volume.path().into()]);
        for (marker, dir) in [
            (Marker::Gradle, "gradle"),
            (Marker::Maven, "maven"),
            (Marker::Flutter, "flutter"),
            (Marker::Composer, "php"),
            (Marker::Terraform, "infra"),
            (Marker::Dotnet, "dotnet"),
            (Marker::Haskell, "hs"),
            (Marker::Elixir, "ex"),
            (Marker::Zig, "zig"),
        ] {
            assert_eq!(
                projects.roots(Base::Marked(marker)),
                [volume.path().join(dir)],
                "{marker:?}"
            );
        }
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
