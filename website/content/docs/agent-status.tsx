import { agents } from "@/lib/features";

export default function AgentStatus() {
  const withRules = agents.filter((a) => a.status);
  const without = agents.filter((a) => !a.status);
  return (
    <>
      <p>
        Hive shows what each agent is doing: <strong>working</strong>,{" "}
        <strong>needs input</strong> (a permission prompt or a question), <strong>done</strong>{" "}
        (finished while you were elsewhere) or <strong>idle</strong>. It never guesses: an agent
        it has no rules for shows no status rather than a wrong one, and Hive never answers an
        agent&apos;s prompts for you.
      </p>

      <h2 id="how">How Hive knows</h2>
      <ol>
        <li>
          <strong>Which agent.</strong> The daemon looks at the program in the foreground of each
          terminal and recognises agents by their command line, also through launchers such as{" "}
          <code>node</code>, <code>npx</code>, <code>uvx</code> or <code>sh -c</code>. So an agent
          you type into a shell is recognised as well as one started with <code>hive new</code>.
          An unrecognised program is checked again every two seconds, in case a launcher starts the
          agent a moment later.
        </li>
        <li>
          <strong>What it is doing.</strong> Per-agent screen rules read the agent&apos;s screen:
          a spinner or &ldquo;esc to interrupt&rdquo; means working, an approval prompt means
          needs input. The rules are shared with{" "}
          <a href="https://herdr.dev">herdr</a>. To avoid flicker, an agent has to look idle for a
          second before Hive reports it idle.
        </li>
        <li>
          <strong>Done</strong> is Hive&apos;s own: an agent that goes from working to idle while
          you aren&apos;t viewing it is marked done until you open it.
        </li>
      </ol>
      <p>
        Detection runs in the daemon, so status keeps updating while no window is open, and every
        window sees the same thing. <code>hive list</code> shows it too.
      </p>

      <h2 id="agents">Recognised agents</h2>
      <p>
        With status rules ({withRules.length}): {withRules.map((a) => a.name).join(", ")}.
      </p>
      <p>
        Recognised and launchable, without status rules ({without.length}):{" "}
        {without.map((a) => a.name).join(", ")}. They can report their own status (below).
      </p>
      <p>
        <code>hive agents</code> lists the agents on your machine; <code>hive agents -a</code>{" "}
        lists all of them. Any other program can be added under{" "}
        <a href="/docs/configuration/#agents">[agents.&lt;name&gt;]</a>.
      </p>

      <h2 id="notify">Getting your attention</h2>
      <ul>
        <li>
          The sidebar marks the agent, and its space, as soon as it needs input or is done.
        </li>
        <li>
          The terminal bell rings when an agent you aren&apos;t viewing needs input or finishes.
          Many terminals turn the bell into a flash, a sound or an urgent window.{" "}
          <code>[notifications] bell = false</code> turns it off.
        </li>
        <li>
          With <a href="/integrations/#desktop-notifications">desktop notifications</a> on, Hive
          also runs <code>notify-send</code> (or your own command) with a title and message.
        </li>
        <li>
          With the <a href="/integrations/#obsidian">Obsidian journal</a> on, each finished turn is
          logged in today&apos;s daily note.
        </li>
      </ul>

      <h2 id="report-agent">Reporting status yourself: report-agent</h2>
      <p>
        An agent without rules, a wrapper script or your own tool can tell Hive its state. Every
        session gets two environment variables: <code>HIVE_SESSION_ID</code> and{" "}
        <code>HIVE_BIN_PATH</code> (the <code>hive</code> binary next to the daemon).
      </p>
      <pre className="code">
        <code>{`"$HIVE_BIN_PATH" report-agent --agent mybot --state working
"$HIVE_BIN_PATH" report-agent --agent mybot --state blocked --message 'approve the migration?'
"$HIVE_BIN_PATH" report-agent --agent mybot --state idle
"$HIVE_BIN_PATH" release-agent --agent mybot   # hand back to screen detection`}</code>
      </pre>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Rule</th>
            <th scope="col">Detail</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>States</td><td><code>idle</code>, <code>working</code>, <code>blocked</code> (shown as needs input) or <code>unknown</code></td></tr>
          <tr><td>Precedence</td><td>A report overrides screen detection for that session until <code>release-agent</code></td></tr>
          <tr><td>Lifetime</td><td>The report lapses when the foreground program changes, so an agent that exits without releasing doesn&apos;t leave a stale status</td></tr>
          <tr><td>Ordering</td><td><code>--seq N</code> numbers reports; one not above the last is ignored. The default is the current time in milliseconds</td></tr>
          <tr><td>Limits</td><td>Agent name up to 64 characters, message up to 200</td></tr>
          <tr><td>Session</td><td>Defaults to <code>$HIVE_SESSION_ID</code>; pass an id to report for another session</td></tr>
        </tbody>
      </table>
      <p>
        For example, a Claude Code hook can report <code>blocked</code> from a Notification hook
        and <code>idle</code> from a Stop hook, though Hive already recognises Claude Code without
        one.
      </p>
    </>
  );
}
