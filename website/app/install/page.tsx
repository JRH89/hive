import type { Metadata } from "next";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Breadcrumbs } from "@/components/breadcrumbs";
import { CopyCommand } from "@/components/copy-command";
import { JsonLd } from "@/components/json-ld";
import { Icon } from "@/components/icon";
import { absolute, ogImage, organization, releasesUrl, repoUrl } from "@/lib/site";

const title = "Install Hive on Linux, macOS and Windows";
const description =
  "Install Hive with one command on Linux, macOS or Windows: checksum-verified static binaries, hive update, building from source and uninstalling.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: "/install/" },
  openGraph: { title, description, url: "/install/", type: "article", images: [ogImage] },
};

const download = `${releasesUrl}/latest/download`;
const unix = `curl -fsSL ${download}/install.sh | sh`;
const windows = `irm '${download}/install.ps1' | iex`;

const Arrow = () => <Icon name="arrow" size={17} />;

export default function Install() {
  return (
    <>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "TechArticle",
          headline: title,
          description,
          url: absolute("/install/"),
          proficiencyLevel: "Beginner",
          publisher: organization,
          about: { "@type": "SoftwareApplication", name: "Hive", url: absolute("/") },
        }}
      />
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <SiteHeader />
      <main id="main" className="section wrapper page">
        <Breadcrumbs trail={[["Install", "/install/"]]} />
        <p className="eyebrow">GET STARTED</p>
        <h1 className="page-title">Install Hive</h1>
        <p className="page-lede">
          One command installs <code>hive</code> and its daemon <code>hived</code>. The installer
          downloads static binaries, verifies their SHA-256 checksum, and needs no root.
        </p>

        <div className="prose">
          <h2 id="linux-macos">Linux and macOS</h2>
          <p>
            For Linux on x86_64 or ARM64 and macOS on Apple silicon or Intel. Binaries go to{" "}
            <code>~/.local/bin</code>.
          </p>
          <CopyCommand command={unix} />

          <h2 id="windows">Windows</h2>
          <p>
            For Windows 10 or later on x86_64, in PowerShell. Binaries go to{" "}
            <code>%LOCALAPPDATA%\hive\bin</code>, which the installer adds to your user PATH.
          </p>
          <CopyCommand command={windows} />
          <p>
            On Windows, agent status comes from agents that report it with{" "}
            <code>hive report-agent</code>, and terminals stay in the space they started in.
          </p>

          <h2 id="check">Check the setup</h2>
          <pre className="code">
            <code>hive doctor</code>
          </pre>
          <p>
            It shows the version, config file, daemon socket, logs, how the interface will look in
            your terminal, and which agents it found on your PATH. It is also the first thing to
            include in a bug report.
          </p>

          <h2 id="first-run">First run</h2>
          <ol>
            <li>
              Run <code>hive</code> in any directory. You get a terminal in a space named after the
              repository.
            </li>
            <li>
              Type <code>claude</code>, <code>codex</code>, <code>gemini</code> or{" "}
              <code>opencode</code>. The sidebar picks up the agent and shows its status.
            </li>
            <li>
              Press <kbd>Ctrl+Space</kbd> then <kbd>:</kbd> for the command palette, or{" "}
              <kbd>Ctrl+Space</kbd> <kbd>d</kbd> to detach. Your agents keep running; run{" "}
              <code>hive</code> again to come back.
            </li>
          </ol>
          <p>
            Install and sign in to each agent separately. Hive runs the agent&apos;s own CLI and
            leaves its permission prompts to you.
          </p>

          <h2 id="options">Installer options</h2>
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Variable</th>
                <th scope="col">Effect</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>HIVE_INSTALL_DIR</code>
                </td>
                <td>Install somewhere other than the default directory.</td>
              </tr>
              <tr>
                <td>
                  <code>HIVE_VERSION</code>
                </td>
                <td>
                  Pin a release, e.g. <code>v0.4.12</code>.
                </td>
              </tr>
            </tbody>
          </table>

          <h2 id="update">Update</h2>
          <pre className="code">
            <code>{"hive update\nhive daemon stop   # restart the daemon on the new version"}</code>
          </pre>
          <p>
            Stopping the daemon ends running sessions, so pick a quiet moment. The next{" "}
            <code>hive</code> starts the new daemon.
          </p>

          <h2 id="source">Build from source</h2>
          <p>Needs Rust 1.89 or later.</p>
          <pre className="code">
            <code>{`git clone ${repoUrl} && cd hive
cargo build --release
install -m 0755 target/release/hive target/release/hived ~/.local/bin/`}</code>
          </pre>
          <p>
            Keep <code>hive</code> and <code>hived</code> in the same directory.
          </p>

          <h2 id="uninstall">Uninstall</h2>
          <pre className="code">
            <code>{`hive daemon stop
rm ~/.local/bin/hive ~/.local/bin/hived
rm -rf ~/.config/hive ~/.local/state/hive   # config, logs, session metadata`}</code>
          </pre>
          <p>
            Worktrees under <code>~/.local/share/hive/worktrees</code> may hold your work: check
            them first. On Windows, stop the daemon, then delete <code>%LOCALAPPDATA%\hive</code>{" "}
            and the config and state folders <code>hive doctor</code> lists.
          </p>
        </div>

        <div className="why-cta">
          <a className="button button-primary" href="/integrations/">
            Explore integrations <Arrow />
          </a>
          <a className="text-link" href={`${releasesUrl}/latest`}>
            All release files <Arrow />
          </a>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
