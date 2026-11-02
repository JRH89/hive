import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { JsonLd } from "@/components/json-ld";
import { DocsNav } from "@/components/docs-nav";
import { docs } from "@/lib/docs";
import { absolute, ogImage } from "@/lib/site";

const title = "Documentation";
const description =
  "Hive documentation: getting started, the interface, every key and command, the full config.toml reference, agent status, worktrees, the daemon and troubleshooting.";

export const metadata: Metadata = {
  title: "Hive documentation",
  description,
  alternates: { canonical: "/docs/" },
  openGraph: { title: "Hive documentation", description, url: "/docs/", images: [ogImage] },
};

export default function Docs() {
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "WebPage",
          "@id": absolute("/docs/"),
          name: "Hive documentation",
          description,
          mainEntity: {
            "@type": "ItemList",
            itemListElement: docs.map((d, n) => ({
              "@type": "ListItem",
              position: n + 1,
              name: d.title,
              url: absolute(`/docs/${d.slug}/`),
            })),
          },
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs trail={[["Docs", "/docs/"]]} />
        <div className="docs-layout">
          <DocsNav />
          <div className="doc">
            <p className="eyebrow">DOCUMENTATION</p>
            <h1 className="page-title">{title}</h1>
            <p className="page-lede">
              Everything about using Hive. New here? Start with{" "}
              <a href="/docs/getting-started/">getting started</a>. In Hive itself,{" "}
              <kbd>Ctrl+Space</kbd> <kbd>?</kbd> lists your keys and <kbd>Ctrl+Space</kbd>{" "}
              <kbd>:</kbd> finds any command.
            </p>
            <ul className="post-list">
              {docs.map((d) => (
                <li key={d.slug}>
                  <h2>
                    <a href={`/docs/${d.slug}/`}>{d.nav}</a>
                  </h2>
                  <p>{d.description}</p>
                </li>
              ))}
              <li>
                <h2>
                  <a href="/integrations/">Integrations</a>
                </h2>
                <p>
                  Obsidian notes, lazygit review, GitHub checks, tasks, desktop notifications, git
                  status and usage: what each does and its options.
                </p>
              </li>
            </ul>
          </div>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
