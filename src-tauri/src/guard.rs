use std::path::{Component, Path, PathBuf};

/// Everything a deletion request must pass: the webview is not trusted to send sane paths.
pub fn check(target: &Path, allowed_roots: &[PathBuf]) -> Result<PathBuf, String> {
    if !target.is_absolute()
        || target
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err("caminho inválido".into());
    }
    let meta =
        std::fs::symlink_metadata(target).map_err(|_| "caminho não existe mais".to_string())?;
    if crate::scan::is_link(&meta) {
        // Deleting through a link would hit whatever it points to, not what the user saw.
        return Err("atalhos e links não são excluídos".into());
    }
    let canon = dunce(target).map_err(|_| "caminho não existe mais".to_string())?;
    let inside = allowed_roots
        .iter()
        .filter_map(|r| dunce(r).ok())
        .any(|root| canon.starts_with(&root) && canon != root);
    if !inside {
        return Err("fora da área analisada".into());
    }
    if is_protected(&canon) {
        return Err("pasta protegida do sistema".into());
    }
    // Remove exactly what was validated, not the raw input.
    Ok(canon)
}

/// A dangling link directly or deeply inside `root`. The link is judged by where it *lives* (its
/// parent is canonicalized), never by what it points to, which no longer exists.
pub fn check_dangling_link(link: &Path, root: &Path) -> Result<(), String> {
    if link.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("caminho inválido".into());
    }
    if !crate::empty::is_dangling(link) {
        return Err("o link não está mais quebrado".into());
    }
    let parent = link.parent().ok_or("caminho inválido")?;
    let (parent, root) = (
        dunce(parent).map_err(|_| "caminho não existe mais".to_string())?,
        dunce(root).map_err(|_| "pasta não encontrada".to_string())?,
    );
    if !parent.starts_with(&root) {
        return Err("fora da área analisada".into());
    }
    if is_protected(&parent) {
        return Err("pasta protegida do sistema".into());
    }
    Ok(())
}

fn dunce(p: &Path) -> std::io::Result<PathBuf> {
    let c = p.canonicalize()?;
    // Strip the Windows verbatim prefix so comparisons against plain paths work.
    let s = c.to_string_lossy();
    Ok(match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => c,
    })
}

/// OS-owned entries at a drive root: paging, hibernation, restore points, the Recycle Bin.
const DRIVE_ROOT_SYSTEM: &[&str] = &[
    "pagefile.sys",
    "hiberfil.sys",
    "swapfile.sys",
    "dumpstack.log",
    "dumpstack.log.tmp",
    "system volume information",
    "$recycle.bin",
    "$windows.~bt",
    "$winreagent",
    "recovery",
    "boot",
    "efi",
    "config.msi",
];

