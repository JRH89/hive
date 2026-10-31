import { Brand } from "@/components/brand";
import { CopyCommand } from "@/components/copy-command";
import { Icon } from "@/components/icon";
import { TerminalShowcase } from "@/components/terminal-showcase";
import { HiveBackdrop } from "@/components/hive-backdrop";
import { ReleaseInstall } from "@/components/release-install";

const Arrow = () => <Icon name="arrow" size={17} />;

const features = [
  {
    number: "01",
    symbol: "terminal" as const,
    title: "Your agents. As they are.",
    text: "Claude Code, Codex, Gemini CLI, OpenCode. Launch the CLI you already use in a real interactive terminal, with its own prompts and permissions intact.",
    foot: "REAL PTYS · REAL INTERACTION",
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
    title: "Stay on the keyboard.",
    text: "Normal input goes straight to your agent. A dedicated leader key handles Hive commands, so managing the terminal stays out of the way of using it.",
    foot: "CTRL+SPACE · YOUR WAY BACK",
  },
];

const faqs = [
  [
    "Is Hive ready to use?",
    "Hive is in early development. The foundation and multi-session experience are working and have been checked with Claude Code and Codex: launch agents, switch from the sidebar, rename sessions, resize, detach, and reattach. Workspace management and Git worktrees are still planned.",
  ],
  [
    "Which agents can I run?",
    "Hive has launch definitions for Claude Code, Codex, Gemini CLI, and OpenCode. You can configure other commands in TOML. Install and authenticate each agent separately; Hive runs its CLI and preserves its permission system.",
  ],
  [
    "What happens when I close Hive?",
    "Closing or detaching the frontend leaves your agent running in the local hived daemon. Reopening Hive reconnects to the live session. Stopping the daemon or rebooting the machine ends those processes; Hive does not recover them after a reboot.",
  ],
  [
    "Does Hive need a cloud account?",
    "Hive itself runs locally and needs no cloud account, browser, or external service. The agents you choose may have their own accounts, API keys, subscriptions, and network requirements.",
  ],
  [
    "Can I run it on macOS or Windows?",
    "Linux is the initial target. macOS and Windows support will be investigated after the Linux implementation is stable.",
  ],
];

export default function Home() {
  return (
    <>
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <header className="site-header wrapper">
        <a className="brand-link" href="#" aria-label="Hive home">
          <Brand />
        </a>
        <nav
          aria-label="Main navigation"
          className="flex items-center gap-7 md:gap-9"
        >
          <a className="nav-link" href="#features">
            Features
          </a>
          <a className="nav-link" href="#how-it-works">
            How it works
          </a>
          <a className="nav-cta" href="#get-started">
            Get started <Arrow />
          </a>
        </nav>
      </header>

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
            Claude on the frontend. Codex on the API.
            <br className="desktop-break" /> All your agents, together in the
            terminal you already love.
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
            <span className="tiny-square" /> Linux-first{" "}
            <span aria-hidden="true">/</span> Early development{" "}
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
            <span>
              <i className="agent-asterisk" aria-hidden="true">
                ✳
              </i>{" "}
              Claude Code
            </span>
            <span>
              <i aria-hidden="true">◉</i> Codex
            </span>
            <span>
              <i aria-hidden="true">✦</i> Gemini CLI
            </span>
            <span>
              <i className="agent-opencode" aria-hidden="true">
                ▣
              </i>{" "}
              OpenCode
            </span>
            <span className="custom-agent">
              <i aria-hidden="true">&gt;_</i> Your command
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
              <span className="tiny-square" /> EARLY DEVELOPMENT
            </span>
          </div>
          <ReleaseInstall />
          <details className="source-build-details">
            <summary>
              Build from source today{" "}
              <span>
                Linux · Rust 1.89+ <span aria-hidden="true">+</span>
              </span>
            </summary>
            <div className="source-build-content">
              <div className="setup-context">
                <p>
                  Build from a checkout of Hive, then launch the agent you
                  already use.
                </p>
                <div className="setup-requirements" aria-label="Requirements">
                  <span>Linux</span>
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
                  <p>From the Hive repository root:</p>
                  <CopyCommand command="cargo build --release" />
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
              Reliability first. More room for your agents next.
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
                  reattach. Verified with Claude Code and Codex.
                </p>
              </div>
            </article>
            <article>
              <span className="roadmap-marker" />
              <div>
                <span className="roadmap-status">UP NEXT · PLANNED</span>
                <h3>A more resilient connection</h3>
                <p>
                  Automatic frontend reconnection, persisted session metadata,
                  and a clearer experience for multiple clients.
                </p>
              </div>
            </article>
            <article>
              <span className="roadmap-marker" />
              <div>
                <span className="roadmap-status">ON THE HORIZON · PLANNED</span>
                <h3>Room for every project</h3>
                <p>
                  Workspaces, Git branch context, optional worktree isolation,
                  and a configurable command palette.
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
        <span>Built for the terminal. Built with Rust.</span>
        <nav aria-label="Footer navigation">
          <a href="#get-started">Get started</a>
          <a href="#roadmap">Roadmap</a>
          <a href="#questions">FAQ</a>
        </nav>
      </footer>
    </>
  );
}
