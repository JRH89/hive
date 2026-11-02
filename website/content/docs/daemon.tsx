export default function Daemon() {
  return (
    <>
      <h2 id="what">What the daemon does</h2>
      <p>
        <code>hived</code> owns every session: the agent process, its terminal (PTY) and an
        emulator holding its screen and scrollback. The <code>hive</code> window only mirrors the
        sessions it shows and forwards your keys, which is why closing it stops nothing. The first{" "}
        <code>hive</code> command starts the daemon in the background (turn that off with{" "}
        <code>[daemon] auto_start = false</code>); one daemon serves every window and command of
        your user.
      </p>
      <pre className="code">
        <code>{`hive daemon status   # running? version, pid, socket
hive daemon start
hive daemon stop     # ends every session`}</code>
      </pre>

      <h2 id="survives">What survives what</h2>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Event</th>
            <th scope="col">Agents</th>
            <th scope="col">What you see</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>Close the window, detach, SSH drops</td><td>Keep running</td><td>Everything, as you left it, next time you run <code>hive</code></td></tr>
          <tr><td>The daemon goes away while a window is open</td><td>Depends on why (below)</td><td>The window keeps the last screen and reconnects on its own</td></tr>
          <tr><td><code>hive daemon stop</code></td><td>Terminated</td><td>A clean start; the sessions are forgotten</td></tr>
          <tr><td>Daemon crash, <code>kill -9</code>, reboot, sleep</td><td>Gone (a reboot ends all processes)</td><td>The sessions listed as <strong>lost</strong> (◌), with their label, agent and directory</td></tr>
        </tbody>
      </table>
      <p>
        A lost session&apos;s process is gone, but you can press Enter on it to relaunch the same
        command in the same directory, or <kbd>x</kbd> to forget it. Many agents can pick up their
        conversation: <code>claude --continue</code>, <code>codex resume</code>. Sleep pauses
        agents rather than ending them; they carry on when the machine wakes. For agents that run
        overnight, use a machine that stays on and connect over SSH.
      </p>

      <h2 id="files">Where Hive keeps things</h2>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">What</th>
            <th scope="col">Linux</th>
            <th scope="col">Override</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>Config</td><td><code>~/.config/hive/config.toml</code></td><td><code>HIVE_CONFIG</code></td></tr>
          <tr><td>Socket and lock</td><td><code>$XDG_RUNTIME_DIR/hive/</code> (mode 0700)</td><td><code>HIVE_RUNTIME_DIR</code></td></tr>
          <tr><td>Logs (<code>hived.log</code>, <code>hive.log</code>) and <code>sessions.json</code></td><td><code>~/.local/state/hive/</code></td><td><code>HIVE_STATE_DIR</code></td></tr>
          <tr><td>Worktrees</td><td><code>~/.local/share/hive/worktrees/</code></td><td><code>[worktrees] dir</code></td></tr>
        </tbody>
      </table>
      <p>
        macOS and Windows use their usual folders; <code>hive doctor</code> prints the actual
        paths. <code>sessions.json</code> holds session and workspace details (labels, commands,
        directories), never environment variables. Logs never contain environment variables or
        agent output. On Windows the daemon listens on a named pipe protected by a token instead
        of a socket.
      </p>

      <h2 id="env">Environment variables</h2>
      <h3>Read by Hive</h3>
      <table className="table">
        <tbody>
          <tr><td><code>HIVE_CONFIG</code></td><td>Config file to use</td></tr>
          <tr><td><code>HIVE_RUNTIME_DIR</code></td><td>Directory for the socket and lock. Keep the path short: socket paths must fit in about 107 bytes</td></tr>
          <tr><td><code>HIVE_STATE_DIR</code></td><td>Directory for logs and session details</td></tr>
          <tr><td><code>HIVE_LOG</code></td><td>Log level, e.g. <code>debug</code></td></tr>
          <tr><td><code>HIVE_DAEMON_BIN</code></td><td>The <code>hived</code> binary to start (default: next to <code>hive</code>, then <code>PATH</code>)</td></tr>
          <tr><td><code>NO_COLOR</code>, <code>COLORTERM</code>, <code>TERM</code></td><td>Colour detection, unless <code>[theme] colors</code> is set</td></tr>
          <tr><td><code>VISUAL</code>, <code>EDITOR</code></td><td>The editor the Obsidian integration opens</td></tr>
          <tr><td><code>HIVE_INSTALL_DIR</code>, <code>HIVE_VERSION</code></td><td>For the install script and <code>hive update</code>: where to install, and which version</td></tr>
        </tbody>
      </table>
      <h3>Set for every session</h3>
      <table className="table">
        <tbody>
          <tr><td><code>HIVE=1</code></td><td>The program runs inside Hive</td></tr>
          <tr><td><code>HIVE_SESSION_ID</code></td><td>The session&apos;s id</td></tr>
          <tr><td><code>HIVE_BIN_PATH</code></td><td>The <code>hive</code> binary, for <a href="/docs/agent-status/#report-agent">report-agent</a></td></tr>
          <tr><td><code>TERM=xterm-256color</code>, <code>COLORTERM=truecolor</code></td><td>What the session&apos;s terminal supports</td></tr>
        </tbody>
      </table>
      <p>
        Otherwise a session starts with the environment of the <code>hive</code> command that
        created it, plus the agent&apos;s <code>env</code> from the config.
      </p>

      <h2 id="private">A private instance</h2>
      <p>
        To try things without touching your real sessions, point Hive at scratch directories. It
        starts a separate daemon there:
      </p>
      <pre className="code">
        <code>{`export HIVE_RUNTIME_DIR=/tmp/hive-test HIVE_STATE_DIR=/tmp/hive-test/state HIVE_CONFIG=/tmp/hive-test/config.toml
hive
hive daemon stop    # when done`}</code>
      </pre>

      <h2 id="upgrade">Upgrading</h2>
      <pre className="code">
        <code>{`hive update         # installs the latest release next to the running hive
hive                # new windows use the new version straight away`}</code>
      </pre>
      <p>
        A running daemon keeps its version, and your sessions, until it stops. Most releases work
        with the old daemon; when one doesn&apos;t, <code>hive</code> says the protocol versions
        differ, and <code>hive daemon stop</code> (which ends your sessions) followed by{" "}
        <code>hive</code> starts the new one. On Windows, run the installer again.
      </p>

      <h2 id="uninstall">Uninstalling</h2>
      <pre className="code">
        <code>{`hive daemon stop
rm ~/.local/bin/hive ~/.local/bin/hived
rm -rf ~/.config/hive ~/.local/state/hive
# Worktrees in ~/.local/share/hive/worktrees may hold your work: check them first.`}</code>
      </pre>
      <p>
        On Windows: <code>hive daemon stop</code>, then delete <code>%LOCALAPPDATA%\hive</code>{" "}
        and the config and state folders <code>hive doctor</code> lists.
      </p>
    </>
  );
}
