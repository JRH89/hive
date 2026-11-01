import type { MetadataRoute } from "next";
import { releases } from "@/lib/changelog";
import { posts } from "@/lib/posts";
import { absolute } from "@/lib/site";

export const dynamic = "force-static";

// When each page's content last changed; bump with real edits only.
const updated = "2026-10-31T23:00:00-07:00";
const latest = releases[0].date;

export default function sitemap(): MetadataRoute.Sitemap {
  const page = (path: string, lastModified: string, priority: number) => ({
    url: absolute(path),
    lastModified,
    priority,
  });
  return [
    page("/", latest, 1),
    page("/install/", updated, 0.9),
    page("/features/", "2026-11-01T12:00:00-07:00", 0.9),
    page("/integrations/", updated, 0.8),
    page("/why/", updated, 0.8),
    page("/blog/", posts[0].date, 0.7),
    ...posts.map((p) => page(`/blog/${p.slug}/`, p.updated ?? p.date, 0.7)),
    page("/changelog/", latest, 0.5),
  ];
}
