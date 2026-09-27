//! Known regenerable locations, distilled from BleachBit cleaners and Winapp2.ini.
//! Only cache folders are listed: never cookies, history, logins or profile data.

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

pub struct JunkDef {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub base: Base,
    pub rels: &'static [&'static str],
    pub min_age: Duration,
}

#[cfg_attr(not(windows), allow(dead_code))]
const DAY: Duration = Duration::from_secs(24 * 3600);
const NOW: Duration = Duration::ZERO;

const fn def(
    id: &'static str,
    label: &'static str,
    group: &'static str,
    base: Base,
    rels: &'static [&'static str],
) -> JunkDef {
    JunkDef {
        id,
        label,
        group,
        base,
        rels,
        min_age: NOW,
    }
}

const SYS: &str = "Sistema";
#[cfg_attr(not(windows), allow(dead_code))]
const WEB: &str = "Navegadores";
#[cfg_attr(not(windows), allow(dead_code))]
const APPS: &str = "Apps";
const DEV: &str = "Desenvolvimento";

fn common() -> Vec<JunkDef> {
    vec![
        def(
            "bun",
            "Bun (cache de pacotes)",
            DEV,
            Base::Home,
            &[".bun/install/cache"],
        ),
        def(
            "cargo",
            "Cargo (tarballs do registry)",
            DEV,
            Base::Home,
            &[".cargo/registry/cache"],
        ),
        def(
            "gradle",
            "Gradle (caches)",
            DEV,
            Base::Home,
            &[".gradle/caches"],
        ),
    ]
}

#[cfg(windows)]
pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        // Files touched in the last day may belong to a running installer.
        JunkDef {
            min_age: DAY,
            ..def("temp", "Arquivos temporários", SYS, Base::Temp, &[""])
        },
        def(
            "inetcache",
            "Cache da Internet do Windows",
            SYS,
            Base::Local,
            &["Microsoft/Windows/INetCache"],
        ),
        def(
            "crashdumps",
            "Despejos de falhas",
            SYS,
            Base::Local,
            &["CrashDumps"],
        ),
        def(
            "wer",
            "Relatórios de erro do Windows",
            SYS,
            Base::Local,
            &[
                "Microsoft/Windows/WER/ReportArchive",
                "Microsoft/Windows/WER/ReportQueue",
            ],
        ),
        def(
            "shaders",
            "Cache de shaders (DirectX/GPU)",
            SYS,
            Base::Local,
            &[
                "D3DSCache",
                "NVIDIA/DXCache",
                "NVIDIA/GLCache",
                "AMD/DxCache",
            ],
        ),
        def(
            "chrome",
            "Google Chrome",
            WEB,
            Base::Local,
            &[
                "Google/Chrome/User Data/*/Cache",
                "Google/Chrome/User Data/*/Code Cache",
                "Google/Chrome/User Data/*/GPUCache",
            ],
        ),
        def(
            "edge",
            "Microsoft Edge",
            WEB,
            Base::Local,
            &[
                "Microsoft/Edge/User Data/*/Cache",
                "Microsoft/Edge/User Data/*/Code Cache",
                "Microsoft/Edge/User Data/*/GPUCache",
            ],
        ),
        def(
            "brave",
            "Brave",
            WEB,
            Base::Local,
            &[
                "BraveSoftware/Brave-Browser/User Data/*/Cache",
                "BraveSoftware/Brave-Browser/User Data/*/Code Cache",
                "BraveSoftware/Brave-Browser/User Data/*/GPUCache",
            ],
        ),
        def(
            "firefox",
            "Firefox",
            WEB,
            Base::Local,
            &["Mozilla/Firefox/Profiles/*/cache2"],
        ),
        def(
            "vscode",
            "VS Code (cache e logs)",
            APPS,
            Base::Roaming,
            &[
                "Code/Cache",
                "Code/CachedData",
                "Code/Code Cache",
                "Code/logs",
            ],
        ),
        def(
            "discord",
            "Discord",
            APPS,
            Base::Roaming,
            &["discord/Cache", "discord/Code Cache", "discord/GPUCache"],
        ),
        def("spotify", "Spotify", APPS, Base::Local, &["Spotify/Data"]),
        def("npm", "npm", DEV, Base::Local, &["npm-cache/_cacache"]),
        def("yarn", "Yarn", DEV, Base::Local, &["Yarn/Cache"]),
        def("pip", "pip", DEV, Base::Local, &["pip/cache"]),
        def(
            "gobuild",
            "Go (build cache)",
            DEV,
            Base::Local,
            &["go-build"],
        ),
        def(
            "nuget",
            "NuGet (http cache)",
            DEV,
            Base::Local,
            &["NuGet/v3-cache"],
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
            &[""],
        ),
        def(
            "logs",
            "Logs e relatórios de falha",
            SYS,
            Base::Home,
            &["Library/Logs"],
        ),
        def(
            "xcode",
            "Xcode DerivedData",
            DEV,
            Base::Home,
            &["Library/Developer/Xcode/DerivedData"],
        ),
        def("npm", "npm", DEV, Base::Home, &[".npm/_cacache"]),
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
            &[""],
        ),
        def("npm", "npm", DEV, Base::Home, &[".npm/_cacache"]),
    ];
    v.extend(common());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_every_def_has_a_path() {
        let defs = definitions();
        let mut ids: Vec<_> = defs.iter().map(|d| d.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), defs.len());
        assert!(defs.iter().all(|d| !d.rels.is_empty()));
    }

    #[test]
    fn no_profile_data_is_targeted() {
        for d in definitions() {
            for r in d.rels {
                let l = r.to_lowercase();
                assert!(
                    !["cookies", "login", "history", "bookmarks"]
                        .iter()
                        .any(|bad| l.contains(bad)),
                    "{r}"
                );
            }
        }
    }
}
