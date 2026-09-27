use crate::model::Kind;
use std::path::Path;

const CACHE_DIRS: &[&str] = &[
    ".cache",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".parcel-cache",
    ".sass-cache",
    "inetcache",
    "code cache",
    "gpucache",
    "shadercache",
    "dxcache",
    "crashdumps",
    ".gradle",
    "deriveddata",
];
/// Names too common to trust outside application data (`Documents/Projeto/tmp` is user work).
const GENERIC_CACHE_DIRS: &[&str] = &["cache", "caches", "temp", "tmp"];
const APP_DATA_DIRS: &[&str] = &["appdata", ".cache", ".local", ".config", "library"];
const BUILD_DIRS: &[&str] = &[
    ".venv",
    "node_modules",
    ".next",
    ".nuxt",
    ".turbo",
    ".svelte-kit",
    ".angular",
    ".terraform",
];
const MEDIA: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "heic", "raw", "cr2", "nef", "psd", "mp4", "mkv", "mov",
    "avi", "webm", "mp3", "flac", "wav", "ogg", "m4a", "aac",
];
const DOCS: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "txt", "md", "epub", "csv",
];
const CODE: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt", "c", "cpp", "h", "cs", "rb", "php",
    "swift", "json", "toml", "yaml", "yml", "html", "css", "sql", "sh", "ps1", "lua", "sma",
];
const ARCHIVES: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "xz", "zst", "bz2", "iso", "dmg", "img", "vhdx",
];
const APPS: &[&str] = &[
    "exe", "msi", "dll", "so", "dylib", "app", "appimage", "deb", "rpm", "pak", "bin", "sys",
];
const LOG_EXT: &[&str] = &["log", "dmp", "tmp", "etl"];

/// Context of the parent directory that some rules need (e.g. `target` only counts next to Cargo.toml).
#[derive(Default, Clone, Copy)]
pub struct DirHints {
    pub has_cargo_toml: bool,
    pub has_build_manifest: bool,
    pub under_app_data: bool,
}

/// Whether a path lies inside per-user application data, where generic cache names are safe.
pub fn under_app_data(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy().to_lowercase();
        APP_DATA_DIRS.contains(&s.as_str())
    })
}

pub fn classify_dir(name: &str, hints: DirHints) -> Option<Kind> {
    let lower = name.to_lowercase();
    if lower == ".git" {
        return Some(Kind::Git);
    }
    if CACHE_DIRS.contains(&lower.as_str())
        || (hints.under_app_data && GENERIC_CACHE_DIRS.contains(&lower.as_str()))
    {
        return Some(Kind::Cache);
    }
    if BUILD_DIRS.contains(&lower.as_str()) {
        return Some(Kind::Build);
    }
    if lower == "target" && hints.has_cargo_toml {
        return Some(Kind::Build);
    }
    if matches!(lower.as_str(), "dist" | "build" | "out" | "bin" | "obj")
        && hints.has_build_manifest
    {
        return Some(Kind::Build);
    }
    None
}

pub fn classify_file(name: &str) -> Kind {
    let ext = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_lowercase(),
        _ => return Kind::Other,
    };
    let e = ext.as_str();
    if LOG_EXT.contains(&e) {
        Kind::Cache
    } else if MEDIA.contains(&e) {
        Kind::Media
    } else if DOCS.contains(&e) {
        Kind::Documents
    } else if CODE.contains(&e) {
        Kind::Code
    } else if ARCHIVES.contains(&e) {
        Kind::Archives
    } else if APPS.contains(&e) {
        Kind::Apps
    } else {
        Kind::Other
    }
}

/// Hints derived from the file names inside a directory.
pub fn hints_for(file_names: &[String]) -> DirHints {
    let mut h = DirHints::default();
    for n in file_names {
        match n.as_str() {
            "Cargo.toml" => {
                h.has_cargo_toml = true;
                h.has_build_manifest = true;
            }
            "package.json" | "pom.xml" | "build.gradle" | "build.gradle.kts" | "CMakeLists.txt"
            | "pyproject.toml" | "go.mod" => h.has_build_manifest = true,
            n if n.ends_with(".csproj") || n.ends_with(".sln") => h.has_build_manifest = true,
            _ => {}
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_build_only_next_to_cargo_toml() {
        let rust = hints_for(&["Cargo.toml".into()]);
        assert_eq!(classify_dir("target", rust), Some(Kind::Build));
        // A folder literally named "target" in Documents is user data, never reclaimable.
        assert_eq!(classify_dir("target", DirHints::default()), None);
    }

    #[test]
    fn dist_needs_a_build_manifest() {
        assert_eq!(
            classify_dir("dist", hints_for(&["package.json".into()])),
            Some(Kind::Build)
        );
        assert_eq!(classify_dir("dist", DirHints::default()), None);
    }

    #[test]
    fn generic_cache_names_count_only_inside_app_data() {
        // "Documents/Projeto/tmp" may hold real work; "AppData/Local/Temp" does not.
        for n in ["tmp", "Temp", "cache", "Caches"] {
            assert_eq!(classify_dir(n, DirHints::default()), None, "{n}");
        }
        let app = DirHints {
            under_app_data: true,
            ..DirHints::default()
        };
        assert_eq!(classify_dir("Temp", app), Some(Kind::Cache));
        assert!(under_app_data(Path::new("/home/u/.cache/pip")));
        assert!(under_app_data(Path::new("C:/Users/u/AppData/Local")));
        assert!(!under_app_data(Path::new("C:/Users/u/Documents/Projeto")));
    }

    #[test]
    fn well_known_dirs() {
        assert_eq!(
            classify_dir("node_modules", DirHints::default()),
            Some(Kind::Build)
        );
        assert_eq!(
            classify_dir("__pycache__", DirHints::default()),
            Some(Kind::Cache)
        );
        assert_eq!(classify_dir(".git", DirHints::default()), Some(Kind::Git));
        assert_eq!(classify_dir("Fotos", DirHints::default()), None);
    }

    #[test]
    fn files_by_extension_case_insensitive() {
        assert_eq!(classify_file("Férias.JPG"), Kind::Media);
        assert_eq!(classify_file("crash.dmp"), Kind::Cache);
        assert_eq!(classify_file("main.rs"), Kind::Code);
        assert_eq!(classify_file(".bashrc"), Kind::Other);
        assert_eq!(classify_file("README"), Kind::Other);
    }
}
