export default function Worktrees() {
  return (
    <>
      <h2 id="workspaces">Workspaces (spaces)</h2>
      <p>
        Every session belongs to a workspace, shown as a <strong>space</strong> in the sidebar.
        Hive picks it from the session&apos;s directory: inside a git repository it is the
        repository (worktrees included, so an agent in a worktree stays in its repository&apos;s
        space); elsewhere it is the folder. Terminals move with you when you <code>cd</code>; see{" "}
        <a href="/docs/interface/#spaces">spaces follow your terminals</a>.
      </p>
      <p>
        Workspaces are remembered across daemon restarts. Manage them with{" "}
        <kbd>Ctrl+Space</kbd> <kbd>w</kbd>, right-click on a space, or:
      </p>
      <pre className="code">
        <code>{`hive workspace list
hive workspace add ~/src/api -n api     # a space without opening a terminal
hive workspace rename api backend
hive workspace rm backend               # only when it has no sessions; nothing on disk changes`}</code>
      </pre>

      <h2 id="why-worktrees">Why worktrees</h2>
      <p>
        Two agents in the same checkout edit the same files, and one&apos;s half-finished change
        breaks the other&apos;s tests. Hive warns with <code>!</code> beside agents that share a
        directory. A git worktree gives an agent its own checkout of the repository on its own
        branch: separate files, shared history. You review and merge its branch when it is done.
      </p>

      <h2 id="create">Starting an agent in a worktree</h2>
      <table className="table">
        <tbody>
          <tr><td><code>hive new codex -w</code></td><td>Codex in a new worktree on branch <code>hive/codex-&lt;id&gt;</code></td></tr>
          <tr><td><code>hive new claude -b fix/login</code></td><td>On branch <code>fix/login</code>, created from the current HEAD if missing</td></tr>
          <tr><td><kbd>Ctrl+Space</kbd> <kbd>C</kbd></td><td>A terminal in a new worktree (<code>hive/shell-&lt;id&gt;</code>); start any agent there</td></tr>
          <tr><td><kbd>Ctrl+Space</kbd> <kbd>A</kbd>, then <kbd>w</kbd></td><td>Pick an agent and give it a worktree</td></tr>
        </tbody>
      </table>
      <p>
        Worktrees go in <code>~/.local/share/hive/worktrees</code> (<code>$XDG_DATA_HOME</code>
        ), or <code>[worktrees] dir</code>. The sidebar marks those sessions <code>wt</code> with
        their branch.
      </p>

      <h2 id="cleanup">Cleaning up</h2>
      <p>
        When you terminate a session in a clean worktree (<kbd>Ctrl+Space</kbd> <kbd>x</kbd>), Hive
        offers <kbd>w</kbd> to remove the worktree too. From the command line:
      </p>
      <pre className="code">
        <code>{`hive worktree list                 # branch, clean or modified, path, sessions in it
hive worktree rm <path>            # remove a clean worktree
hive worktree rm <path> --force    # also discard uncommitted changes (asks first)`}</code>
      </pre>

      <h2 id="safety">Safety rules</h2>
      <ul>
        <li>A worktree with uncommitted changes is never removed without <code>--force</code> and a confirmation.</li>
        <li>A worktree a session is still running in is never removed.</li>
        <li>Branches are never deleted. Merge or delete them with git when you are done.</li>
        <li>Hive runs ordinary <code>git worktree</code> commands, so <code>git worktree list</code> and your other tools see the same thing.</li>
      </ul>
      <p>
        More on the workflow in{" "}
        <a href="/blog/git-worktrees-for-ai-coding-agents/">git worktrees for AI coding agents</a>.
      </p>
    </>
  );
}
