//! Chromium/Electron app caches, launchers and messengers. Only cache, GPU, logs and crash folders;
//! never local storage, IndexedDB or anything that could hold a login.

use crate::junk_defs::{caches, Base, JunkDef, APPS};

/// The cache folders every Electron app keeps next to its profile.
macro_rules! electron {
    ($id:expr, $label:expr, $dir:literal) => {
        caches(
            $id,
            $label,
            APPS,
            Base::Config,
            &[
                concat!($dir, "/Cache"),
                concat!($dir, "/Code Cache"),
                concat!($dir, "/GPUCache"),
                concat!($dir, "/DawnGraphiteCache"),
                concat!($dir, "/DawnWebGPUCache"),
                concat!($dir, "/CachedData"),
                concat!($dir, "/CachedExtensionVSIXs"),
                concat!($dir, "/CachedProfilesData"),
                concat!($dir, "/Service Worker/CacheStorage"),
                concat!($dir, "/Service Worker/ScriptCache"),
                concat!($dir, "/Crashpad/completed"),
                concat!($dir, "/logs"),
            ],
        )
    };
}

pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        electron!(
            "vscode-insiders",
            "VS Code Insiders (cache e logs)",
            "Code - Insiders"
        ),
        electron!("vscodium", "VSCodium (cache e logs)", "VSCodium"),
        electron!("windsurf", "Windsurf (cache e logs)", "Windsurf"),
        electron!("trae", "Trae (cache e logs)", "Trae"),
        electron!("kiro", "Kiro (cache e logs)", "Kiro"),
        electron!("claude-desktop", "Claude Desktop (cache e logs)", "Claude"),
        electron!(
            "lmstudio-app",
            "LM Studio (cache do aplicativo)",
            "LM Studio"
        ),
        electron!("notion", "Notion (cache e logs)", "Notion"),
        electron!("postman", "Postman (cache e logs)", "Postman"),
        electron!("obsidian", "Obsidian (cache do aplicativo)", "obsidian"),
        electron!("figma", "Figma (cache e logs)", "Figma"),
        electron!("insomnia", "Insomnia (cache e logs)", "Insomnia"),
        electron!(
            "github-desktop",
            "GitHub Desktop (cache e logs)",
            "GitHub Desktop"
        ),
    ];
    // Windows already lists these four under Roaming in `junk_defs`.
    #[cfg(not(windows))]
    v.extend([
        electron!("vscode", "VS Code (cache e logs)", "Code"),
        electron!("cursor", "Cursor (cache e logs)", "Cursor"),
        electron!("discord", "Discord (cache e logs)", "discord"),
        electron!("slack", "Slack (cache e logs)", "Slack"),
    ]);
    v.extend(platform());
    v
}

#[cfg(windows)]
fn platform() -> Vec<JunkDef> {
    vec![
        caches(
            "epic-web",
            "Epic Games Launcher (cache web e logs)",
            APPS,
            Base::Local,
            &[
                "EpicGamesLauncher/Saved/webcache*",
                "EpicGamesLauncher/Saved/Logs",
            ],
        ),
        caches(
            "telegram",
            "Telegram (cache de mídia)",
            APPS,
            Base::Roaming,
            &[
                "Telegram Desktop/tdata/user_data/cache",
                "Telegram Desktop/tdata/user_data/media_cache",
            ],
        ),
        caches(
            "teams-new",
            "Microsoft Teams (cache do app novo)",
            APPS,
            Base::Local,
            &[
                "Packages/MSTeams_*/LocalCache/Microsoft/MSTeams/EBWebView/Default/Cache",
                "Packages/MSTeams_*/LocalCache/Microsoft/MSTeams/EBWebView/Default/Code Cache",
                "Packages/MSTeams_*/LocalCache/Microsoft/MSTeams/EBWebView/Default/GPUCache",
            ],
        ),
        caches(
            "onedrive-logs",
            "OneDrive (logs)",
            APPS,
            Base::Local,
            &["Microsoft/OneDrive/logs"],
        ),
    ]
}

#[cfg(target_os = "macos")]
fn platform() -> Vec<JunkDef> {
    vec![
        caches(
            "steam-cache",
            "Steam (shaders e cache web)",
            APPS,
            Base::Local,
            &["Steam/steamapps/shadercache", "Steam/config/htmlcache"],
        ),
        caches(
            "telegram",
            "Telegram (cache de mídia)",
            APPS,
            Base::Local,
            &[
                "Telegram Desktop/tdata/user_data/cache",
                "Telegram Desktop/tdata/user_data/media_cache",
            ],
        ),
        caches(
            "crash-reports",
            "Relatórios de falha do macOS",
            APPS,
            Base::Home,
            &["Library/Application Support/CrashReporter"],
        ),
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform() -> Vec<JunkDef> {
    vec![
        caches(
            "steam-cache",
            "Steam (shaders e cache web)",
            APPS,
            Base::Local,
            &["Steam/steamapps/shadercache", "Steam/config/htmlcache"],
        ),
        caches(
            "telegram",
            "Telegram (cache de mídia)",
            APPS,
            Base::Local,
            &[
                "TelegramDesktop/tdata/user_data/cache",
                "TelegramDesktop/tdata/user_data/media_cache",
            ],
        ),
        caches(
            "flatpak-snap",
            "Flatpak e Snap (caches dos apps)",
            APPS,
            Base::Home,
            &[".var/app/*/cache", "snap/*/common/.cache"],
        ),
    ]
}
