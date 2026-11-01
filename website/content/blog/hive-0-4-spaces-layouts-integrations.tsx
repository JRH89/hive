export default function Post() {
  return (
    <>
      <p>
        Hive 0.4 started as &ldquo;macOS and Windows&rdquo; and grew into the release where Hive
        became a workspace rather than a list of sessions. Here is what changed across 0.4.0 to
        0.4.12, grouped by what you will notice. The <a href="/changelog/">changelog</a> has every
        detail.
      </p>

      <h2 id="everywhere">Linux, macOS and Windows</h2>
      <p>
        Hive now ships static binaries for Linux (x86_64, ARM64), macOS (Apple silicon, Intel) and
        Windows 10+, all tested in CI. On Windows the daemon talks over a named pipe with a
        per-daemon token and runs agents in ConPTY; a PowerShell one-liner installs it. See{" "}
        <a href="/install/">install</a>.
      </p>

      <h2 id="terminal-first">Terminal first</h2>
      <p>
        New panes, splits and tabs open a terminal. Start an agent by typing its name, just as you
        would anywhere else, and Hive recognises it: the pane title and sidebar say{" "}
        <code>claude</code> or <code>codex</code> instead of &ldquo;shell&rdquo;.{" "}
        <kbd>Ctrl+Space</kbd> <kbd>A</kbd> still picks an agent from a list.
      </p>

      <h2 id="spaces">Spaces that follow your cd</h2>
      <p>
        Every repository, or plain folder, you work in is a space with its own tabs.{" "}
        <code>cd</code> into another repository and within two seconds the terminal moves to that
        repository&apos;s space. The sidebar&apos;s <strong>+ new</strong> opens a fresh space;{" "}
        <kbd>Alt+↑</kbd>/<kbd>Alt+↓</kbd> switch spaces and <kbd>Alt+←</kbd>/<kbd>Alt+→</kbd> their
        tabs. The AGENTS list shows every agent in every space, with its status, and{" "}
        <kbd>Alt+1</kbd>–<kbd>9</kbd> jumps straight to one.
      </p>

      <h2 id="worktrees">Worktree terminals</h2>
      <p>
        <kbd>Ctrl+Space</kbd> <kbd>C</kbd> opens a terminal in its own git worktree on a fresh
        branch, so two agents never edit the same checkout. Background:{" "}
        <a href="/blog/git-worktrees-for-ai-coding-agents/">git worktrees for AI coding agents</a>.
      </p>

      <h2 id="mouse">Right-click menus</h2>
      <p>
        Right-click a space to open it, start a terminal there, rename or close it; a tab to rename
        it, split it or close it; an agent to open, rename, split beside or terminate it. Tabs can
        have names. Closing a pane or tab now ends its sessions (a running agent asks first), so
        nothing keeps running out of sight; detach to leave everything running.
      </p>

      <h2 id="layouts">Layouts</h2>
      <p>
        Describe a set of panes once and open it with <kbd>Ctrl+Space</kbd> <kbd>L</kbd>:
      </p>
      <pre className="code">
        <code>{`[layouts.web]
spaces = ["web"]
panes = [
  { agent = "claude" },
  { run = ["npm", "run", "dev"], split = "right", label = "server" },
  { split = "down" },
]`}</code>
      </pre>
      <p>In a space a layout names, it opens without asking.</p>

      <h2 id="integrations">Integrations</h2>
      <p>
        Optional and off by default, each with a key that tells you how to enable it:
      </p>
      <ul>
        <li>
          <a href="/integrations/#obsidian">Obsidian notes</a> and a daily journal of what your
          agents did
        </li>
        <li>
          <a href="/integrations/#review">Review</a> with lazygit, tig or git diff beside the agent
        </li>
        <li>
          <a href="/integrations/#github">Pull request checks</a> with the GitHub CLI
        </li>
        <li>
          <a href="/integrations/#tasks">Project tasks</a> from just, npm, make or cargo
        </li>
        <li>
          <a href="/integrations/#desktop-notifications">Desktop notifications</a>
        </li>
        <li>
          <a href="/integrations/#git-status">Git status</a> under each space, like{" "}
          <code>main ~3 ↑2</code>
        </li>
        <li>
          <a href="/integrations/#usage">Usage and cost</a> via ccusage
        </li>
      </ul>

      <h2 id="upgrade">Upgrading</h2>
      <pre className="code">
        <code>{`hive update
hive daemon stop   # restarts the daemon on the new version; ends running sessions`}</code>
      </pre>
      <p>
        Questions and bug reports are welcome on{" "}
        <a href="https://github.com/jrh89/hive/issues">GitHub</a>; include the output of{" "}
        <code>hive doctor</code>.
      </p>
    </>
  );
}
