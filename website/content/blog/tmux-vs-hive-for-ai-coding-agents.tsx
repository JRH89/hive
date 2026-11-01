export default function Post() {
  return (
    <>
      <p>
        Most people who run Claude Code or Codex for long stretches end up in tmux, and for good
        reason. Hive borrows tmux&apos;s core idea and adds what agents need. Here is how they
        compare, and when to use which.
      </p>

      <h2 id="what-tmux-does-well">What tmux does well</h2>
      <ul>
        <li>
          <strong>It is everywhere.</strong> Installed or one package away on nearly every server
          you will ever SSH into.
        </li>
        <li>
          <strong>Sessions survive.</strong> A server process owns the terminals, so closing the
          window or losing the connection doesn&apos;t kill your agent.
        </li>
        <li>
          <strong>It is endlessly scriptable.</strong> Decades of configs, plugins and scripts.
        </li>
        <li>
          <strong>It is general.</strong> Editors, logs, REPLs: tmux doesn&apos;t care what runs
          inside.
        </li>
      </ul>
      <p>
        If you already live in tmux and run one or two agents, it may be all you need.
      </p>

      <h2 id="where-agents-strain-tmux">Where agents strain tmux</h2>
      <p>
        tmux treats every pane the same, so it can&apos;t tell you what an agent is doing. With
        several agents across projects the questions become: which one is waiting for me, which
        finished, and are two of them editing the same files? You can build answers with hooks and
        scripts; tmux doesn&apos;t ship them.
      </p>

      <h2 id="what-hive-adds">What Hive adds</h2>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Need</th>
            <th scope="col">tmux</th>
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
            <td>Build it yourself</td>
            <td>Built in, for 20+ agents</td>
          </tr>
          <tr>
            <td>Every agent across projects in one list</td>
            <td>Build it yourself</td>
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
            <td>Mouse menus, clickable sidebar and tabs</td>
            <td>Partly</td>
            <td>Yes</td>
          </tr>
          <tr>
            <td>Native Windows</td>
            <td>No (WSL)</td>
            <td>Yes</td>
          </tr>
        </tbody>
      </table>
      <p>
        Hive also has optional <a href="/integrations/">integrations</a> that tmux users usually
        script: lazygit review beside the agent, GitHub PR checks, project tasks, desktop
        notifications, an Obsidian journal of agent activity, git status per project and token
        usage.
      </p>

      <h2 id="keys">Keys feel familiar</h2>
      <p>
        Hive uses a leader key like tmux&apos;s prefix: <kbd>Ctrl+Space</kbd> by default, then{" "}
        <kbd>v</kbd> or <kbd>%</kbd> to split right, <kbd>-</kbd> or <kbd>&quot;</kbd> to split
        below, <kbd>z</kbd> to zoom, <kbd>d</kbd> to detach. Every key can be rebound in{" "}
        <code>config.toml</code>, and <kbd>Ctrl+Space</kbd> <kbd>:</kbd> opens a palette of every
        command.
      </p>

      <h2 id="both">Using both</h2>
      <p>
        They don&apos;t conflict. Hive is a terminal program, so it runs inside a tmux pane (we test
        it that way), and its default leader doesn&apos;t collide with tmux&apos;s{" "}
        <kbd>Ctrl+b</kbd>. A common setup on a remote machine: tmux for the SSH session, Hive inside
        it for the agents.
      </p>

      <h2 id="which">Which should you use?</h2>
      <ul>
        <li>
          <strong>tmux</strong> if you run one agent at a time, already have a tmux setup you love,
          or work on machines where you can&apos;t install anything.
        </li>
        <li>
          <strong>Hive</strong> if you run several agents, across more than one repository, and
          want to know at a glance which one needs you.
        </li>
      </ul>
      <p>
        Hive is free, MIT-licensed and installs with one command. <a href="/install/">Try it</a>, or
        read the longer <a href="/why/">comparison with tmux, Zellij and herdr</a>.
      </p>
    </>
  );
}
