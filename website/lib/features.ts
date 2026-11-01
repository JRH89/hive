// Hive's features in depth, for /features/. Keep claims in step with
// docs/user-guide.md.

/** Every agent in hive-agents' catalog; `status` = ships screen rules. */
export const agents: { name: string; status: boolean }[] = [
  { name: "Claude Code", status: true },
  { name: "Codex", status: true },
  { name: "Gemini CLI", status: true },
  { name: "OpenCode", status: true },
  { name: "GitHub Copilot", status: true },
  { name: "Cursor Agent", status: true },
  { name: "Amp", status: true },
  { name: "Antigravity", status: true },
  { name: "Cline", status: true },
  { name: "Devin", status: true },
  { name: "Droid", status: true },
  { name: "Grok", status: true },
  { name: "Hermes", status: true },
  { name: "Kilo Code", status: true },
  { name: "Kimi", status: true },
  { name: "Kiro", status: true },
  { name: "Letta", status: true },
  { name: "Maki", status: true },
  { name: "Muse", status: true },
  { name: "Oh My Pi", status: true },
  { name: "Pi", status: true },
  { name: "Qoder", status: true },
  { name: "Qwen Code", status: true },
  { name: "Aider", status: false },
  { name: "Crush", status: false },
  { name: "Goose", status: false },
  { name: "Truss", status: false },
];

export type Feature = {
  id: string;
  name: string;
  /** One-line summary, also used in structured data. */
  summary: string;
  body: string[];
  /** A code sample shown beside the text. */
  code?: string;
  /** A screenshot in /screenshots/ shown beside the text. */
  shot?: { file: string; alt: string };
  /** Further reading on the site. */
  more?: { label: string; href: string };
};

