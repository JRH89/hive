import { SiteHeader } from "@/components/site-header";
import { Brand } from "@/components/brand";
import { CopyCommand } from "@/components/copy-command";
import { Icon } from "@/components/icon";
import { TerminalShowcase } from "@/components/terminal-showcase";
import { HiveBackdrop } from "@/components/hive-backdrop";
import { ReleaseInstall } from "@/components/release-install";

const siteUrl = "https://hive.codefrontlabs.com/";
const repoUrl = "https://github.com/jrh89/hive";

const Arrow = () => <Icon name="arrow" size={17} />;

const features = [
  {
    number: "01",
    symbol: "terminal" as const,
    title: "Your agents. As they are.",
    text: "Claude Code, Codex, Gemini CLI, OpenCode, truss and twenty more. Hive recognises the agent, even one started from a shell, and shows whether it is working, needs you, or is done. Its prompts and permissions stay intact.",
    foot: "REAL PTYS · LIVE AGENT STATUS",
  },
  {
    number: "02",
    symbol: "layers" as const,
    title: "Leave the terminal. Keep the session.",
    text: "The daemon owns the process. Detach from Hive or close its frontend, and your agent stays running. Open Hive again to pick up the live session.",
    foot: "DETACH · REATTACH · CONTINUE",
  },
  {
    number: "03",
    symbol: "keyboard" as const,
    title: "Terminal first.",
    text: "Click + new and you get a terminal. Type claude or codex to start an agent, cd anywhere first. A leader key handles Hive commands, tmux-style splits and tabs keep several agents in view, and the mouse works too.",
    foot: "CTRL+SPACE · SPLITS · TABS",
  },
  {
    number: "04",
    symbol: "branch" as const,
    title: "Every project in its place.",
    text: "Each repository you work in is a space with its own tabs, and a terminal follows you when you cd into another repo. Ctrl+Space C opens a terminal in its own worktree, so two agents never edit the same checkout.",
    foot: "SPACES · BRANCHES · WORKTREES",
  },
  {
    number: "05",
    symbol: "search" as const,
    title: "Find anything in a keystroke.",
    text: "A fuzzy command palette for every action, session and workspace. Rebind any key, pick a theme, and it still works in 16 colours or plain ASCII.",
    foot: "PALETTE · KEYMAPS · THEMES",
  },
  {
    number: "06",
    symbol: "detach" as const,
    title: "Survives the bumps.",
    text: "The interface reconnects on its own if the daemon restarts. Open a session in two windows: one types, the other watches, and taking over is explicit.",
    foot: "RECONNECT · OBSERVE · TAKE OVER",
  },
];

const integrations = [
  {
    name: "Obsidian notes",
    key: "Ctrl+Space N",
    text: "Open your repository's Obsidian vault beside your agents, in an editor or an agent of your choice. With journaling on, Hive appends “claude finished in #3 api (main)” to today's daily note, so your vault keeps a log of what your agents did.",
    config: "[integrations.obsidian]",
  },
  {
    name: "Review changes",
    key: "Ctrl+Space G",
    text: "lazygit, tig or git diff in a split beside the agent, opened in the agent's own directory or worktree. See what it changed without leaving Hive.",
    config: "[integrations.review]",
  },
  {
    name: "Pull request checks",
    key: "Ctrl+Space H",
    text: "Watch the CI checks of the session's branch with the GitHub CLI, live, next to the agent that pushed it.",
    config: "[integrations.github]",
  },
  {
    name: "Project tasks",
    key: "Ctrl+Space T",
    text: "Pick a just, npm, make or cargo task from the project and run it beside the agent: the dev server, the tests, the linter.",
    config: "[integrations.tasks]",
  },
  {
    name: "Desktop notifications",
    key: "automatic",
    text: "A notification when an agent you aren't looking at finishes or needs your input. Walk away; Hive calls you back.",
    config: "[integrations.desktop_notify]",
  },
  {
    name: "Git status",
    key: "sidebar",
    text: "Changed files and commits ahead or behind under every repository space, e.g. main ~3 ↑2. Know which project has uncommitted agent work at a glance.",
    config: "[integrations.git_status]",
  },
  {
    name: "Usage and cost",
    key: "Ctrl+Space U",
    text: "Token usage and estimated cost of your agents in a pane, live, via ccusage or any tool you choose. Agents keep their own logs; Hive puts the numbers beside the work.",
    config: "[integrations.usage]",
  },
  {
    name: "Layouts",
    key: "Ctrl+Space L",
    text: "Name a set of panes once, say Claude, the dev server and a terminal, and open them together in a new tab. A layout can belong to a space and opens there without asking.",
    config: "[layouts.<name>]",
  },
];

