export default function Orchestration() {
  return (
    <>
      <p>
        Hive can coordinate multiple agents across named roles and a shared markdown task board,
        either from the CLI, from the interactive board inside Hive, or through a local stdio MCP
        server. Everything stays on disk in your repository and runs locally.
      </p>

      <h2 id="setup">1. Configure roles with hive setup</h2>
      <p>
        Run <code>hive setup</code> to detect which agent CLIs are installed on your{" "}
        <code>PATH</code> and interactively write <code>[roles.*]</code> entries to{" "}
        <code>config.toml</code>. Built-in defaults already exist for <code>lead</code>,{" "}
        <code>planner</code>, <code>coder</code> (with <code>worktree = true</code>), and{" "}
        <code>reviewer</code>.
      </p>
      <pre className="code">
        <code>{`hive setup           # interactive role & agent wizard
hive roles           # list configured roles and routing strengths
hive roles --json    # machine-readable output for scripts`}</code>
      </pre>
      <p>
        Each role in <code>config.toml</code> maps a name to an agent, optional arguments, worktree
        isolation, routing <code>strengths</code>, and default <code>instructions</code>:
      </p>
      <pre className="code">
        <code>{`[roles.coder]
agent = "codex"
args = []
worktree = true
strengths = ["implementation", "refactoring", "tests", "bugfix"]
instructions = "Implement the assigned task on your worktree branch and run tests."`}</code>
      </pre>
      <p>
        Passing <code>--role auto</code> to <code>hive task add</code> or <code>hive spawn</code>{" "}
        matches the task title or prompt against each role&apos;s <code>strengths</code> and picks
        the best fit.
      </p>

      <h2 id="board">2. Shared task board</h2>
      <p>
        Manage tasks from the CLI or open the interactive Kanban board with <code>hive board</code>{" "}
        (or <kbd>Ctrl+Space</kbd> <kbd>B</kbd> inside Hive):
      </p>
      <pre className="code">
        <code>{`hive task add "Fix auth race" --role auto --body "Reproduce with concurrent refresh"
hive task list --ready
hive task show 1
hive task update 1 --status in_progress --note "Assigned to coder"
hive board`}</code>
      </pre>
      <p>Inside the board:</p>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Key</th>
            <th scope="col">Action</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><kbd>n</kbd></td>
            <td>Create a new task</td>
          </tr>
          <tr>
            <td><kbd>a</kbd></td>
            <td>Assign or change a task&apos;s role</td>
          </tr>
          <tr>
            <td><kbd>s</kbd></td>
            <td>Spawn an agent on the selected task (uses the role&apos;s worktree setting)</td>
          </tr>
          <tr>
            <td><kbd>m</kbd> / <kbd>Enter</kbd></td>
            <td>Advance the task&apos;s status</td>
          </tr>
        </tbody>
      </table>

      <h2 id="spawn">3. Spawn, message, and wait</h2>
      <p>
        Drive specialist workers directly from scripts or from a lead agent&apos;s terminal:
      </p>
      <pre className="code">
        <code>{`hive spawn --role coder --task 1 -w         # launch coder in its own worktree
hive spawn --role auto --prompt "Review PR" # route automatically by strengths
hive msg 4 "Focus on token refresh"         # send a message to session 4
hive wait 4 --timeout 300                   # block until session 4 finishes or is blocked`}</code>
      </pre>

      <h2 id="mcp">4. MCP server (hive mcp)</h2>
      <p>
        Run <code>hive mcp</code> to expose a local stdio MCP server with 9 orchestration tools to
        any MCP-compatible agent (such as Claude Code, Codex, or Gemini CLI):
      </p>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Tool</th>
            <th scope="col">What it does</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><code>task_list</code></td>
            <td>List tasks on the workspace board (filter by status, role, or ready)</td>
          </tr>
          <tr>
            <td><code>task_show</code></td>
            <td>Read a task&apos;s full description, metadata, and activity log</td>
          </tr>
          <tr>
            <td><code>task_add</code></td>
            <td>Add a task (supports <code>role: &quot;auto&quot;</code> and dependencies)</td>
          </tr>
          <tr>
            <td><code>task_update</code></td>
            <td>Update status, role, linked session/branch, or append a log note</td>
          </tr>
          <tr>
            <td><code>agent_spawn</code></td>
            <td>Spawn a role-based worker session (with optional worktree and prompt)</td>
          </tr>
          <tr>
            <td><code>agent_message</code></td>
            <td>Send a message to a running session</td>
          </tr>
          <tr>
            <td><code>agent_wait</code></td>
            <td>Wait for a session to finish or report blocked</td>
          </tr>
          <tr>
            <td><code>session_list</code></td>
            <td>List active Hive sessions with state, workspace, and branch</td>
          </tr>
          <tr>
            <td><code>role_list</code></td>
            <td>List configured orchestration roles, models, and routing strengths</td>
          </tr>
        </tbody>
      </table>
    </>
  );
}
