// The documentation pages, in reading order. Bodies are in
// content/docs/<slug>.tsx, mapped in lib/doc-bodies.ts.

export type Doc = {
  slug: string;
  /** Short name for the docs navigation. */
  nav: string;
  title: string;
  description: string;
};

export const docs: Doc[] = [
  {
    slug: "getting-started",
    nav: "Getting started",
    title: "Getting started with Hive",
    description:
      "Install Hive, open it, start your first AI coding agents, detach and come back. The concepts you need: daemon, sessions, spaces, worktrees.",
  },
  {
    slug: "interface",
    nav: "The interface",
    title: "The Hive interface: sidebar, panes, tabs and spaces",
    description:
      "What the sidebar, tab bar, status symbols and marks mean, how panes, tabs and spaces work, the mouse, and using Hive from several windows.",
  },
  {
    slug: "keys",
    nav: "Keys",
    title: "Hive key bindings",
    description:
      "Every Hive command with its default key after Ctrl+Space, the prefix-free Alt chords, and how to rebind or unbind any of them in config.toml.",
  },
  {
    slug: "configuration",
    nav: "Configuration",
    title: "Hive configuration reference",
    description:
      "Every config.toml option with its type and default: general, terminal, daemon, notifications, integrations, worktrees, theme, keys, agents and layouts.",
  },
  {
    slug: "cli",
    nav: "Command line",
    title: "Hive command-line reference",
    description:
      "Every hive command and option: new, list, attach, kill, agents, workspace, worktree, report-agent, doctor, update and daemon, with examples.",
  },
  {
    slug: "agent-status",
    nav: "Agent status",
    title: "Agent status in Hive and the report-agent protocol",
    description:
      "How Hive knows an agent is working, needs input or is done, which agents it recognises, the bell and notifications, and how scripts report their own status.",
  },
  {
    slug: "worktrees",
    nav: "Workspaces and worktrees",
    title: "Workspaces and git worktrees in Hive",
    description:
      "How Hive groups sessions into spaces by repository, and gives an agent its own git worktree and branch, with the rules that keep your work safe.",
  },
  {
    slug: "daemon",
    nav: "Daemon, files and environment",
    title: "The Hive daemon, files and environment variables",
    description:
      "What hived does, what survives a crash or reboot, where Hive keeps its files, the environment variables it reads and sets, upgrading and uninstalling.",
  },
  {
    slug: "troubleshooting",
    nav: "Troubleshooting",
    title: "Troubleshooting Hive",
    description:
      "Fixes for common Hive problems: keys that do nothing, agents not found, protocol mismatches, odd glyphs or colours, lost sessions, and how to report a bug.",
  },
];

export const docBySlug = (slug: string) => docs.find((d) => d.slug === slug);

/** When the docs last changed; bump with real edits. */
export const docsUpdated = "2026-11-01T19:00:00-08:00";
