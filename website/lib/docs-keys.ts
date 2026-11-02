// Every Hive command with its default keys. A Rust test
// (hive-tui keymap::tests::website_key_docs_match_the_defaults) checks this
// file against the keymap, so keep `name` and `keys` exactly as in the code.

export type KeyCommand = {
  /** Name used in [keys.bind] and [keys.global]. */
  name: string;
  /** Default keys after the leader. */
  keys: string[];
  /** Default prefix-free chords. */
  global?: string[];
  title: string;
  /** The [integrations.*] section or other condition it needs. */
  needs?: string;
};

export type KeyGroup = { title: string; commands: KeyCommand[] };

export const keyGroups: KeyGroup[] = [
  {
    title: "Find and switch",
    commands: [
      { name: "palette", keys: [":", "P"], title: "Command palette: every command, session and workspace" },
      { name: "search_sessions", keys: ["/"], title: "Search sessions by name, agent, workspace or branch" },
      { name: "next_session", keys: ["n", "j", "down"], title: "Next session" },
      { name: "prev_session", keys: ["p", "k", "up"], title: "Previous session" },
      { name: "next_workspace", keys: [], global: ["alt+down"], title: "Next space" },
      { name: "prev_workspace", keys: [], global: ["alt+up"], title: "Previous space" },
      { name: "workspaces", keys: ["w"], title: "Switch workspace (space)" },
      { name: "sidebar", keys: ["s", "tab"], title: "Focus the sidebar" },
      { name: "toggle_sidebar", keys: ["b"], title: "Show or hide the sidebar" },
    ],
  },
  {
    title: "Start things",
    commands: [
      { name: "new_session", keys: ["c"], title: "New terminal in the focused pane's directory" },
      { name: "new_agent", keys: ["A"], title: "New agent: pick one from a list" },
      { name: "new_worktree_terminal", keys: ["C"], title: "New terminal in its own git worktree (own branch)" },
      { name: "new_worktree_session", keys: [], title: "New agent in its own worktree (pick one)" },
      { name: "new_space", keys: ["S"], title: "New space: a terminal in your home folder" },
      { name: "add_workspace", keys: [], title: "Add a folder as a space" },
      { name: "layout", keys: ["L"], title: "Open a layout", needs: "a [layouts.<name>]" },
    ],
  },
  {
    title: "Panes and tabs",
    commands: [
      { name: "split_right", keys: ["v", "%"], title: "Split: new terminal on the right" },
      { name: "split_down", keys: ["-", "\""], title: "Split: new terminal below" },
      { name: "focus_next_pane", keys: ["o"], title: "Focus the next pane" },
      { name: "zoom", keys: ["z"], title: "Zoom the focused pane; again to restore" },
      { name: "close_pane", keys: ["X"], title: "Close the pane and end its session" },
      { name: "new_tab", keys: ["t"], title: "New tab with a terminal" },
      { name: "next_tab", keys: ["]"], global: ["alt+right"], title: "Next tab (with one tab: next agent in the space)" },
      { name: "prev_tab", keys: ["["], global: ["alt+left"], title: "Previous tab" },
      { name: "close_tab", keys: [], title: "Close the tab and end its sessions" },
    ],
  },
  {
    title: "Sessions",
    commands: [
      { name: "rename_session", keys: ["r"], title: "Rename session" },
      { name: "kill_session", keys: ["x"], title: "Terminate session (asks first)" },
      { name: "take_control", keys: ["a"], title: "Take input control from another window" },
      { name: "detach", keys: ["d", "q"], title: "Detach: leave Hive, agents keep running" },
      { name: "rename_workspace", keys: [], title: "Rename workspace" },
    ],
  },
  {
    title: "Integrations",
    commands: [
      { name: "integrations", keys: ["I"], title: "Integrations: turn each one on or off" },
      { name: "notes", keys: ["N"], title: "Open the workspace's Obsidian vault", needs: "[integrations.obsidian]" },
      { name: "review", keys: ["G"], title: "Review changes (lazygit, tig or git diff)", needs: "[integrations.review]" },
      { name: "pull_request", keys: ["H"], title: "Pull request checks for the branch", needs: "[integrations.github]" },
      { name: "run_task", keys: ["T"], title: "Run a project task beside the session", needs: "[integrations.tasks]" },
      { name: "usage", keys: ["U"], title: "Token usage and cost of your agents", needs: "[integrations.usage]" },
    ],
  },
  {
    title: "Help",
    commands: [
      { name: "help", keys: ["?"], title: "Help: your current key bindings" },
      { name: "settings", keys: [], title: "Show where the config file is" },
    ],
  },
];
