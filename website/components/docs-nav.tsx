import { docs } from "@/lib/docs";

/** The docs table of contents beside every docs page. */
export function DocsNav({ current }: { current?: string }) {
  return (
    <nav className="docs-nav" aria-label="Documentation">
      <p className="docs-nav-title">
        <a href="/docs/">Documentation</a>
      </p>
      <ol>
        {docs.map((d) => (
          <li key={d.slug}>
            <a href={`/docs/${d.slug}/`} aria-current={d.slug === current ? "page" : undefined}>
              {d.nav}
            </a>
          </li>
        ))}
        <li>
          <a href="/integrations/">Integrations</a>
        </li>
      </ol>
    </nav>
  );
}
