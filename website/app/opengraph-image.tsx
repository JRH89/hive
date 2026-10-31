import { ImageResponse } from "next/og";

export const dynamic = "force-static";
export const alt = "Hive — One terminal. Many agents. A terminal multiplexer for AI coding agents.";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

const sessions = [
  ["frontend", "claude", true],
  ["api", "codex", false],
  ["tests", "gemini", false],
  ["docs", "opencode", false],
] as const;

export default function OpenGraphImage() {
  return new ImageResponse(
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        background: "#111318",
        color: "#e8eaf0",
        padding: "72px",
        fontFamily: "sans-serif",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", flex: 1 }}>
        <div style={{ display: "flex", alignItems: "center", gap: "18px" }}>
          <svg width="64" height="64" viewBox="0 0 64 64">
            <g fill="none" stroke="#F2B84B" strokeWidth="3">
              <path d="m32 8 21 12v24L32 56 11 44V20Z" />
              <path d="m22 25 7 7-7 7m12 0h10" />
            </g>
          </svg>
          <span style={{ fontSize: 34, letterSpacing: 6, fontWeight: 700 }}>HIVE</span>
        </div>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            marginTop: "64px",
            fontSize: 76,
            fontWeight: 700,
            lineHeight: 1.05,
            letterSpacing: -2,
          }}
        >
          <span>One terminal.</span>
          <span style={{ color: "#F2B84B" }}>Many agents.</span>
        </div>
        <span style={{ marginTop: "36px", fontSize: 28, color: "#8992a3", maxWidth: 560 }}>
          An open-source terminal multiplexer for AI coding agents. Built with Rust.
        </span>
      </div>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          width: "380px",
          alignSelf: "center",
          border: "2px solid #2a2f3a",
          borderRadius: "10px",
          padding: "24px",
          gap: "14px",
          fontSize: 26,
          fontFamily: "monospace",
        }}
      >
        <span style={{ color: "#8992a3", fontSize: 18, letterSpacing: 3 }}>SESSIONS</span>
        {sessions.map(([name, agent, current]) => (
          <div
            key={name}
            style={{
              display: "flex",
              justifyContent: "space-between",
              padding: "10px 14px",
              borderRadius: "6px",
              background: current ? "#2a2416" : "transparent",
              borderLeft: current ? "4px solid #F2B84B" : "4px solid transparent",
            }}
          >
            <span>
              <span style={{ color: "#79c99e", marginRight: 14 }}>●</span>
              {name}
            </span>
            <span style={{ color: "#8992a3" }}>{agent}</span>
          </div>
        ))}
      </div>
    </div>,
    size,
  );
}
