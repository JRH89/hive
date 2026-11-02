// The hive command line. A Rust test (hive-cli tests/cli.rs
// website_cli_docs_list_every_command) checks every subcommand is here.

export type CliCommand = {
  usage: string;
  summary: string;
  options?: [flag: string, meaning: string][];
  examples?: string;
  notes?: string;
};

export const cliCommands: CliCommand[] = [
  {
    usage: "hive",
    summary:
      "Open the interface. Reattaches to the newest live session, or shows the start screen. Starts the daemon if it isn't running (unless [daemon] auto_start = false).",
  },
  {
    usage: "hive new [AGENT] [-- ARGS...]",
    summary:
      "Launch an agent in a new session and attach to it. AGENT is any name from hive agents, shell, or one from [agents.*]; without it, [general] default_agent.",
    options: [
      ["-l, --label <LABEL>", "Session label (defaults to the agent name)"],
      ["-C, --cwd <DIR>", "Working directory (defaults to the current directory)"],
      ["-W, --workspace <WORKSPACE>", "Workspace name or id (defaults to the one containing the directory)"],
      ["-w, --worktree", "Run in a new git worktree on branch hive/<agent>-<id>"],
      ["-b, --branch <BRANCH>", "Branch for the worktree (implies --worktree; created if missing)"],
      ["-d, --detached", "Create the session without attaching"],
    ],
    examples: `hive new claude -l api -C ~/src/api
hive new codex -w                 # own worktree
hive new codex -b fix/login       # own worktree on branch fix/login
hive new shell -d -l db           # a background shell
hive new claude -- --model opus   # arguments after -- go to the agent`,
  },
  {
    usage: "hive list",
    summary: "List sessions with id, label, agent, state, workspace, branch and directory.",
  },
  {
    usage: "hive attach <SESSION>",
    summary: "Open the interface on session SESSION (its id).",
  },
  {
    usage: "hive kill <SESSION>",
    summary:
      "Terminate a session and remove it. Hive sends SIGHUP and SIGTERM to its process group, then SIGKILL after [daemon] kill_timeout_secs.",
  },
  {
    usage: "hive status",
    summary: "One-line summary of the daemon and its sessions.",
  },
  {
    usage: "hive agents [-a]",
    summary:
      "List the agents Hive can launch and recognise, and which are installed. -a also lists known agents that are not installed.",
  },
  {
    usage: "hive workspace list",
    summary: "List workspaces (spaces).",
  },
  {
    usage: "hive workspace add <PATH> [-n NAME]",
    summary: "Add a workspace for a directory (its repository root inside git).",
  },
  {
    usage: "hive workspace rename <WORKSPACE> <NAME>",
    summary: "Rename a workspace.",
  },
  {
    usage: "hive workspace rm <WORKSPACE>",
    summary: "Forget a workspace that has no sessions. Nothing on disk changes.",
  },
  {
    usage: "hive worktree list [-W WORKSPACE]",
    summary: "List a repository's worktrees: branch, clean or modified, path, and the sessions running in each.",
  },
  {
    usage: "hive worktree rm <PATH> [--force] [-y]",
    summary:
      "Remove a worktree. Refused while a session runs in it, and refused with uncommitted changes unless --force (which asks to confirm unless -y). The branch is always kept.",
  },
  {
    usage: "hive report-agent --agent <NAME> --state <STATE> [--message TEXT] [--seq N] [SESSION]",
    summary:
      "Report an agent's status for a session: idle, working, blocked or unknown. Meant for agents and scripts; SESSION defaults to $HIVE_SESSION_ID.",
    notes: "Reports numbered (--seq) at or below the last one are ignored, so late reports can't overwrite newer ones. The default is the current time in milliseconds.",
    examples: `"$HIVE_BIN_PATH" report-agent --agent mybot --state blocked --message 'approve edit?'`,
  },
  {
    usage: "hive release-agent --agent <NAME> [SESSION]",
    summary: "Clear an agent's status report; screen detection takes over again.",
  },
  {
    usage: "hive doctor",
    summary:
      "Check the installation: config file and whether it is valid, socket path, daemon, logs, detected colours and glyphs, and which agents are on PATH. Changes nothing, never starts the daemon, and exits non-zero on a problem.",
  },
  {
    usage: "hive update [--tag vX.Y.Z]",
    summary:
      "Install the latest release (or --tag) next to the running hive, verifying its SHA-256 checksum. Linux and macOS. A running daemon keeps the old version until hive daemon stop.",
  },
  {
    usage: "hive daemon start",
    summary: "Start the daemon if it isn't running.",
  },
  {
    usage: "hive daemon stop",
    summary: "Stop the daemon. This terminates every session.",
  },
  {
    usage: "hive daemon status",
    summary: "Show whether the daemon is running, with its version, pid and socket.",
  },
];
