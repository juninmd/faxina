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

## Layout

- `src-tauri/src/scan.rs` parallel walk; `category.rs` name/extension rules; `view.rs` pruning, suggestions, in-memory removal; `guard.rs` deletion checks; `junk.rs` + `junk_defs.rs` known caches; `dupes.rs` duplicate finder; `commands.rs` the only Tauri surface.
- `src/lib/api.ts` typed `invoke` wrappers (keep in sync with `commands.rs` serde shapes); `src/components/*` one component per file; `src/hooks/useHole.ts` drives the delete animation.

## Safety invariants (do not weaken)

- `Node.reclaimable` is set only by name rules or inheritance from a matching ancestor. Never derive it from the display `kind` (a folder *colored* Build still holds source code). Test: `dominant_color_does_not_make_a_folder_reclaimable`.
- Every path from the webview goes through `guard::check` against the current scan/duplicate roots before removal.
- Junk cleaning takes IDs, never paths; locations come only from `junk_defs.rs`. Never list cookies, logins, history or profile data (a test scans for those words).
- Default delete target is the OS trash; permanent delete is an explicit opt-in.

## Release flow

Conventional Commits on `main` → CI green → `release.yml` runs semantic-release (bumps `package.json`, writes `CHANGELOG.md`, tags `vX.Y.Z`, creates a draft) → tauri-action builds and signs every platform into that draft → the draft is published, which moves `releases/latest/download/latest.json` for the updater. The app version comes from `package.json` (`tauri.conf.json` → `"version": "../package.json"`); `Cargo.toml`'s version is not used. Never hand-edit `CHANGELOG.md` or the version.

## Gotchas

- Windows with "Animation effects" off reports `prefers-reduced-motion: reduce`; the black hole then runs its short form unless the user picks "Animações: sempre".
- `window.__TAURI_INTERNALS__` is frozen; to drive the dev app over CDP (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`), call React props through the fiber instead of patching `invoke`. From Git Bash, pass Windows paths with `/` — MSYS rewrites backslashes in arguments.
- Duplicate hashing of multi-GB files has no intra-file progress; the counter moves per file.
