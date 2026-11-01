export default function Post() {
  return (
    <>
      <p>
        Coding agents write a lot of code quickly, and all of it lands in your working tree. The
        habit that keeps that safe is the same one you would use for a colleague&apos;s pull
        request: review before it is committed. Here is a review routine that takes a few minutes
        and catches most of what goes wrong.
      </p>

      <h2 id="isolate">Start with a branch you can throw away</h2>
      <p>
        Review is easiest when the agent&apos;s work is separate from everything else. Give each
        agent its own branch, ideally in its own{" "}
        <a href="/blog/git-worktrees-for-ai-coding-agents/">git worktree</a>, so its changes never
        mix with yours and discarding them is one command. Then the whole review is one diff:
      </p>
      <pre className="code">
        <code>{`git diff main...agent/login-fix --stat   # what changed, by file
git diff main...agent/login-fix          # the changes themselves`}</code>
      </pre>

      <h2 id="overview">1. Get the overview first</h2>
      <p>
        Before reading any code, look at the shape of the change with <code>git status</code> and{" "}
        <code>git diff --stat</code>. Ask:
      </p>
      <ul>
        <li>Did it touch files that have nothing to do with the task?</li>
        <li>Are there new files you didn&apos;t expect, such as scripts, fixtures or config?</li>
        <li>
          Did a lockfile change? A changed <code>package-lock.json</code> or{" "}
          <code>Cargo.lock</code> means new or upgraded dependencies, which deserve a look on
          their own.
        </li>
        <li>Is the size about right? A one-line bug fix that changes 400 lines needs explaining.</li>
      </ul>

      <h2 id="read">2. Read the diff by hunk</h2>
      <p>
        Interactive staging turns reading into deciding:
      </p>
      <pre className="code">
        <code>{`git add -p    # y to stage a hunk, n to skip, s to split it`}</code>
      </pre>
      <p>
        Whatever you stage, you have read and accepted. Whatever is left over gets a second look,
        or goes back to the agent. A terminal UI makes this faster: <code>lazygit</code> stages
        lines and hunks with a key, and <code>tig</code> is a quick read-only browser.
      </p>

      <h2 id="watch-for">3. What to watch for</h2>
      <ul>
        <li>
          <strong>Weakened tests.</strong> An agent asked to make tests pass may loosen an
          assertion, skip a test or delete it. Read changes under <code>tests/</code> first.
        </li>
        <li>
          <strong>Swallowed errors.</strong> A new empty <code>catch</code>, an{" "}
          <code>unwrap_or_default()</code> or a broad <code>except</code> that hides the original
          failure.
        </li>
        <li>
          <strong>Invented APIs.</strong> Calls to functions or options that don&apos;t exist in the
          version you use. The type checker catches some; read the rest.
        </li>
        <li>
          <strong>Secrets and local paths.</strong> Tokens, <code>.env</code> values, or{" "}
          <code>/home/you/...</code> paths copied into code or config.
        </li>
        <li>
          <strong>Scope creep.</strong> Unrequested refactors, renames and reformatting that bury
          the real change. Ask the agent to split them out.
        </li>
      </ul>

      <h2 id="run">4. Run it yourself</h2>
      <p>
        An agent saying the tests pass is not the same as the tests passing on your machine. Run
        the test suite, the linter and the type checker yourself, and try the change by hand if it
        touches anything a user sees. It takes a minute and catches the cases where the agent ran a
        narrower command than you would.
      </p>

      <h2 id="commit">5. Commit in small, described steps</h2>
      <p>
        Commit what you have reviewed, with a message saying what changed and why. If the agent
        did several things, make several commits. Small commits are easier to revert when
        something turns up later, and the history tells you which changes came from which task.
      </p>

      <h2 id="in-hive">Review beside the agent in Hive</h2>
      <p>
        Hive puts this routine next to the agent. Enable the review integration and{" "}
        <kbd>Ctrl+Space</kbd> <kbd>G</kbd> opens lazygit, tig or <code>git diff</code> in a split
        beside the focused agent, in its own directory or worktree:
      </p>
      <pre className="code">
        <code>{`# ~/.config/hive/config.toml
[integrations.review]
enabled = true

[integrations.git_status]
enabled = true   # main ~3 ↑2 under every project in the sidebar`}</code>
      </pre>
      <p>
        With git status on, the sidebar shows changed files and unpushed commits for every
        project, so uncommitted agent work doesn&apos;t sit unnoticed in a repository you
        haven&apos;t opened today. And because the sidebar marks an agent <em>done</em> when it
        finishes while you are elsewhere, you know when there is something to review. See{" "}
        <a href="/integrations/#review">review</a> and{" "}
        <a href="/integrations/#git-status">git status</a>.
      </p>
    </>
  );
}
