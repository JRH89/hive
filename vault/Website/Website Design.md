---
tags: [website, design]
status: implemented-browser-qa-pending
date: 2026-10-30
---
# Website Design

## Intent and approval
Introduce Hive to developers who already use terminal coding agents. The user approved the terminal-first landing-page direction and requested Next.js, React, TypeScript, Tailwind, existing AGENTS branding, and isolation from Claude's Rust work. Detailed implementation plan approved on 2026-10-30; implemented in `website/`.

## Approaches
| Approach | Tradeoff | Outcome |
|---|---|---|
| Terminal-first landing page | Explains the experience visually; needs honest preview labeling | Selected by user |
| Documentation-first site | Direct command reference; less immediate product introduction | Not selected |

## Visual design
- Follow AGENTS §2: charcoal `#111318`, slate `#1C2028`, amber `#F2B84B`, soft white `#E8EAF0`, muted gray `#8992A3`.
- Minimal SVG hexagon containing a terminal cursor; reuse as favicon. No required icon fonts, decorative gradients, or bee illustrations.
- Strong, spacious typography with monospace for commands and terminal content. Self-hosted Geist and Geist Mono variable fonts use next/font/local from pinned npm packages, with system fallbacks; no build-time font downloads.
- Restrained grid, thin rules, visible keyboard focus, responsive layout and reduced-motion support.

## Page structure
1. Header: Hive mark, Features, How it works, Get started.
2. Hero: exact tagline **One terminal. Many agents.**, concise product description, early-development badge, build-from-source CTA.
3. Large illustrative multi-session preview: selectable Claude/Codex/shell sessions and a local detach/reattach demonstration, explicitly labeled illustrative. No simulated chat service or live-daemon connection.
4. Supported launch commands: Claude Code, Codex, Gemini CLI, OpenCode, custom commands; users install agents separately.
5. Features: real PTYs, independent background daemon, familiar terminal input.
6. Architecture: terminal frontend → local Unix socket → daemon → PTY/agent, with explicit daemon-lifetime limitation.
7. Getting started: verified repository-local build commands, keep binaries together, launch and detach instructions, clipboard feedback with fallback.
8. Roadmap: multiplexing is current per the newly verified Phase 2; automatic reconnect is next, with workspaces/worktrees still planned.
9. FAQ and minimal footer; no fabricated public repository URL or license.

## Technical boundaries
Independent Next.js static-export package in `website/`, React, strict TypeScript, Tailwind, local assets. Server-render static content; client components limited to clipboard interaction and the illustrative session preview. No database, analytics, accounts, daemon API, cloud service, or Rust dependency changes. Deployment is outside this repository-local task.

## Research
Reviewed https://herdr.dev/tui/ and https://herdr.apposters.com/. Take inspiration from tangible terminal previews, clear persistence explanations, and accessible getting-started paths. Do not reuse copy, layouts, assets, metrics, or unsupported remote/orchestration claims.

## Acceptance
Production export and typecheck succeed. At 375px and desktop widths the page remains readable without body overflow. Keyboard navigation, anchors and clipboard failure feedback work. Metadata and favicon are original. Rust changes are untouched. Record actual verification in [[2026-10-30]].

