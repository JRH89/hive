import { ImageResponse } from "next/og";
import { postBySlug, posts } from "@/lib/posts";

// /blog/og/<slug>.png: a real .png path, so the static export and the
// trailing-slash rule leave it alone (an extensionless opengraph-image got
// redirected and served as application/octet-stream).

const size = { width: 1200, height: 630 };

export const dynamic = "force-static";
export const dynamicParams = false;

export function generateStaticParams() {
  return posts.map((p) => ({ file: `${p.slug}.png` }));
}

export async function GET(_: Request, { params }: { params: Promise<{ file: string }> }) {
  const post = postBySlug((await params).file.replace(/\.png$/, ""));
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
