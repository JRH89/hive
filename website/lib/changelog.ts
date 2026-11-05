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
    version: "0.5.6",
    date: "2026-11-04T16:25:52-08:00",
    title: "Put your spaces in any order",
    changes: [
      "Drag a space up or down the sidebar to reorder the list, or press Alt+Shift+Up / Alt+Shift+Down. The order is remembered.",
    ],
  },
  {
    version: "0.5.5",
    date: "2026-11-04T16:06:43-08:00",
    title: "Your agents survive turning the computer off",
    changes: [
      "Shutting down or rebooting no longer loses agents. The system stops every program at once, so an agent could end a moment before Hive saved it and be forgotten. Now it comes back as ended with the rest, and Ctrl+Space R resumes it.",
    ],
  },
  {
    version: "0.5.4",
    date: "2026-11-04T15:23:40-08:00",
    title: "Windows on Arm",
    changes: [
      "Native Windows Arm64 binaries (Snapdragon and other Arm PCs). Before, the Windows installer stopped with \"no prebuilt binary\" on these machines.",
      "The Windows installer checks the CPU in a way that works in both Windows PowerShell 5.1 and PowerShell 7, and installs the x64 build (run under emulation) for older releases without an Arm one.",
    ],
  },
  {
    version: "0.5.3",
    date: "2026-11-03T14:57:47-08:00",
    title: "Quitting an agent with Ctrl+C removes it too",
    changes: [
      "An agent you quit with Ctrl+C now leaves the sidebar like one you quit normally. Before, it stayed listed as ended.",
      "Sessions restored more than once now get the fixed relaunch from 0.5.2. Before, they kept the old wrapper, and quitting the agent left a bare shell listed under its name.",
      "The hexagon on the empty screen has its straight sides back.",
    ],
  },
  {
    version: "0.5.2",
    date: "2026-11-02T21:20:51-08:00",
    title: "Restored agents leave when you quit them",
    changes: [
      "Quitting an agent that Ctrl+Space R brought back (one first started from a terminal) now ends its session, so it leaves the sidebar. Before, a bare shell took over the pane and stayed listed under the agent's name.",
      "The daemon now notices an agent quitting even when the process that started it lives on, so the sidebar stops showing it.",
    ],
  },
  {
    version: "0.5.1",
    date: "2026-11-02T13:49:41-08:00",
    title: "q closes the notes pane",
    changes: [
      "Quitting the notes app (q) or lazygit in the review pane now closes its pane, instead of leaving it behind to be terminated by hand. Pull request checks, tasks and usage still keep their output after they finish.",
    ],
  },
  {
    version: "0.5.0",
    date: "2026-11-02T13:26:52-08:00",
    title: "Notes and context for your agents",
    changes: [
      "hive notes: Hive's own notes app for a markdown or Obsidian vault, in a pane. A note tree, rendered notes (headings, lists, checkboxes, code, tables, callouts), wikilinks you follow with Tab and Enter, backlinks, search, new and daily notes, e to edit and o to open the note in Obsidian. Notes changed elsewhere reload on their own.",
      "Ctrl+Space N now opens it beside the focused pane, instead of running $EDITOR (which could open a desktop editor and leave an empty tab). [integrations.obsidian] agent still picks something else.",
      "hive context search, get, list and add: agents can look up project knowledge in the vault and record decisions in it. Output is path:line, or --json.",
      "Sessions in a space with a vault get HIVE_VAULT.",
      "Every integration opens in a split by default.",
    ],
  },
  {
    version: "0.4.18",
    date: "2026-11-02T12:26:15-08:00",
    title: "A tree sidebar, if you want one",
    changes: [
      "An optional sidebar layout: one tree, each space with its agents indented under it. Space in the sidebar (or a click on the arrow) folds a space, and a folded space still shows its most urgent agent's status. Turn it on with sidebar_tree in the palette, or [general] sidebar_tree = true. The two-list sidebar stays the default.",
    ],
  },
  {
    version: "0.4.17",
    date: "2026-11-01T22:51:18-08:00",
    title: "Quit agents leave the list",
    changes: [
      "Quitting an agent (or typing exit in a terminal) removes it from the sidebar and closes its pane, including agents that quit while no window was open. One that fails stays, marked exited with its code, so you can read what went wrong; x removes it. Task, review and other integration panes keep their output.",
    ],
  },
  {
    version: "0.4.16",
    date: "2026-11-01T22:23:55-08:00",
    title: "Restarts keep your sessions",
    changes: [
      "Stopping or upgrading the daemon no longer forgets your sessions. They come back as ended, in their spaces, and hive tells you how many when it opens.",
      "Ctrl+Space R restores every ended session at once (Enter on one restores just that one). Claude Code resumes with --continue and Codex with resume --last, so each picks up its last conversation in that directory. An agent you started from a terminal runs in that terminal again.",
      "hive daemon stop lists the sessions it would end and asks first; -y skips the question. It also saves them, so stopping a daemon from 0.4.15 or older with the new hive keeps them too.",
    ],
  },
  {
    version: "0.4.15",
    date: "2026-11-01T21:09:53-08:00",
    title: "Confirmations you can't miss",
    changes: [
      "Terminating a session or closing a pane, tab or space with a running agent now asks in a dialog in the middle of the screen, naming the agents it would stop. y or Enter goes ahead, a goes ahead and never asks again (saved as [general] confirm_terminate = false), anything else cancels.",
      "Hive warns at startup when the running daemon is older than hive. Old daemons keep old behaviour, such as the 3-second wait when closing shells that 0.4.12 removed; hive daemon stop then hive updates it.",
    ],
  },
  {
    version: "0.4.14",
    date: "2026-11-01T18:17:53-08:00",
    title: "Documentation",
    changes: [
      "Full documentation at hive.codefrontlabs.com/docs: getting started, the interface, every key, a configuration reference with every option and default, every command, agent status and report-agent, worktrees, the daemon and its files, and troubleshooting.",
      "[branding] show_tagline = false now hides the tagline on the start screen; it was accepted but ignored.",
      "examples/config.toml lists every command name for [keys.bind].",
    ],
  },
  {
    version: "0.4.13",
    date: "2026-11-01T17:54:36-08:00",
    title: "Turn integrations on from the menu",
    changes: [
      "Ctrl+Space I (or \"integrations\" in the palette) lists every integration with a checkbox. Enter switches one on or off at once, no restart, and saves it to config.toml, keeping your comments.",
      "The key of an integration that is off now points to the Integrations list.",
    ],
  },
  {
    version: "0.4.12",
    date: "2026-10-31T22:31:24-07:00",
    title: "Closing ends the session",
    changes: [
      "Closing a pane (Ctrl+Space X) or a tab ends its sessions instead of hiding them, so nothing keeps running out of sight. Plain terminals close at once; if a running agent would stop, Hive asks first. Detach to leave everything running.",
      "Terminating a session sends SIGHUP like closing a terminal window, so shells exit right away instead of after a 3-second timeout.",
      "The key of an integration that is off (e.g. Ctrl+Space U) now says how to turn it on instead of doing nothing.",
    ],
  },
  {
    version: "0.4.10",
    date: "2026-10-31T21:54:01-07:00",
    title: "Maintenance",
    changes: [
      "The unused [general] restore_workspace setting is now ignored explicitly; configs that set it still load.",
      "Release builds move to Node 24 GitHub Actions ahead of Node 20's removal.",
    ],
  },
  {
    version: "0.4.9",
    date: "2026-10-31T15:51:21-07:00",
    title: "Layouts",
    changes: [
      "Optional [layouts.<name>]: a set of panes (agents, commands such as npm run dev, terminals) that Ctrl+Space L opens together in a new tab of the current space. A layout listing a space opens there without asking; otherwise you pick one.",
      "Tool panes (notes, review, checks, tasks, usage) no longer appear in the AGENTS list or set off the shared-directory warning.",
    ],
  },
  {
    version: "0.4.8",
    date: "2026-10-31T14:57:53-07:00",
    title: "Right-click menus, git status, usage",
    changes: [
      "Right-click a space (open, new terminal here, rename, close), a tab (rename, split right or below, close) or an agent (open, rename, split, terminate). Tabs can have names.",
      "Optional [integrations.git_status]: changed files and commits ahead/behind under each repository space, e.g. main ~3 ↑2.",
      "Optional [integrations.usage]: Ctrl+Space U opens token usage and cost in a pane (ccusage blocks --live by default).",
      "Website: a Why Hive page comparing Hive with tmux, Zellij and herdr.",
    ],
  },
  {
    version: "0.4.7",
    date: "2026-10-31T14:42:59-07:00",
    title: "Every agent in one list",
    changes: [
      "The sidebar's AGENTS list shows every agent in every space, grouped by space, with the space it runs in. Alt+1–9 jump to any of them, switching space as needed.",
      "The website and README describe Hive's integrations: Obsidian notes and journal, change review, pull request checks, project tasks and desktop notifications.",
    ],
  },
  {
    version: "0.4.6",
    date: "2026-10-31T14:08:53-07:00",
    title: "+ new opens a new space",
    changes: [
      "The sidebar's + new (or Ctrl+Space S) opens a new space with a terminal in your home folder. cd into a repo or folder and the terminal becomes that place's space; the temporary space disappears. The tab row's + still adds a tab to the current space.",
      "hive with nothing running opens a terminal right away instead of an empty screen.",
    ],
  },
  {
    version: "0.4.5",
    date: "2026-10-31T13:51:08-07:00",
    title: "A space for every folder",
    changes: [
      "cd into any folder, not just a git repository, and the terminal moves to a space for it (e.g. ~/Work). Your home directory's space only holds terminals in home itself.",
      "A terminal started inside a folder gets that folder's space instead of joining the home space.",
      "hive daemon stop works even when config.toml has a mistake.",
    ],
  },
  {
    version: "0.4.4",
    date: "2026-10-31T12:28:25-07:00",
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
