export default function GettingStarted() {
  return (
    <>
      <h2 id="install">1. Install</h2>
      <p>On Linux or macOS:</p>
      <pre className="code">
        <code>{`curl -fsSL https://github.com/jrh89/hive/releases/latest/download/install.sh | sh`}</code>
      </pre>
      <p>
        The script downloads static binaries, checks their SHA-256 checksum and puts{" "}
        <code>hive</code> and <code>hived</code> in <code>~/.local/bin</code>. Windows and
        building from source are on the <a href="/install/">install page</a>. Then
        check the setup:
      </p>
      <pre className="code">
        <code>{`hive doctor`}</code>
      </pre>
      <p>
        It lists your config file, the daemon, what your terminal supports, and which agent CLIs
        (Claude Code, Codex, Gemini CLI and others) are on your <code>PATH</code>. Hive runs the
        agents you already have; it doesn&apos;t install them or need API keys of its own.
      </p>

      <h2 id="concepts">2. Five words to know</h2>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Term</th>
            <th scope="col">Meaning</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><strong>Daemon</strong> (<code>hived</code>)</td>
            <td>A background process that owns every agent and its terminal. <code>hive</code> starts it for you. Agents live as long as it does, which is why closing the window doesn&apos;t stop them.</td>
          </tr>
          <tr>
            <td><strong>Session</strong></td>
            <td>One terminal running in the daemon: a shell, or an agent such as Claude Code. Each has an id that is never reused, a label and a directory.</td>
          </tr>
          <tr>
            <td><strong>Space</strong> (workspace)</td>
            <td>A group of sessions, normally one git repository. Hive assigns it from the session&apos;s directory, and moves a terminal when you <code>cd</code> elsewhere.</td>
          </tr>
          <tr>
            <td><strong>Worktree</strong></td>
            <td>An optional separate git checkout on its own branch, so two agents in one repository never edit the same files.</td>
          </tr>
          <tr>
            <td><strong>Leader</strong></td>
            <td><kbd>Ctrl+Space</kbd>. Everything you type goes to the agent except the leader and the key after it.</td>
          </tr>
        </tbody>
      </table>

      <h2 id="first-run">3. Open Hive and start an agent</h2>
      <pre className="code">
        <code>{`cd ~/src/my-project
hive`}</code>
      </pre>
      <p>
        You get a sidebar on the left and a terminal on the right. Type <code>claude</code> (or{" "}
        <code>codex</code>, <code>gemini</code>, …) in that terminal as you normally would. Hive
        notices the agent and lists it under AGENTS with its status: working, needs input or
        done.
      </p>
      <p>Start a second one beside it:</p>
      <ul>
        <li>
          <kbd>Ctrl+Space</kbd> <kbd>v</kbd> splits the screen with a new terminal on the right.
          Type <code>codex</code> there.
        </li>
        <li>
          <kbd>Ctrl+Space</kbd> <kbd>t</kbd> opens a new tab instead.
        </li>
        <li>
          <kbd>Ctrl+Space</kbd> <kbd>A</kbd> picks an agent from a list; press <kbd>w</kbd> in the
          list to give it its own worktree.
        </li>
      </ul>
      <p>
        Or from any shell: <code>hive new claude</code> starts Claude Code in the current
        directory, and <code>hive new codex -w</code> starts Codex in its own worktree.
      </p>

      <h2 id="moving-around">4. Move around</h2>
      <table className="table">
        <tbody>
          <tr><td><kbd>Alt+1</kbd>–<kbd>Alt+9</kbd></td><td>Jump to an agent by its number in the sidebar, from anywhere</td></tr>
          <tr><td><kbd>Ctrl+Space</kbd> <kbd>n</kbd> / <kbd>p</kbd></td><td>Next / previous session</td></tr>
          <tr><td><kbd>Alt+←</kbd> <kbd>Alt+→</kbd></td><td>Previous / next tab</td></tr>
          <tr><td><kbd>Alt+↑</kbd> <kbd>Alt+↓</kbd></td><td>Previous / next space</td></tr>
          <tr><td><kbd>Ctrl+Space</kbd> <kbd>:</kbd></td><td>The command palette: every command, session and space, searchable</td></tr>
          <tr><td><kbd>Ctrl+Space</kbd> <kbd>?</kbd></td><td>Help with your current keys</td></tr>
        </tbody>
      </table>
      <p>
        Everything in the sidebar and tab bar is clickable too. The full list is on the{" "}
        <a href="/docs/keys/">keys page</a>.
      </p>

      <h2 id="detach">5. Leave and come back</h2>
      <p>
        <kbd>Ctrl+Space</kbd> <kbd>d</kbd> detaches: the window closes and every agent keeps
        working. Closing the terminal window or losing an SSH connection does the same. Run{" "}
        <code>hive</code> again, from any terminal, to get back exactly where you were. The
        sidebar shows which agents finished (<strong>done</strong>) or are waiting for you
        (<strong>needs input</strong>) while you were away.
      </p>
      <p>
        To stop an agent, close its pane (<kbd>Ctrl+Space</kbd> <kbd>X</kbd>) or terminate it (
        <kbd>Ctrl+Space</kbd> <kbd>x</kbd>); Hive asks first if an agent is still running.
      </p>

      <h2 id="next">Next steps</h2>
      <ul>
        <li>
          Turn on extras such as lazygit review, desktop notifications or an Obsidian journal with{" "}
          <kbd>Ctrl+Space</kbd> <kbd>I</kbd>. See <a href="/integrations/">integrations</a>.
        </li>
        <li>
          Change the leader, theme or agents in <a href="/docs/configuration/">configuration</a>.
        </li>
        <li>
          Learn what the sidebar shows in <a href="/docs/interface/">the interface</a>.
        </li>
      </ul>
    </>
  );
}
