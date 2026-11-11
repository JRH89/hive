"use client";

import { useEffect, useRef, useState } from "react";
import { HiveBackdrop } from "./hive-backdrop";

function webglAvailable() {
  try {
    const canvas = document.createElement("canvas");
    return !!(canvas.getContext("webgl2") || canvas.getContext("webgl"));
  } catch {
    return false;
  }
}

/**
 * The hero's three.js scene. three.js loads after hydration, so it never
 * delays the headline. Without WebGL the static SVG honeycomb stays; with
 * reduced motion a single still frame is drawn.
 */
export function HeroScene() {
  const host = useRef<HTMLDivElement>(null);
  const [mode, setMode] = useState<"pending" | "webgl" | "fallback">("pending");

  useEffect(() => {
    if (!webglAvailable()) {
      setMode("fallback");
      return;
    }
    let disposed = false;
    let dispose: (() => void) | undefined;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    import("@/lib/hive-scene")
      .then(({ createHiveScene }) => {
        if (disposed || !host.current) return;
        const scene = createHiveScene(host.current, { reduced });
        dispose = scene.dispose;
        setMode("webgl");
      })
      .catch(() => {
        if (!disposed) setMode("fallback");
      });
    return () => {
      disposed = true;
      dispose?.();
    };
  }, []);

  return (
    <>
      {mode === "fallback" && <HiveBackdrop />}
      <div
        ref={host}
        className="hero-scene"
        data-ready={mode === "webgl" ? "" : undefined}
        aria-hidden="true"
      />
    </>
  );
}
