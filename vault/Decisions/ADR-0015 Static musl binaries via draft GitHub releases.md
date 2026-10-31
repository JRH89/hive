---
status: accepted
date: 2026-10-30
---
# ADR-0015: Static musl binaries via draft GitHub releases

## Context
Phase 6 needs Linux release binaries and install instructions. glibc builds tie binaries to the build host's glibc version. The website already advertises `releases/latest/download/install.sh` for `jrh89/hive`. No dependency in the build graph compiles C.

## Decision
| Piece | Choice |
|---|---|
| Targets | `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` (static-pie, ~5 MB each), built natively on `ubuntu-24.04` / `ubuntu-24.04-arm` runners. No cross toolchain |
| Assets | `hive-<target>.tar.gz` + `.sha256` + `install.sh`. **No version in asset names**, so `releases/latest/download/<name>` always resolves |
| Archive | `hive`, `hived` (must stay side by side), README, `docs/`, `config.example.toml` |
| Installer | POSIX `sh`; picks the target from `uname -m`, verifies SHA-256 (fails closed), installs both binaries atomically to `~/.local/bin` (`HIVE_INSTALL_DIR`, `HIVE_VERSION`, `HIVE_DOWNLOAD_URL` overrides). Warns that an old `hived` may be running |
| Pipeline | Tag `v*` → check tag equals the crate version → test → build → `scripts/smoke.sh` → package → **draft** release. A maintainer publishes it by hand |
| Profile | `lto = "thin"`, `strip = "debuginfo"` (keep symbols for readable panic backtraces) |

## Consequences
- Releases need no runtime dependencies beyond a Linux kernel.
- Nothing is published automatically. A bad tag yields a draft, not a public release.
- The archive ships `LICENSE` (MIT, chosen 2026-10-30).
- macOS/Windows need separate work: PTY and daemon details differ (see [[Phase 6 - Testing and Distribution]]).
