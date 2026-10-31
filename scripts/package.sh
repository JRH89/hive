#!/bin/sh
# Packages release binaries for one target into <out>/hive-<target>.tar.gz
# plus a .sha256 file. Asset names carry no version so that
# releases/latest/download/<name> always points at the newest release.
#   scripts/package.sh x86_64-unknown-linux-musl dist
set -eu
target=${1:?usage: package.sh <target> <out dir>}
out=${2:?usage: package.sh <target> <out dir>}
name="hive-$target"
stage="$out/$name"
rm -rf "$stage"
mkdir -p "$stage"
cp "target/$target/release/hive" "target/$target/release/hived" "$stage/"
cp README.md LICENSE "$stage/"
cp examples/config.toml "$stage/config.example.toml"
cp -r docs "$stage/"
tar -C "$out" -czf "$out/$name.tar.gz" "$name"
rm -rf "$stage"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$out" && sha256sum "$name.tar.gz" >"$name.tar.gz.sha256")
else
    (cd "$out" && shasum -a 256 "$name.tar.gz" >"$name.tar.gz.sha256")
fi
echo "$out/$name.tar.gz"
