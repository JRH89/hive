import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { Icon } from "@/components/icon";

export const metadata: Metadata = {
  title: { absolute: "Why Hive: compared with tmux, Zellij and herdr" },
  description:
    "What sets Hive apart from tmux, Zellij and herdr: agent status across projects, one-key worktrees, spaces that follow cd, and built-in integrations.",
  alternates: { canonical: "/why/" },
};

const Arrow = () => <Icon name="arrow" size={17} />;

const reasons = [
  {
    title: "Built for agents, not just terminals.",
    text: "tmux and Zellij are superb general multiplexers, and Hive keeps what makes them great: sessions live in a background server and survive a closed terminal or a dropped SSH connection. Then it adds what running agents needs. Hive recognises Claude Code, Codex, Gemini CLI, OpenCode and twenty more, even when started by hand in a terminal, and shows which one is working, which one needs you, and which one is done.",
  },
  {
    title: "Every agent, every project, one list.",
    text: "The sidebar lists every agent across all your spaces with its status, so an agent waiting for approval in another repository can't hide. Alt+1–9 jumps straight to it.",
  },
  {
    title: "Spaces that follow you.",
    text: "There is nothing to set up. Open a terminal and cd into a repository: Hive gives that repository a space with its own tabs. cd somewhere else and the terminal moves with you.",
  },
  {
    title: "Isolation in one key.",
    text: "Two agents in one checkout overwrite each other's work. Ctrl+Space C opens a terminal in its own git worktree on a fresh branch. Hive warns when agents share a directory and never deletes a worktree with uncommitted changes.",
  },
  {
    title: "Your tools, beside your agents.",
    text: "Optional integrations put the rest of your workflow next to the work: your Obsidian vault and a daily journal of what your agents did, lazygit review, GitHub pull request checks, project tasks, git status per space, token usage and desktop notifications. All off by default and invisible until you turn them on.",
  },
  {
    title: "Runs where you do.",
    text: "Static binaries for Linux, macOS and Windows, a one-line installer, hive update. It works in 16 colours, with NO_COLOR, in plain ASCII without Nerd Fonts, and needs no account, browser or cloud.",
  },
];

type Cell = "yes" | "no" | string;

const rows: [string, Cell, Cell][] = [
  ["Sessions survive closing the terminal or SSH", "yes", "yes"],
  ["Knows each agent's status: working, needs input, done", "yes", "Not built in"],
  ["Lists every agent across projects in one place", "yes", "Not built in"],
  ["Groups work by repository, following cd", "yes", "Not built in"],
  ["New terminal in its own git worktree, one key", "yes", "Not built in"],
  ["Warns when two agents share a directory", "yes", "no"],
  ["Obsidian notes and a journal of agent activity", "yes", "no"],
  ["Review, PR checks, tasks and usage beside each agent", "yes", "Scriptable"],
  ["Right-click menus, clickable sidebar and tabs", "yes", "Varies"],
];

function Mark({ value }: { value: Cell }) {
  if (value === "yes") {
    return (
      <span className="compare-yes">
        <Icon name="check" size={16} /> Yes
      </span>
    );
  }
  if (value === "no") return <span className="compare-no">No</span>;
  return <span className="compare-no">{value}</span>;
}

export default function Why() {
  return (
    <>
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper why">
        <Breadcrumbs trail={[["Why Hive", "/why/"]]} />
        <div className="section-heading">
          <div>
            <p className="eyebrow">WHAT SETS HIVE APART</p>
            <h1 className="why-title">
              A multiplexer that
              <br />
              knows your agents.
            </h1>
          </div>
          <p className="section-aside">
            Fast, local, open source.
            <br />
            Built for people who run many agents.
          </p>
        </div>

        <div className="why-grid">
          {reasons.map((r, i) => (
            <article className="why-reason" key={r.title}>
              <span className="why-number">{String(i + 1).padStart(2, "0")}</span>
              <h2>{r.title}</h2>
              <p>{r.text}</p>
            </article>
          ))}
        </div>

        <h2 className="why-subtitle">Hive and general multiplexers</h2>
        <div className="compare-scroll">
          <table className="compare">
            <thead>
              <tr>
                <th scope="col" />
                <th scope="col">Hive</th>
                <th scope="col">tmux, Zellij</th>
              </tr>
            </thead>
            <tbody>
              {rows.map(([label, hive, other]) => (
                <tr key={label}>
                  <th scope="row">{label}</th>
                  <td>
                    <Mark value={hive} />
                  </td>
                  <td>
                    <Mark value={other} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="why-note">
          tmux and Zellij can be extended with scripts and plugins; “not built in” means you
          would assemble it yourself.
        </p>

        <h2 className="why-subtitle">Hive and herdr</h2>
        <div className="why-prose">
          <p>
            herdr showed what an agent multiplexer should feel like, and Hive is glad to share
            its approach: both keep agents in a background server and turn many terminals into
            one view of who needs you. Hive even reads agent status with herdr-compatible rules
            (Apache-2.0, credited).
          </p>
          <p>
            herdr goes further on reaching your agents from elsewhere: clients over SSH and on
            your phone, a socket API and a plugin marketplace. Hive bets on a complete local
            workspace out of the box: integrations built in, worktree isolation in one key,
            spaces that follow your cd, native Windows, and an MIT licence.
          </p>
        </div>

        <div className="why-cta">
          <a className="button button-primary" href="/#get-started">
            Get started with Hive <Arrow />
          </a>
          <a className="text-link" href="/changelog/">
            See what&apos;s new <Arrow />
          </a>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
