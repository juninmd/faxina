//! Known regenerable locations, distilled from BleachBit cleaners and Winapp2.ini.
//! Only cache folders are listed: never cookies, history, logins or profile data.
//!
//! A definition is a set of [`Rule`]s. Each rule's `path` is relative to `base` and may use
//! `*`/`?` inside a segment (`Chrome*`, `*Cache*`) like Winapp2 does. `Scope::Contents` wipes a
//! folder (the folder itself stays so the app keeps working); `Scope::Files` deletes only files
//! matching `patterns`, optionally `recurse`ing. `excludes` is the Winapp2 `ExcludeKey` equivalent.

use std::time::Duration;

// Each OS uses a different subset of bases.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Base {
    Home,
    Local,
    Roaming,
    Cache,
    Temp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Remove every entry inside the directory (files and subdirectories); the directory stays.
    Contents,
    /// Remove only files whose name matches `patterns`; directories are never removed.
    Files,
}

#[derive(Clone, Copy)]
pub struct Rule {
    pub path: &'static str,
    pub scope: Scope,
    pub patterns: &'static [&'static str],
    pub recurse: bool,
    pub excludes: &'static [&'static str],
}

/// Wipes the contents of one directory.
pub const fn whole(path: &'static str) -> Rule {
    Rule {
        path,
        scope: Scope::Contents,
        patterns: &[],
        recurse: false,
        excludes: &[],
    }
}

/// Deletes only files matching `patterns` (Winapp2 `FileKey=path|pattern|RECURSE`).
pub const fn matching(
    path: &'static str,
    patterns: &'static [&'static str],
    recurse: bool,
) -> Rule {
    Rule {
        path,
        scope: Scope::Files,
        patterns,
        recurse,
        excludes: &[],
    }
}

/// Preserves entries whose name matches `excludes` (Winapp2 `ExcludeKey`).
#[allow(dead_code)]
pub const fn excluding(mut rule: Rule, excludes: &'static [&'static str]) -> Rule {
    rule.excludes = excludes;
    rule
}

pub struct JunkDef {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub base: Base,
    pub rules: Vec<Rule>,
    pub min_age: Duration,
}

#[cfg_attr(not(windows), allow(dead_code))]
const DAY: Duration = Duration::from_secs(24 * 3600);
const NOW: Duration = Duration::ZERO;

fn def(
    id: &'static str,
    label: &'static str,
    group: &'static str,
    base: Base,
    rules: Vec<Rule>,
) -> JunkDef {
    JunkDef {
        id,
        label,
        group,
        base,
        rules,
        min_age: NOW,
    }
}

const SYS: &str = "Sistema";
#[cfg_attr(not(windows), allow(dead_code))]
const WEB: &str = "Navegadores";
#[cfg_attr(not(windows), allow(dead_code))]
const APPS: &str = "Apps";
const DEV: &str = "Desenvolvimento";

/// Caches every Chromium-family browser keeps next to its profiles. `root` is the browser data
/// folder (the parent of `User Data`); `*Cache*` catches Cache, Code Cache, GPUCache, Media Cache
/// and the top-level GrShaderCache/ShaderCache/DawnCache that Faxina used to miss.
macro_rules! chromium_caches {
    ($name:ident, $root:literal) => {
        const $name: &[Rule] = &[
            whole(concat!($root, "/User Data/*Cache*")),
            whole(concat!($root, "/User Data/*/*Cache*")),
            whole(concat!($root, "/User Data/*/Service Worker")),
            whole(concat!($root, "/User Data/*/blob_storage")),
            whole(concat!($root, "/User Data/*/File System")),
            whole(concat!($root, "/User Data/*/Shared Dictionary/cache")),
        ];
    };
}

chromium_caches!(CHROME_CACHES, "Google/Chrome*");
chromium_caches!(EDGE_CACHES, "Microsoft/Edge*");
chromium_caches!(BRAVE_CACHES, "BraveSoftware/Brave-*");
chromium_caches!(VIVALDI_CACHES, "Vivaldi");

