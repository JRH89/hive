export default function Post() {
  return (
    <>
      <p>
        Zellij and Hive are both terminal workspaces written in Rust, and both keep sessions running
        in a background server. They aim at different jobs: Zellij is a general, highly
        configurable multiplexer, and Hive is built around running AI coding agents. Here is how
        that plays out day to day.
      </p>

      <h2 id="zellij">What Zellij does well</h2>
      <ul>
        <li>
          <strong>Discoverable from the first minute.</strong> A status bar shows the keys for
          the current mode, so you rarely need a cheat sheet.
        </li>
        <li>
          <strong>Rich layouts.</strong> Layout files describe tabs and panes, with commands to run
          in each, and floating panes sit over the rest.
        </li>
        <li>
          <strong>Plugins.</strong> A WebAssembly plugin system powers much of its own interface,
          and you can write your own.
        </li>
        <li>
          <strong>General purpose.</strong> Editors, servers, logs: Zellij treats every pane alike.
        </li>
      </ul>
      <p>
        If you want one multiplexer for everything you do in a terminal, and agents are a part of
        it, Zellij is an excellent choice.
      </p>

      <h2 id="agents">Where agents need more</h2>
      <p>
        Treating every pane alike is the limitation once you run several agents. A multiplexer
        that doesn&apos;t know a pane holds Claude Code can&apos;t tell you that it has stopped at a
        permission prompt, that the Codex in another tab finished ten minutes ago, or that two
        agents are editing the same checkout. You can assemble some of that with plugins and agent
        hooks; it isn&apos;t there out of the box.
      </p>

      <h2 id="compare">Side by side</h2>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Need</th>
            <th scope="col">Zellij</th>
            <th scope="col">Hive</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Sessions survive closing the terminal</td>
            <td>Yes</td>
            <td>Yes</td>
          </tr>
          <tr>
            <td>Agent status: working, needs input, done</td>
            <td>Not built in</td>
            <td>Built in, for 20+ agents</td>
          </tr>
          <tr>
            <td>Every agent across projects in one list</td>
            <td>Not built in</td>
            <td>Built in</td>
          </tr>
          <tr>
            <td>Group work by repository</td>
            <td>Sessions, by hand</td>
            <td>Spaces that follow your cd</td>
          </tr>
          <tr>
            <td>Agent in its own git worktree</td>
            <td>Scripts</td>
            <td>One key</td>
          </tr>
          <tr>
            <td>Warning when agents share a directory</td>
            <td>No</td>
            <td>Yes</td>
          </tr>
          <tr>
            <td>Layouts</td>
            <td>Yes, very flexible</td>
            <td>Yes, simpler</td>
          </tr>
          <tr>
            <td>Plugin system</td>
            <td>Yes (WebAssembly)</td>
            <td>No; built-in integrations instead</td>
          </tr>
          <tr>
            <td>Floating panes</td>
            <td>Yes</td>
            <td>No</td>
          </tr>
          <tr>
            <td>Native Windows</td>
            <td>No</td>
            <td>Yes</td>
          </tr>
        </tbody>
      </table>

      <h2 id="keys">Keys and agents</h2>
      <p>
        Agent CLIs use plenty of Ctrl chords themselves, so a multiplexer&apos;s keys can get in
        the way. Zellij handles this with modes, including a locked mode that passes keys straight
        through. Hive uses a single leader key, <kbd>Ctrl+Space</kbd> by default: everything you
        type goes to the agent except the leader and the one key after it. Both are fully
        rebindable.
      </p>

      <h2 id="hive-extras">What Hive adds around the agents</h2>
      <p>
        Instead of plugins, Hive ships optional <a href="/integrations/">integrations</a> aimed at
        agent work, each off until you enable it: lazygit review beside an agent, GitHub pull
        request checks, project tasks, desktop notifications, an Obsidian journal of what your
        agents did, git status per project and token usage.
      </p>

      <h2 id="which">Which should you use?</h2>
      <ul>
        <li>
          <strong>Zellij</strong> if you want a general workspace with floating panes and plugins,
          and run agents occasionally.
        </li>
        <li>
          <strong>Hive</strong> if agents are most of what you run: several at once, across
          repositories, and you want to know which one needs you without checking each tab.
        </li>
      </ul>
      <p>
        They can share a machine, too: Hive is an ordinary terminal program and runs inside a
        Zellij pane. For the bigger picture, see{" "}
        <a href="/why/">how Hive compares with tmux, Zellij and herdr</a>.
      </p>
    </>
  );
}
