# Hive website

Independent Next.js / React / TypeScript / Tailwind landing page. This package does not change the Rust build or require Node to use Hive.

## Develop

Node.js 22.13+ and npm are required. From `website/`:

```sh
npm ci
npm run dev
```

Open http://127.0.0.1:4310. The server binds to loopback only.

Development and production use Next.js's supported Webpack backend. This avoids Turbopack's internal worker-port requirement in restricted environments.

## Verify and export

```sh
npm run typecheck
npm run build
npm run preview
```

The static export lives in `out/`; preview serves it at http://127.0.0.1:4310. Stop with Ctrl+C. Hosting needs only the contents of `out/`, including `_next/`; no Next.js server, secrets, account, or database is needed. Deployment is not configured.

## Content and design

- Brand source: repository `AGENTS.md`, section 2.
- Design and plan: `vault/Website/`.
- Check `vault/00-Index/Status.md` and implementation before changing feature claims. The illustrative preview lets visitors select Claude Code, Codex, or a shell, and try detaching/reattaching. It is a local presentation, not a live agent or daemon connection.
- Future capabilities must remain clearly labeled planned. Do not invent public repository links, release downloads, install scripts, license, or usage statistics.
- Local SVG branding and self-hosted variable Geist / Geist Mono fonts (licenses in public/licenses/); no externally hosted assets, tracking, or build-time font downloads.
- Clipboard buttons announce success or offer manual selection when unavailable. Native FAQ disclosures work without JavaScript. Reduced-motion preference disables smooth scrolling.

## Manual QA

Check 375px and desktop widths, keyboard focus and skip link, every navigation anchor, FAQ disclosures, and clipboard success/denial. Long command blocks may scroll within their panel; the page body should not overflow. Check reduced motion. Record actual checks in the vault; do not infer browser QA from a successful build.

## Release installation section

The user-confirmed release destination is `https://github.com/jrh89/hive/releases`. `components/release-install.tsx` contains planned Linux/macOS/Windows install UI. All platforms currently have `published: false`; there are no live installer copy controls or binary downloads.

The proposed asset contract is `install.sh` for Linux/macOS and `install.ps1` for Windows, using GitHub's `/releases/latest/download/<asset>` convention. Those filenames and commands are previews, not claims that assets exist. Before enabling a platform, implement and test its installer, publish matching binaries and scripts, configure actual download URLs, and update the availability copy. macOS/Windows remain planned support. No installer scripts or release pipeline were implemented in this website task.
