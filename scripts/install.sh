#!/bin/sh
# Installs hive and hived from a GitHub release.
#
#   curl -fsSL https://github.com/jrh89/hive/releases/latest/download/install.sh | sh
#
# Environment:
#   HIVE_VERSION       release tag to install (default: latest), e.g. v0.2.0
#   HIVE_INSTALL_DIR   where to put the binaries (default: ~/.local/bin)
#   HIVE_DOWNLOAD_URL  base URL holding the release assets (overrides the above)
set -eu

repo="jrh89/hive"
version="${HIVE_VERSION:-latest}"
dir="${HIVE_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf 'hive-install: %s\n' "$*"; }
die() { say "$*" >&2; exit 1; }

[ "$(uname -s)" = Linux ] || die "only Linux is supported for now (this is $(uname -s))"
case "$(uname -m)" in
    x86_64 | amd64) target=x86_64-unknown-linux-musl ;;
    aarch64 | arm64) target=aarch64-unknown-linux-musl ;;
    *) die "no prebuilt binary for $(uname -m); build from source instead (see README)" ;;
esac

if [ -n "${HIVE_DOWNLOAD_URL:-}" ]; then
    base="$HIVE_DOWNLOAD_URL"
elif [ "$version" = latest ]; then
    base="https://github.com/$repo/releases/latest/download"
else
    base="https://github.com/$repo/releases/download/$version"
fi

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        die "need curl or wget"
    fi
}

name="hive-$target"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

say "downloading $name ($version)"
fetch "$base/$name.tar.gz" "$tmp/$name.tar.gz" || die "download failed: $base/$name.tar.gz"
fetch "$base/$name.tar.gz.sha256" "$tmp/$name.tar.gz.sha256" || die "checksum download failed"

if command -v sha256sum >/dev/null 2>&1; then
    (cd "$tmp" && sha256sum -c "$name.tar.gz.sha256" >/dev/null) || die "checksum mismatch"
elif command -v shasum >/dev/null 2>&1; then
    (cd "$tmp" && shasum -a 256 -c "$name.tar.gz.sha256" >/dev/null) || die "checksum mismatch"
else
    die "need sha256sum or shasum to verify the download"
fi

tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$dir"
# hive finds hived next to itself, so they are installed together.
for bin in hive hived; do
    cp "$tmp/$name/$bin" "$dir/$bin.new"
    chmod 0755 "$dir/$bin.new"
    mv -f "$dir/$bin.new" "$dir/$bin"
done
say "installed $("$dir/hive" --version) to $dir"

case ":$PATH:" in
    *":$dir:"*) ;;
    *) say "note: $dir is not on your PATH; add it to your shell profile" ;;
esac
if "$dir/hive" daemon status 2>/dev/null | grep -q "running (pid"; then
    say "note: an older hived may still be running. 'hive daemon stop' restarts it (this ends running sessions)"
fi
say "run 'hive doctor' to check your setup, then 'hive' to start"