fn common() -> Vec<JunkDef> {
    vec![
        def(
            "bun",
            "Bun (cache de pacotes)",
            DEV,
            Base::Home,
            vec![whole(".bun/install/cache")],
        ),
        def(
            "cargo",
            "Cargo (tarballs do registry)",
            DEV,
            Base::Home,
            vec![whole(".cargo/registry/cache")],
        ),
        def(
            "gradle",
            "Gradle (caches)",
            DEV,
            Base::Home,
            vec![whole(".gradle/caches"), whole(".gradle/wrapper/dists")],
        ),
        def(
            "rustup",
            "Rustup (downloads e temporários)",
            DEV,
            Base::Home,
            vec![whole(".rustup/downloads"), whole(".rustup/tmp")],
        ),
        def(
            "maven",
            "Maven (repositório local)",
            DEV,
            Base::Home,
            vec![whole(".m2/repository")],
        ),
        def(
            "nuget-home",
            "NuGet (pacotes baixados)",
            DEV,
            Base::Home,
            vec![whole(".nuget/packages")],
        ),
        def(
            "gomodcache",
            "Go (cache de módulos)",
            DEV,
            Base::Home,
            vec![whole("go/pkg/mod/cache/download")],
        ),
        def(
            "pnpm-store",
            "pnpm (store de pacotes)",
            DEV,
            Base::Home,
            vec![whole(".local/share/pnpm/store"), whole(".pnpm-store")],
        ),
    ]
}

#[cfg(windows)]
pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        // Files touched in the last day may belong to a running installer.
        JunkDef {
            min_age: DAY,
            ..def(
                "temp",
                "Arquivos temporários",
                SYS,
                Base::Temp,
                vec![whole("")],
            )
        },
        def(
            "inetcache",
            "Cache da Internet do Windows",
            SYS,
            Base::Local,
            vec![whole("Microsoft/Windows/INetCache")],
        ),
        def(
            "wincaches",
            "Caches do shell e do Windows",
            SYS,
            Base::Local,
            vec![
                whole("Microsoft/Windows/Caches"),
                whole("Microsoft/Terminal Server Client/Cache"),
            ],
        ),
        def(
            "crashdumps",
            "Despejos de falhas",
            SYS,
            Base::Local,
            vec![whole("CrashDumps")],
        ),
        def(
            "wer",
            "Relatórios de erro do Windows",
            SYS,
            Base::Local,
            vec![
                whole("Microsoft/Windows/WER/ReportArchive"),
                whole("Microsoft/Windows/WER/ReportQueue"),
            ],
        ),
        def(
            "shaders",
            "Cache de shaders (DirectX/GPU)",
            SYS,
            Base::Local,
            vec![
                whole("D3DSCache"),
                whole("NVIDIA/DXCache"),
                whole("NVIDIA/GLCache"),
                whole("AMD/DxCache"),
            ],
        ),
        // Microsoft Store / UWP apps: application cache and temp state, plus logs only.
        def(
            "uwp",
            "Apps da Microsoft Store (cache)",
            SYS,
            Base::Local,
            vec![
                whole("Packages/*/AC"),
                whole("Packages/*/TempState"),
                matching("Packages/*/Settings", &["*.log", "*.log.*"], false),
                matching(
                    "Packages/*/SystemAppData/Helium",
                    &["*.log", "*.log.*"],
                    false,
                ),
            ],
        ),
        def(
            "chrome",
            "Google Chrome",
            WEB,
            Base::Local,
            CHROME_CACHES.to_vec(),
        ),
        def(
            "edge",
            "Microsoft Edge",
            WEB,
            Base::Local,
            EDGE_CACHES.to_vec(),
        ),
        def("brave", "Brave", WEB, Base::Local, BRAVE_CACHES.to_vec()),
        def(
            "vivaldi",
            "Vivaldi",
            WEB,
            Base::Local,
            VIVALDI_CACHES.to_vec(),
        ),
        def(
            "firefox",
            "Firefox",
            WEB,
            Base::Local,
            vec![
                whole("Mozilla/Firefox/Profiles/*/*cache*"),
                whole("Mozilla/Firefox/Profiles/*/thumbnails"),
                whole("Mozilla/Firefox/Profiles/*/storage/temporary"),
                matching("Mozilla/Firefox/Profiles/*", &["*.corrupt"], true),
            ],
        ),
        def(
            "thunderbird",
            "Thunderbird",
            WEB,
            Base::Local,
            vec![whole("Thunderbird/Profiles/*/*cache*")],
        ),
        def(
            "vscode",
            "VS Code (cache e logs)",
            APPS,
            Base::Roaming,
            vec![
                whole("Code/Cache"),
                whole("Code/CachedData"),
                whole("Code/Code Cache"),
                whole("Code/logs"),
            ],
        ),
        def(
            "discord",
            "Discord",
            APPS,
            Base::Roaming,
            vec![
                whole("discord/Cache"),
                whole("discord/Code Cache"),
                whole("discord/GPUCache"),
                whole("discord/Service Worker"),
            ],
        ),
        def(
            "slack",
            "Slack",
            APPS,
            Base::Roaming,
            vec![
                whole("Slack/Cache"),
                whole("Slack/Code Cache"),
                whole("Slack/GPUCache"),
                whole("Slack/Service Worker"),
                whole("Slack/logs"),
            ],
        ),
        def(
            "spotify",
            "Spotify",
            APPS,
            Base::Local,
            vec![whole("Spotify/Data")],
        ),
        def(
            "npm",
            "npm",
            DEV,
            Base::Local,
            vec![whole("npm-cache/_cacache")],
        ),
        def("yarn", "Yarn", DEV, Base::Local, vec![whole("Yarn/Cache")]),
        def("pip", "pip", DEV, Base::Local, vec![whole("pip/cache")]),
        def(
            "gobuild",
            "Go (build cache)",
            DEV,
            Base::Local,
            vec![whole("go-build")],
        ),
        def(
            "pnpm-local",
            "pnpm (store)",
            DEV,
            Base::Local,
            vec![whole("pnpm/store"), whole("pnpm-store")],
        ),
        def(
            "nuget",
            "NuGet (http cache)",
            DEV,
            Base::Local,
            vec![whole("NuGet/v3-cache")],
        ),
    ];
    v.extend(common());
    v
}