export const features: Feature[] = [
  {
    id: "agent-status",
    name: "Live agent status",
    summary: "See which agent is working, which needs input and which is done, across every project.",
    body: [
      "Hive recognises the agent in each session, even one you started by typing claude in a plain terminal, and shows its state in the sidebar: working, needs input (a permission prompt or a question) or done. Done means it finished while you were looking elsewhere, and clears when you open it.",
      "Status comes from per-agent screen rules shared with herdr. An agent without rules shows no status rather than a guess, and any script can report its own state with hive report-agent. The terminal bell rings when an agent you aren't viewing needs you.",
    ],
    shot: { file: "main", alt: "Claude Code working beside a git log split in the Hive sidebar" },
  },
  {
    id: "persistent-sessions",
    name: "Sessions that outlive the terminal",
    summary: "A local daemon owns every agent, so closing the window or detaching never stops one.",
    body: [
      "hive is only a client. The hived daemon owns each agent's process, its pseudo-terminal and its screen. Detach with Ctrl+Space d, close the window or lose the SSH connection, and the agents keep working. Run hive again and you are back on the same screens.",
      "If the daemon itself restarts, the interface reconnects on its own. Sessions that were running when the daemon stopped unexpectedly come back marked lost, and enter relaunches one.",
    ],
    code: `hive new claude -d -l api   # start in the background
hive list                   # every session
hive attach 3               # open session 3
hive                        # reopen the last view`,
  },
  {
    id: "panes-and-tabs",
    name: "Split panes, tabs and zoom",
    summary: "tmux-style splits and tabs, each pane its own terminal at its own size.",
    body: [
      "Split right with Ctrl+Space v or %, below with - or \", zoom a pane with z and move focus with o or a click. Each pane has a title row with its session and status. Tabs sit above the panes, are clickable and can be renamed.",
      "Closing a pane or tab ends its sessions, so nothing keeps running out of sight; if that would stop a running agent, Hive asks first. Tabs you aren't looking at cost nothing, because Hive only mirrors what is on screen.",
    ],
    code: `Ctrl+Space v   split right     Ctrl+Space t   new tab
Ctrl+Space -   split below     Ctrl+Space ]   next tab
Ctrl+Space z   zoom pane       Ctrl+Space X   close pane
Ctrl+Space o   next pane       Alt+← / Alt+→  switch tab`,
  },
  {
    id: "spaces",
    name: "Spaces that follow your cd",
    summary: "Each repository gets its own tabs, and a terminal moves there when you cd into it.",
    body: [
      "Every repository or folder you work in is a space with its own set of tabs. cd into another repository and within a couple of seconds that terminal, and its tab, moves to that repository's space, created if needed.",
      "Alt+↑ and Alt+↓ switch spaces without the leader key, even while you type to an agent. The AGENTS list under the spaces shows every agent in every space, numbered, so Alt+1 to Alt+9 jumps straight to one wherever it runs.",
    ],
    code: `~/src/api $ cd ../web    # the tab moves to the web space

Alt+↑ / Alt+↓    previous / next space
Alt+← / Alt+→    previous / next tab
Alt+1 … Alt+9    jump to an agent
Ctrl+Space S     new space`,
  },
  {
    id: "worktrees",
    name: "One-key git worktrees",
    summary: "Give an agent its own checkout on a fresh branch so agents never edit the same files.",
    body: [
      "Ctrl+Space C opens a terminal in a new git worktree on a fresh branch; start any agent there. From the command line, hive new codex -w does the same, and -b names the branch. The sidebar marks worktree agents and warns with ! when two agents share a directory.",
      "Hive is careful with your work: it never removes a worktree with uncommitted changes without --force and a confirmation, never removes one a session is still running in, and never deletes branches.",
    ],
    code: `hive new codex -w             # branch hive/codex-<id>
hive new claude -b login-fix  # your own branch name
hive worktree list
hive worktree rm <path>`,
    more: { label: "Git worktrees for AI coding agents", href: "/blog/git-worktrees-for-ai-coding-agents/" },
  },
  {
    id: "command-palette",
    name: "Command palette and search",
    summary: "Fuzzy-search every command, session and workspace from one prompt.",
    body: [
      "Ctrl+Space : opens a palette of every command, session and workspace; Ctrl+Space / searches sessions by name, agent, workspace or branch. The palette and the help screen are generated from your own key bindings, so they always match.",
      "Mouse users get the same reach: click spaces, agents and tabs, and right-click any of them for open, rename, split, close or terminate.",
    ],
    shot: { file: "palette", alt: "The Hive command palette listing commands and sessions" },
  },
  {
    id: "keys",
    name: "A leader key you can rebind",
    summary: "Ctrl+Space by default; every action can be rebound or unbound in config.toml.",
    body: [
      "Everything you type goes to the agent except the leader key and the command after it. The defaults follow tmux and herdr, and Ctrl+Space twice sends a literal Ctrl+Space. A few Alt chords work without the leader; unbind them if an agent needs them.",
      "Every key is configurable, and unbound commands stay in the palette. hive doctor checks the file before you open the interface.",
    ],
    code: `# ~/.config/hive/config.toml
[keys]
leader = "ctrl+a"

[keys.bind]            # after the leader
new_session = "n"
kill_session = ""      # unbind

[keys.global]          # no leader
prev_workspace = []    # free Alt+Up for the agent`,
  },
  {
    id: "several-windows",
    name: "Watch from several windows",
    summary: "Open a session in two terminals: one types, the other watches.",
    body: [
      "The first window to open a session controls it; others watch, and the status bar says so. Ctrl+Space a takes control explicitly. The agent's terminal size always follows the controlling window, so a second, smaller window never squashes the first.",
    ],
  },
  {
    id: "themes",
    name: "Themes that work in any terminal",
    summary: "Truecolor to 16 colours, NO_COLOR, plain ASCII, and no Nerd Fonts required.",
    body: [
      "Pick hive-dark, hive-light, ansi (your terminal's own 16 colours) or mono, and override single colours with hex values. Hive detects truecolor, 256 and 16 colours and honours NO_COLOR. On the Linux console or a non-UTF-8 locale it switches its own symbols to ASCII, and leaves the agent's output untouched.",
    ],
    code: `[general]
theme = "hive-light"

[theme]
colors = "256"     # truecolor | 256 | 16 | none
glyphs = "ascii"   # unicode | ascii
accent = "#e0a526"`,
  },
  {
    id: "custom-agents",
    name: "Any agent, configured in TOML",
    summary: "Add your own commands or override a built-in agent's arguments and environment.",
    body: [
      "Agents are data, not code. Add a command under [agents.<name>] and start it with hive new <name>. Hive runs it directly from its arguments, never through a shell, and never writes environment variables to disk. Agents run unmodified, with their own permission prompts; Hive never answers them for you.",
    ],
    code: `[agents.aider]
command = "aider"
args = ["--dark-mode"]
env = { AIDER_AUTO_COMMITS = "false" }

[agents.claude]        # override a built-in
args = ["--model", "opus"]`,
  },
  {
    id: "platforms",
    name: "Linux, macOS and Windows",
    summary: "Static binaries for five platforms, a one-line installer and hive update.",
    body: [
      "Hive ships prebuilt binaries for Linux (x86_64, ARM64), macOS (Apple silicon, Intel) and Windows 10+, all tested with real terminals in CI. It is one static Rust binary pair with no runtime to install. hive update upgrades in place, and hive doctor reports what Hive sees on your machine.",
      "On Windows the daemon uses a named pipe and ConPTY. Agent status there comes from agents that report it with hive report-agent, and spaces don't yet follow cd.",
    ],
    more: { label: "Install Hive", href: "/install/" },
  },
];
