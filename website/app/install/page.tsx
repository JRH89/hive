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
  "Install Hive on Linux, macOS or Windows with a checksum-verified installer, npm, Bun, Homebrew, or an Arch Linux package.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: "/install/" },
  openGraph: { title, description, url: "/install/", type: "article", images: [ogImage] },
};

const download = `${releasesUrl}/latest/download`;
const unix = `curl -fsSL ${download}/install.sh | sh`;
const windows = `irm '${download}/install.ps1' | iex`;
const archPackage = "codefront-hive-bin-0.6.0-1-x86_64.pkg.tar.zst";
const archDownload = `${releasesUrl}/download/v0.6.0/${archPackage}`;
const archInstall = `curl -fLO ${archDownload} &&
curl -fLO ${archDownload}.sha256 &&
sha256sum -c ${archPackage}.sha256 &&
sudo pacman -U ./${archPackage}`;

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
            For Windows 10 or later on x86_64 or Arm64, in PowerShell (Windows PowerShell or PowerShell 7). Binaries go to{" "}
            <code>%LOCALAPPDATA%\hive\bin</code>, which the installer adds to your user PATH.
          </p>
          <CopyCommand command={windows} />
          <p>
            On Windows, agent status comes from agents that report it with{" "}
            <code>hive report-agent</code>, and terminals stay in the space they started in.
          </p>

          <h2 id="package-managers">Package managers</h2>
          <h3>npm, Bun, pnpm, and Yarn</h3>
          <CopyCommand command="npm install -g @codefrontlabs/hive" />
          <CopyCommand command="bun add -g @codefrontlabs/hive" />
          <p>
            Use <code>pnpm add -g @codefrontlabs/hive</code> with pnpm or{" "}
            <code>yarn dlx -p @codefrontlabs/hive hive</code> with Yarn. Yarn may hold new
            versions for up to 24 hours by default. The launcher uses Node.js. If you only have Bun, use <code>bunx --bun @codefrontlabs/hive</code>.
            GitHub Releases also has the <a href={`${releasesUrl}/latest`}>npm package archive</a>.
          </p>

          <h3>Homebrew</h3>
          <CopyCommand
            command={`brew tap jrh89/hive ${repoUrl} && brew install jrh89/hive/codefront-hive`}
          />

          <h3>Arch Linux</h3>
          <p>
            Install the GitHub release with pacman on x86_64. The command downloads the{" "}
            <a href={archDownload}>Arch package</a> and its{" "}
            <a href={`${archDownload}.sha256`}>checksum</a>, then verifies and installs it.
          </p>
          <CopyCommand command={archInstall} />
          <p>
            For ARM64, build and install from the{" "}
            <a href={`${repoUrl}/tree/master/packaging/aur`}>PKGBUILD</a> with{" "}
            <code>makepkg -si</code>. AUR publication is pending.
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
            If you installed with a package manager, update with the same manager. Stopping the
            daemon ends running sessions (it lists them and asks first). The next{" "}
            <code>hive</code> starts the new daemon, and <kbd>Ctrl+Space</kbd> <kbd>R</kbd> restores
            the sessions, with agents resuming their conversations.
          </p>

          <h2 id="manual">Manual download</h2>
          <p>
            Download the archive for your OS and CPU from{" "}
            <a href={`${releasesUrl}/latest`}>GitHub Releases</a>, verify its SHA-256 checksum,
            and extract <code>hive</code> and <code>hived</code> into the same directory on your PATH.
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
