import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { JsonLd } from "@/components/json-ld";
import { Icon } from "@/components/icon";
import { postBySlug, posts } from "@/lib/posts";
import { bodies } from "@/lib/post-bodies";
import { absolute, author, organization, person } from "@/lib/site";

type Props = { params: Promise<{ slug: string }> };

export const dynamicParams = false;

export function generateStaticParams() {
  return posts.map((p) => ({ slug: p.slug }));
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const post = postBySlug((await params).slug);
  if (!post) return {};
  const url = `/blog/${post.slug}/`;
  const image = { url: `/blog/og/${post.slug}.png`, width: 1200, height: 630, alt: post.title };
  return {
    title: post.title,
    description: post.description,
    keywords: post.tags,
    authors: [author],
    alternates: { canonical: url },
    openGraph: {
      type: "article",
      title: post.title,
      description: post.description,
      url,
      publishedTime: post.date,
      modifiedTime: post.updated ?? post.date,
      authors: [author.url],
      tags: post.tags,
      images: [image],
    },
    twitter: {
      card: "summary_large_image",
      title: post.title,
      description: post.description,
      images: [image.url],
    },
  };
}

const day = new Intl.DateTimeFormat("en", { dateStyle: "long", timeZone: "America/Los_Angeles" });
const Arrow = () => <Icon name="arrow" size={17} />;

export default async function BlogPost({ params }: Props) {
  const post = postBySlug((await params).slug);
  const Body = post && bodies[post.slug];
  if (!post || !Body) notFound();
  const url = absolute(`/blog/${post.slug}/`);
  const others = posts.filter((p) => p.slug !== post.slug).slice(0, 3);
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "BlogPosting",
          "@id": `${url}#article`,
          mainEntityOfPage: url,
          url,
          headline: post.title,
          description: post.description,
          image: absolute(`/blog/og/${post.slug}.png`),
          datePublished: post.date,
          dateModified: post.updated ?? post.date,
          keywords: post.tags.join(", "),
          timeRequired: `PT${post.minutes}M`,
          inLanguage: "en",
          author: person,
          publisher: organization,
          isPartOf: { "@id": absolute("/blog/#blog") },
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs
          trail={[
            ["Blog", "/blog/"],
            [post.title, `/blog/${post.slug}/`],
          ]}
        />
        <article className="post">
          <header>
            <p className="post-meta">
              <time dateTime={post.date}>{day.format(new Date(post.date))}</time>
              <span aria-hidden="true">·</span>
              {post.tags.join(", ")}
              <span aria-hidden="true">·</span>
              {post.minutes} min read
            </p>
            <h1 className="page-title">{post.title}</h1>
            <p className="page-lede">{post.description}</p>
            <p className="post-byline">
              By <a href={author.url}>{author.name}</a>
            </p>
          </header>
          <div className="prose">
            <Body />
          </div>
        </article>

        <aside className="post-more" aria-labelledby="more-title">
          <h2 id="more-title">Keep reading</h2>
          <ul>
            {others.map((p) => (
              <li key={p.slug}>
                <a href={`/blog/${p.slug}/`}>{p.title}</a>
              </li>
            ))}
          </ul>
          <div className="why-cta">
            <a className="button button-primary" href="/install/">
              Install Hive <Arrow />
            </a>
            <a className="text-link" href="/blog/">
              All posts <Arrow />
            </a>
          </div>
        </aside>
      </main>
      <SiteFooter />
    </>
  );
}
