// Hive's optional integrations: the home page grid and /integrations/.

export type Integration = {
  id: string;
  name: string;
  /** Key or where it shows. */
  key: string;
  text: string;
  config: string;
  /** A working config.toml snippet. */
  snippet: string;
  /** Extra detail for /integrations/. */
  details: string[];
};

export const integrations: Integration[] = [
  {
    id: "obsidian",
    name: "Obsidian notes",
    key: "Ctrl+Space N",
    text: "Open your repository's Obsidian vault beside your agents, in an editor or an agent of your choice. With journaling on, Hive appends “claude finished in #3 api (main)” to today's daily note, so your vault keeps a log of what your agents did.",
    config: "[integrations.obsidian]",
    snippet: `[integrations.obsidian]
enabled = true
journal = true        # log agent activity
journal_dir = "Daily" # daily notes folder
agent = ""            # "" = $EDITOR
open_in = "tab"       # or "split"`,
    details: [
      "The vault is the nearest folder with .obsidian/ at or above the session's directory, or one level below the repository root.",
      "Asking again goes back to the notes session that is already open.",
    ],
  },
  {
    id: "review",
    name: "Review changes",
    key: "Ctrl+Space G",
    text: "lazygit, tig or git diff in a split beside the agent, opened in the agent's own directory or worktree. See what it changed without leaving Hive.",
    config: "[integrations.review]",
    snippet: `[integrations.review]
enabled = true
command = []      # [] = first installed
open_in = "split" # or "tab"`,
    details: [
      "With no command set, Hive uses the first of lazygit, tig or git diff that is installed.",
      "It opens in the focused agent's directory, so a worktree agent is reviewed in its worktree.",
    ],
  },
  {
    id: "github",
    name: "Pull request checks",
    key: "Ctrl+Space H",
    text: "Watch the CI checks of the session's branch with the GitHub CLI, live, next to the agent that pushed it.",
    config: "[integrations.github]",
    snippet: `[integrations.github]
enabled = true`,
    details: ["Runs gh pr checks --watch for the session's branch. Needs the GitHub CLI, signed in."],
  },
  {
    id: "tasks",
    name: "Project tasks",
    key: "Ctrl+Space T",
    text: "Pick a just, npm, make or cargo task from the project and run it beside the agent: the dev server, the tests, the linter.",
    config: "[integrations.tasks]",
    snippet: `[integrations.tasks]
enabled = true`,
    details: ["Reads the justfile, package.json scripts, Makefile targets or Cargo commands next to the focused session."],
  },
  {
    id: "desktop-notifications",
    name: "Desktop notifications",
    key: "automatic",
    text: "A notification when an agent you aren't looking at finishes or needs your input. Walk away; Hive calls you back.",
    config: "[integrations.desktop_notify]",
    snippet: `[integrations.desktop_notify]
enabled = true
# the title and body are appended
command = ["notify-send", "--app-name=Hive"]`,
    details: ["Any command works: the title and body are passed as its last two arguments."],
  },
  {
    id: "git-status",
    name: "Git status",
    key: "sidebar",
    text: "Changed files and commits ahead or behind under every repository space, e.g. main ~3 ↑2. Know which project has uncommitted agent work at a glance.",
    config: "[integrations.git_status]",
    snippet: `[integrations.git_status]
enabled = true`,
    details: ["Refreshed every few seconds with git status, without taking Git's index lock."],
  },
  {
    id: "usage",
    name: "Usage and cost",
    key: "Ctrl+Space U",
    text: "Token usage and estimated cost of your agents in a pane, live, via ccusage or any tool you choose. Agents keep their own logs; Hive puts the numbers beside the work.",
    config: "[integrations.usage]",
    snippet: `[integrations.usage]
enabled = true
command = []      # ccusage blocks --live
open_in = "split"`,
    details: ["Hive never reads agent logs itself; it opens the tool you choose. A missing tool gets an install hint."],
  },
  {
    id: "layouts",
    name: "Layouts",
    key: "Ctrl+Space L",
    text: "Name a set of panes once, say Claude, the dev server and a terminal, and open them together in a new tab. A layout can belong to a space and opens there without asking.",
    config: "[layouts.<name>]",
    snippet: `[layouts.web]
spaces = ["web"]  # opens here without asking
panes = [
  { agent = "claude" },
  { run = ["npm", "run", "dev"], split = "right", label = "server" },
  { split = "down" }, # a terminal
]`,
    details: [
      "Each pane is an agent (default: a terminal) or a command in run, started directly without a shell, split right or down from the previous one.",
      "With no layout for the space, the only layout opens, or the palette lists them.",
    ],
  },
];
