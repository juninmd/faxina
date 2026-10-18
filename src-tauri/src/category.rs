use crate::model::Kind;
use std::borrow::Cow;
use std::path::Path;

const CACHE_DIRS: &[&str] = &[
    ".cache",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".hypothesis",
    ".pytype",
    ".pyre",
    ".parcel-cache",
    ".sass-cache",
    "inetcache",
    "code cache",
    "gpucache",
    "shadercache",
    "grshadercache",
    "dawncache",
    "graphitedawncache",
    "service worker",
    "media cache",
    "cacheddata",
    "dxcache",
    "crashdumps",
    ".gradle",
    "deriveddata",
    ".ipynb_checkpoints",
    ".import_linter_cache",
    ".rpt2_cache",
    ".nx",
    ".vite",
    ".astro",
    ".docusaurus",
    ".terragrunt-cache",
    ".cxx",
    ".externalnativebuild",
    ".ccls-cache",
    ".clangd",
    ".bloop",
    ".metals",
    ".zig-cache",
    "zig-cache",
];
/// Names too common to trust outside application data (`Documents/Projeto/tmp` is user work).
const GENERIC_CACHE_DIRS: &[&str] = &["cache", "caches", "temp", "tmp"];
const APP_DATA_DIRS: &[&str] = &["appdata", ".cache", ".local", ".config", "library"];
const BUILD_DIRS: &[&str] = &[
    ".venv",
    "venv",
    ".tox",
    ".nox",
    ".eggs",
    "node_modules",
    "bower_components",
    ".pnpm-store",
    ".next",
    ".nuxt",
    ".turbo",
    ".svelte-kit",
    ".angular",
    ".terraform",
    ".stack-work",
    "htmlcov",
    ".nyc_output",
    "__pypackages__",
    "jspm_packages",
    ".output",
    ".aws-sam",
    "storybook-static",
    "elm-stuff",
    "dist-newstyle",
    "zig-out",
];
/// Build artifact folders whose name is too generic to trust without a matching manifest.
const MANIFEST_BUILD_DIRS: &[&str] = &[
    "pods",
    "carthage",
    "_build",
    ".dart_tool",
    ".build",
    ".swiftpm",
    "coverage",
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
const LOG_EXT: &[&str] = &["log", "dmp", "tmp", "temp", "etl"];

/// Context of the parent directory that some rules need (e.g. `target` only counts next to Cargo.toml).
#[derive(Default, Clone, Copy)]
pub struct DirHints {
    /// Cargo.toml, pom.xml or build.sbt: the tools whose output folder is called `target`.
    pub has_target_manifest: bool,
    pub has_build_manifest: bool,
    pub under_app_data: bool,
}

/// Whether a path lies inside per-user application data, where generic cache names are safe.
pub fn under_app_data(path: &Path) -> bool {
    path.components()
        .any(|c| is_app_data_name(&c.as_os_str().to_string_lossy()))
}

/// Lets the scan carry the flag down one level at a time instead of re-reading the whole path.
pub fn is_app_data_name(name: &str) -> bool {
    APP_DATA_DIRS.contains(&lowercase(name).as_ref())
}

/// Most folder names are already lowercase ASCII, so the common case allocates nothing.
fn lowercase(name: &str) -> Cow<'_, str> {
    if name
        .bytes()
        .any(|b| b.is_ascii_uppercase() || !b.is_ascii())
    {
        Cow::Owned(name.to_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

pub fn classify_dir(name: &str, hints: DirHints) -> Option<Kind> {
    let lower = lowercase(name);
    let lower = lower.as_ref();
    if lower == ".git" {
        return Some(Kind::Git);
    }
    if CACHE_DIRS.contains(&lower) || (hints.under_app_data && GENERIC_CACHE_DIRS.contains(&lower))
    {
        return Some(Kind::Cache);
    }
    if BUILD_DIRS.contains(&lower) {
        return Some(Kind::Build);
    }
    if lower == "target" && hints.has_target_manifest {
        return Some(Kind::Build);
    }
    if hints.has_build_manifest
        && (matches!(lower, "dist" | "build" | "out" | "bin" | "obj")
            || MANIFEST_BUILD_DIRS.contains(&lower)
            || lower.starts_with("cmake-build-"))
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
pub fn hints_for<'a>(file_names: impl IntoIterator<Item = &'a str>) -> DirHints {
    let mut h = DirHints::default();
    for n in file_names {
        match n {
            "Cargo.toml" | "pom.xml" | "build.sbt" => {
                h.has_target_manifest = true;
                h.has_build_manifest = true;
            }
            "package.json" | "build.gradle" | "build.gradle.kts" | "CMakeLists.txt"
            | "pyproject.toml" | "go.mod" | "Podfile" | "Cartfile" | "mix.exs" | "pubspec.yaml"
            | "dune-project" | "Package.swift" | "build.zig" => h.has_build_manifest = true,
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
        let rust = hints_for(["Cargo.toml"]);
        assert_eq!(classify_dir("target", rust), Some(Kind::Build));
        // A folder literally named "target" in Documents is user data, never reclaimable.
        assert_eq!(classify_dir("target", DirHints::default()), None);
    }

    #[test]
    fn target_also_follows_maven_and_sbt_manifests() {
        for manifest in ["pom.xml", "build.sbt"] {
            assert_eq!(
                classify_dir("target", hints_for([manifest])),
                Some(Kind::Build),
                "{manifest}"
            );
        }
        // package.json is a build manifest but its output is never called `target`.
        assert_eq!(classify_dir("target", hints_for(["package.json"])), None);
    }

    #[test]
    fn toolchain_dependency_and_cache_dirs_are_recognized() {
        let none = DirHints::default();
        for n in [
            "__pypackages__",
            "jspm_packages",
            ".output",
            "elm-stuff",
            "dist-newstyle",
            "zig-out",
            "storybook-static",
        ] {
            assert_eq!(classify_dir(n, none), Some(Kind::Build), "{n}");
        }
        for n in [
            ".ipynb_checkpoints",
            ".nx",
            ".vite",
            ".astro",
            ".terragrunt-cache",
            ".cxx",
            ".clangd",
            ".zig-cache",
        ] {
            assert_eq!(classify_dir(n, none), Some(Kind::Cache), "{n}");
        }
    }

    #[test]
    fn swiftpm_and_coverage_need_a_manifest() {
        for n in [".build", "coverage"] {
            assert_eq!(classify_dir(n, DirHints::default()), None, "{n}");
        }
        let swift = hints_for(["Package.swift"]);
        assert_eq!(classify_dir(".build", swift), Some(Kind::Build));
        assert_eq!(classify_dir("Coverage", swift), Some(Kind::Build));
    }

    #[test]
    fn dist_needs_a_build_manifest() {
        assert_eq!(
            classify_dir("dist", hints_for(["package.json"])),
            Some(Kind::Build)
        );
        assert_eq!(classify_dir("dist", DirHints::default()), None);
    }

    #[test]
    fn generic_build_dirs_need_a_matching_manifest() {
        // A user folder called "Pods" in Documents is not an Xcode artifact.
        for n in ["Pods", "Carthage", "_build", ".dart_tool"] {
            assert_eq!(classify_dir(n, DirHints::default()), None, "{n}");
        }
        assert_eq!(
            classify_dir("Pods", hints_for(["Podfile"])),
            Some(Kind::Build)
        );
        assert_eq!(
            classify_dir("_build", hints_for(["mix.exs"])),
            Some(Kind::Build)
        );
        let cmake = hints_for(["CMakeLists.txt"]);
        assert_eq!(classify_dir("cmake-build-debug", cmake), Some(Kind::Build));
        assert_eq!(classify_dir("cmake-build-debug", DirHints::default()), None);
    }

    #[test]
    fn browser_cache_dirs_are_recognized() {
        for n in [
            "GrShaderCache",
            "ShaderCache",
            "DawnCache",
            "Service Worker",
            "Media Cache",
            "Code Cache",
        ] {
            assert_eq!(
                classify_dir(n, DirHints::default()),
                Some(Kind::Cache),
                "{n}"
            );
        }
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
        assert!(is_app_data_name("AppData") && !is_app_data_name("Documents"));
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
