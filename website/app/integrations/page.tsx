import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { JsonLd } from "@/components/json-ld";
import { Icon } from "@/components/icon";
import { integrations } from "@/lib/integrations";
import { absolute, ogImage } from "@/lib/site";

const title = "Integrations: Obsidian, lazygit, GitHub and more";
const description =
  "Optional Hive integrations beside your AI coding agents: Obsidian journal, lazygit review, GitHub PR checks, tasks, notifications, git status, usage and layouts.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: "/integrations/" },
  openGraph: { title, description, url: "/integrations/", images: [ogImage] },
};

const Arrow = () => <Icon name="arrow" size={17} />;

export default function Integrations() {
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "ItemList",
          name: "Hive integrations",
          itemListElement: integrations.map((i, n) => ({
            "@type": "ListItem",
            position: n + 1,
            name: i.name,
            url: absolute(`/integrations/#${i.id}`),
          })),
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs trail={[["Integrations", "/integrations/"]]} />
        <p className="eyebrow">YOUR TOOLS, BESIDE YOUR AGENTS</p>
        <h1 className="page-title">Integrations</h1>
        <p className="page-lede">
          Every integration is off by default and hidden from the palette until you turn it on.
          Press <kbd>Ctrl+Space</kbd> <kbd>I</kbd> (or type &ldquo;integrations&rdquo; in the
          palette) for a checklist: Enter switches one on or off at once and saves it to{" "}
          <code>~/.config/hive/config.toml</code>. Each opens as an ordinary session beside your
          agents, so it can be split, moved and closed like any other.
        </p>

        <nav aria-label="Integrations" className="toc">
          {integrations.map((i) => (
            <a key={i.id} href={`#${i.id}`}>
              {i.name}
            </a>
          ))}
        </nav>

        <div className="integration-sections">
          {integrations.map((i) => (
            <section key={i.id} id={i.id} className="integration-section" aria-labelledby={`${i.id}-title`}>
              <div>
                <h2 id={`${i.id}-title`}>{i.name}</h2>
                <p className="integration-key">
                  {i.key.startsWith("Ctrl") ? <kbd>{i.key}</kbd> : i.key}
                </p>
                <p>{i.text}</p>
                {i.details.map((d) => (
                  <p key={d}>{d}</p>
                ))}
              </div>
              <pre className="code">
                <code>{i.snippet}</code>
              </pre>
            </section>
          ))}
        </div>

        <div className="why-cta">
          <a className="button button-primary" href="/install/">
            Install Hive <Arrow />
          </a>
          <a className="text-link" href="/why/">
            Why Hive <Arrow />
          </a>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
