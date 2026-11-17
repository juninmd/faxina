//! Caches, logs and crash reports of popular Windows desktop apps and game launchers.
//!
//! Locations were picked by hand from the Winapp2.ini shipped with FluentCleaner
//! (<https://github.com/builtbybel/FluentCleaner>), which comes from the Winapp2 project
//! (<https://github.com/MoscaDotTo/Winapp2>, CC BY-SA 4.0). Only folder locations are reused, never
//! the entries themselves: whole folders that the app rebuilds on its own. Anything Winapp2 lists
//! next to them that could hold user data (cookies, recent files, chat databases, unsynced Drive
//! content, backups) is deliberately left out, as are `%ProgramFiles%` and registry keys.

use crate::junk_defs::JunkDef;
#[cfg(windows)]
use crate::junk_defs::{caches, Base, APPS};

/// `(id, label, folders)`; every folder is wiped whole.
#[cfg(windows)]
type Row = (&'static str, &'static str, &'static [&'static str]);

#[cfg(windows)]
const ROAMING: &[Row] = &[
    (
        "zoom",
        "Zoom (logs, relatórios e avatares)",
        &["Zoom/logs", "Zoom/Reports", "Zoom/data/ConfAvatar"],
    ),
    (
        "whatsapp-desktop",
        "WhatsApp Desktop (cache e logs)",
        &[
            "WhatsApp/*Cache",
            "WhatsApp/Crashpad/reports",
            "WhatsApp/logs",
        ],
    ),
    (
        "signal-desktop",
        "Signal Desktop (cache e logs)",
        &[
            "Signal/*Cache",
            "Signal/Crashpad/reports",
            "Signal/logs",
            "Signal/Temp",
        ],
    ),
    (
        "obs-studio",
        "OBS Studio (logs e relatórios de falha)",
        &[
            "obs-studio/logs",
            "obs-studio/crashes",
            "obs-studio/profiler_data",
        ],
    ),
    ("vlc", "VLC (capas de álbum em cache)", &["vlc/art"]),
    (
        "minecraft",
        "Minecraft (logs, relatórios de falha e cache web)",
        &[
            ".minecraft/logs",
            ".minecraft/crash-reports",
            ".minecraft/webcache2",
        ],
    ),
    (
        "libreoffice",
        "LibreOffice (temporários e relatórios de falha)",
        &["LibreOffice/*/user/temp", "LibreOffice/*/crash"],
    ),
    ("handbrake", "HandBrake (logs)", &["HandBrake/logs"]),
];

#[cfg(windows)]
const LOCAL: &[Row] = &[
    (
        "dropbox",
        "Dropbox (logs e relatórios de falha)",
        &[
            "Dropbox/logs",
            "Dropbox/CrashReports",
            "Dropbox/QuitReports",
        ],
    ),
    (
        "google-drive",
        "Google Drive (logs e miniaturas)",
        &[
            "Google/DriveFS/Logs",
            "Google/DriveFS/Crashpad/reports",
            "Google/DriveFS/*/Thumbnails_Cache",
        ],
    ),
    (
        "battlenet",
        "Battle.net (cache web e logs)",
        &[
            "Battle.net/BrowserCaches",
            "Battle.net/Logs",
            "Battle.net/Errors",
            "Blizzard Entertainment/Battle.net/Cache",
        ],
    ),
    (
        "ea-app",
        "EA App (cache e logs)",
        &[
            "EADesktop/cache",
            "EALaunchHelper/cache/qmlcache",
            "Electronic Arts/EA Desktop/Logs",
        ],
    ),
    (
        "riot-client",
        "Riot Client (logs)",
        &["Riot Games/Riot Client/Logs"],
    ),
    (
        "roblox",
        "Roblox (logs e instaladores baixados)",
        &["Roblox/logs", "Roblox/Downloads"],
    ),
    (
        "nvidia-app",
        "NVIDIA App (cache web)",
        &[
            "NVIDIA Corporation/NVIDIA App/CefCache/*Cache*",
            "NVIDIA Corporation/NVIDIA App/CefCache/Default/*Cache*",
        ],
    ),
    (
        "acrobat-reader",
        "Adobe Acrobat Reader (cache web)",
        &["Adobe/AcroCef/DC/Acrobat/Cache"],
    ),
    (
        "gimp",
        "GIMP (temporários e relatórios de falha)",
        &["GIMP/*/tmp", "GIMP/*/CrashLog"],
    ),
    ("krita", "Krita (cache)", &["krita/cache"]),
    (
        "audacity",
        "Audacity (relatórios de falha)",
        &["Audacity/CrashReports"],
    ),
    ("qbittorrent", "qBittorrent (logs)", &["qBittorrent/logs"]),
];

/// One `caches(..)` definition per row, all under `base`.
#[cfg(windows)]
fn table(base: Base, rows: &[Row]) -> Vec<JunkDef> {
    rows.iter()
        .map(|&(id, label, paths)| caches(id, label, APPS, base, paths))
        .collect()
}

#[cfg(windows)]
pub fn definitions() -> Vec<JunkDef> {
    let mut v = table(Base::Roaming, ROAMING);
    v.extend(table(Base::Local, LOCAL));
    v.push(caches(
        "java-deployment",
        "Java (cache de applets e Web Start)",
        APPS,
        Base::Home,
        &["AppData/LocalLow/Sun/Java/Deployment/cache"],
    ));
    v
}

#[cfg(not(windows))]
pub fn definitions() -> Vec<JunkDef> {
    Vec::new()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn only_whole_cache_folders_are_listed() {
        for d in definitions() {
            for r in &d.rules {
                assert_eq!(r.scope, crate::junk_defs::Scope::Contents, "{}", d.id);
                assert!(
                    !r.path.contains('\\') && !r.path.starts_with('/'),
                    "{}",
                    d.id
                );
            }
        }
    }
}
