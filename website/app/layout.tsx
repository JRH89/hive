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

export const metadata: Metadata = {
  title: "Hive — One terminal. Many agents.",
  description:
    "A native Rust terminal multiplexer for AI coding agents. Real terminals, a local daemon, and sessions that stay running when you detach. Linux-first and in early development.",
  applicationName: "Hive",
  openGraph: {
    title: "Hive — One terminal. Many agents.",
    description:
      "A home for your coding agents. Native to your terminal. Built with Rust.",
    type: "website",
    locale: "en_US",
  },
  twitter: { card: "summary", title: "Hive — One terminal. Many agents." },
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
