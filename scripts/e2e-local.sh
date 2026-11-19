#!/usr/bin/env bash
# Fast local e2e: build the app, the daemon and the fake agent with cargo
# (incremental, RocksDB is compiled once and kept in the target directory)
# instead of `nix build`, then run the suite against those binaries.
# Run it inside the dev shell. Arguments go to wdio, e.g. --spec <file>.
set -euo pipefail
cd "$(dirname "$0")/.."

target="${CARGO_TARGET_DIR:-$PWD/target}"

cargo build -p cerboid -p cerbo --bins
# `tauri build` embeds the built frontend (no dev server needed); --debug keeps
# cargo's incremental dev profile, --no-bundle skips .deb/AppImage packaging.
bun run tauri build --debug --no-bundle

export CERBO_DESKTOP_BIN="$target/debug/cerbo-desktop"
export CERBOID_BIN="$target/debug/cerboid"
export CERBO_E2E_FAKE_AGENT="$target/debug/cerbo-fake-agent"
exec e2e/run.sh "$@"
