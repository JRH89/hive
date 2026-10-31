---
tags: [status]
updated: 2026-10-30
---
# Status

**Phase 6 complete; published 2026-10-30.** All six roadmap phases are merged to master. Public repo: https://github.com/JRH89/hive (MIT). CI is green on GitHub (fmt, clippy, tests, MSRV, static x86_64/aarch64 builds and smoke tests). Website: Vercel project `hive` (scope `jrh89s-projects`, root `website/`, deploys on push to master), domain `hive.codefrontlabs.com`. See [[Phase 6 - Testing and Distribution]].

**Released v0.1.0** (2026-10-30): https://github.com/JRH89/hive/releases/tag/v0.1.0. Static x86_64/aarch64 musl archives, checksums and `install.sh`. The website install section now shows Linux as available.

**Phase 7 in progress** on branch `phase-7-herdr-parity` (uncommitted): agent discovery, detection and status are done; controls/mouse and panes/tabs are next. See [[Phase 7 - herdr parity]].

**Open:** macOS runtime testing. `mouse_support` and `restore_workspace` config keys are inert.

## Gate before every change
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Known gaps carried forward
- No scrollback viewing in the TUI (the daemon keeps scrollback in its emulator but the client only mirrors the visible screen). See [[Risks and Open Questions]].
- Mouse is not captured or forwarded.
- Shift+Enter is not distinguishable (no kitty keyboard protocol).
- Processes don't survive a daemon crash (by design, spec §4). Only metadata does, as `Lost` sessions ([[ADR-0009 Persist session metadata, mark lost sessions]]).
- Killing an interactive shell session waits the full kill timeout (bash ignores SIGTERM).
- New sessions launched from the TUI start in the active workspace's root (or `hive`'s cwd). No free-form directory prompt.
- Worktrees can't be listed from the TUI, and custom branch names are CLI-only (`hive new -b`).
- Sidebar/picker keys aren't configurable; theme is fixed at startup.
- ~~No activity indicator for background sessions~~: done in Phase 7 via daemon-side detection ([[ADR-0016 Agent detection from process and herdr manifests]]).

## Website
- Next.js static export in `website/`, deployed by Vercel from GitHub (`vercel.json`: framework nextjs, image content types for the extensionless OG/apple-icon routes).
- SEO (2026-10-30): canonical, title/description/keywords, OG and Twitter large-image cards (generated 1200×630 PNG), robots.txt, sitemap.xml, manifest, apple icon, SoftwareApplication + FAQPage JSON-LD. Content updated to the finished roadmap. Verified by headless Chromium screenshots at 1440 and 390 px and by checking the exported head tags.
- DNS for codefrontlabs.com is on Cloudflare: `hive` is a CNAME to `cname.vercel-dns.com` (DNS only). Live since 2026-10-30, Vercel certificate valid to 2027-01-29.
