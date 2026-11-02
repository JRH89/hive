import { configSections } from "@/lib/docs-config";

export default function ConfigurationDoc() {
  return (
    <>
      <p>
        Hive reads <code>~/.config/hive/config.toml</code> (or the file named by{" "}
        <code>$HIVE_CONFIG</code>) when <code>hive</code> starts. Every key is optional and a
        missing file means all defaults. Unknown keys are an error, so typos don&apos;t pass
        silently. After editing, run <code>hive doctor</code> to validate the file, then restart{" "}
        <code>hive</code>; running agents are not affected.
      </p>
      <p>
        Integrations are the exception: <kbd>Ctrl+Space</kbd> <kbd>I</kbd> switches them on and
        off at once and writes <code>enabled</code> for you, keeping your comments.{" "}
        <a href="https://github.com/jrh89/hive/blob/master/examples/config.toml">
          examples/config.toml
        </a>{" "}
        is a complete, commented file to start from.
      </p>

      <nav aria-label="Sections" className="toc">
        {configSections.map((s) => (
          <a key={s.id} href={`#${s.id}`}>
            {s.table}
          </a>
        ))}
      </nav>

      {configSections.map((s) => (
        <section key={s.id}>
          <h2 id={s.id}>
            <code>{s.table}</code>
          </h2>
          <p>
            {s.intro}
            {s.id === "integrations" ? (
              <>
                {" "}
                See <a href="/integrations/">integrations</a>.
              </>
            ) : null}
            {s.id === "keys" ? (
              <>
                {" "}
                See <a href="/docs/keys/">keys</a>.
              </>
            ) : null}
          </p>
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Key</th>
                <th scope="col">Type</th>
                <th scope="col">Default</th>
                <th scope="col">Meaning</th>
              </tr>
            </thead>
            <tbody>
              {s.options.map((o) => (
                <tr key={o.key}>
                  <td>
                    <code>{o.key.replace(/^[a-z_]+\.(?=[^.]+$)/, "")}</code>
                  </td>
                  <td>{o.type}</td>
                  <td>
                    <code>{o.default}</code>
                  </td>
                  <td>{o.meaning}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ))}

      <h2 id="examples">Examples</h2>
      <h3>Your own agent, and a variant of a built-in</h3>
      <pre className="code">
        <code>{`[agents.aider]
command = "aider"
args = ["--dark-mode"]
env = { AIDER_AUTO_COMMITS = "false" }

[agents.claude-opus]       # shows up as its own agent
command = "claude"
args = ["--model", "opus"]`}</code>
      </pre>
      <p>
        Agents start with the environment of the <code>hive</code> command that launched them,
        plus <code>env</code>. Hive recognises a custom agent&apos;s status when its command is a
        known agent (here, Claude Code); otherwise see{" "}
        <a href="/docs/agent-status/#report-agent">report-agent</a>.
      </p>
      <h3>A layout for a web project</h3>
      <pre className="code">
        <code>{`[layouts.web]
spaces = ["web"]           # opens without asking in the "web" space
panes = [
  { agent = "claude" },
  { run = ["npm", "run", "dev"], split = "right", label = "server" },
  { split = "down" },      # a terminal below the server
]`}</code>
      </pre>
      <p>
        A layout with no panes or an unknown agent stops <code>hive</code> from starting, with
        the reason.
      </p>
      <h3>Light terminal, no mouse, quiet</h3>
      <pre className="code">
        <code>{`[general]
theme = "hive-light"

[terminal]
mouse_support = false

[notifications]
bell = false`}</code>
      </pre>
    </>
  );
}
