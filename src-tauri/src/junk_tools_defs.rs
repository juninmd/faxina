//! Package-manager, toolchain and creative-app caches beyond the core list in `junk_defs`.
//! Everything here is re-downloaded or re-generated on demand; none of it is user data.

use crate::junk_defs::{def, whole, Base, JunkDef, DEV};

pub fn definitions() -> Vec<JunkDef> {
    let v = vec![
        def(
            "cargo-src",
            "Cargo (fontes extraídos e repositórios git)",
            DEV,
            Base::Home,
            vec![
                whole(".cargo/registry/src"),
                whole(".cargo/git/checkouts"),
                whole(".cargo/git/db"),
            ],
        ),
        def(
            "ivy",
            "Ivy (cache de dependências JVM)",
            DEV,
            Base::Home,
            vec![whole(".ivy2/cache")],
        ),
        def(
            "node-gyp-home",
            "node-gyp (headers do Node)",
            DEV,
            Base::Home,
            vec![whole(".node-gyp")],
        ),
        def(
            "pub-home",
            "Dart/Flutter (cache de pacotes)",
            DEV,
            Base::Home,
            vec![
                whole(".pub-cache/hosted"),
                whole(".pub-cache/git"),
                whole(".pub-cache/_temp"),
            ],
        ),
    ];
    // Only the OS-specific lists mutate it, so Linux would warn on a plain `let mut`.
    #[cfg(any(windows, target_os = "macos"))]
    let mut v = v;
    #[cfg(any(windows, target_os = "macos"))]
    v.extend(home_cache());
    #[cfg(windows)]
    v.extend(windows());
    v
}

/// `~/.cache` is where these tools write on every OS; on Linux the whole folder is already
/// offered as "Caches de apps", so listing them again would count the same bytes twice.
#[cfg(any(windows, target_os = "macos"))]
fn home_cache() -> Vec<JunkDef> {
    vec![
        def(
            "ml-models",
            "Modelos de IA baixados (Hugging Face, PyTorch)",
            DEV,
            Base::Home,
            vec![whole(".cache/huggingface/hub"), whole(".cache/torch")],
        ),
        def(
            "browser-binaries",
            "Navegadores de teste (Puppeteer, Prisma)",
            DEV,
            Base::Home,
            vec![whole(".cache/puppeteer"), whole(".cache/prisma")],
        ),
        def(
            "pre-commit",
            "pre-commit (ambientes dos hooks)",
            DEV,
            Base::Home,
            vec![whole(".cache/pre-commit")],
        ),
    ]
}

#[cfg(windows)]
fn windows() -> Vec<JunkDef> {
    use crate::junk_defs::{matching, APPS, SYS};
    vec![
        def(
            "python-tools",
            "Python (uv, Poetry, Pipenv)",
            DEV,
            Base::Local,
            vec![
                whole("uv/cache"),
                // Cache/virtualenvs holds the projects' environments, so only the download caches go.
                whole("pypoetry/Cache/cache"),
                whole("pypoetry/Cache/artifacts"),
                whole("pipenv/Cache"),
            ],
        ),
        def(
            "node-binaries",
            "Binários baixados (Cypress, Electron, Playwright)",
            DEV,
            Base::Local,
            vec![
                whole("Cypress/Cache"),
                whole("electron/Cache"),
                whole("electron-builder/Cache"),
                whole("ms-playwright"),
                whole("node-gyp/Cache"),
            ],
        ),
        def(
            "yarn-berry",
            "Yarn Berry (cache global)",
            DEV,
            Base::Local,
            vec![whole("Yarn/Berry/cache")],
        ),
        def(
            "jvm-tools",
            "Coursier (cache de dependências JVM)",
            DEV,
            Base::Local,
            vec![whole("Coursier/Cache")],
        ),
        def(
            "composer",
            "Composer (PHP)",
            DEV,
            Base::Local,
            vec![
                whole("Composer/files"),
                whole("Composer/repo"),
                whole("Composer/vcs"),
            ],
        ),
        def(
            "pub-local",
            "Dart/Flutter (cache de pacotes)",
            DEV,
            Base::Local,
            vec![
                whole("Pub/Cache/hosted"),
                whole("Pub/Cache/git"),
                whole("Pub/Cache/_temp"),
            ],
        ),
        def(
            "sccache",
            "sccache (cache de compilação)",
            DEV,
            Base::Local,
            vec![whole("Mozilla/sccache/cache")],
        ),
        def(
            "jetbrains",
            "JetBrains (índices, caches e logs)",
            DEV,
            Base::Local,
            vec![
                whole("JetBrains/*/caches"),
                whole("JetBrains/*/log"),
                whole("JetBrains/*/tmp"),
                whole("Google/AndroidStudio*/caches"),
                whole("Google/AndroidStudio*/log"),
            ],
        ),
        def(
            "thumbs",
            "Miniaturas e ícones do Explorer",
            SYS,
            Base::Local,
            vec![matching(
                "Microsoft/Windows/Explorer",
                &["thumbcache_*.db", "iconcache_*.db"],
                false,
            )],
        ),
        def(
            "steam-web",
            "Steam (cache da loja e da comunidade)",
            APPS,
            Base::Local,
            vec![
                whole("Steam/htmlcache/Cache"),
                whole("Steam/htmlcache/Code Cache"),
                whole("Steam/htmlcache/GPUCache"),
            ],
        ),
        def(
            "cursor",
            "Cursor (cache e logs)",
            APPS,
            Base::Roaming,
            vec![
                whole("Cursor/Cache"),
                whole("Cursor/CachedData"),
                whole("Cursor/Code Cache"),
                whole("Cursor/GPUCache"),
                whole("Cursor/logs"),
            ],
        ),
        def(
            "teams",
            "Microsoft Teams (cache clássico)",
            APPS,
            Base::Roaming,
            vec![
                whole("Microsoft/Teams/Cache"),
                whole("Microsoft/Teams/Code Cache"),
                whole("Microsoft/Teams/GPUCache"),
                whole("Microsoft/Teams/blob_storage"),
                whole("Microsoft/Teams/tmp"),
            ],
        ),
        def(
            "adobe-media",
            "Adobe (cache de mídia e picos de áudio)",
            APPS,
            Base::Roaming,
            vec![
                whole("Adobe/Common/Media Cache Files"),
                whole("Adobe/Common/Media Cache"),
                whole("Adobe/Common/Peak Files"),
            ],
        ),
    ]
}
