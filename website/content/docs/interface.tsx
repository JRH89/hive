export default function Interface() {
  return (
    <>
      <pre className="code" aria-label="The Hive interface">
        <code>{`  SPACES                  │   1  2  +                                   api
                          │
  ● api                 2 │   (the focused agent's terminal)
    main ~3 ↑1            │
  · docs                1 │
                          │
  + new            ● menu │
 ─────────────────────────│
  AGENTS                  │
 ▌◐ claude              1 │
    api · main            │
  ◆ codex               2 │
    api · wt hive/codex-2 │
  ● #1 api · main · running · 2 active      Ctrl+Space leader · then : commands`}</code>
      </pre>

      <h2 id="sidebar">The sidebar</h2>
      <p>
        <strong>SPACES</strong> lists the places you are working: one per repository (or folder),
        with its branch and how many sessions it has. A space shows the status of its most urgent
        agent. Only spaces with something open are listed.
      </p>
      <p>
        <strong>AGENTS</strong> lists every agent in every space, numbered for{" "}
        <kbd>Alt+1</kbd>–<kbd>Alt+9</kbd>, so you see which one needs you wherever you are. It
        includes agents you started by hand in a terminal, listed by name (e.g.{" "}
        <code>claude</code>). Plain terminals and tool panes such as lazygit aren&apos;t listed.
      </p>
      <p>
        Click a space to switch to it, an agent to open it, <strong>+ new</strong> for a new space
        and <strong>menu</strong> for the command palette. <kbd>Ctrl+Space</kbd> <kbd>b</kbd>{" "}
        hides the sidebar; it also hides itself in narrow terminals.
      </p>

      <h3 id="status">Agent status</h3>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Symbol</th>
            <th scope="col">ASCII</th>
            <th scope="col">Meaning</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>◐</td><td><code>~</code></td><td>working</td></tr>
          <tr><td>◆</td><td><code>!</code></td><td>needs input: a permission prompt or a question</td></tr>
          <tr><td>✓</td><td><code>+</code></td><td>done: finished while you were looking elsewhere; clears when you open it</td></tr>
          <tr><td>●</td><td><code>*</code></td><td>idle, or a session without a recognised agent</td></tr>
        </tbody>
      </table>
      <p>
        See <a href="/docs/agent-status/">agent status</a> for how Hive knows.
      </p>

      <h3 id="marks">Session marks</h3>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Mark</th>
            <th scope="col">ASCII</th>
            <th scope="col">Meaning</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>○</td><td><code>o</code></td><td>exited</td></tr>
          <tr><td>✕</td><td><code>x</code></td><td>failed to start, or crashed</td></tr>
          <tr><td>◌</td><td><code>?</code></td><td>lost: it was running when the daemon stopped unexpectedly. Enter relaunches it</td></tr>
          <tr><td>▌</td><td><code>&gt;</code></td><td>the session you are looking at</td></tr>
          <tr><td><code>wt</code></td><td><code>wt</code></td><td>runs in its own git worktree</td></tr>
          <tr><td><code>!</code></td><td><code>!</code></td><td>shares its directory with another running agent: they may edit the same files</td></tr>
          <tr><td><code>~3 ↑1 ↓2</code></td><td><code>~3 ^1 v2</code></td><td>with git status on: changed files, commits ahead, behind</td></tr>
        </tbody>
      </table>
      <p>
        The ASCII column is what you see on the Linux console, without a UTF-8 locale, or with{" "}
        <code>[theme] glyphs = &quot;ascii&quot;</code>.
      </p>

      <h2 id="panes-tabs">Panes and tabs</h2>
      <p>
        Split the screen to watch several agents at once: <kbd>Ctrl+Space</kbd> <kbd>v</kbd>{" "}
        (right) or <kbd>-</kbd> (below) opens a new terminal in a split. Each pane is its own
        terminal at its own size, with a title row showing its session and status. Keys go to the
        focused pane; click a pane or press <kbd>o</kbd> to focus another, <kbd>z</kbd> to zoom
        it.
      </p>
      <p>
        Tabs hold sets of panes. The tab row above the panes (<code>1  2  +</code>) is
        clickable: pick a tab, or <code>+</code> for a new terminal tab. Tabs belong to a space,
        so the row shows the current space&apos;s tabs and <kbd>Alt+↑</kbd> <kbd>Alt+↓</kbd>{" "}
        switches space and its tabs together.
      </p>
      <p>
        Closing a pane or tab ends its sessions, so nothing keeps running out of sight; Hive asks
        first if that would stop a running agent. To leave everything running, detach instead. The
        arrangement of panes lasts until you quit <code>hive</code>; the sessions keep running
        either way. Tabs you aren&apos;t looking at cost nothing: Hive only mirrors what is on
        screen.
      </p>

      <h2 id="spaces">Spaces follow your terminals</h2>
      <p>
        <strong>+ new</strong> (or <kbd>Ctrl+Space</kbd> <kbd>S</kbd>) opens a terminal in your home
        folder in a space of its own. <code>cd</code> somewhere and within a couple of seconds the
        terminal and its tab move to that place&apos;s space, created if needed:
      </p>
      <ul>
        <li>inside a git repository, the repository&apos;s space;</li>
        <li>elsewhere, the deepest existing space containing the folder;</li>
        <li>otherwise a new space for the folder itself (<code>cd ~/Work</code> makes a <code>Work</code> space).</li>
      </ul>
      <p>
        Your home folder&apos;s space only holds terminals in home itself. On Windows, terminals
        stay in the space they started in. To add a folder as a space without opening a terminal,
        use <em>Add a folder as a space</em> in the palette (Tab completes the path).
      </p>

      <h2 id="palette">The command palette</h2>
      <p>
        <kbd>Ctrl+Space</kbd> <kbd>:</kbd> (or <strong>menu</strong>) searches every command,
        session and space by fuzzy match; Enter runs or opens it. <kbd>Ctrl+Space</kbd>{" "}
        <kbd>/</kbd> searches sessions only, by label, agent, space or branch. The palette and the
        help screen are built from your own key bindings, so they always match them.
      </p>

      <h2 id="mouse">Mouse</h2>
      <p>
        Everything in the sidebar and the tab bar is clickable, the wheel scrolls the sidebar, and
        clicking a pane focuses it. In a terminal, clicks and the wheel go to agents that use the
        mouse; full-screen programs without mouse support scroll with the wheel. Hold{" "}
        <kbd>Shift</kbd> to select text with your terminal as usual.
      </p>
      <p>Right-click for menus:</p>
      <ul>
        <li>a space: open, new terminal here, rename, close space (asks first);</li>
        <li>a tab: rename, split right or below, close tab;</li>
        <li>an agent: open, rename, split right or below, terminate.</li>
      </ul>
      <p>
        Turn the mouse off with <code>[terminal] mouse_support = false</code>.
      </p>

      <h2 id="windows">Several windows</h2>
      <p>
        You can run <code>hive</code> in two terminals, or on two machines over SSH, at once. When
        both show the same session, the first controls it and the second watches; the status bar
        says <em>observing</em>. <kbd>Ctrl+Space</kbd> <kbd>a</kbd> in the watching window takes
        control. The session&apos;s terminal size follows the window in control.
      </p>
    </>
  );
}
