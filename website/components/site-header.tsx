"use client";

import { useEffect, useRef, useState } from "react";
import { Brand } from "@/components/brand";
import { Icon } from "@/components/icon";

export function SiteHeader() {
  const [open, setOpen] = useState(false);
  const toggle = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    function escape(event: KeyboardEvent) {
      if (event.key === "Escape" && open) {
        setOpen(false);
        toggle.current?.focus();
      }
    }
    const desktop = window.matchMedia("(min-width: 761px)");
    function closeOnDesktop() { if (desktop.matches) setOpen(false); }
    document.addEventListener("keydown", escape);
    desktop.addEventListener("change", closeOnDesktop);
    return () => {
      document.removeEventListener("keydown", escape);
      desktop.removeEventListener("change", closeOnDesktop);
    };
  }, [open]);

  return (
    <div className="header-shell">
      <header className="site-header wrapper">
        <a className="brand-link" href="/" aria-label="Hive home" onClick={() => setOpen(false)}><Brand /></a>
        <button ref={toggle} type="button" className="menu-toggle" aria-expanded={open} aria-controls="main-navigation" onClick={() => setOpen(!open)}>
          {open ? "Close" : "Menu"}
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
            <path d={open ? "m6 6 12 12M6 18 18 6" : "M4 7h16M4 12h16M4 17h16"} />
          </svg>
        </button>
        <nav id="main-navigation" aria-label="Main navigation" className={`main-navigation ${open ? "is-open" : ""}`} onClick={() => setOpen(false)} onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget) && event.relatedTarget !== toggle.current) setOpen(false);
        }}>
          <a className="nav-link" href="/#features">Features</a>
          <a className="nav-link" href="/#integrations">Integrations</a>
          <a className="nav-link" href="/why/">Why Hive</a>
          <a className="nav-link" href="/#how-it-works">How it works</a>
          <a className="nav-link" href="/changelog/">Changelog</a>
          <a className="nav-link nav-github" href="https://github.com/jrh89/hive" aria-label="Hive on GitHub"><Icon name="github" size={18} /><span>GitHub</span></a>
          <a className="nav-cta" href="/#get-started">Get started <Icon name="arrow" size={17} /></a>
        </nav>
      </header>
    </div>
  );
}
