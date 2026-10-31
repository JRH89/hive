import { ImageResponse } from "next/og";

export const dynamic = "force-static";
export const size = { width: 180, height: 180 };
export const contentType = "image/png";

export default function AppleIcon() {
  return new ImageResponse(
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "#111318",
      }}
    >
      <svg width="132" height="132" viewBox="0 0 64 64">
        <g fill="none" stroke="#F2B84B" strokeWidth="3">
          <path d="m32 8 21 12v24L32 56 11 44V20Z" />
          <path d="m22 25 7 7-7 7m12 0h10" />
        </g>
      </svg>
    </div>,
    size,
  );
}
