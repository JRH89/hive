---
tags: [status]
updated: 2026-10-31
---
# Status

**Phase 6 complete; published 2026-10-30.** All six roadmap phases are merged to master. Public repo: https://github.com/JRH89/hive (MIT). CI is green on GitHub (fmt, clippy, tests, MSRV, static x86_64/aarch64 builds and smoke tests). Website: Vercel project `hive` (scope `jrh89s-projects`, root `website/`, deploys on push to master), domain `hive.codefrontlabs.com`. See [[Phase 6 - Testing and Distribution]].

**Released v0.1.0** (2026-10-30): https://github.com/JRH89/hive/releases/tag/v0.1.0. Static x86_64/aarch64 musl archives, checksums and `install.sh`. The website install section now shows Linux as available.

**Phase 8 done** ([[Phase 8 - Ease of use]]): herdr-style spaces/agents sidebar, Alt-arrow navigation, five optional integrations, `hive update`. **Phase 7 released as v0.2.0**: agent discovery, detection, status, controls, mouse, split panes and tabs are done (protocol v6). An optional Obsidian notes integration (off by default, [[ADR-0019 Optional integrations, off and invisible by default]]) is done too. See [[Phase 7 - herdr parity]].

**Phase 9 (macOS) done**: CI runs clippy, tests and smoke on macos-14; release builds aarch64/x86_64-apple-darwin ([[Phase 9 - macOS]]). **Phase 10 (Windows) done** ([[Phase 10 - Windows]]); **Released v0.4.0** (2026-10-31): Linux, macOS and Windows binaries, `install.ps1`.

**Released v0.4.11** (2026-10-31): closing a pane or tab ends its sessions; disabled keys explain themselves. v0.4.10: maintenance (restore_workspace ignored, CI on Node 24). v0.4.9: layouts (`[layouts.*]`, leader `L`, [[ADR-0023 Layouts are client-side]]). v0.4.8: right-click menus, git status + usage integrations, Why Hive page. v0.4.7: AGENTS lists every agent; website integrations section. v0.4.6: "+ new" opens a new space (protocol v7). v0.4.5: a space for every folder (not just repos). v0.4.4: spaces follow cd, worktree terminals (leader `C`), README screenshots, website changelog + RSS (`/changelog/rss.xml`).

**Open:** none. (`restore_workspace` is accepted and ignored so old configs load; removed from the spec.)

## Gate before every change
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Known gaps carried forward
- No scrollback viewing in the TUI (the daemon keeps scrollback in its emulator but the client only mirrors the visible screen). See [[Risks and Open Questions]].
- Shift+Enter is not distinguishable (no kitty keyboard protocol).
- Processes don't survive a daemon crash (by design, spec §4). Only metadata does, as `Lost` sessions ([[ADR-0009 Persist session metadata, mark lost sessions]]).
- Killing an interactive shell session waits the full kill timeout (bash ignores SIGTERM).
- New terminals start in the focused pane's directory; `cd` moves them to the right space (not on Windows).
- Worktrees can't be listed from the TUI, and custom branch names are CLI-only (`hive new -b`).
- Sidebar/picker keys aren't configurable; theme is fixed at startup.
- ~~No activity indicator for background sessions~~: done in Phase 7 via daemon-side detection ([[ADR-0016 Agent detection from process and herdr manifests]]).

## Website
- Next.js static export in `website/`, deployed by Vercel from GitHub (`vercel.json`: framework nextjs, image content types for the extensionless OG/apple-icon routes).
- SEO (2026-10-30): canonical, title/description/keywords, OG and Twitter large-image cards (generated 1200×630 PNG), robots.txt, sitemap.xml, manifest, apple icon, SoftwareApplication + FAQPage JSON-LD. Content updated to the finished roadmap. Verified by headless Chromium screenshots at 1440 and 390 px and by checking the exported head tags.
- DNS for codefrontlabs.com is on Cloudflare: `hive` is a CNAME to `cname.vercel-dns.com` (DNS only). Live since 2026-10-30, Vercel certificate valid to 2027-01-29.
- Website UI refinement (2026-10-30, local): actual SVG agent marks, fixed header, mobile navigation menu, and explicit Codefront Labs product credit. TypeScript and production static export pass; browser QA unavailable in this session. See [[Website Design]].
