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

- `src-tauri/src/listing.rs` directory listing with file ids (Windows: one `FileIdBothDirectoryInfo` call per 64 KB of entries, NTFS only) so hard links count once; `scan.rs` parallel walk; `category.rs` name/extension rules; `view.rs` pruning, suggestions, biggest files, in-memory removal; `guard.rs` deletion checks; `junk.rs` (`preview` is the dry run behind the dialog's "Simular": same plan as `clean`, removes nothing) + `junk_defs.rs` known caches (globs, file patterns, exclusions); `junk_leftovers_defs.rs` Windows setup/update logs, kernel dumps, Windows Update downloads, `Windows.old` and installer payloads (the big ones opt-in; never `Windows\Installer`, `Package Cache` or `WinSxS`); `junk_projects.rs` finds project roots on every volume by manifest (`Marker`: Gradle, Maven, Flutter, Composer, Terraform, .NET, Haskell, Elixir, Zig, plus Node/Rust/Python) and their build folders are opt-in in `junk_optin_defs.rs`; `dupes.rs` duplicate finder (skips dot-folders below the root, OS/credential trees and `.DS_Store`/`Thumbs.db`; a file whose size or mtime changed while hashing is dropped; `ExcludedNames` skips exact names the user typed, never paths or wildcards); `empty.rs` empty folders and dangling links (links are removed only via `guard::check_dangling_link`, never through `delete_paths`); `worktrees.rs` linked git worktrees whose commits are all on a remote-tracking ref (git CLI with a cleared environment and `core.fsmonitor` off; only `.git` files pointing into `worktrees/`, never submodules; a repository declaring its own `filter.*` is refused instead of inspected because `git status` would execute it); `history.rs` per-root scan snapshots in the app data dir (`history.json`) and the "what grew" comparison (read-only); `commands.rs` the only Tauri surface.
- UI shell ("Órbita"): `Rail.tsx` switches four spaces (`src/lib/spaces.ts` maps every `Page` to Agora/Explorar/Limpar/Ajustes and feeds the Ctrl K `CommandPalette`); `Orbit.tsx` is the one tray for map marks and picked cache categories, and its "Engolir" goes through `ConfirmDialog` then `swallowOrbit` in `App.tsx` (`src/lib/orbit.ts` `planOrbit` routes each item to the command allowed to remove it: map marks, big files, duplicates and empty folders via `delete_paths` with the chosen destination; dangling links via `delete_broken_links` and worktrees via `delete_worktrees`, both always trash; caches via `junk_clean` by ID). The panels also keep their own direct delete buttons; after a swallow they remount (`epoch`) so no removed row stays listed. Palette colors come from overriding Tailwind's slate/amber scales in `styles.css`; shell styles live in `orbit.css` (avoid class names already in `theme.css`, e.g. `.orbit`, `.app-shell`).
- `src/lib/api.ts` typed `invoke` wrappers (keep in sync with `commands.rs` serde shapes); `src/components/*` one component per file; `src/hooks/useHole.ts` drives the delete animation.

## Safety invariants (do not weaken)

- `Node.reclaimable` is set only by name rules or inheritance from a matching ancestor. Never derive it from the display `kind` (a folder *colored* Build still holds source code). Test: `dominant_color_does_not_make_a_folder_reclaimable`.
- Every path from the webview goes through `guard::check` against the current scan/duplicate roots before removal. Reviewed junk caches living inside `%SystemRoot%`/`%ProgramData%` (Windows Update downloads, log and installer leftovers) go through `guard::check_reviewed_cache`, which relaxes only the OS-tree rule: links, drive-root OS entries, the user's known folders and paths outside the analysed root stay refused. `C:\Windows\Installer`, `ProgramData\Package Cache` and `WinSxS` are never targets. Test: `reviewed_caches_inside_os_trees_are_allowed_but_nothing_else_is`. `delete_paths` also checks the paths against the duplicate groups last shown (`dupes::refused_selection`): it refuses a request that takes every copy of a group and any file whose size or mtime changed since the search. Test: `a_request_can_never_take_every_copy_of_a_group`.
- A worktree leaves the disk only after git confirms every commit is on a remote-tracking ref, the tree is clean and the entry is not locked; `delete_worktrees` only accepts paths from the list last shown, re-runs that check per path (a local edit after the search is refused) and removes only the admin dir of that one worktree, always to the OS trash. The git call has a deadline and a cleared `GIT_*` environment (machine config stays in play so filters like `filter.lfs` behave as usual), a repository-declared `filter.*` is refused, and the admin dir must canonically sit under the repository's common dir. Test: `a_removal_is_bound_to_the_worktrees_last_shown`, `removal_refuses_changes_made_after_the_search_then_cleans_the_git_metadata`.
- Junk cleaning takes IDs, never paths; locations come only from `junk_defs.rs`. Never list cookies, logins, history or profile data (a test scans for those words). The trash/permanent choice picked in the dialog travels down to `junk::clean` — never assume one of them.
- Anything user-chosen or costly to get back (model stores, emulators, backups, Docker disks) lives in `junk_optin_defs.rs` through `opt_in(..)`: the UI leaves it unticked with the warning. Chat transcripts, cloud credential caches and profile data stay out entirely.
- Generic names (`cache`, `tmp`, `temp`) are Cache only under app data (`DirHints::under_app_data`); anywhere else they may be user work.
- Drive-root OS entries (`pagefile.sys`, `hiberfil.sys`, `System Volume Information`, …) are protected in `guard.rs` and filtered out of suggestions and of the biggest-files list.
- `JunkDef.procs` names programs that keep a cache open (browsers on Windows). `junk::clean` refuses those targets while the process runs and `JunkItem.running` lets the UI say which one to close. Test: `a_running_program_blocks_its_definition_only`.
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