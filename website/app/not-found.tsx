import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";

export const metadata: Metadata = {
  title: "Page not found",
  robots: { index: false, follow: true },
};

export default function NotFound() {
  return (
    <>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <p className="eyebrow">404</p>
        <h1 className="page-title">Page not found</h1>
        <p className="page-lede">
          Try the <a className="text-link" href="/">home page</a>, the{" "}
          <a className="text-link" href="/blog/">blog</a> or{" "}
          <a className="text-link" href="/install/">install instructions</a>.
        </p>
      </main>
      <SiteFooter />
    </>
  );
}
