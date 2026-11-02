// Every config.toml option. A Rust test (hive-core
// config::tests::website_config_docs_cover_the_example) checks each key in
// examples/config.toml is listed here.

export type ConfigOption = {
  /** Dotted path, e.g. "general.theme". */
  key: string;
  type: string;
  default: string;
  meaning: string;
};

export type ConfigSection = {
  id: string;
  table: string;
  intro: string;
  options: ConfigOption[];
};

export const configSections: ConfigSection[] = [
  {
    id: "general",
    table: "[general]",
    intro: "Defaults for new sessions and the look of the interface.",
    options: [
      { key: "general.default_agent", type: "string", default: '"claude"', meaning: "Agent hive new starts when you don't name one." },
      { key: "general.theme", type: "string", default: '"hive-dark"', meaning: "hive-dark, hive-light, ansi (your terminal's 16 colours) or mono (no colour)." },
    ],
  },
  {
    id: "branding",
    table: "[branding]",
    intro: "Hive's own chrome.",
    options: [
      { key: "branding.show_header", type: "bool", default: "true", meaning: "Show the tab bar row above the panes." },
      { key: "branding.show_tagline", type: "bool", default: "true", meaning: "Show the tagline on the start screen." },
    ],
  },
  {
    id: "terminal",
    table: "[terminal]",
    intro: "How agents' terminals behave.",
    options: [
      { key: "terminal.scrollback_lines", type: "integer", default: "10000", meaning: "Lines of history the daemon keeps per session." },
      { key: "terminal.mouse_support", type: "bool", default: "true", meaning: "Clickable sidebar and tabs, wheel scrolling, and mouse input for agents that use it. Hold Shift to select text natively." },
    ],
  },
  {
    id: "daemon",
    table: "[daemon]",
    intro: "The background daemon, hived.",
    options: [
      { key: "daemon.auto_start", type: "bool", default: "true", meaning: "hive starts the daemon when it isn't running. Off: run hive daemon start yourself." },
      { key: "daemon.kill_timeout_secs", type: "integer", default: "3", meaning: "Seconds between asking a session to quit and forcing it." },
    ],
  },
  {
    id: "notifications",
    table: "[notifications]",
    intro: "How Hive gets your attention when an agent you aren't viewing needs input or finishes.",
    options: [
      { key: "notifications.bell", type: "bool", default: "true", meaning: "Ring the terminal bell." },
    ],
  },
  {
    id: "integrations",
    table: "[integrations.*]",
    intro:
      "Optional tools, all off by default. Turn them on with Ctrl+Space I, which saves enabled here, or edit the file. Details and examples are on the integrations page.",
    options: [
      { key: "integrations.obsidian.enabled", type: "bool", default: "false", meaning: "Ctrl+Space N opens the workspace's Obsidian vault." },
      { key: "integrations.obsidian.agent", type: "string", default: '""', meaning: "Agent that opens the vault; empty uses $VISUAL, then $EDITOR, then vi." },
      { key: "integrations.obsidian.open_in", type: '"tab" | "split"', default: '"tab"', meaning: "Where the notes pane opens." },
      { key: "integrations.obsidian.journal", type: "bool", default: "false", meaning: "Append a line to today's daily note when an agent finishes." },
      { key: "integrations.obsidian.journal_dir", type: "string", default: '""', meaning: "Folder of daily notes inside the vault; empty is the vault root." },
      { key: "integrations.review.enabled", type: "bool", default: "false", meaning: "Ctrl+Space G reviews changes in the session's directory." },
      { key: "integrations.review.command", type: "string array", default: "[]", meaning: "Program to run; empty picks lazygit, else tig status, else git diff." },
      { key: "integrations.review.open_in", type: '"tab" | "split"', default: '"split"', meaning: "Where it opens." },
      { key: "integrations.github.enabled", type: "bool", default: "false", meaning: "Ctrl+Space H shows pull request checks for the branch." },
      { key: "integrations.github.command", type: "string array", default: "[]", meaning: 'Empty runs ["gh", "pr", "checks", "--watch"].' },
      { key: "integrations.github.open_in", type: '"tab" | "split"', default: '"split"', meaning: "Where it opens." },
      { key: "integrations.tasks.enabled", type: "bool", default: "false", meaning: "Ctrl+Space T picks a just, package.json, make or cargo task to run." },
      { key: "integrations.tasks.open_in", type: '"tab" | "split"', default: '"split"', meaning: "Where the task runs." },
      { key: "integrations.desktop_notify.enabled", type: "bool", default: "false", meaning: "A desktop notification when an agent you aren't viewing needs input or finishes." },
      { key: "integrations.desktop_notify.command", type: "string array", default: '["notify-send", "--app-name=Hive"]', meaning: "Program to run; the title and body are appended as two arguments." },
      { key: "integrations.git_status.enabled", type: "bool", default: "false", meaning: "Changed files and commits ahead/behind under each repository space." },
      { key: "integrations.usage.enabled", type: "bool", default: "false", meaning: "Ctrl+Space U shows token usage and cost." },
      { key: "integrations.usage.command", type: "string array", default: "[]", meaning: 'Empty runs ["ccusage", "blocks", "--live"].' },
      { key: "integrations.usage.open_in", type: '"tab" | "split"', default: '"split"', meaning: "Where it opens." },
    ],
  },
  {
    id: "worktrees",
    table: "[worktrees]",
    intro: "Git worktrees Hive creates for agents.",
    options: [
      { key: "worktrees.dir", type: "path", default: "$XDG_DATA_HOME/hive/worktrees", meaning: "Where new worktrees go (on most Linux systems ~/.local/share/hive/worktrees)." },
    ],
  },
  {
    id: "theme",
    table: "[theme]",
    intro: "Colour depth, glyphs and colour overrides on top of [general] theme.",
    options: [
      { key: "theme.colors", type: "string", default: '"auto"', meaning: "auto, truecolor, 256, 16 or none. Auto reads NO_COLOR, COLORTERM and TERM." },
      { key: "theme.glyphs", type: "string", default: '"auto"', meaning: "auto, unicode or ascii. Auto picks ascii on the Linux console or without a UTF-8 locale." },
      { key: "theme.accent", type: "#rrggbb", default: "theme's", meaning: "Accent colour. Also: accent_dim, surface, text, muted, success, warning, error." },
    ],
  },
  {
    id: "keys",
    table: "[keys]",
    intro: "The leader key and bindings. Command names are on the keys page.",
    options: [
      { key: "keys.leader", type: "key", default: '"ctrl+space"', meaning: "Pressed before every Hive command. Needs Ctrl, Alt or an F-key. Pressing it twice sends it to the agent." },
      { key: "keys.jump_modifier", type: "string", default: '"alt"', meaning: 'Modifier for 1-9 to jump to agents without the leader: "alt", "ctrl+alt", or "" for off.' },
      { key: "keys.bind", type: "table", default: "defaults", meaning: "command = key or [keys], pressed after the leader. Replaces that command's defaults; [] unbinds it." },
      { key: "keys.global", type: "table", default: "Alt+arrows", meaning: "command = key, active without the leader even while typing. Needs Ctrl or Alt; [] frees a key for the agent." },
    ],
  },
  {
    id: "agents",
    table: "[agents.<name>]",
    intro: "Add an agent or override a built-in. Commands run directly, never through a shell.",
    options: [
      { key: "agents.<name>.command", type: "string", default: "required", meaning: "Program to run, by name on PATH or as an absolute path." },
      { key: "agents.<name>.args", type: "string array", default: "[]", meaning: "Arguments, passed as they are." },
      { key: "agents.<name>.env", type: "table", default: "{}", meaning: "Extra environment variables. Never written to disk by Hive." },
    ],
  },
  {
    id: "layouts",
    table: "[layouts.<name>]",
    intro: "Panes opened together in a new tab with Ctrl+Space L. The first pane fills the tab; each next one splits the one before it.",
    options: [
      { key: "layouts.<name>.spaces", type: "string array", default: "[]", meaning: "Spaces where Ctrl+Space L opens this layout without asking." },
      { key: "layouts.<name>.panes", type: "array of tables", default: "required", meaning: "The panes, in order. Each takes the keys below." },
      { key: "panes[].agent", type: "string", default: '"shell"', meaning: "Agent to start." },
      { key: "panes[].run", type: "string array", default: "[]", meaning: 'A command to run instead of an agent, e.g. ["npm", "run", "dev"].' },
      { key: "panes[].split", type: '"right" | "down"', default: '"right"', meaning: "Where it goes relative to the previous pane." },
      { key: "panes[].label", type: "string", default: '""', meaning: "Session label; empty uses the agent's or command's name." },
    ],
  },
];
