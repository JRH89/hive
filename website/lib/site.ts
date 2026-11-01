// Shared facts about the site, used by pages, metadata and structured data.

export const siteUrl = "https://hive.codefrontlabs.com";
export const repoUrl = "https://github.com/jrh89/hive";
export const releasesUrl = `${repoUrl}/releases`;

export const author = { name: "Jared Hooker", url: "https://github.com/jrh89" };

/** `/install/` → `https://hive.codefrontlabs.com/install/`. */
export const absolute = (path: string) => `${siteUrl}${path}`;

/** The site-wide social image, for pages that set their own openGraph
 * (which replaces the inherited one). */
export const ogImage = {
  url: "/opengraph-image.png",
  width: 1200,
  height: 630,
  alt: "Hive: one terminal, many agents",
};

export const organization = {
  "@type": "Organization",
  "@id": absolute("/#organization"),
  name: "Codefront Labs",
  url: "https://codefrontlabs.com",
  logo: absolute("/apple-icon.png"),
};

export const person = {
  "@type": "Person",
  "@id": absolute("/#author"),
  name: author.name,
  url: author.url,
};
