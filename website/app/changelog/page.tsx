import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { Icon } from "@/components/icon";
import { releases } from "@/lib/changelog";

const repoUrl = "https://github.com/jrh89/hive";

export const metadata: Metadata = {
  title: "Changelog",
  description: "What changed in each Hive release.",
  alternates: {
    canonical: "/changelog/",
    types: { "application/rss+xml": "/changelog/rss.xml" },
  },
};

const day = new Intl.DateTimeFormat("en", { dateStyle: "long", timeZone: "America/Los_Angeles" });

export default function Changelog() {
  return (
    <>
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper changelog">
        <div className="section-heading">
          <div>
            <p className="eyebrow">RELEASE NOTES</p>
            <h1 className="changelog-title">Changelog</h1>
          </div>
          <p className="section-aside">
            <a className="text-link" href="/changelog/rss.xml">
              RSS feed <Icon name="arrow" size={17} />
            </a>
          </p>
        </div>
        <ol className="changelog-list">
          {releases.map((r) => (
            <li key={r.version} id={`v${r.version}`}>
              <div className="changelog-meta">
                <a href={`${repoUrl}/releases/tag/v${r.version}`} className="changelog-version">
                  v{r.version}
                </a>
                <time dateTime={r.date}>{day.format(new Date(r.date))}</time>
              </div>
              <div>
                <h2>{r.title}</h2>
                <ul>
                  {r.changes.map((c) => (
                    <li key={c}>{c}</li>
                  ))}
                </ul>
              </div>
            </li>
          ))}
        </ol>
      </main>
    </>
  );
}