/// OS-owned entry at a drive root, or anything inside it: `C:\$Recycle.Bin\<sid>\file` is as
/// much the system's as `C:\$Recycle.Bin` itself.
fn at_protected_drive_entry(p: &Path) -> bool {
    let Some(root) = p.ancestors().last() else {
        return true;
    };
    let Ok(rest) = p.strip_prefix(root) else {
        return false;
    };
    rest.components()
        .next()
        .and_then(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .is_some_and(|n| DRIVE_ROOT_SYSTEM.contains(&n.as_str()))
}

/// The protected-path rules, with the (filesystem-touching) directory lookups done once. A scan of
/// a whole drive feeds millions of paths through this; recomputing them per path would be I/O bound.
pub struct Protection {
    exact: Vec<PathBuf>,
    system: Vec<PathBuf>,
}

impl Protection {
    pub fn new() -> Self {
        let exact = [
            dirs::home_dir(),
            dirs::desktop_dir(),
            dirs::document_dir(),
            dirs::download_dir(),
            dirs::picture_dir(),
            dirs::audio_dir(),
            dirs::video_dir(),
        ]
        .into_iter()
        .flatten()
        .filter_map(|d| dunce(&d).ok())
        .collect();
        Self {
            exact,
            // Credential stores stay protected even when a scan root contains them.
            system: system_dirs().into_iter().chain(credential_dirs()).collect(),
        }
    }

    pub fn is_protected(&self, p: &Path) -> bool {
        // A volume root itself is never deletable.
        if p.parent().is_none() {
            return true;
        }
        if at_protected_drive_entry(p) {
            return true;
        }
        // The known folders themselves, and anything that contains them (home, C:\).
        if self.exact.iter().any(|d| d == p || d.starts_with(p)) {
            return true;
        }
        self.is_system(p)
    }

    /// OS and credential trees must not be explored for project cleanup.
    pub fn is_system(&self, p: &Path) -> bool {
        at_protected_drive_entry(p) || self.system.iter().any(|d| p.starts_with(d))
    }
}

impl Default for Protection {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_protected(p: &Path) -> bool {
    Protection::new().is_protected(p)
}

fn credential_dirs() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let home = dunce(&home).unwrap_or(home);
    [
        ".ssh",
        ".gnupg",
        ".aws",
        ".azure",
        ".kube",
        ".docker",
        ".config/gcloud",
        ".config/gh",
        ".git-credentials",
        ".npmrc",
        ".netrc",
    ]
    .iter()
    .map(|rel| home.join(rel))
    .collect()
}

fn system_dirs() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut v: Vec<PathBuf> = [
            "SystemRoot",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramData",
        ]
        .iter()
        .filter_map(|k| std::env::var_os(k).map(PathBuf::from))
        .collect();
        v.push(PathBuf::from(r"C:\Windows"));
        v
    }
    #[cfg(not(windows))]
    {
        [
            "/System",
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/lib",
            "/lib64",
            "/boot",
            "/dev",
            "/proc",
            "/sys",
            "/Library",
            "/Applications",
            "/private/var/db",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_children_of_root_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let child = root.join("node_modules");
        std::fs::create_dir(&child).unwrap();
        assert!(check(&child, std::slice::from_ref(&root)).is_ok());
        // The scan root itself is never deletable.
        assert!(check(&root, std::slice::from_ref(&root)).is_err());
    }

    #[test]
    fn rejects_traversal_and_outside_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("a");
        std::fs::create_dir(&root).unwrap();
        let sneaky = root.join("..").join("a");
        assert_eq!(
            check(&sneaky, std::slice::from_ref(&root)).unwrap_err(),
            "caminho inválido"
        );
        let other = tempfile::tempdir().unwrap();
        assert_eq!(
            check(other.path(), &[root]).unwrap_err(),
            "fora da área analisada"
        );
    }

    #[test]
    fn rejects_relative_and_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let roots = [tmp.path().to_path_buf()];
        assert!(check(Path::new("relative/x"), &roots).is_err());
        assert!(check(&tmp.path().join("ghost"), &roots).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn links_are_refused_even_inside_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        assert_eq!(
            check(&root.join("link"), std::slice::from_ref(&root)).unwrap_err(),
            "atalhos e links não são excluídos"
        );
    }

    #[test]
    fn home_and_its_ancestors_are_protected() {
        let home = dunce(&dirs::home_dir().unwrap()).unwrap();
        assert!(is_protected(&home));
        assert!(is_protected(home.parent().unwrap()));
        #[cfg(windows)]
        assert!(is_protected(Path::new(r"C:\Windows\System32")));
    }

    #[test]
    fn credential_stores_are_protected_even_inside_a_scan_root() {
        let home = dunce(&dirs::home_dir().unwrap()).unwrap();
        assert!(is_protected(&home.join(".ssh")));
        assert!(is_protected(&home.join(".ssh").join("id_ed25519")));
        assert!(is_protected(&home.join(".aws").join("credentials")));
        assert!(is_protected(&home.join(".config").join("gcloud")));
    }

    #[test]
    fn token_files_under_home_are_protected() {
        let home = dunce(&dirs::home_dir().unwrap()).unwrap();
        assert!(is_protected(&home.join(".docker").join("config.json")));
        assert!(is_protected(
            &home.join(".config").join("gh").join("hosts.yml")
        ));
        assert!(is_protected(&home.join(".git-credentials")));
        assert!(is_protected(&home.join(".npmrc")));
        assert!(is_protected(&home.join(".netrc")));
    }

    #[test]
    fn drive_root_system_files_are_protected() {
        let root = std::env::temp_dir()
            .ancestors()
            .last()
            .unwrap()
            .to_path_buf();
        for n in [
            "pagefile.sys",
            "hiberfil.sys",
            "System Volume Information",
            "$Recycle.Bin",
        ] {
            assert!(is_protected(&root.join(n)), "{n}");
        }
        // Same names deeper in the tree are ordinary files.
        assert!(!is_protected(&root.join("backup").join("pagefile.sys")));
        // OS-owned folders protect their contents too: the Recycle Bin is not user space.
        assert!(is_protected(
            &root.join("$Recycle.Bin").join("S-1-5-21").join("$R.docx")
        ));
        assert!(is_protected(
            &root.join("System Volume Information").join("tracking.log")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn only_dangling_links_inside_the_root_can_be_removed() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let broken = root.path().join("broken");
        std::os::unix::fs::symlink(root.path().join("gone"), &broken).unwrap();
        assert!(check_dangling_link(&broken, root.path()).is_ok());

        // A working link points at real data: it is not this feature's to remove.
        std::fs::write(root.path().join("real"), "x").unwrap();
        let ok = root.path().join("ok");
        std::os::unix::fs::symlink(root.path().join("real"), &ok).unwrap();
        assert!(check_dangling_link(&ok, root.path()).is_err());

        // Same dangling link, but living outside the searched root.
        let stray = outside.path().join("broken");
        std::os::unix::fs::symlink(outside.path().join("gone"), &stray).unwrap();
        assert!(check_dangling_link(&stray, root.path()).is_err());
    }
}
