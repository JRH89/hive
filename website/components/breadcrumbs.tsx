import { JsonLd } from "./json-ld";
import { absolute } from "@/lib/site";

/** Visible breadcrumbs plus their BreadcrumbList. `trail` excludes Home; the
 * last entry is the current page. */
export function Breadcrumbs({ trail }: { trail: [name: string, path: string][] }) {
  const all: [string, string][] = [["Home", "/"], ...trail];
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "BreadcrumbList",
          itemListElement: all.map(([name, path], i) => ({
            "@type": "ListItem",
            position: i + 1,
            name,
            item: absolute(path),
          })),
        }}
      />
      <nav aria-label="Breadcrumb" className="breadcrumbs">
        <ol>
          {all.map(([name, path], i) =>
            i === all.length - 1 ? (
              <li key={path} aria-current="page">
                {name}
              </li>
            ) : (
              <li key={path}>
                <a href={path}>{name}</a>
              </li>
            ),
          )}
        </ol>
      </nav>
    </>
  );
}
