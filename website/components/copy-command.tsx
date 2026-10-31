"use client";

import { useEffect, useState } from "react";
import { Icon } from "./icon";

export function CopyCommand({ command }: { command: string }) {
  const [status, setStatus] = useState<"idle" | "copied" | "failed">("idle");

  useEffect(() => {
    if (status !== "copied") return;
    const timeout = window.setTimeout(() => setStatus("idle"), 2400);
    return () => window.clearTimeout(timeout);
  }, [status]);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setStatus("copied");
    } catch {
      setStatus("failed");
    }
  }

  return (
    <div className="copy-command">
      <div className="command-line">
        <span className="prompt" aria-hidden="true">
          $
        </span>
        <code>{command}</code>
        <button
          type="button"
          onClick={copy}
          aria-label={`Copy command: ${command}`}
        >
          {status === "copied" ? (
            <Icon name="check" size={15} />
          ) : (
            <svg
              width="15"
              height="15"
              viewBox="0 0 16 16"
              fill="none"
              aria-hidden="true"
            >
              <rect
                x="5.5"
                y="5.5"
                width="8"
                height="8"
                rx="1"
                stroke="currentColor"
              />
              <path
                d="M10.5 3V2.5a1 1 0 0 0-1-1h-7a1 1 0 0 0-1 1v7a1 1 0 0 0 1 1H3"
                stroke="currentColor"
              />
            </svg>
          )}
          <span>{status === "copied" ? "Copied" : "Copy"}</span>
        </button>
      </div>
      <span className="copy-feedback" role="status" aria-live="polite">
        {status === "failed"
          ? "Copy unavailable. Select the command above to copy it manually."
          : status === "copied"
            ? "Command copied to clipboard."
            : ""}
      </span>
    </div>
  );
}
