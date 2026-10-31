// Release notes, newest first. Shared by /changelog and /changelog/rss.xml.

export type Release = {
  version: string;
  /** When the tag was pushed (ISO 8601). */
  date: string;
  title: string;
  changes: string[];
};

export const releases: Release[] = [
  {
    version: "0.4.4",
    date: "2026-10-31T13:30:00-07:00",
    title: "Worktree terminals",
    changes: [
      "Ctrl+Space C opens a terminal in its own git worktree on a fresh branch, so agents never edit the same files. Ctrl+Space A picks an agent from a list.",
      "Pane titles and the sidebar name the agent running in a terminal (claude, codex, ...) instead of “shell”.",
      "A branch whose worktree directory was deleted by hand can be used again (Hive prunes stale worktrees first).",
      "Clearer command names: New terminal, Split: terminal on the right, New space (a folder).",
      "README with screenshots and how-tos; this changelog and its RSS feed.",
    ],
  },
  {
    version: "0.4.3",
    date: "2026-10-31T12:01:07-07:00",
    title: "Spaces follow cd",
    changes: [
      "cd into another repository and the terminal's tab moves to that repository's space within two seconds (Linux and macOS).",
      "Every terminal gets a tab; tabs you close stay closed.",
      "+ new opens a terminal right away. No directory to type.",
      "The spaces list hides spaces with nothing running.",
    ],
  },
  {
    version: "0.4.2",
    date: "2026-10-31T11:39:55-07:00",
    title: "Spaces own their tabs",
    changes: [
      "Each space keeps its own tabs; Alt+←/→ switches tabs within the space, Alt+↑/↓ switches spaces.",
      "The agents list shows agents only, not plain terminals.",
    ],
  },
  {
    version: "0.4.1",
    date: "2026-10-31T01:45:50-07:00",
    title: "Terminal first",
    changes: [
      "New, splits and tabs open a terminal; start an agent by typing its name, like herdr.",
      "A simpler layout: tabs above the panes, the sidebar full height.",
    ],
  },
  {
    version: "0.4.0",
    date: "2026-10-31T01:18:30-07:00",
    title: "macOS and Windows",
    changes: [
      "macOS (Apple silicon and Intel) supported and tested in CI.",
      "Windows 10+ port: named-pipe transport with a per-daemon token, ConPTY sessions, a PowerShell installer.",
      "Terminal tabs like herdr.",
    ],
  },
  {
    version: "0.3.1",
    date: "2026-10-30T23:10:14-07:00",
    title: "Reliable daemon stop",
    changes: ["hive daemon stop works across protocol versions, so upgrades can restart an older daemon."],
  },
  {
    version: "0.3.0",
    date: "2026-10-30T23:01:23-07:00",
    title: "Spaces and agents sidebar",
    changes: [
      "A herdr-style sidebar of spaces and their agents; everything is clickable.",
      "Alt-arrow navigation and hive update.",
      "Optional integrations, off by default: review, pull request checks, tasks, desktop notifications and an Obsidian journal.",
    ],
  },
  {
    version: "0.2.0",
    date: "2026-10-30T22:44:28-07:00",
    title: "Agent status, panes and tabs",
    changes: [
      "Detects over twenty agents and shows whether each is working, needs input or is done.",
      "Split panes and tabs, herdr-style keys, global chords and mouse support.",
      "Optional Obsidian notes integration.",
    ],
  },
  {
    version: "0.1.0",
    date: "2026-10-30T21:44:46-07:00",
    title: "First release",
    changes: [
      "Agents run in real PTYs owned by a background daemon; detach, reattach and automatic reconnection.",
      "Workspaces by repository, live git branches and optional worktrees per agent.",
      "Command palette, themes and configurable keys. Static Linux binaries and a checksum-verified installer.",
    ],
  },
];
