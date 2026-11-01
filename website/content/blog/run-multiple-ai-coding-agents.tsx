export default function Post() {
  return (
    <>
      <p>
        One coding agent is a pair programmer. Three or four running at once are a team, and a
        team needs some organisation. Run several in a few terminal tabs and the same problems
        show up every time: two agents edit the same file, one sits waiting for approval in a tab
        you forgot about, and closing the wrong window takes an hour of context with it.
      </p>
      <p>
        These four rules fix most of it. They hold whichever agents you use (Claude Code, Codex,
        Gemini CLI, OpenCode or a mix) and whether or not you use Hive.
      </p>

      <h2 id="separate-checkouts">1. Give every agent its own checkout</h2>
      <p>
        Agents don&apos;t coordinate. Two agents in the same directory read and write the same
        files, run the same tests against each other&apos;s half-finished changes, and can leave
        your working tree in a state neither of them meant. The fix is old and boring: one
        checkout per agent.
      </p>
      <p>
        Git worktrees make that cheap. A worktree is a second working directory backed by the same
        repository, on its own branch:
      </p>
      <pre className="code">
        <code>{`git worktree add -b agent/login-fix ../shop-login-fix
cd ../shop-login-fix && claude`}</code>
      </pre>
      <p>
        Each agent then works on its own branch, and you merge what you like. Our{" "}
        <a href="/blog/git-worktrees-for-ai-coding-agents/">guide to worktrees for agents</a>{" "}
        covers the pitfalls. In Hive, <kbd>Ctrl+Space</kbd> <kbd>C</kbd> opens a terminal in a new
        worktree on a fresh branch, and the sidebar warns when two agents share a directory.
      </p>

      <h2 id="visible-status">2. Make status visible at a glance</h2>
      <p>
        The real cost of parallel agents is attention. An agent blocked on a permission prompt
        does nothing until you notice. With five tabs, you notice late.
      </p>
      <p>You want three states visible without opening anything:</p>
      <ul>
        <li>
          <strong>Working</strong>: leave it alone.
        </li>
        <li>
          <strong>Needs input</strong>: a question or a permission prompt. Go now.
        </li>
        <li>
          <strong>Done</strong>: finished while you were elsewhere. Review it.
        </li>
      </ul>
      <p>
        Hive reads each agent&apos;s screen with per-agent rules and shows these states in the
        sidebar for every agent in every project, even agents you started by typing{" "}
        <code>claude</code> in a plain terminal. Agents it has no rules for show as unknown rather
        than a guess. Your own scripts can report their state with <code>hive report-agent</code>
        , and the optional desktop notifications call you back when something needs you.
      </p>

      <h2 id="sessions-survive">3. Keep sessions alive when the terminal goes away</h2>
      <p>
        A long agent run shouldn&apos;t depend on a terminal window staying open. Closing a window,
        an SSH connection dropping or a terminal crash shouldn&apos;t end the session.
      </p>
      <p>
        That is what a multiplexer server is for. tmux does it, and so does Hive: its daemon,{" "}
        <code>hived</code>, owns the agents&apos; terminals. Detach with <kbd>Ctrl+Space</kbd>{" "}
        <kbd>d</kbd> or just close the window, and the agents keep working. Run <code>hive</code>{" "}
        again to pick up exactly where you were. (A reboot or a sleeping laptop still stops them;
        for agents that run overnight, use an always-on machine and connect over SSH.)
      </p>

      <h2 id="one-place-per-project">4. One place per project</h2>
      <p>
        When agents work across several repositories, a flat list of tabs stops making sense. Group
        work by project: each repository gets its own set of tabs, and switching projects switches
        the whole set.
      </p>
      <p>
        Hive calls these spaces and creates them for you. Open a terminal, <code>cd</code> into a
        repository, and within a couple of seconds the terminal moves to that repository&apos;s
        space. <kbd>Alt+↑</kbd>/<kbd>Alt+↓</kbd> moves between spaces, <kbd>Alt+←</kbd>/
        <kbd>Alt+→</kbd> between a space&apos;s tabs, and the AGENTS list shows every agent of every
        space with its status, so nothing hides in a project you aren&apos;t looking at.
      </p>

      <h2 id="workflow">A workflow that puts it together</h2>
      <ol>
        <li>
          Run <code>hive</code> in your repository. You get a terminal.
        </li>
        <li>
          Start the main agent: type <code>claude</code>.
        </li>
        <li>
          For a second, independent task press <kbd>Ctrl+Space</kbd> <kbd>C</kbd> and start{" "}
          <code>codex</code> in the new worktree.
        </li>
        <li>
          Split a pane for the dev server (<kbd>Ctrl+Space</kbd> <kbd>v</kbd>), or define a{" "}
          <a href="/integrations/#layouts">layout</a> that opens agent, server and terminal in one
          go.
        </li>
        <li>
          Work on something else. When the sidebar shows <em>needs input</em>, answer it; when it
          shows <em>done</em>, review the diff (<kbd>Ctrl+Space</kbd> <kbd>G</kbd> opens lazygit
          beside the agent if you enable review).
        </li>
        <li>Merge the branches you want. Remove worktrees you don&apos;t need.</li>
      </ol>
      <p>
        None of this needs a cloud service or an account: it all runs locally, and the agents run
        unmodified with their own permission prompts intact.
      </p>
    </>
  );
}
