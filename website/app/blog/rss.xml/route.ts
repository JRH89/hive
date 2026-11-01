import { posts } from "@/lib/posts";
import { siteUrl } from "@/lib/site";

export const dynamic = "force-static";

const esc = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

export function GET() {
  const items = posts
    .map((p) => {
      const link = `${siteUrl}/blog/${p.slug}/`;
      return `<item>
<title>${esc(p.title)}</title>
<link>${link}</link>
<guid isPermaLink="true">${link}</guid>
<pubDate>${new Date(p.date).toUTCString()}</pubDate>
<description>${esc(p.description)}</description>
${p.tags.map((t) => `<category>${esc(t)}</category>`).join("")}
</item>`;
    })
    .join("\n");
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
<title>Hive blog</title>
<link>${siteUrl}/blog/</link>
<atom:link href="${siteUrl}/blog/rss.xml" rel="self" type="application/rss+xml"/>
<description>Guides and release notes for running AI coding agents with Hive.</description>
<language>en</language>
<lastBuildDate>${new Date(posts[0].date).toUTCString()}</lastBuildDate>
${items}
</channel>
</rss>
`;
  return new Response(xml, {
    headers: { "Content-Type": "application/rss+xml; charset=utf-8" },
  });
}
