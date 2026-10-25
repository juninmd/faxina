# AGENTS.md

Faxina: Tauri 2 desktop disk cleaner. Rust backend (`src-tauri/`) scans, classifies and deletes; React 19 + Tailwind v4 frontend (`src/`) renders the treemap/rings, the cleaners and the black-hole delete animation. UI copy is pt-BR; code, comments and commits are English.

## Commands

| Task | Command |
|---|---|
| Install | `bun install` |
| Run app | `bun tauri dev` |
| Lint / format | `bun run lint` / `bun run format` (Biome) |
| Types | `bun run typecheck` |
| Frontend tests | `bun run test` |
| Rust gates | `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` |
| Demo data | `bun scripts/demo-fixture.ts <new-dir>` (refuses existing paths) |
| Release bundle | `bun tauri build` (needs `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]` because `createUpdaterArtifacts` is on) |

`cargo` needs `../dist` to exist (`tauri::generate_context!` embeds it): run `bun run build` once before `cargo test` on a clean checkout.

CI gates (`.github/workflows/ci.yml`): commitlint (PRs), reusable `validate` (frontend), `cargo fmt --check` + `clippy -D warnings` + `cargo test` on Ubuntu/Windows/macOS, and an informational `bun audit`/`cargo audit` job (`continue-on-error`; open advisories live in `SECURITY.md`). `security.yml` runs the shared security scan; `release.yml` only runs after green CI on `main`.

## Layout

- `src-tauri/src/listing.rs` directory listing with file ids (Windows: one `FileIdBothDirectoryInfo` call per 64 KB of entries, NTFS only) so hard links count once; `scan.rs` parallel walk; `category.rs` name/extension rules; `view.rs` pruning, suggestions, biggest files, in-memory removal; `guard.rs` deletion checks; `junk.rs` + `junk_defs.rs` known caches (globs, file patterns, exclusions); `dupes.rs` duplicate finder; `empty.rs` empty folders and dangling links (links are removed only via `guard::check_dangling_link`, never through `delete_paths`); `history.rs` per-root scan snapshots in the app data dir (`history.json`) and the "what grew" comparison (read-only); `commands.rs` the only Tauri surface.
- `src/lib/api.ts` typed `invoke` wrappers (keep in sync with `commands.rs` serde shapes); `src/components/*` one component per file; `src/hooks/useHole.ts` drives the delete animation.

## Safety invariants (do not weaken)

- `Node.reclaimable` is set only by name rules or inheritance from a matching ancestor. Never derive it from the display `kind` (a folder *colored* Build still holds source code). Test: `dominant_color_does_not_make_a_folder_reclaimable`.
- Every path from the webview goes through `guard::check` against the current scan/duplicate roots before removal.
- Junk cleaning takes IDs, never paths; locations come only from `junk_defs.rs`. Never list cookies, logins, history or profile data (a test scans for those words). The trash/permanent choice picked in the dialog travels down to `junk::clean` — never assume one of them.
- Anything user-chosen or costly to get back (model stores, emulators, backups, Docker disks) lives in `junk_optin_defs.rs` through `opt_in(..)`: the UI leaves it unticked with the warning. Chat transcripts, cloud credential caches and profile data stay out entirely.
- Generic names (`cache`, `tmp`, `temp`) are Cache only under app data (`DirHints::under_app_data`); anywhere else they may be user work.
- Drive-root OS entries (`pagefile.sys`, `hiberfil.sys`, `System Volume Information`, …) are protected in `guard.rs` and filtered out of suggestions and of the biggest-files list.
- Default delete target is the OS trash; permanent delete is an explicit opt-in.
- The hard-link set belongs to one scan (`Links` is created inside `scan()`); keeping it on the long-lived `Progress` made every later scan report an empty tree. Test: `rescanning_with_the_same_progress_is_stable`.

## Release flow

Conventional Commits on `main` → CI green → `release.yml` runs semantic-release (bumps `package.json`, writes `CHANGELOG.md`, tags `vX.Y.Z`, creates a draft) → tauri-action builds and signs every platform into that draft (with `uploadUpdaterJson: false`) → `scripts/updater-manifest.ts` writes one `latest.json` from the signed assets (parallel jobs used to overwrite each other's entries) → the draft is published, which moves `releases/latest/download/latest.json` for the updater. The app version comes from `package.json` (`tauri.conf.json` → `"version": "../package.json"`); `Cargo.toml`'s version is not used. Never hand-edit `CHANGELOG.md` or the version.

## Gotchas

- Windows with "Animation effects" off reports `prefers-reduced-motion: reduce`; the black hole then runs its short form unless the user picks "Animações: sempre".
- `window.__TAURI_INTERNALS__` is frozen; to drive the dev app over CDP (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`), call React props through the fiber instead of patching `invoke`. From Git Bash, pass Windows paths with `/` — MSYS rewrites backslashes in arguments.
- Duplicate hashing of multi-GB files has no intra-file progress; the counter moves per file.

<!-- base-actions-main-policy -->
## GitHub Actions — padrão obrigatório (@main)

- Referencie as actions internas sempre pelo branch, nunca por SHA:
  - `uses: juninmd/base-actions/<action>@main`
  - `uses: juninmd/base-actions/.github/workflows/<arquivo>.yml@main`
- Não fixe `@<sha>` em `juninmd/base-actions`: a atualização é centralizada no
  `main` e passa a valer no próximo run.
- PRs cujos checks não iniciam por billing/spending limit são mesclados pelo bot central
  `juninmd/base-actions/.github/workflows/auto-merge-billing-blocked.yml`.
  Para impedir o merge automático de um PR, aplique a label `do-not-merge`.