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

pub fn is_protected(p: &Path) -> bool {
    let Some(parent) = p.parent() else {
        return true;
    };
    let at_drive_root = parent.parent().is_none();
    let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase());
    if at_drive_root && name.is_some_and(|n| DRIVE_ROOT_SYSTEM.contains(&n.as_str())) {
        return true;
    }
    let exact: Vec<PathBuf> = [
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
    if exact.iter().any(|d| d == p || d.starts_with(p)) {
        return true;
    }
    system_dirs().iter().any(|d| p.starts_with(d))
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
    }
}
