# Security

## Reporting a vulnerability

Please open a private advisory (Security → *Report a vulnerability*) rather than a public
issue. Include the app version, your platform and the steps to reproduce.

## Supported versions

Only the latest published release is supported: the app checks for updates on launch and
installs them with your approval.

## Dependency advisories

`src-tauri/Cargo.lock` and `bun.lock` are checked with `cargo audit` and `bun audit` when
dependencies change. The same two commands also run in CI, in the `Dependency audit
(informational)` job in `.github/workflows/ci.yml`, on every pull request and on pushes to `main`,
so a new finding shows up without waiting for someone to remember. That job is deliberately not a
gate (`continue-on-error` on the job and on each audit step), because the advisories below can
stay open until upstream moves. Every advisory is either fixed or recorded below with why it
cannot be fixed yet, its real exposure, an owner and a date to look at it again. Suppressing an
advisory without that record is not allowed.

| Advisory | Crate | Exposure | Why it is accepted | Owner | Review by |
|---|---|---|---|---|---|
| RUSTSEC-2024-0429<br>(GHSA-wrw7-89jp-8q8g) | `glib 0.18.5` | Linux only: the gtk-rs 0.18 train that `tauri`/`wry` pull in (`webkit2gtk 2.0.2 → gtk 0.18.2 → glib`, also reached through `muda`/`tao`) | Unsoundness in the `Iterator`/`DoubleEndedIterator` impls of `VariantStrIter`. Neither this app nor `wry` iterates that type; it is a soundness bug in an API we never call. The fix needs `glib >= 0.20`, but the whole gtk-rs 0.18 train has to move first and `webkit2gtk 2.0.2` is already its newest release. Blocked upstream. | @juninmd | 2027-01-17 |
| RUSTSEC-2026-0367, -0368, -0369 | `zip 4.6.1` | Windows only: the updater unpacks the `.zip` installer (`tauri-plugin-updater`, `#[cfg(windows)]`). The advisories are Unix permission and symlink semantics, so the reachable impact here is minimal | `ZipArchive::extract` follows symlink targets and applies Unix permission bits outside the destination. The app installs updates through `download_and_install`, which verifies the package's Minisign signature over the downloaded bytes **before** any extraction (`updater.rs:837`), so only archives signed with this repository's key reach `extract()`. The fix needs `zip >= 9`, and even `tauri-plugin-updater 2.14.0` still requires `zip ^4`. Blocked upstream. | @juninmd | 2027-01-17 |
| RUSTSEC-2026-0386 | `tokio 1.53.1` | Reached through `hyper` and `reqwest` (HTTP client used by the updater). The app never calls `AsyncFd` or its constructors: no `AsyncFd` in `src-tauri/src`. Unsoundness in `AsyncFd::new` and related constructors (I/O safety). No patched release is listed by `cargo audit` at the time of writing. Blocked upstream. | @juninmd | 2027-01-17 |

`rustls` was in this table: 0.23.46 patches RUSTSEC-2026-0346 through -0349, and the lockfile is
pinned to it.

Audit warnings that carry no advisory are tracked as ordinary dependency work:
`proc-macro-error 1.0.4` is unmaintained (build-time proc-macro, and no newer release exists);
`yoke-derive` moved off the yanked 0.8.3 to 0.8.4.

Advisories reported by `bun audit` live in devDependencies (semantic-release, Tailwind/Vite).
They run only in the build and release pipeline, are never shipped in the app bundle, and are
tracked as ordinary dependency updates rather than accepted risks.

### Rechecking

```console
cd src-tauri && cargo audit
bun audit
```
