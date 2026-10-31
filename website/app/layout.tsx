import type { Metadata, Viewport } from "next";
import localFont from "next/font/local";
import "./globals.css";

const geist = localFont({
  src: "../node_modules/@fontsource-variable/geist/files/geist-latin-wght-normal.woff2",
  variable: "--font-geist",
  display: "swap",
  weight: "100 900",
});
const geistMono = localFont({
  src: "../node_modules/@fontsource-variable/geist-mono/files/geist-mono-latin-wght-normal.woff2",
  variable: "--font-geist-mono",
  display: "swap",
  weight: "100 900",
});

const siteUrl = "https://hive.codefrontlabs.com";
const title = "Hive — Terminal multiplexer for AI coding agents";
const description =
  "Hive is an open-source, Rust-built terminal multiplexer for AI coding agents. Run Claude Code, Codex, Gemini CLI and OpenCode side by side in real terminals, with sessions that keep running when you detach.";

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: { default: title, template: "%s · Hive" },
  description,
  applicationName: "Hive",
  keywords: [
    "terminal multiplexer",
    "AI coding agents",
    "Claude Code",
    "Codex CLI",
    "Gemini CLI",
    "OpenCode",
    "tmux alternative",
    "TUI",
    "Rust",
    "git worktrees",
    "Linux",
    "developer tools",
  ],
  authors: [{ name: "Jared Hooker", url: "https://github.com/jrh89" }],
  creator: "Jared Hooker",
  publisher: "Codefront Labs",
  category: "Developer tools",
  alternates: { canonical: "/" },
  robots: {
    index: true,
    follow: true,
    googleBot: { index: true, follow: true, "max-image-preview": "large" },
  },
  openGraph: {
    title: "Hive — One terminal. Many agents.",
    description,
    url: "/",
    siteName: "Hive",
    type: "website",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: "Hive — One terminal. Many agents.",
    description,
  },
};

export const viewport: Viewport = {
  themeColor: "#111318",
  colorScheme: "dark",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" className={`${geist.variable} ${geistMono.variable}`}>
      <body>{children}</body>
    </html>
  );
}
