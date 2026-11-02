import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { JsonLd } from "@/components/json-ld";
import { DocsNav } from "@/components/docs-nav";
import { docBySlug, docs, docsUpdated } from "@/lib/docs";
import { docBodies } from "@/lib/doc-bodies";
import { absolute, ogImage, organization, repoUrl } from "@/lib/site";

type Props = { params: Promise<{ slug: string }> };

export const dynamicParams = false;

export function generateStaticParams() {
  return docs.map((d) => ({ slug: d.slug }));
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const doc = docBySlug((await params).slug);
  if (!doc) return {};
  const url = `/docs/${doc.slug}/`;
  return {
    title: doc.title,
    description: doc.description,
    alternates: { canonical: url },
    openGraph: { type: "article", title: doc.title, description: doc.description, url, images: [ogImage] },
  };
}

export default async function DocPage({ params }: Props) {
  const doc = docBySlug((await params).slug);
  const Body = doc && docBodies[doc.slug];
  if (!doc || !Body) notFound();
  const at = docs.indexOf(doc);
  const prev = docs[at - 1];
  const next = docs[at + 1];
  const url = absolute(`/docs/${doc.slug}/`);
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "TechArticle",
          "@id": `${url}#article`,
          mainEntityOfPage: url,
          url,
          headline: doc.title,
          description: doc.description,
          dateModified: docsUpdated,
          inLanguage: "en",
          publisher: organization,
          about: { "@id": absolute("/#software") },
          isPartOf: { "@type": "WebPage", "@id": absolute("/docs/"), name: "Hive documentation" },
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs
          trail={[
            ["Docs", "/docs/"],
            [doc.nav, `/docs/${doc.slug}/`],
          ]}
        />
        <div className="docs-layout">
          <DocsNav current={doc.slug} />
          <article className="doc">
            <h1 className="page-title">{doc.title}</h1>
            <p className="page-lede">{doc.description}</p>
            <div className="prose">
              <Body />
            </div>
            <nav className="doc-pager" aria-label="Previous and next page">
              {prev ? (
                <a href={`/docs/${prev.slug}/`}>
                  <span>Previous</span>
                  {prev.nav}
                </a>
              ) : (
                <span />
              )}
              {next ? (
                <a href={`/docs/${next.slug}/`} className="doc-pager-next">
                  <span>Next</span>
                  {next.nav}
                </a>
              ) : null}
            </nav>
            <p className="doc-edit">
              Something wrong or missing?{" "}
              <a href={`${repoUrl}/issues/new`}>Open an issue</a>.
            </p>
          </article>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
