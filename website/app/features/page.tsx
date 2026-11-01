import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { JsonLd } from "@/components/json-ld";
import { Icon } from "@/components/icon";
import { agents, features } from "@/lib/features";
import { integrations } from "@/lib/integrations";
import { absolute, ogImage } from "@/lib/site";

const title = "Features: live agent status, sessions, worktrees";
const description =
  "Everything Hive does for AI coding agents: live status for 27 agents, sessions that survive closing the terminal, spaces, git worktrees, splits and tabs.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: "/features/" },
  openGraph: { title, description, url: "/features/", images: [ogImage] },
};

const Arrow = () => <Icon name="arrow" size={17} />;

export default function Features() {
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "WebPage",
          "@id": absolute("/features/#webpage"),
          url: absolute("/features/"),
          name: `${title} · Hive`,
          description,
          isPartOf: { "@id": absolute("/#website") },
          about: { "@id": absolute("/#software") },
          primaryImageOfPage: absolute("/screenshots/main.png"),
          mainEntity: {
            "@type": "ItemList",
            name: "Hive features",
            itemListElement: features.map((f, n) => ({
              "@type": "ListItem",
              position: n + 1,
              name: f.name,
              description: f.summary,
              url: absolute(`/features/#${f.id}`),
            })),
          },
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs trail={[["Features", "/features/"]]} />
        <p className="eyebrow">ONE TERMINAL. MANY AGENTS.</p>
        <h1 className="page-title">Everything Hive does</h1>
        <p className="page-lede">
          Hive is an open-source terminal multiplexer built for AI coding agents. It runs Claude
          Code, Codex, Gemini CLI, OpenCode and two dozen more side by side in real terminals,
          tells you which one needs you, and keeps them running when you close the window.
        </p>

        <nav aria-label="Features" className="toc">
          {features.map((f) => (
            <a key={f.id} href={`#${f.id}`}>
              {f.name}
            </a>
          ))}
          <a href="#agents">Supported agents</a>
          <a href="#integrations">Integrations</a>
        </nav>

        <div className="integration-sections">
          {features.map((f) => (
            <section key={f.id} id={f.id} className="integration-section" aria-labelledby={`${f.id}-title`}>
              <div>
                <h2 id={`${f.id}-title`}>{f.name}</h2>
                <p className="feature-summary">{f.summary}</p>
                {f.body.map((p) => (
                  <p key={p}>{p}</p>
                ))}
                {f.more && (
                  <p>
                    <a className="text-link" href={f.more.href}>
                      {f.more.label} <Arrow />
                    </a>
                  </p>
                )}
              </div>
              {f.shot ? (
                <figure className="feature-shot">
                  {/* Static export: plain img with intrinsic size. */}
                  {/* eslint-disable-next-line @next/next/no-img-element */}
                  <img src={`/screenshots/${f.shot.file}.png`} alt={f.shot.alt} width={2443} height={1553} loading="lazy" />
                </figure>
              ) : f.code ? (
                <pre className="code">
                  <code>{f.code}</code>
                </pre>
              ) : null}
            </section>
          ))}

          <section id="agents" className="integration-section" aria-labelledby="agents-title">
            <div>
              <h2 id="agents-title">Supported agents</h2>
              <p className="feature-summary">
                Hive recognises {agents.length} coding agent CLIs, {agents.filter((a) => a.status).length}{" "}
                of them with built-in status rules.
              </p>
              <p>
                Install and sign in to each agent as usual; <code>hive agents</code> lists the ones
                Hive found on your PATH. Agents marked without status still run and are named in the
                sidebar, and can report their state with <code>hive report-agent</code>. Anything
                else runs as a <a className="text-link" href="#custom-agents">custom agent</a>.
              </p>
            </div>
            <ul className="agent-catalog">
              {agents.map((a) => (
                <li key={a.name}>
                  {a.name}
                  {!a.status && <span> · no status rules</span>}
                </li>
              ))}
            </ul>
          </section>

          <section id="integrations" className="integration-section" aria-labelledby="integrations-title">
            <div>
              <h2 id="integrations-title">Integrations</h2>
              <p className="feature-summary">Your tools beside your agents, each off until you enable it.</p>
              <p>
                Each integration opens as an ordinary session that can be split, moved and closed like
                any other, and its key tells you how to turn it on.
              </p>
              <p>
                <a className="text-link" href="/integrations/">
                  Every integration explained <Arrow />
                </a>
              </p>
            </div>
            <ul className="agent-catalog">
              {integrations.map((i) => (
                <li key={i.id}>
                  <a href={`/integrations/#${i.id}`}>{i.name}</a>
                </li>
              ))}
            </ul>
          </section>
        </div>

        <div className="why-cta">
          <a className="button button-primary" href="/install/">
            Install Hive <Arrow />
          </a>
          <a className="text-link" href="/why/">
            How Hive compares <Arrow />
          </a>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
