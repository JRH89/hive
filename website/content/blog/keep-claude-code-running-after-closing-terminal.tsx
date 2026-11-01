export default function Post() {
  return (
    <>
      <p>
        You start Claude Code or Codex on a long task, close the laptop lid or the terminal window,
        and come back to find it gone. This article explains why that happens and walks through
        four ways to keep an agent running after the terminal closes, from the quick fix to a
        setup for several agents.
      </p>

      <h2 id="why">Why closing the terminal kills the agent</h2>
      <p>
        An interactive agent runs attached to a terminal. When that terminal goes away (you close
        the window or an SSH connection drops), the system sends a hangup signal to everything
        running in it, and the agent exits. Running it in the background with <code>&amp;</code>{" "}
        or <code>nohup</code> doesn&apos;t help either: an interactive interface needs a terminal to
        draw on and read keys from.
      </p>
      <p>
        So the fix is to give the agent a terminal that doesn&apos;t belong to your window: one
        owned by a background process that keeps running when you leave.
      </p>

      <h2 id="tmux">Option 1: tmux (or screen)</h2>
      <p>
        A terminal multiplexer runs a server that owns the terminals. Your window is just a view of
        them.
      </p>
      <pre className="code">
        <code>{`tmux new -s agent     # start a session
claude                # run the agent inside it
# Ctrl+b then d       # detach; the agent keeps running
tmux attach -t agent  # come back later, even over a new SSH connection`}</code>
      </pre>
      <p>
        This is the classic answer and works everywhere tmux is installed. GNU screen does the same
        with <code>screen -S agent</code> and <code>screen -r agent</code>.
      </p>

      <h2 id="headless">Option 2: run the agent headless</h2>
      <p>
        If the task needs no back-and-forth, skip the interactive interface. Claude Code&apos;s
        print mode runs one prompt and exits, which <code>nohup</code> can keep alive:
      </p>
      <pre className="code">
        <code>{`nohup claude -p "fix the failing tests in src/cart" > claude.log 2>&1 &`}</code>
      </pre>
      <p>
        The catch: nobody is there to answer permission prompts, so the agent can only do what its
        permission settings already allow. That suits narrow, well-defined jobs, not exploratory
        work.
      </p>

      <h2 id="hive">Option 3: Hive</h2>
      <p>
        Hive works like tmux underneath: a background daemon, <code>hived</code>, owns every
        agent&apos;s terminal. Detach with <kbd>Ctrl+Space</kbd> <kbd>d</kbd>, or just close the
        window, and the agents keep working. Run <code>hive</code> again to get them back.
      </p>
      <pre className="code">
        <code>{`hive                       # open Hive; type claude in the terminal
hive new codex -d -l api   # or start one in the background
hive list                  # what is running
hive attach 2              # open one`}</code>
      </pre>
      <p>
        Where it differs from tmux is what it knows about the agents: the sidebar shows which one
        is working, which needs input and which is done, across every project. When you come
        back after an hour, you see at a glance which agent is waiting on you. It also reconnects on
        its own if the daemon restarts. See{" "}
        <a href="/blog/tmux-vs-hive-for-ai-coding-agents/">tmux or Hive</a> for a side-by-side.
      </p>

      <h2 id="ssh">Option 4: an always-on machine over SSH</h2>
      <p>
        None of the above survives a reboot, and a sleeping laptop pauses everything on it. For
        agents that run overnight, put them on a machine that stays on (a desktop, a home server or
        a cloud VM), run tmux or Hive there, and connect over SSH. Dropped connections no longer
        matter: reconnect and reattach. If your network is flaky, <code>mosh</code> keeps the SSH
        side of things alive through roaming and sleep.
      </p>

      <h2 id="recover">If the agent already died</h2>
      <p>
        The process is gone, but the conversation usually isn&apos;t. Claude Code can pick up where
        it left off with <code>claude --continue</code> (the most recent conversation in this
        directory) or <code>claude --resume</code> (choose one). Codex has{" "}
        <code>codex resume</code>. Changes the agent already made are on disk; check{" "}
        <code>git status</code> before you start it again.
      </p>

      <h2 id="summary">Which to pick</h2>
      <ul>
        <li>
          <strong>One agent, already use tmux:</strong> tmux.
        </li>
        <li>
          <strong>A narrow task with no prompts:</strong> headless mode.
        </li>
        <li>
          <strong>Several agents, several projects:</strong> Hive, so you can tell which one needs
          you when you come back.
        </li>
        <li>
          <strong>Overnight runs:</strong> any of these on an always-on machine.
        </li>
      </ul>
    </>
  );
}