#[cfg(target_os = "macos")]
pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        def(
            "caches",
            "Caches de apps (~/Library/Caches)",
            SYS,
            Base::Cache,
            vec![whole("")],
        ),
        def(
            "logs",
            "Logs e relatórios de falha",
            SYS,
            Base::Home,
            vec![whole("Library/Logs")],
        ),
        def(
            "xcode",
            "Xcode DerivedData",
            DEV,
            Base::Home,
            vec![whole("Library/Developer/Xcode/DerivedData")],
        ),
        def("npm", "npm", DEV, Base::Home, vec![whole(".npm/_cacache")]),
    ];
    v.extend(common());
    v
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        def(
            "xdgcache",
            "Caches de apps (~/.cache)",
            SYS,
            Base::Cache,
            vec![whole("")],
        ),
        def("npm", "npm", DEV, Base::Home, vec![whole(".npm/_cacache")]),
    ];
    v.extend(common());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_every_def_has_a_rule() {
        let defs = definitions();
        let mut ids: Vec<_> = defs.iter().map(|d| d.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), defs.len());
        assert!(defs.iter().all(|d| !d.rules.is_empty()));
    }

    #[test]
    fn no_profile_data_is_targeted() {
        // A test scans the definitions for these words: privacy data is never a "cache".
        const FORBIDDEN: &[&str] = &[
            "cookie",
            "login",
            "logins",
            "history",
            "bookmark",
            "session",
            "password",
            "credential",
            "token",
            "signon",
            "autofill",
            "web data",
            "key3.db",
            "key4.db",
        ];
        let bad = |s: &str| FORBIDDEN.iter().any(|w| s.to_lowercase().contains(w));
        for d in definitions() {
            for r in &d.rules {
                assert!(!bad(r.path), "{}: {}", d.id, r.path);
                for p in r.patterns {
                    assert!(!bad(p), "{}: {}", d.id, p);
                }
                for e in r.excludes {
                    assert!(!bad(e), "{}: {}", d.id, e);
                }
            }
        }
    }

    #[test]
    fn file_rules_are_well_formed() {
        for d in definitions() {
            for r in &d.rules {
                match r.scope {
                    Scope::Files => {
                        assert!(
                            !r.patterns.is_empty(),
                            "{}: {} has no pattern",
                            d.id,
                            r.path
                        );
                        for p in r.patterns {
                            assert!(!p.contains('/') && !p.contains('\\'), "{}: {p}", d.id);
                        }
                    }
                    Scope::Contents => assert!(r.patterns.is_empty(), "{}: {}", d.id, r.path),
                }
            }
        }
    }
}
