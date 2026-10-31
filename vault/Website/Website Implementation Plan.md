# Hive Website Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an original terminal-first Hive landing page without interfering with concurrent Rust development.

**Architecture:** Independent static-export Next.js package in `website/`. Static page content and local SVG branding; a small client component handles copying commands.

**Tech Stack:** Next.js, React, TypeScript, Tailwind CSS, npm.

**Spec:** [[Website Design]] (`vault/Website/Website Design.md`).

## Global Constraints
- Exact tagline: One terminal. Many agents.
- Follow AGENTS §2 colors and restrained hexagon branding.
- Do not modify Rust, Cargo, or Claude-owned files.
- Use current verified capabilities; mark future features planned.
- No publication, invented URLs, installer, metrics, or license claims.

## Review Focus
- Narrow viewport and long commands: no body overflow; command areas may scroll.
- Clipboard denied or unavailable: accessible failure feedback and selectable command.
- Keyboard-only use: visible focus, semantic controls, working anchor navigation.
- Reduced motion: no required animation or typewriter delays.
- Product accuracy: current verified capabilities; daemon lifetime stated explicitly. Phase 2 completed during implementation, so sidebar/switching are current and reconnect/workspaces remain planned.

### Task 1: Independent website and complete page
**Files:** Create `website/package.json`, `package-lock.json`, `tsconfig.json`, `next-env.d.ts`, `next.config.ts`, `postcss.config.mjs`, `.gitignore`, `app/layout.tsx`, `app/page.tsx`, `app/globals.css`, `app/icon.svg`, `components/brand.tsx`, `components/copy-command.tsx`, `README.md`.

**Interfaces:** `Brand` renders reusable local SVG; `CopyCommand({command}: {command: string})` renders selectable text and clipboard control. Static export outputs `website/out/`.

- [x] Scaffold stable compatible framework dependencies and independent package lock; document Node requirements and npm commands.
- [x] Configure strict TypeScript, Tailwind, static export, metadata, and local branding.
- [x] Implement all sections from the design, with responsive CSS and accurate current/planned distinctions.
- [x] Implement clipboard control with success and error messages announced through `aria-live`.
- [x] Run `npm run typecheck` and `npm run build` from `website/`; resolve failures.
- [ ] Serve and visually inspect desktop and 375px layouts; test keyboard navigation, all links, copy success/failure, and reduced motion. Record any unavailable browser tooling.

Preview served HTTP 200; export/anchors/assets checked. Browser inventory empty, so visual and real interaction QA remain unverified. See [[Website Design#Implementation and evidence]].

### Task 2: Handoff and vault evidence
**Files:** Update `vault/Website/Website Design.md`, `vault/Log/2026-10-30.md`, and append website-only status to `vault/00-Index/Status.md`.

**Interfaces:** Existing phase status remains owned by Rust work; website notes link using Obsidian wikilinks.

- [x] Record exact verification commands and results, preview instructions, and remaining limitations.
- [x] Inspect `git diff --check` and `git status --short`; confirm changes stay inside owned paths.
- [ ] Mark website implementation complete only after validation. Leave Rust phase checkboxes unchanged.

Implementation/build complete; final browser acceptance remains open. No Rust roadmap checkboxes changed.
