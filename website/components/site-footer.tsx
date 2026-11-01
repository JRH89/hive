import { Brand } from "./brand";
import { repoUrl } from "@/lib/site";

export function SiteFooter() {
  return (
    <footer className="site-footer wrapper">
      <a className="brand-link" href="/" aria-label="Hive home">
        <Brand />
      </a>
      <span>A Codefront Labs product. Built with Rust.</span>
      <nav aria-label="Footer navigation">
        <a href="/install/">Install</a>
        <a href="/integrations/">Integrations</a>
        <a href="/why/">Why Hive</a>
        <a href="/blog/">Blog</a>
        <a href="/changelog/">Changelog</a>
        <a href="/blog/rss.xml">RSS</a>
        <a href="/#questions">FAQ</a>
        <a href={repoUrl}>GitHub</a>
        <a href={`${repoUrl}/blob/master/LICENSE`}>MIT License</a>
      </nav>
    </footer>
  );
}
