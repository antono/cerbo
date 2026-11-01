#!/usr/bin/env bash
# Runs the desktop e2e suite; uses a virtual X display when none is available.
set -euo pipefail
cd "$(dirname "$0")"

if [ -z "${DISPLAY:-}" ]; then
  exec xvfb-run -a wdio run wdio.conf.ts "$@"
fi
exec wdio run wdio.conf.ts "$@"
