# Security

## Reporting a vulnerability

Please open a private advisory (Security → *Report a vulnerability*) rather than a public
issue. Include the app version, your platform and the steps to reproduce.

## Supported versions

Only the latest published release is supported: the app checks for updates on launch and
installs them with your approval.

## Dependency advisories

`src-tauri/Cargo.lock` and `bun.lock` are checked with `cargo audit` and `bun audit` when
dependencies change. That check is a manual step today: CI runs fmt, clippy, tests and the
frontend gates, but no audit gate. Every advisory is either fixed or recorded below with why it
cannot be fixed yet, its real exposure, an owner and a date to look at it again. Suppressing an
advisory without that record is not allowed.

| Advisory | Crate | Exposure | Why it is accepted | Owner | Review by |
|---|---|---|---|---|---|
| RUSTSEC-2024-0429<br>(GHSA-wrw7-89jp-8q8g) | `glib 0.18.5` | Linux only: the gtk-rs 0.18 train that `tauri`/`wry` pull in (`webkit2gtk 2.0.2 → gtk 0.18.2 → glib`, also reached through `muda`/`tao`) | Unsoundness in the `Iterator`/`DoubleEndedIterator` impls of `VariantStrIter`. Neither this app nor `wry` iterates that type; it is a soundness bug in an API we never call. The fix needs `glib >= 0.20`, but the whole gtk-rs 0.18 train has to move first and `webkit2gtk 2.0.2` is already its newest release. Blocked upstream. | @juninmd | 2027-01-17 |
| RUSTSEC-2026-0367, -0368, -0369 | `zip 4.6.1` | Windows only: the updater unpacks the `.zip` installer (`tauri-plugin-updater`, `#[cfg(windows)]`). The advisories are Unix permission and symlink semantics, so the reachable impact here is minimal | `ZipArchive::extract` follows symlink targets and applies Unix permission bits outside the destination. The app installs updates through `download_and_install`, which verifies the package's Minisign signature over the downloaded bytes **before** any extraction (`updater.rs:837`), so only archives signed with this repository's key reach `extract()`. The fix needs `zip >= 9`, and even `tauri-plugin-updater 2.14.0` still requires `zip ^4`. Blocked upstream. | @juninmd | 2027-01-17 |

`rustls` was in this table: 0.23.46 patches RUSTSEC-2026-0346 through -0349, and the lockfile is
pinned to it.

Audit warnings that carry no advisory are tracked as ordinary dependency work:
`proc-macro-error 1.0.4` is unmaintained (build-time proc-macro, no newer release exists) and
`yoke-derive 0.8.3` is yanked, with 0.8.4 available.

Advisories reported by `bun audit` live in devDependencies (semantic-release, Tailwind/Vite).
They run only in the build and release pipeline, are never shipped in the app bundle, and are
tracked as ordinary dependency updates rather than accepted risks.

### Rechecking

```console
cd src-tauri && cargo audit
bun audit
```