## Implementation and evidence
- Concurrent update: Claude completed and recorded Phase 2 during website work. Copy was refreshed against [[Phase 2 - Multiplexing]] and README: sidebar, switching and labels are working; reconnect/workspaces/worktrees remain planned. The illustration still demonstrates one detach/reattach workflow.
- Independent Next.js 16.4.3 / React 19.3.1 / Tailwind 4.3.3 / strict TypeScript package, with reproducible npm lockfile. Static export in ignored `website/out/`; no Rust dependency changes.
- Original hexagon/cursor SVG mark and favicon, responsive charcoal/amber page, local system fonts, no external assets or analytics. Terminal preview is labeled illustrative.
- `npm run typecheck`: exit 0. `npm run build`: exit 0, static homepage, not-found and icon routes generated. Export totals approximately 1.2 MiB including framework files.
- Export smoke check: one h1, unique IDs, 13 valid anchor links, eight asset references present, no external asset requests, illustration/planned/daemon-lifetime labels present.
- `npm run preview`: loopback server started; homepage HTTP 200 confirmed. Initial sandbox blocked listening/access; checks succeeded with approved network permissions.
- Independent static review: no critical/important findings. Minor: repeated successful copies may not repeat the live announcement while the button still says Copied; initial success and failure are announced. Retained as a minor follow-up.
- Browser inventory was empty. Desktop/375px rendering, actual clipboard behavior, keyboard interaction and reduced-motion behavior were reviewed in source but **not verified in a browser**. Manual QA remains open; see [[Website Implementation Plan]].
- No deployment, merge, or commit performed. Existing Rust work remains owned by Claude.

- Final build backend: Next.js Webpack (`next build --webpack`, also selected for dev). Turbopack initially passed but failed on rebuild when its internal worker attempted a sandbox-restricted local port; retry with escalation reproduced the failure. Webpack production build and typecheck passed on final content. Repeated export check: 13 valid anchors, eight referenced local assets, Phase 2 claims present.

## Polish pass — 2026-10-30
- User requested refinement, then supplied screenshots rejecting the oversized hero and sparse, stretched setup section. Final direction: restrained headline (desktop max 76px, previously 118px), reduced hero padding, balanced section typography, and compact three-step setup cards instead of a tall split-column block.
- Self-hosted Geist/Geist Mono variable fonts, original SVG line icons, consistent border/focus/hover treatment and optional short entry animation; reduced motion disables animation. Font licenses included in static output.
- Preview now offers three clearly illustrative selectable sessions and detach/reattach. Running daemon state remains explicit, no speculative thinking/attention states, no live service. Corrected shell launch adapter to `shell` after code review.
- Clipboard success resets after 2.4 seconds with timer cleanup; permission failure continues to show manual-selection fallback.
- New static review found one important example-command issue (`sh` vs `shell`) and two minor copy issues; all corrected and reviewer confirmed no remaining critical/important findings. Browser tools still expose no surfaces. Supplied screenshots guided scale/layout correction; final browser visual QA remains unperformed.

- Additional user request: subtle hive-like hero background. Added original inline SVG honeycomb clusters on the hero sides, two restrained amber cells and small connector lines, with an open center. Decorative, pointer-transparent, no image requests or animation; reduced opacity on mobile.

## Release section — user-confirmed scope
- User expects future downloadable releases and OS-specific installation commands; confirmed public GitHub destination `jrh89/hive`, with no final asset URLs yet.
- Added Linux/macOS/Windows selector, shell commands for Linux/macOS and PowerShell command for Windows. Proposed GitHub asset contract: `install.sh` / `install.ps1`, using documented latest/download links. Commands labeled planned/not available; installer copying absent and binary downloads disabled until assets are actually published.
- Current Linux source steps remain in a compact native disclosure. macOS/Windows support stays explicitly planned. No packaging/installer implementation, release publication, or Rust changes.
- Fresh reviewer checked release section: no critical/important findings, accurate unavailable states, named OS controls and internal scrolling for long commands. No final browser interaction result claimed.

## Agent marks and navigation — 2026-10-30
- Context: agent strip used font-dependent Unicode placeholders (Claude rendered as an emoji); narrow layouts hid navigation links.
- Decision: use local Lobe Icons SVG marks for Claude, Codex, Gemini and OpenCode, with their MIT license and source attribution. Keep the restrained monochrome treatment. Fix the header at the top with matching document/anchor offsets; expose all links in a mobile menu with expanded state, Escape handling and focus-exit dismissal. Footer explicitly reads “A Codefront Labs product. Built with Rust.”
- Consequences: no third-party asset requests. Mobile navigation uses a small client component. Browser interaction/visual QA remains unverified in this session because no browser surfaces are connected.
