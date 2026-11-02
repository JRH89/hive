import { cliCommands } from "@/lib/docs-cli";

/** The command words of a usage line: "hive worktree rm <PATH>" -> hive worktree rm. */
const words = (usage: string) => {
  const all = usage.split(" ");
  const end = all.findIndex((w) => !/^[a-z][a-z-]*$/.test(w));
  return end === -1 ? all : all.slice(0, end);
};
const id = (usage: string) => words(usage).join("-");

export default function CliDoc() {
  return (
    <>
      <p>
        <code>hive</code> is both the interface and a command line. Every command talks to the
        daemon, starting it if needed, so sessions you create from a script show up in the
        interface and the other way round. <code>hive --help</code> and{" "}
        <code>hive &lt;command&gt; --help</code> print the same information.
      </p>
      <p>
        <code>SESSION</code> is a session id, as shown by <code>hive list</code> and in the
        sidebar. <code>WORKSPACE</code> is a workspace name or id.
      </p>

      <nav aria-label="Commands" className="toc">
        {cliCommands.map((c) => (
          <a key={c.usage} href={`#${id(c.usage)}`}>
            {words(c.usage).join(" ")}
          </a>
        ))}
      </nav>

      {cliCommands.map((c) => (
        <section key={c.usage}>
          <h2 id={id(c.usage)}>
            <code>{c.usage}</code>
          </h2>
          <p>{c.summary}</p>
          {c.options ? (
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">Option</th>
                  <th scope="col">Meaning</th>
                </tr>
              </thead>
              <tbody>
                {c.options.map(([flag, meaning]) => (
                  <tr key={flag}>
                    <td>
                      <code>{flag}</code>
                    </td>
                    <td>{meaning}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : null}
          {c.notes ? <p>{c.notes}</p> : null}
          {c.examples ? (
            <pre className="code">
              <code>{c.examples}</code>
            </pre>
          ) : null}
        </section>
      ))}

      <h2 id="exit-codes">Exit codes and scripting</h2>
      <p>
        Commands exit with 0 on success and non-zero with a message on stderr otherwise.{" "}
        <code>hive new -d</code> prints <code>created session 4 (codex) in /path</code>, so a
        script can start several agents in the background and open them later with{" "}
        <code>hive</code> or <code>hive attach 4</code>.
      </p>
    </>
  );
}
