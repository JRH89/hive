import type { MetadataRoute } from "next";

export const dynamic = "force-static";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: { userAgent: "*", allow: "/" },
    sitemap: "https://hive.codefrontlabs.com/sitemap.xml",
    host: "https://hive.codefrontlabs.com",
  };
}
