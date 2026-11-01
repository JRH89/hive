import { ImageResponse } from "next/og";
import { postBySlug, posts } from "@/lib/posts";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = "Hive blog post";

export function generateStaticParams() {
  return posts.map((p) => ({ slug: p.slug }));
}

export default async function Image({ params }: { params: Promise<{ slug: string }> }) {
  const post = postBySlug((await params).slug);
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: 72,
          background: "#111318",
          color: "#e8eaf0",
          fontFamily: "sans-serif",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 18, fontSize: 34, color: "#f2b84b" }}>
          <div style={{ width: 22, height: 22, background: "#f2b84b" }} />
          HIVE BLOG
        </div>
        <div style={{ display: "flex", fontSize: 68, fontWeight: 700, lineHeight: 1.1, letterSpacing: -1 }}>
          {post?.title ?? "Hive"}
        </div>
        <div style={{ display: "flex", fontSize: 28, color: "#8992a3" }}>
          hive.codefrontlabs.com · terminal multiplexer for AI coding agents
        </div>
      </div>
    ),
    size,
  );
}
