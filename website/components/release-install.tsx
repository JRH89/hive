"use client";

import { useState } from "react";
import { Icon } from "./icon";
import { CopyCommand } from "./copy-command";

const releasesUrl = "https://github.com/jrh89/hive/releases";
const assetBase = `${releasesUrl}/latest/download`;

// Proposed release-asset contract, not published endpoints. Enable a platform
// only after its installer and binaries are uploaded and tested on that OS.
const platforms = [
  {
    id: "linux",
    name: "Linux",
    shell: "bash / sh",
    command: `curl -fsSL ${assetBase}/install.sh | sh`,
    published: false,
    description:
      "Linux is Hive’s first target. A prebuilt binary and a one-command installer are planned.",
    download: "Linux binary",
  },
  {
    id: "macos",
    name: "macOS",
    shell: "Terminal",
    command: `curl -fsSL ${assetBase}/install.sh | sh`,
    published: false,
    description:
      "A native macOS release is planned after Linux is stable. The installer will select the right build for your Mac.",
    download: "macOS binary",
  },
  {
    id: "windows",
    name: "Windows",
    shell: "PowerShell",
    command: `irm '${assetBase}/install.ps1' | iex`,
    published: false,
    description:
      "Native Windows support and a PowerShell installer are planned after Linux is stable.",
    download: "Windows binary",
  },
] as const;

function PlatformIcon({ platform }: { platform: string }) {
  if (platform === "linux") return <Icon name="terminal" size={17} />;
  if (platform === "windows")
    return (
      <svg
        width="16"
        height="16"
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
      >
        <path d="M1 3.2 8.5 2v7H1zm9-1.4L19 .5V9h-9zM1 10.5h7.5v7L1 16.3zm9 0h9V19l-9-1.3z" />
      </svg>
    );
  return (
    <svg
      width="17"
      height="17"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <rect x="4" y="3" width="16" height="12" rx="1.5" />
      <path d="M8 19h8m-4-4v4" />
    </svg>
  );
}

export function ReleaseInstall() {
  const [selected, setSelected] = useState(0);
  const platform = platforms[selected];

  return (
    <div className="release-installer">
      <div className="release-toolbar">
        <div
          className="platform-options"
          role="group"
          aria-label="Choose your operating system"
        >
          {platforms.map((item, index) => (
            <button
              key={item.id}
              type="button"
              aria-pressed={selected === index}
              aria-controls="release-instructions"
              onClick={() => setSelected(index)}
            >
              <PlatformIcon platform={item.id} />
              {item.name}
            </button>
          ))}
        </div>
        <span className="release-soon">
          <span className="tiny-square" /> RELEASES COMING SOON
        </span>
      </div>
      <div
        id="release-instructions"
        className="release-instructions"
        aria-label={`${platform.name} installation`}
      >
        <div className="release-intro">
          <div>
            <span className="micro-label">ONE COMMAND. YOUR TERMINAL.</span>
            <h3>Hive for {platform.name}</h3>
          </div>
          <span className="release-shell">{platform.shell}</span>
        </div>
        <p className="release-description">{platform.description}</p>
        <div className="release-command">
          {platform.published ? (
            <CopyCommand command={platform.command} />
          ) : (
            <>
              <div className="release-command-meta">
                <span>Planned install command</span>
                <span>Not available yet</span>
              </div>
              <pre>
                <code>
                  <span className="prompt" aria-hidden="true">
                    {platform.id === "windows" ? "PS>" : "$"}
                  </span>
                  {platform.command}
                </code>
              </pre>
            </>
          )}
        </div>
        <div className="release-actions">
          <button type="button" className="release-download" disabled>
            <Icon name="arrow" size={15} />
            <span>{platform.download}</span>
            <span className="release-download-soon">Coming soon</span>
          </button>
          <a href={releasesUrl} target="_blank" rel="noreferrer">
            Releases on GitHub <span aria-hidden="true">↗</span>
            <span className="sr-only"> (opens in a new tab)</span>
          </a>
        </div>
        <p className="release-note-text" role="status" aria-live="polite">
          {platform.name} installers are not published yet. Commands and asset
          names may change before release.
        </p>
      </div>
    </div>
  );
}
