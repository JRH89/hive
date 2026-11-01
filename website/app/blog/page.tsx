import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { JsonLd } from "@/components/json-ld";
import { Icon } from "@/components/icon";
import { posts } from "@/lib/posts";
import { absolute, ogImage, organization, person } from "@/lib/site";

const title = "Blog: running AI coding agents well";
const description =
  "Guides and release notes from the Hive team: running several AI coding agents at once, git worktrees for agents, tmux versus Hive, and what's new.";

export const metadata: Metadata = {
  title,
  description,
  alternates: {
    canonical: "/blog/",
    types: { "application/rss+xml": "/blog/rss.xml" },
  },
  openGraph: { title, description, url: "/blog/", images: [ogImage] },
};

const day = new Intl.DateTimeFormat("en", { dateStyle: "long", timeZone: "America/Los_Angeles" });

export default function Blog() {
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "Blog",
          "@id": absolute("/blog/#blog"),
          name: "Hive blog",
          description,
          url: absolute("/blog/"),
          publisher: organization,
          blogPost: posts.map((p) => ({
            "@type": "BlogPosting",
            headline: p.title,
            description: p.description,
            url: absolute(`/blog/${p.slug}/`),
            datePublished: p.date,
            dateModified: p.updated ?? p.date,
            author: person,
          })),
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs trail={[["Blog", "/blog/"]]} />
        <div className="section-heading">
          <div>
            <p className="eyebrow">NOTES FROM THE HIVE</p>
            <h1 className="page-title">Blog</h1>
          </div>
          <p className="section-aside">
            <a className="text-link" href="/blog/rss.xml">
              RSS feed <Icon name="arrow" size={17} />
            </a>
          </p>
        </div>
        <ol className="post-list">
          {posts.map((p) => (
            <li key={p.slug}>
              <article>
                <p className="post-meta">
                  <time dateTime={p.date}>{day.format(new Date(p.date))}</time>
                  <span aria-hidden="true">·</span>
                  {p.tags.join(", ")}
                  <span aria-hidden="true">·</span>
                  {p.minutes} min read
                </p>
                <h2>
                  <a href={`/blog/${p.slug}/`}>{p.title}</a>
                </h2>
                <p>{p.description}</p>
              </article>
            </li>
          ))}
        </ol>
      </main>
      <SiteFooter />
    </>
  );
}
