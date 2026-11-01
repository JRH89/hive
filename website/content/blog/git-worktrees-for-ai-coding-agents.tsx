export default function Post() {
  return (
    <>
      <p>
        If you run more than one coding agent on a repository, give each its own checkout. Git has
        had the tool for this since 2015: <code>git worktree</code>. This guide covers the commands,
        the things that trip people up, and how to clean up without losing work.
      </p>

      <h2 id="what-is-a-worktree">What a worktree is</h2>
      <p>
        A worktree is an extra working directory attached to the same repository. All worktrees
        share one object database, so creating one is fast and costs only the size of the checked
        out files. Each worktree has its own branch, index and working files, so an agent in one
        can&apos;t touch the files of an agent in another.
      </p>
      <p>
        Compared with cloning the repository again, worktrees share history and remotes, and a
        commit made in one is immediately visible to the others: no pushing between clones.
      </p>

      <h2 id="commands">The commands you need</h2>
      <pre className="code">
        <code>{`# New worktree on a new branch, next to the main checkout
git worktree add -b agent/search ../shop-search

# New worktree on an existing branch
git worktree add ../shop-review feature/review

# See them all
git worktree list

# Remove one (refuses if it has uncommitted changes)
git worktree remove ../shop-search

# Forget worktrees whose directories you deleted by hand
git worktree prune`}</code>
      </pre>
      <p>
        Then start the agent inside it: <code>cd ../shop-search &amp;&amp; claude</code>. When it
        is done, review and merge its branch like any other.
      </p>

      <h2 id="pitfalls">Pitfalls</h2>
      <h3>One branch, one worktree</h3>
      <p>
        Git won&apos;t check out the same branch in two worktrees. That is a feature: give each
        agent its own branch (<code>agent/&lt;task&gt;</code> is a good convention) and merge
        afterwards.
      </p>
      <h3>Untracked files don&apos;t come along</h3>
      <p>
        A new worktree contains only tracked files. Your <code>.env</code>, local config and other
        ignored files stay in the main checkout. Copy what the agent needs, or keep a script that
        sets a worktree up.
      </p>
      <h3>Dependencies are per worktree</h3>
      <p>
        <code>node_modules</code>, Python virtual environments and build directories are ignored
        files too, so each worktree needs its own install. Package managers with a shared store
        (pnpm, uv, Cargo&apos;s registry cache) keep this fast; plan for the first build to take a
        while.
      </p>
      <h3>Ports and databases collide</h3>
      <p>
        Two dev servers from two worktrees both want port 3000, and both may point at the same
        local database. Give each worktree its own port, and be careful with migrations.
      </p>
      <h3>Removing a worktree can lose work</h3>
      <p>
        <code>git worktree remove</code> refuses when there are uncommitted changes, unless you add{" "}
        <code>--force</code>. Deleting the directory with <code>rm -rf</code> skips that check
        entirely. Commit or stash first, and never force-remove an agent&apos;s worktree you
        haven&apos;t looked at.
      </p>

      <h2 id="where-to-put-them">Where to put them</h2>
      <p>
        Siblings of the main checkout (<code>../shop-search</code>) are easy to find. A dedicated
        folder keeps your projects directory tidy. Avoid putting worktrees inside the main
        checkout: tools that scan the repository will find a second copy of everything.
      </p>

      <h2 id="with-hive">Worktrees in Hive</h2>
      <p>
        Hive does the bookkeeping. <kbd>Ctrl+Space</kbd> <kbd>C</kbd> opens a terminal in a new
        worktree on a fresh branch named <code>hive/shell-&lt;id&gt;</code>; start any agent there.
        From the command line:
      </p>
      <pre className="code">
        <code>{`hive new codex -w             # new worktree, branch hive/codex-<id>
hive new claude -b login-fix  # new worktree on branch login-fix
hive worktree list
hive worktree rm <path>`}</code>
      </pre>
      <p>
        Worktrees live under <code>~/.local/share/hive/worktrees</code> by default (set{" "}
        <code>[worktrees] dir</code> to change it). The sidebar marks worktree agents with their
        branch, and Hive warns when two agents share a directory. It never removes a worktree with
        uncommitted changes or one still in use without being forced, and it never deletes
        branches: your agent&apos;s work stays until you decide.
      </p>
      <p>
        For the bigger picture of running agents in parallel, see{" "}
        <a href="/blog/run-multiple-ai-coding-agents/">
          how to run several AI coding agents at once
        </a>
        .
      </p>
    </>
  );
}
