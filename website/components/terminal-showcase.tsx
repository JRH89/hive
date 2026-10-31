"use client";

import { useState } from "react";
import { Brand } from "./brand";
import { Icon } from "./icon";

const sessions = [
  {
    id: "claude",
    name: "Claude Code",
    label: "Frontend",
    initials: "Cl",
    command: "hive new claude -l Frontend",
    prompt: "Help me simplify the navigation component.",
    intro:
      "The navigation can be smaller without losing its keyboard behavior.",
    file: "src/components/navigation.tsx",
    lines: [
      "export function Navigation() {",
      "  return (",
      '    <nav aria-label="Main navigation">',
      "      <NavigationLinks />",
      "    </nav>",
      "  );",
      "}",
    ],
    outro: "One component. Clear focus states. Familiar HTML.",
  },
  {
    id: "codex",
    name: "Codex",
    label: "API review",
    initials: "Cx",
    command: "hive new codex -l 'API review'",
    prompt: "Review the API changes for edge cases.",
    intro: "I’ll trace the request lifecycle and check the failure paths.",
    file: "src/api/session.ts",
    lines: [
      "export async function getSession(id: string) {",
      "  const session = await store.find(id);",
      "",
      "  if (!session) {",
      "    return { status: 404, error: 'Not found' };",
      "  }",
      "  return { status: 200, data: session };",
      "}",
    ],
    outro: "Checking missing sessions, expired tokens, and retry behavior.",
  },
  {
    id: "shell",
    name: "Terminal",
    label: "Project shell",
    initials: ">_",
    command: "hive new shell -l 'Project shell'",
    prompt: "git diff --stat",
    intro: " src/components/navigation.tsx | 18 ++++++------------",
    file: "~/projects/atlas",
    lines: [
      "$ git status --short",
      " M src/components/navigation.tsx",
      " M src/api/session.ts",
      "",
      "$",
      "# Same project. Independent terminals.",
      "# Switch sessions without stopping a process.",
    ],
    outro: "A shell is a session too. Bring any terminal command.",
  },
] as const;

export function TerminalShowcase() {
  const [selected, setSelected] = useState(0);
  const [detached, setDetached] = useState(false);
  const session = sessions[selected];

  return (
    <figure
      className="showcase"
      aria-label="Interactive illustration of Hive sessions"
    >
      <div className="showcase-toolbar">
        <span>
          <span className="tiny-square" /> YOUR AGENTS. ONE PLACE.
        </span>
        <span className="showcase-hint">
          Select a session. Make yourself at home.{" "}
          <span aria-hidden="true">↙</span>
        </span>
      </div>
      <div className={`showcase-window${detached ? " is-detached" : ""}`}>
        <div className="showcase-titlebar">
          <div className="window-dots" aria-hidden="true">
            <i />
            <i />
            <i />
          </div>
          <span>
            hive <span className="faint">/</span> ~/projects/atlas
          </span>
          <span className="showcase-window-label">ILLUSTRATIVE PREVIEW</span>
        </div>
        <div className="showcase-body">
          <aside className="showcase-sidebar" aria-label="Example sessions">
            <div className="showcase-brand">
              <Brand compact />
              <span>HIVE</span>
              <span className="showcase-version">local</span>
            </div>
            <p className="showcase-sidebar-label">
              AGENTS <span>03</span>
            </p>
            <div className="showcase-session-list">
              {sessions.map((item, index) => (
                <button
                  key={item.id}
                  type="button"
                  aria-pressed={selected === index}
                  aria-controls="showcase-panel"
                  disabled={detached}
                  onClick={() => setSelected(index)}
                  className={`showcase-session${selected === index ? " selected" : ""}`}
                >
                  <span className={`session-initials ${item.id}`}>
                    {item.initials}
                  </span>
                  <span className="session-info">
                    <strong>{item.name}</strong>
                    <span>{item.label}</span>
                  </span>
                  <span className="session-number">{index + 1}</span>
                </button>
              ))}
            </div>
            <div className="showcase-sidebar-bottom">
              <span className="status-dot" />
              <span>3 processes running</span>
            </div>
          </aside>
          <div className="showcase-content">
            {!detached ? (
              <section
                id="showcase-panel"
                aria-label={`${session.name}: ${session.label}`}
                className="showcase-panel"
              >
                <div className="showcase-panel-heading">
                  <span>
                    <span className="status-dot" /> {session.label}
                    <span className="showcase-agent-name">
                      {" "}
                      / {session.name}
                    </span>
                  </span>
                  <Icon name="terminal" size={16} />
                </div>
                <div className="showcase-output">
                  <p className="showcase-launch">
                    <span className="prompt">$</span>
                    {session.command}
                  </p>
                  <p className="showcase-user">
                    <span aria-hidden="true">❯</span> {session.prompt}
                  </p>
                  <p className="showcase-response">{session.intro}</p>
                  <div className="showcase-code">
                    <div className="showcase-file">
                      <span aria-hidden="true">↳</span> {session.file}
                      <span>EXAMPLE</span>
                    </div>
                    <pre>
                      <code>
                        {session.lines.map((line, index) => (
                          <span className="showcase-code-line" key={index}>
                            <span className="line-number" aria-hidden="true">
                              {String(index + 1).padStart(2, "0")}
                            </span>
                            <span>{line || " "}</span>
                            {"\n"}
                          </span>
                        ))}
                      </code>
                    </pre>
                  </div>
                  <p className="showcase-outro">
                    <span aria-hidden="true">↳</span> {session.outro}
                  </p>
                  <p className="showcase-input">
                    <span aria-hidden="true">❯</span>
                    <span className="terminal-cursor" aria-hidden="true" />
                    <span className="showcase-input-hint">
                      Your terminal. Your next move.
                    </span>
                  </p>
                </div>
              </section>
            ) : (
              <div id="showcase-panel" className="showcase-detached">
                <div className="detached-icon">
                  <Icon name="detach" size={28} />
                </div>
                <span className="micro-label">FRONTEND DETACHED</span>
                <h3>
                  You left the window.
                  <br />
                  Your sessions didn’t.
                </h3>
                <p>
                  All three processes stay with the local daemon.
                  <br />
                  Reattach to access your running sessions.
                </p>
                <code>
                  <span className="prompt">$</span> hive
                </code>
                <span className="detached-caveat">
                  Running processes last while hived is alive.
                </span>
              </div>
            )}
          </div>
        </div>
        <div className="showcase-statusbar">
          <span>
            <span className="status-dot" />
            {detached ? "daemon running" : "hived connected"}
            <span className="showcase-status-extra"> · 3 sessions</span>
          </span>
          <span>
            <kbd>ctrl</kbd>
            <kbd>space</kbd>
            <span>leader</span>
          </span>
        </div>
      </div>
      <figcaption className="showcase-caption">
        <span role="status" aria-live="polite">
          {detached
            ? "Preview detached. The example processes remain running."
            : `${session.name} selected · Illustrative terminal preview`}
        </span>
        <button
          type="button"
          onClick={() => setDetached((value) => !value)}
          aria-pressed={detached}
        >
          <Icon name="detach" size={14} />
          {detached ? "Reattach to preview" : "Try detaching"}
          <span aria-hidden="true">↗</span>
        </button>
      </figcaption>
    </figure>
  );
}
