import { releases } from "@/lib/changelog";

export const dynamic = "force-static";

const site = "https://hive.codefrontlabs.com";

const esc = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

export function GET() {
  const items = releases
    .map((r) => {
      const link = `${site}/changelog/#v${r.version}`;
      const body = `<ul>${r.changes.map((c) => `<li>${esc(c)}</li>`).join("")}</ul>`;
      return `<item>
<title>${esc(`v${r.version}: ${r.title}`)}</title>
<link>${link}</link>
<guid isPermaLink="false">hive-v${r.version}</guid>
<pubDate>${new Date(r.date).toUTCString()}</pubDate>
<description>${esc(body)}</description>
</item>`;
    })
    .join("\n");
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
<title>Hive releases</title>
<link>${site}/changelog/</link>
<atom:link href="${site}/changelog/rss.xml" rel="self" type="application/rss+xml"/>
<description>What changed in each Hive release.</description>
<language>en</language>
<lastBuildDate>${new Date(releases[0].date).toUTCString()}</lastBuildDate>
${items}
</channel>
</rss>
`;
  return new Response(xml, {
    headers: { "Content-Type": "application/rss+xml; charset=utf-8" },
  });
}
