export default function Post() {
  return (
    <>
      <p>
        A coding agent that stops for a permission prompt does nothing until you notice. Most of
        the time lost with agents isn&apos;t the agent being slow; it is the agent sitting at
        &ldquo;Allow this command?&rdquo; in a tab you aren&apos;t looking at. Here is how to get
        a notification when Claude Code or Codex needs you, or has finished.
      </p>

      <h2 id="claude-code">Claude Code: hooks</h2>
      <p>
        Claude Code runs commands on events you choose, configured as hooks in a settings file:{" "}
        <code>~/.claude/settings.json</code> for every project, or{" "}
        <code>.claude/settings.json</code> in one repository. Two events matter here:
      </p>
      <ul>
        <li>
          <strong>Notification</strong> fires when Claude Code sends a notification. Its matcher
          picks the type: <code>permission_prompt</code> for permission requests,{" "}
          <code>idle_prompt</code> when it is waiting for your input. Leave the matcher out to
          catch every notification.
        </li>
        <li>
          <strong>Stop</strong> fires when Claude finishes responding.
        </li>
      </ul>
      <p>On Linux, with <code>notify-send</code>:</p>
      <pre className="code">
        <code>{`{
  "hooks": {
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          { "type": "command", "command": "notify-send 'Claude Code' 'Needs permission'" }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "notify-send 'Claude Code' 'Finished'" }
        ]
      }
    ]
  }
}`}</code>
      </pre>
      <p>
        On macOS, swap the command for{" "}
        <code>osascript -e &apos;display notification &quot;Finished&quot; with title &quot;Claude Code&quot;&apos;</code>
        . A hook receives details about the event as JSON on standard input, so a small script
        can include the project name in the message. The{" "}
        <a href="https://code.claude.com/docs/en/hooks">hooks reference</a> lists every event and
        matcher.
      </p>

      <h2 id="codex">Codex: notify and TUI notifications</h2>
      <p>
        Codex has two settings in <code>~/.codex/config.toml</code>. The simple one asks the
        terminal itself to notify you:
      </p>
      <pre className="code">
        <code>{`[tui]
notifications = true                 # or a list of event types
notification_condition = "unfocused" # or "always"
notification_method = "auto"         # auto, osc9 or bel`}</code>
      </pre>
      <p>
        That uses an OSC 9 desktop notification or the terminal bell, depending on what your
        terminal supports. For a custom action, <code>notify</code> names a program that Codex
        runs with a JSON description of the event:
      </p>
      <pre className="code">
        <code>{`notify = ["python3", "/home/me/bin/codex-notify.py"]`}</code>
      </pre>
      <p>
        Check the <a href="https://learn.chatgpt.com/docs/config-file/config-reference">Codex
        configuration reference</a> for the event types your version sends.
      </p>

      <h2 id="limits">Where per-agent hooks fall short</h2>
      <p>Hooks work, but they get awkward once you run several agents:</p>
      <ul>
        <li>Each agent needs its own setup, in its own format, on every machine.</li>
        <li>
          A notification tells you <em>something</em> needs you, not which of five sessions, and
          getting there means hunting through tabs.
        </li>
        <li>Agents without hooks give you nothing.</li>
      </ul>

      <h2 id="hive">One setup for every agent with Hive</h2>
      <p>
        Hive watches every agent&apos;s screen itself, using per-agent rules for Claude Code, Codex,
        Gemini CLI, OpenCode and about twenty more, so it needs no setup inside the agents. Its
        sidebar shows each agent as working, needs input or done, across every project, and the
        terminal bell rings when an agent you aren&apos;t viewing needs you or finishes. For desktop
        notifications, add two lines:
      </p>
      <pre className="code">
        <code>{`# ~/.config/hive/config.toml
[integrations.desktop_notify]
enabled = true`}</code>
      </pre>
      <p>
        By default that runs <code>notify-send</code>; set <code>command</code> to anything else
        (Hive passes the title and body as its last two arguments). When the notification arrives,{" "}
        <kbd>Alt+1</kbd>–<kbd>9</kbd> jumps straight to the agent, whichever project it is in.
        Scripts and agents without rules can report their state with{" "}
        <code>hive report-agent</code>.
      </p>
      <p>
        You can use both: keep a Stop hook that runs your tests, and let Hive handle the
        &ldquo;come back, I need you&rdquo; part for every agent at once. See{" "}
        <a href="/features/#agent-status">live agent status</a> and{" "}
        <a href="/integrations/#desktop-notifications">desktop notifications</a>.
      </p>
    </>
  );
}
