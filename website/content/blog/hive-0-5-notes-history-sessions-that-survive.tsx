export default function Post() {
  return (
    <>
      <p>
        Hive 0.5 gives your agents a memory and makes your sessions much harder to lose. It covers
        everything since the <a href="/blog/hive-0-4-spaces-layouts-integrations/">0.4 post</a>:
        0.4.13 to 0.5.14, grouped by what you will notice. The <a href="/changelog/">changelog</a>{" "}
        has every detail.
      </p>

      <h2 id="notes">Notes, in a pane</h2>
      <p>
        <code>hive notes</code> is Hive&apos;s own notes app for a markdown or Obsidian vault. It
        shows a note tree and renders notes with headings, lists, checkboxes, code, tables and
        callouts. Follow wikilinks with <kbd>Tab</kbd> and <kbd>Enter</kbd>, see backlinks, search,
        and make new or daily notes. <kbd>e</kbd> edits a note, <kbd>o</kbd> opens it in Obsidian,
        and notes changed elsewhere reload on their own. <kbd>Ctrl+Space</kbd> <kbd>N</kbd> opens it
        beside the focused pane, and <kbd>q</kbd> closes it. The files stay plain markdown, so
        Obsidian, your editor and git all keep working.
      </p>

      <h2 id="context">Context for your agents</h2>
      <p>
        An agent that starts fresh every session keeps relearning your project.{" "}
        <code>hive context</code> lets it look things up in the vault and write down what it
        decided:
      </p>
      <pre className="code">
        <code>{`hive context search how are worktrees cleaned up
hive context get "ADR-0012"
hive context list
hive context add Hive/Decisions "Auth tokens now expire after 1h"`}</code>
      </pre>
      <p>
        Results are <code>path:line</code>, or JSON with <code>--json</code>. <code>add</code>{" "}
        only appends a dated entry and never rewrites a note. Sessions in a space with a vault get{" "}
        <code>HIVE_VAULT</code>, so an agent knows where to look. Tell it about the command in your{" "}
        <code>CLAUDE.md</code> or <code>AGENTS.md</code>. See <a href="/docs/notes/">notes</a>.
      </p>

      <h2 id="sessions">Sessions that survive</h2>
      <p>
        Stopping or upgrading the daemon, or turning the computer off, used to forget your
        sessions. Now they come back as ended, in their spaces, and <code>hive</code> tells you how
        many when it opens. <kbd>Ctrl+Space</kbd> <kbd>R</kbd> restores them all (<kbd>Enter</kbd>{" "}
        on one restores just that one). Claude Code resumes with <code>--continue</code> and Codex
        with <code>resume --last</code>, so each picks up its last conversation in that directory.
      </p>
      <p>
        The other half is tidiness. An agent you quit, normally or with <kbd>Ctrl+C</kbd>, leaves
        the sidebar and closes its pane. One that fails stays, marked with its exit code, so you
        can read what went wrong. Terminating anything with a running agent asks first, in a dialog
        that names the agents it would stop.
      </p>

      <h2 id="history">Terminal history and copying</h2>
      <p>
        Scroll back with the mouse wheel or <kbd>Shift+Page Up</kbd>. The history comes back after
        you reattach, and typing returns you to live output. Drag over a pane to select text: it
        is copied when you let go. Apps that use the mouse still get it; hold <kbd>Shift</kbd>{" "}
        there to select with your terminal instead.
      </p>

      <h2 id="sidebar">The sidebar</h2>
      <ul>
        <li>
          An optional tree layout, with each space&apos;s agents indented under it. A folded space
          still shows its most urgent agent&apos;s status. Turn it on with{" "}
          <code>sidebar_tree</code> in the palette.
        </li>
        <li>
          Drag spaces to reorder them, or press <kbd>Alt+Shift+↑</kbd>/<kbd>Alt+Shift+↓</kbd>. The
          order is remembered.
        </li>
        <li>Scrollbars you can click and drag when a list holds more than fits.</li>
      </ul>

      <h2 id="settings">Settings without editing files</h2>
      <p>
        <kbd>Ctrl+Space</kbd> <kbd>I</kbd> lists every integration with a checkbox. <kbd>Enter</kbd>{" "}
        switches one on or off immediately and saves it to <code>config.toml</code>, keeping your
        comments. From the palette you can also choose a shell for a new tab or split, or save a
        default shell. New terminals use the shell that launched Hive when it can tell, including
        Git Bash and PowerShell.
      </p>

      <h2 id="windows">Windows, properly</h2>
      <p>
        <kbd>Ctrl+Space</kbd> works in Windows Terminal and PowerShell, which reported it as a
        different key before. Windows Terminal gets full colour instead of 16, and there are native
        Arm64 builds for Snapdragon PCs. The full-screen interface is now tested end to end on
        Linux, macOS and Windows in CI.
      </p>

      <h2 id="upgrade">Upgrading</h2>
      <p>
        From 0.5.13, opening <code>hive</code> installs a newer release first, checked at most once
        an hour. Turn that off with <code>auto_update = false</code> under <code>[general]</code>,
        or skip it once with <code>HIVE_NO_AUTO_UPDATE=1</code>. From an older version:
      </p>
      <pre className="code">
        <code>{`hive update
hive daemon stop   # restarts the daemon on the new version; your sessions come back`}</code>
      </pre>
      <p>
        <code>hive daemon stop</code> lists the sessions it would end and asks first. Full docs are
        at <a href="/docs/">/docs</a>. Questions and bug reports are welcome on{" "}
        <a href="https://github.com/jrh89/hive/issues">GitHub</a>; include the output of{" "}
        <code>hive doctor</code>.
      </p>
    </>
  );
}
