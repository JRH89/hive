const problems: [string, React.ReactNode][] = [
  [
    "A Ctrl+Space key goes to the wrong command",
    <>Keys are case-sensitive. <kbd>n</kbd> is the next session; <kbd>N</kbd> (Shift+n) opens notes. <kbd>Ctrl+Space</kbd> <kbd>?</kbd> lists the keys as you have them.</>,
  ],
  [
    "An integration's key does nothing, or says it is off",
    <>Integrations are off until enabled. Press <kbd>Ctrl+Space</kbd> <kbd>I</kbd> and turn it on; it works at once.</>,
  ],
  [
    "Ctrl+Space does nothing at all",
    <>Your desktop or input method uses it (input-method switching often does). Choose another leader: <code>[keys] leader = &quot;ctrl+a&quot;</code>.</>,
  ],
  [
    "The agent needs a key Hive takes",
    <>Only the leader and the Alt chords are intercepted. Press the leader twice to send it to the agent; free an Alt chord with <code>[keys.global] next_tab = []</code>.</>,
  ],
  [
    "Review opens git diff instead of lazygit",
    <>Neither lazygit nor tig is on <code>PATH</code>. Install one, or set <code>[integrations.review] command</code>.</>,
  ],
  [
    "GitHub checks or usage pane exits at once",
    <>The tool isn&apos;t installed or set up: <code>gh</code> needs <code>gh auth login</code>; usage needs <code>ccusage</code> (<code>npm i -g ccusage</code>) or another <code>command</code>.</>,
  ],
  [
    "No Obsidian vault in <project>",
    <>Hive looks for a folder with <code>.obsidian/</code> at or above the agent&apos;s directory, or one level below the repository root. Open the vault once in Obsidian to create it.</>,
  ],
  [
    "No desktop notifications",
    <><code>notify-send</code> is missing (on Debian and Ubuntu it is in <code>libnotify-bin</code>), or your desktop has no notification service. On macOS set <code>[integrations.desktop_notify] command</code> to a tool such as <code>terminal-notifier</code>.</>,
  ],
  [
    "daemon speaks protocol vN, this client speaks vM",
    <>A daemon from an older version is still running. <code>hive daemon stop</code> (ends its sessions), then <code>hive</code>.</>,
  ],
  [
    "`claude` was not found on PATH",
    <>The agent isn&apos;t installed, or isn&apos;t on the <code>PATH</code> of the shell you ran <code>hive</code> from. Check <code>hive doctor</code>, or give an absolute <code>command</code> under <code>[agents.&lt;name&gt;]</code>.</>,
  ],
  [
    "hive: invalid config",
    <>The message names the key and line. Fix it and check with <code>hive doctor</code>. A config error never leaves a broken screen behind.</>,
  ],
  [
    "An agent shows no status",
    <>Hive only shows status for agents it has rules for, or that <a href="/docs/agent-status/#report-agent">report it</a>. <code>hive agents -a</code> lists which have rules.</>,
  ],
  [
    "Boxes, question marks or misaligned borders",
    <>The font or locale lacks the glyphs. Use a UTF-8 locale or set <code>[theme] glyphs = &quot;ascii&quot;</code>. Nerd Fonts are never needed.</>,
  ],
  [
    "Colours look wrong",
    <>Force a depth with <code>[theme] colors = &quot;256&quot;</code>, try <code>theme = &quot;ansi&quot;</code> to use your terminal&apos;s palette, or <code>theme = &quot;hive-light&quot;</code> on a light background.</>,
  ],
  [
    "Sessions show ◌ lost",
    <>The daemon stopped unexpectedly (crash, <code>kill -9</code>, reboot). The processes are gone; Enter relaunches one in the same directory, <kbd>x</kbd> forgets it. See <a href="/docs/daemon/#survives">what survives what</a>.</>,
  ],
  [
    "Typing does nothing; the status bar says observing",
    <>Another window controls the session. <kbd>Ctrl+Space</kbd> <kbd>a</kbd> takes control.</>,
  ],
  [
    "Kept worktree …: it has uncommitted changes",
    <>Working as intended. Commit or stash, or <code>hive worktree rm &lt;path&gt; --force</code>.</>,
  ],
  [
    "hived did not come up",
    <>See <code>hived.log</code>. Another daemon may hold the lock: <code>hive daemon status</code>.</>,
  ],
  [
    "Socket errors with a custom HIVE_RUNTIME_DIR",
    <>Socket paths must be under about 107 bytes. Use a shorter directory; <code>hive doctor</code> warns about this.</>,
  ],
  [
    "Selecting text with the mouse doesn't work",
    <>Hold <kbd>Shift</kbd> while selecting, or turn the mouse off with <code>[terminal] mouse_support = false</code>.</>,
  ],
];

export default function Troubleshooting() {
  return (
    <>
      <h2 id="doctor">Start with hive doctor</h2>
      <pre className="code">
        <code>{`hive doctor`}</code>
      </pre>
      <p>
        It shows the config file and whether it is valid, the socket path, whether the daemon is
        running, where the logs are, the colours and glyphs detected, and which agents are on your{" "}
        <code>PATH</code>. It changes nothing, never starts the daemon, and exits non-zero if it
        finds a problem.
      </p>

      <h2 id="logs">Logs</h2>
      <p>
        <code>hived.log</code> (daemon) and <code>hive.log</code> (window) are in{" "}
        <code>~/.local/state/hive/</code>. For more detail, restart with debug logging:
      </p>
      <pre className="code">
        <code>{`hive daemon stop && HIVE_LOG=debug hive`}</code>
      </pre>

      <h2 id="problems">Common problems</h2>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Symptom</th>
            <th scope="col">Cause and fix</th>
          </tr>
        </thead>
        <tbody>
          {problems.map(([symptom, fix]) => (
            <tr key={symptom}>
              <td>{symptom}</td>
              <td>{fix}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2 id="windows">Windows</h2>
      <ul>
        <li>Terminals stay in the space they started in (spaces don&apos;t follow <code>cd</code>).</li>
        <li><code>hive update</code> isn&apos;t available; rerun the PowerShell installer.</li>
      </ul>

      <h2 id="bug">Reporting a bug</h2>
      <p>
        <a href="https://github.com/jrh89/hive/issues/new">Open an issue</a> with the output of{" "}
        <code>hive doctor</code>, your terminal, and the relevant part of <code>hived.log</code>{" "}
        (with <code>HIVE_LOG=debug</code> if you can reproduce it). Check the log for anything
        private before sharing.
      </p>
    </>
  );
}
