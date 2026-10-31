#!/bin/sh
# Smoke-tests built binaries against a private daemon:
#   scripts/smoke.sh target/release
set -eu
bin=$(cd "${1:?usage: smoke.sh <dir with hive and hived>}" && pwd)
tmp=$(mktemp -d)
export HIVE_RUNTIME_DIR="$tmp/rt" HIVE_STATE_DIR="$tmp/state" HIVE_CONFIG="$tmp/config.toml"
export HOME="$tmp" SHELL=/bin/sh
cleanup() {
    "$bin/hive" daemon stop >/dev/null 2>&1 || true
    rm -rf "$tmp"
}
trap cleanup EXIT

"$bin/hive" --version
"$bin/hive" doctor
"$bin/hive" new shell -d -l smoke
"$bin/hive" list | grep -q smoke
"$bin/hive" status | grep -q "1 active"
"$bin/hive" kill 1
"$bin/hive" list | grep -q "no sessions"
"$bin/hive" daemon stop
echo "smoke test passed"
