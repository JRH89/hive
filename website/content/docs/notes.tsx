export default function Notes() {
  return (
    <>
      <p>
        Hive has its own notes app and a context store for your agents, built on a plain markdown
        vault. It works alongside Obsidian: same files, nothing to import, and Obsidian doesn&apos;t
        need to be installed.
      </p>

      <h2 id="vault">The vault</h2>
      <p>
        A vault is a folder of <code>.md</code> files. Hive uses the folder with{" "}
        <code>.obsidian/</code> at or above where you are, within the repository, or one directly
        inside the repository root (<code>vault/</code>, <code>docs/</code>, …). Set{" "}
        <code>HIVE_VAULT</code> to use another one. Every session started in a space with a vault
        gets <code>HIVE_VAULT</code> set, so agents and scripts can find it.
      </p>

      <h2 id="app">The notes app</h2>
      <pre className="code">
        <code>{`╭ Notes · vault ─────────╮╭ ADR-0012 Worktrees ──────────────────────────╮
│▾ Decisions             ││ ADR-0012: Worktrees created by the daemon    │
│    ADR-0011 Workspaces ││                                              │
│    ADR-0012 Worktrees  ││ Context                                      │
│▾ Log                   ││ Phase 4 requires optional worktree isolation │
│    2026-11-02          ││ …                                            │
│  Home                  ││ Linked from                                  │
│  Status                ││ • Home  • Roadmap                3 links · 40%│
╰────────────────────────╯╰──────────────────────────────────────────────╯
 ⏎ open  / search  n new  t today  e edit  o obsidian  ? keys  q quit`}</code>
      </pre>
      <p>
        With <code>[integrations.obsidian] enabled = true</code>, <kbd>Ctrl+Space</kbd>{" "}
        <kbd>N</kbd> opens it beside the focused pane. You can also run <code>hive notes</code> in
        any terminal, or <code>hive notes DIR</code> for a folder of your choice. The note under the
        cursor shows on the right as you move, and notes changed elsewhere (in Obsidian, by an
        agent) reload on their own.
      </p>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">Key</th>
            <th scope="col">Does</th>
          </tr>
        </thead>
        <tbody>
          <tr><td><kbd>j</kbd> <kbd>k</kbd> <kbd>↑</kbd> <kbd>↓</kbd></td><td>Move in the tree, or scroll the note</td></tr>
          <tr><td><kbd>Enter</kbd> <kbd>→</kbd></td><td>Open a note, or fold or unfold a folder</td></tr>
          <tr><td><kbd>Esc</kbd> <kbd>←</kbd></td><td>Back to the tree; fold a folder</td></tr>
          <tr><td><kbd>Tab</kbd> <kbd>Shift+Tab</kbd></td><td>Next or previous link in the note, including the &quot;Linked from&quot; list</td></tr>
          <tr><td><kbd>Enter</kbd> on a link</td><td>Follow it. A link to a note that doesn&apos;t exist creates it, as in Obsidian. Web links open in your browser</td></tr>
          <tr><td><kbd>Backspace</kbd></td><td>Back to the previous note</td></tr>
          <tr><td><kbd>/</kbd></td><td>Search every note; the preview follows the result you pick</td></tr>
          <tr><td><kbd>n</kbd></td><td>New note in the selected folder (<code>Folder/Name</code> for another)</td></tr>
          <tr><td><kbd>t</kbd></td><td>Today&apos;s daily note, in <code>journal_dir</code></td></tr>
          <tr><td><kbd>e</kbd></td><td>Edit the note in <code>$VISUAL</code> or <code>$EDITOR</code> (else nvim, vim, vi or nano)</td></tr>
          <tr><td><kbd>o</kbd></td><td>Open the note in the Obsidian app</td></tr>
          <tr><td><kbd>?</kbd> <kbd>q</kbd></td><td>All keys; quit</td></tr>
        </tbody>
      </table>
      <p>
        It renders headings, lists, checkboxes, quotes and callouts, code, tables and{" "}
        <code>[[wikilinks]]</code> (with <code>#heading</code> and <code>|alias</code>). To open
        the vault with something else instead, set <code>agent</code> under{" "}
        <code>[integrations.obsidian]</code> to an <code>[agents.*]</code> entry.
      </p>

      <h2 id="context">Context for agents</h2>
      <p>
        <code>hive context</code> lets agents look things up in the vault and record what they
        decide. It reads the files directly, with no daemon and no index to keep fresh, and prints{" "}
        <code>path:line</code> so an agent can open the exact spot. Add <code>--json</code> for
        programs.
      </p>
      <pre className="code">
        <code>{`$ hive context search how are dirty worktrees handled
Decisions/ADR-0012 Worktrees created by the daemon, never deleted dirty.md:5
    # ADR-0012: Worktrees created by the daemon, never deleted dirty
Roadmap/Phase 4 - Workspaces and Git.md:17
    | Safe cleanup | … Dirty worktree: TUI said "Kept worktree…

$ hive context get ADR-0012          # the whole note, its links and backlinks
$ hive context list -n 10            # recently changed notes
$ hive context add Hive/Decisions "Tokens expire after 1h; refresh in the client"`}</code>
      </pre>
      <p>
        <code>add</code> appends a dated entry (signed with the session, and{" "}
        <code>--by NAME</code> if given) and never rewrites a note. Search ranks file names, tags
        and exact phrases above passing mentions, and matches plurals.
      </p>
      <p>
        To have your agents use it, add a line like this to the project&apos;s{" "}
        <code>CLAUDE.md</code> or <code>AGENTS.md</code>:
      </p>
      <pre className="code">
        <code>{`Project knowledge lives in the notes vault. Before non-trivial work, run
\`hive context search <topic>\` and read the relevant notes with \`hive context get\`.
Record decisions with \`hive context add Hive/Decisions "<what and why>"\`.`}</code>
      </pre>
    </>
  );
}
