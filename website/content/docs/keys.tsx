import { keyGroups } from "@/lib/docs-keys";

const pretty = (k: string) =>
  k
    .replace(/\bdown\b/, "↓")
    .replace(/\bup\b/, "↑")
    .replace(/\bleft\b/, "←")
    .replace(/\bright\b/, "→")
    .replace(/\btab\b/, "Tab")
    .replace(/\balt\+/, "Alt+")
    .replace(/\bctrl\+/, "Ctrl+");

const Keys = ({ keys }: { keys: string[] }) =>
  keys.length === 0 ? (
    <span className="muted">palette</span>
  ) : (
    <>
      {keys.map((k, i) => (
        <span key={k}>
          {i > 0 ? " " : ""}
          <kbd>{pretty(k)}</kbd>
        </span>
      ))}
    </>
  );

export default function KeysDoc() {
  return (
    <>
      <p>
        Everything you type goes to the agent, except the <strong>leader</strong> key,{" "}
        <kbd>Ctrl+Space</kbd>, and the one key you press after it. Press the leader, let go, then
        press the command key. Keys are case-sensitive: <kbd>n</kbd> is the next session,{" "}
        <kbd>N</kbd> (Shift+n) opens notes. To send <kbd>Ctrl+Space</kbd> itself to the agent,
        press it twice.
      </p>
      <p>
        In Hive, <kbd>Ctrl+Space</kbd> <kbd>?</kbd> shows your current bindings, and{" "}
        <kbd>Ctrl+Space</kbd> <kbd>:</kbd> finds any command by name, including those without a
        key (shown as <em>palette</em> below).
      </p>

      {keyGroups.map((g) => (
        <section key={g.title}>
          <h2 id={g.title.toLowerCase().replace(/[^a-z]+/g, "-")}>{g.title}</h2>
          <table className="table">
            <thead>
              <tr>
                <th scope="col">After Ctrl+Space</th>
                <th scope="col">Action</th>
                <th scope="col">Name</th>
              </tr>
            </thead>
            <tbody>
              {g.commands.map((c) => (
                <tr key={c.name}>
                  <td>
                    {c.global && c.keys.length === 0 ? null : <Keys keys={c.keys} />}
                    {c.global ? (
                      <>
                        {c.keys.length > 0 ? " or " : ""}
                        <kbd>{pretty(c.global[0])}</kbd> <span className="muted">no leader</span>
                      </>
                    ) : null}
                  </td>
                  <td>
                    {c.title}
                    {c.needs ? <span className="muted"> (needs {c.needs})</span> : null}
                  </td>
                  <td>
                    <code>{c.name}</code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ))}

      <h2 id="jump">Jumping to an agent</h2>
      <p>
        <kbd>Ctrl+Space</kbd> <kbd>1</kbd>–<kbd>9</kbd>, or <kbd>Alt+1</kbd>–<kbd>Alt+9</kbd>{" "}
        without the leader, opens an agent by its number in the sidebar&apos;s AGENTS list,
        switching space if needed. The digits are reserved and can&apos;t be rebound.
      </p>

      <h2 id="sidebar">In the sidebar</h2>
      <p>
        After <kbd>Ctrl+Space</kbd> <kbd>s</kbd>: <kbd>↑</kbd> <kbd>↓</kbd> (or <kbd>j</kbd>{" "}
        <kbd>k</kbd>) move, <kbd>Enter</kbd> opens (or relaunches a lost session), <kbd>n</kbd>{" "}
        opens a new terminal, <kbd>x</kbd> terminates, <kbd>r</kbd> renames, <kbd>w</kbd> opens
        workspaces, <kbd>p</kbd> the palette, <kbd>/</kbd> session search, <kbd>?</kbd> help,{" "}
        <kbd>q</kbd> detaches and <kbd>Esc</kbd> or <kbd>Tab</kbd> goes back to the terminal.
      </p>

      <h2 id="rebinding">Changing keys</h2>
      <p>
        Bindings live in <code>~/.config/hive/config.toml</code>. Use the names from the tables
        above. Setting a command replaces its default keys; an empty list unbinds it, and it stays
        in the palette.
      </p>
      <pre className="code">
        <code>{`[keys]
leader = "ctrl+a"          # needs Ctrl, Alt or an F-key
jump_modifier = "alt"      # Alt+1-9; "ctrl+alt", or "" for off

[keys.bind]                # after the leader
new_session = "n"
add_workspace = ["W", "f2"]
kill_session = []          # unbind

[keys.global]              # without the leader; needs Ctrl or Alt
next_tab = "ctrl+alt+right"
prev_workspace = []        # give Alt+Up back to the agent`}</code>
      </pre>
      <p>
        If <kbd>Ctrl+Space</kbd> does nothing, your desktop or input method probably uses it;
        choose another leader. Hive refuses a config with clashing keys and says which, and{" "}
        <code>hive doctor</code> checks the file without opening the interface.
      </p>
    </>
  );
}