const faqs = [
  [
    "Is Hive ready to use?",
    "Yes, on Linux, macOS and Windows. Hive is young but complete: terminals and agents in split panes and tabs, live agent status, detach and reattach, automatic reconnection, spaces that follow your cd, Git worktrees, a command palette, themes and configurable keys. It is tested with real terminals in CI on all three platforms. Version 0.4.9 ships prebuilt binaries and a one-line installer; hive update upgrades in place.",
  ],
  [
    "Which agents can I run?",
    "Hive knows over twenty agents, including Claude Code, Codex, Gemini CLI, OpenCode, Copilot, Cursor Agent and truss, and offers the ones you have installed. You can configure other commands in TOML. Install and authenticate each agent separately; Hive runs its CLI and preserves its permission system.",
  ],
  [
    "What happens when I close Hive?",
    "Closing or detaching the frontend leaves your agent running in the local hived daemon. Reopening Hive reconnects to the live session. Stopping the daemon or rebooting the machine ends those processes; Hive does not recover them after a reboot.",
  ],
  [
    "Do agents step on each other's changes?",
    "Not if you don't let them. Agents don't isolate themselves: two agents in the same directory edit the same files. Press Ctrl+Space C (or run hive new codex -w) to open a terminal in its own Git worktree on a fresh branch, then start the agent there. Hive warns when two agents share a directory, and never removes a worktree with uncommitted changes.",
  ],
  [
    "What are integrations?",
    "Optional extras that connect Hive to tools you already use: Obsidian notes and a daily journal of agent activity, change review with lazygit, GitHub pull request checks, project tasks, desktop notifications, git status in the sidebar and token usage. Each is off by default and invisible until you enable it with two lines in config.toml, and each opens as an ordinary session beside your agents.",
  ],
  [
    "Is Hive open source?",
    "Yes. Hive is MIT-licensed and developed in the open on GitHub at github.com/jrh89/hive.",
  ],
  [
    "Does Hive need a cloud account?",
    "Hive itself runs locally and needs no cloud account, browser, or external service. The agents you choose may have their own accounts, API keys, subscriptions, and network requirements.",
  ],
  [
    "Can I run it on macOS or Windows?",
    "Yes. Hive supports Linux (x86_64, aarch64), macOS (Apple silicon, Intel) and Windows 10+ (x86_64), and CI tests all three. On Windows, agent status comes only from agents that report it with hive report-agent, and spaces don't yet follow cd.",
  ],
];

const structuredData = [
  {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "Hive",
    alternateName: "Hive terminal multiplexer",
    description:
      "An open-source terminal multiplexer for AI coding agents. Run Claude Code, Codex, Gemini CLI and OpenCode side by side in real terminals, managed by a local daemon that keeps sessions running when you detach.",
    url: siteUrl,
    applicationCategory: "DeveloperApplication",
    applicationSubCategory: "Terminal multiplexer",
    operatingSystem: "Linux, macOS, Windows",
    softwareVersion: "0.4.9",
    downloadUrl: `${repoUrl}/releases/latest`,
    installUrl: siteUrl + "#get-started",
    license: "https://opensource.org/licenses/MIT",
    codeRepository: repoUrl,
    programmingLanguage: "Rust",
    isAccessibleForFree: true,
    offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
    author: { "@type": "Person", name: "Jared Hooker" },
    publisher: { "@type": "Organization", name: "Codefront Labs" },
  },
  {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: faqs.map(([question, answer]) => ({
      "@type": "Question",
      name: question,
      acceptedAnswer: { "@type": "Answer", text: answer },
    })),
  },
];

