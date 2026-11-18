#!/usr/bin/env bash
# Runs the desktop e2e suite; uses a virtual X display when none is available.
set -euo pipefail
cd "$(dirname "$0")"

if [ -z "${DISPLAY:-}" ]; then
  # Xvfb defaults to 100 DPI and a 640x480 screen. WebKitGTK then runs at a
  # devicePixelRatio of 100/96, and WebDriver clicks land left of their
  # target; the 1200px session window would not fit either.
  exec xvfb-run -a -s "-screen 0 1920x1080x24 -dpi 96" wdio run wdio.conf.ts "$@"
fi
exec wdio run wdio.conf.ts "$@"