export default function Home() {
  return (
    <>
      <script
        type="application/ld+json"
        // Static, build-time data; `<` is escaped so it can't close the tag.
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(structuredData).replace(/</g, "\\u003c"),
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />

      <main id="main">
        <section className="hero wrapper" aria-labelledby="hero-title">
          <HiveBackdrop />
          <div className="hero-eyebrow">
            <span className="status-dot" /> A NATIVE WORKSPACE FOR YOUR CODING
            AGENTS
          </div>
          <h1 id="hero-title">
            One terminal.
            <br />
            <span>Many agents.</span>
          </h1>
          <p className="hero-description">
            Hive is an open-source terminal multiplexer for AI coding agents.
            <br className="desktop-break" /> Run Claude Code, Codex, Gemini CLI
            and OpenCode side by side, in the terminal you already love.
          </p>
          <div className="hero-actions flex flex-wrap items-center justify-center gap-4">
            <a href="#get-started" className="button button-primary">
              Get started with Hive <Arrow />
            </a>
            <a href="#how-it-works" className="button button-quiet">
              See how it works <span aria-hidden="true">↓</span>
            </a>
          </div>
          <p className="release-note">
            <span className="tiny-square" /> Linux · macOS · Windows{" "}
            <span aria-hidden="true">/</span> Open source · MIT{" "}
            <span aria-hidden="true">/</span> Built to stay local
          </p>

          <TerminalShowcase />
        </section>

        <section
          className="agents-strip wrapper"
          aria-labelledby="agents-title"
        >
          <p id="agents-title" className="micro-label">
            BRING YOUR OWN AGENT
          </p>
          <div className="agent-list">
            {[
              ["claude", "Claude Code"],
              ["codex", "Codex"],
              ["gemini", "Gemini CLI"],
              ["opencode", "OpenCode"],
            ].map(([agent, label]) => (
              <span key={agent}>
                {/* Local SVG brand assets; the adjacent text supplies the name. */}
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img className="agent-logo" src={`/agents/${agent}.svg`} width={28} height={28} alt="" aria-hidden="true" />
                {label}
              </span>
            ))}
            <span className="custom-agent">
              <Icon name="terminal" size={28} /> Your command
            </span>
          </div>
          <p className="agent-disclaimer">
            Your tools, installed separately. Hive gives them a terminal.
          </p>
        </section>

        <section
          id="features"
          className="section wrapper"
          aria-labelledby="features-title"
        >
          <div className="section-heading">
            <div>
              <p className="eyebrow">LESS FRICTION. MORE FLOW.</p>
              <h2 id="features-title">
                Less terminal management.
                <br />
                More time in the work.
              </h2>
            </div>
            <p className="section-aside">
              Native processes. Familiar commands.
              <br />
              Everything in its place.
            </p>
          </div>
          <div className="features-grid">
            {features.map((feature) => (
              <article className="feature" key={feature.number}>
                <div className="feature-top">
                  <span>{feature.number}</span>
                  <span className="feature-symbol">
                    <Icon name={feature.symbol} size={24} />
                  </span>
                </div>
                <h3>{feature.title}</h3>
                <p>{feature.text}</p>
                <span className="feature-foot">{feature.foot}</span>
              </article>
            ))}
          </div>
        </section>

        <section
          id="integrations"
          className="section wrapper"
          aria-labelledby="integrations-title"
        >
          <div className="section-heading">
            <div>
              <p className="eyebrow">MORE THAN A MULTIPLEXER</p>
              <h2 id="integrations-title">
                Your tools,
                <br />
                beside your agents.
              </h2>
            </div>
            <p className="section-aside">
              Optional integrations. Off by default,
              <br />
              invisible until you turn them on.
            </p>
          </div>
          <div className="integrations-grid">
            {integrations.map((item) => (
              <article className="integration" key={item.name}>
                <div className="integration-top">
                  <h3>{item.name}</h3>
                  <kbd>{item.key}</kbd>
                </div>
                <p>{item.text}</p>
                <code>{item.config}</code>
              </article>
            ))}
          </div>
          <pre className="integration-config">
            <code>{`# ~/.config/hive/config.toml
[integrations.obsidian]
enabled = true
journal = true        # log agent activity to today's note`}</code>
          </pre>
        </section>

        <section
          id="screenshots"
          className="section wrapper"
          aria-labelledby="screenshots-title"
        >
          <div className="section-heading">
            <div>
              <p className="eyebrow">THE REAL THING</p>
              <h2 id="screenshots-title">What it looks like.</h2>
            </div>
            <p className="section-aside">
              Captured from a live session.
              <br />
              Spaces on the left, tabs on top.
            </p>
          </div>
          <div className="screens">
            {[
              ["main", "Claude working beside a git log split, in the shop space"],
              ["worktree", "Codex in its own worktree, waiting for approval"],
              ["palette", "The command palette: Ctrl+Space :"],
            ].map(([name, caption]) => (
              <figure key={name}>
                {/* Static export: plain img with intrinsic size. */}
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={`/screenshots/${name}.png`} alt={caption} width={2443} height={1553} loading="lazy" />
                <figcaption>{caption}</figcaption>
              </figure>
            ))}
          </div>
        </section>

        <section
          id="how-it-works"
          className="architecture-section"
          aria-labelledby="architecture-title"
        >
          <div className="wrapper architecture-grid">
            <div>
              <p className="eyebrow">LOCAL BY DESIGN</p>
              <h2 id="architecture-title">
                Close the window.
                <br />
                Keep the momentum.
              </h2>
              <p className="body-copy">
                Hive is a client. <code>hived</code> is the foundation. The
                daemon owns your agent process, its terminal, and its screen
                state. Your frontend just connects.
              </p>
              <p className="architecture-note">
                <span className="note-mark" aria-hidden="true">
                  ↳
                </span>{" "}
                Sessions last while the daemon is alive. Stopping it or
                rebooting ends the running processes.
              </p>
            </div>
            <div className="architecture-card">
              <div className="architecture-caption">
                <span className="micro-label">ON YOUR MACHINE</span>
                <span className="local-badge">
                  <span className="status-dot" /> local
                </span>
              </div>
              <div className="architecture-node">
                <code>hive</code>
                <span>Render · input · resize</span>
                <span className="node-type">CLIENT</span>
              </div>
              <div className="architecture-wire">
                <span />
                Unix domain socket
                <span />
              </div>
              <div className="architecture-node daemon-node">
                <code>hived</code>
                <span>Processes · PTYs · terminal state</span>
                <span className="node-type">DAEMON</span>
              </div>
              <div className="architecture-wire">
                <span />
                Real interactive terminal
                <span />
              </div>
              <div className="architecture-node">
                <code>your agent</code>
                <span>The CLI you already know</span>
                <span className="node-type">PROCESS</span>
              </div>
              <p className="architecture-bottom">
                No browser runtime. No Hive cloud account.
              </p>
            </div>
          </div>
        </section>

        <section
          id="get-started"
          className="section wrapper"
          aria-labelledby="start-title"
        >
          <div className="section-heading setup-heading">
            <div>
              <p className="eyebrow">MAKE ROOM FOR YOUR AGENTS</p>
              <h2 id="start-title">Your terminal. Your platform.</h2>
            </div>
            <span className="development-badge">
              <span className="tiny-square" /> v0.4.9 RELEASED
            </span>
          </div>
          <ReleaseInstall />
          <details className="source-build-details">
            <summary>
              Or build from source{" "}
              <span>
                Rust 1.89+ <span aria-hidden="true">+</span>
              </span>
            </summary>
            <div className="source-build-content">
              <div className="setup-context">
                <p>
                  Build from a checkout of Hive, then launch the agent you
                  already use.
                </p>
                <div className="setup-requirements" aria-label="Requirements">
                  <span>Linux, macOS or Windows</span>
                  <span>Rust 1.89+</span>
                  <span>Your agent CLI</span>
                </div>
              </div>
              <div className="setup-steps">
                <div className="setup-step">
                  <div className="setup-step-title">
                    <span>01</span>
                    <h3>Build Hive</h3>
                    <Icon name="terminal" size={19} />
                  </div>
                  <p>Clone the repository and build:</p>
                  <CopyCommand command="git clone https://github.com/jrh89/hive && cd hive && cargo build --release" />
                  <p className="setup-footnote">
                    Keep <code>hive</code> and <code>hived</code> together in{" "}
                    <code>target/release/</code>.
                  </p>
                </div>
                <div className="setup-step">
                  <div className="setup-step-title">
                    <span>02</span>
                    <h3>Bring an agent</h3>
                    <Icon name="layers" size={19} />
                  </div>
                  <p>
                    Start with Codex, or swap in <code>claude</code>.
                  </p>
                  <CopyCommand command="./target/release/hive new codex" />
                  <p className="setup-footnote">
                    Your installed CLI. Its own prompts and permissions.
                  </p>
                </div>
                <div className="setup-step">
                  <div className="setup-step-title">
                    <span>03</span>
                    <h3>Leave. Come back.</h3>
                    <Icon name="detach" size={19} />
                  </div>
                  <p>
                    <kbd>Ctrl</kbd> + <kbd>Space</kbd>, then <kbd>d</kbd>{" "}
                    detaches.
                  </p>
                  <CopyCommand command="./target/release/hive" />
                  <p className="setup-footnote">
                    Reopen Hive to access your running sessions.
                  </p>
                </div>
              </div>
              <div className="setup-bottom">
                <span>
                  <span className="status-dot" /> Sessions stay running while
                  the daemon is alive.
                </span>
                <a className="text-link" href="#roadmap">
                  What’s next <Arrow />
                </a>
              </div>
            </div>
          </details>
        </section>

        <section
          id="roadmap"
          className="roadmap-section wrapper"
          aria-labelledby="roadmap-title"
        >
          <div className="roadmap-intro">
            <p className="eyebrow">A FOUNDATION, THEN A WORKSPACE</p>
            <h2 id="roadmap-title">Built in the right order.</h2>
            <p className="body-copy">
              Reliability first, then room for your agents, then the polish to
              run many of them at once.
            </p>
          </div>
          <div className="roadmap-list">
            <article>
              <span className="roadmap-marker current" />
              <div>
                <span className="roadmap-status available">WORKING TODAY</span>
                <h3>Many agents. One view.</h3>
                <p>
                  Real PTYs, a session sidebar, switching, labels, detach and
                  reattach, automatic reconnection, and explicit control when
                  several windows share a session.
                </p>
              </div>
            </article>
            <article>
              <span className="roadmap-marker current" />
              <div>
                <span className="roadmap-status available">WORKING TODAY</span>
                <h3>Room for every project</h3>
                <p>
                  Workspaces, live Git branches, optional worktree isolation, a
                  command palette, themes, and configurable keys.
                </p>
              </div>
            </article>
            <article>
              <span className="roadmap-marker current" />
              <div>
                <span className="roadmap-status available">RELEASED · v0.3.0</span>
                <h3>Many agents at a glance</h3>
                <p>
                  Agent status, split panes and tabs, a clickable sidebar of
                  spaces and agents, Alt-arrow navigation, optional review,
                  task and notification integrations, and hive update.
                </p>
              </div>
            </article>
            <article>
              <span className="roadmap-marker current" />
              <div>
                <span className="roadmap-status available">RELEASED · v0.4.9</span>
                <h3>Everywhere, terminal first</h3>
                <p>
                  macOS and Windows binaries. New opens a terminal, spaces
                  follow your cd, each space keeps its own tabs, and worktree
                  terminals keep agents apart.{" "}
                  <a className="text-link" href="/changelog/">
                    Full changelog <Arrow />
                  </a>
                </p>
              </div>
            </article>
          </div>
        </section>

        <section
          id="questions"
          className="section wrapper faq-grid"
          aria-labelledby="faq-title"
        >
          <div>
            <p className="eyebrow">A FEW DETAILS</p>
            <h2 id="faq-title">Good questions.</h2>
          </div>
          <div className="faq-list">
            {faqs.map(([question, answer]) => (
              <details key={question}>
                <summary>
                  {question}
                  <span aria-hidden="true" className="faq-toggle">
                    +
                  </span>
                </summary>
                <p>{answer}</p>
              </details>
            ))}
          </div>
        </section>

        <section className="closing wrapper">
          <div className="closing-mark" aria-hidden="true">
            <Brand compact />
          </div>
          <p>Your agents. Your terminal.</p>
          <h2>Good work deserves a home.</h2>
          <a className="button button-primary" href="#get-started">
            Get started with Hive <Arrow />
          </a>
          <span className="closing-caption">
            HIVE — ONE TERMINAL. MANY AGENTS.
          </span>
        </section>
      </main>

      <footer className="site-footer wrapper">
        <a className="brand-link" href="#" aria-label="Hive home">
          <Brand />
        </a>
        <span>A Codefront Labs product. Built with Rust.</span>
        <nav aria-label="Footer navigation">
          <a href="#get-started">Get started</a>
          <a href="#roadmap">Roadmap</a>
          <a href="/why/">Why Hive</a>
          <a href="/changelog/">Changelog</a>
          <a href="/changelog/rss.xml">RSS</a>
          <a href="#questions">FAQ</a>
          <a href={repoUrl}>GitHub</a>
          <a href={`${repoUrl}/blob/master/LICENSE`}>MIT License</a>
        </nav>
      </footer>
    </>
  );
}
