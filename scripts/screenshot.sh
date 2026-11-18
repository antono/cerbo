#!/usr/bin/env bash
# Screenshot the desktop app for UI reviews, without touching your own vaults
# or your screen: a copy of test-vault/ and a throwaway XDG config, run on a
# virtual X display (Xvfb). Tools come from nixpkgs, nothing to install.
#
#   scripts/screenshot.sh                      # -> target/screenshots/cerbo.png
#   scripts/screenshot.sh -o ui.png -s 800x600 # other file, other window size
#   scripts/screenshot.sh -c 150,140           # click there before the shot
#                                              # (150,140 opens the first page)
#
# Or build everything in one go: `nix run .#screenshot -- -s 800x600` (run it
# from the repo; it uses ./test-vault). From the repo the binary is
# result/bin/cerbo-desktop (`nix build .#cerbo-desktop`; new files must be
# `git add -N`'d first or the build leaves them out), or CERBO_DESKTOP_BIN.
# Xvfb runs at 96 DPI so a WxH shot is WxH CSS pixels (its default 100 DPI
# scales the page by 1.04). The app is forced onto X11: under a Wayland
# session it would otherwise open on the real desktop and the shot comes out
# blank.
set -euo pipefail

# From the Nix store (nix run .#screenshot) the repo is the current directory.
repo=$(cd "$(dirname "$0")/.." && pwd)
[ -d "$repo/test-vault" ] || repo=$PWD
[ -d "$repo/test-vault" ] || { echo "no test-vault/ in $repo; run from the cerbo repo" >&2; exit 1; }
out="$repo/target/screenshots/cerbo.png"
size=1200x760
click=""
settle=4

usage() { sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }
while getopts "o:s:c:w:h" opt; do
  case $opt in
    o) out=$(realpath -m "$OPTARG") ;;
    s) size=$OPTARG ;;
    c) click=$OPTARG ;;
    w) settle=$OPTARG ;;
    h) usage ;;
    *) usage 1 ;;
  esac
done

bin=$(realpath -m "${CERBO_DESKTOP_BIN:-$repo/result/bin/cerbo-desktop}")
[ -x "$bin" ] || { echo "no cerbo-desktop at $bin; run: nix build .#cerbo-desktop" >&2; exit 1; }
[[ $size =~ ^([0-9]+)x([0-9]+)$ ]] || { echo "size must be WIDTHxHEIGHT, got $size" >&2; exit 1; }
width=${BASH_REMATCH[1]} height=${BASH_REMATCH[2]}

work=$(mktemp -d "${TMPDIR:-/tmp}/cerbo-shot-XXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/config/cerbo" "$(dirname "$out")"
cp -rf "$repo/test-vault" "$work/vault"
cat > "$work/config/cerbo/vaults.toml" <<EOF
[[vaults]]
id = "00000000-0000-4000-8000-000000000137"
name = "Notes"
path = "$work/vault"
isAuto = false
EOF

# Runs inside Xvfb with xdotool and imagemagick on PATH.
cat > "$work/shoot.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
unset WAYLAND_DISPLAY
export GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1
export XDG_CONFIG_HOME="$WORK/config" XDG_CACHE_HOME="$WORK/cache" XDG_DATA_HOME="$WORK/data"
"$BIN" > "$WORK/app.log" 2>&1 &
app=$!
trap 'kill $app 2>/dev/null || true' EXIT
for _ in $(seq 60); do
  win=$(xdotool search --name '^Cerbo' 2>/dev/null | head -1) && [ -n "$win" ] && break
  kill -0 $app 2>/dev/null || { echo "app exited; log:" >&2; cat "$WORK/app.log" >&2; exit 1; }
  sleep 0.5
done
[ -n "${win:-}" ] || { echo "no Cerbo window after 30s; log:" >&2; cat "$WORK/app.log" >&2; exit 1; }
# The app has more than one X window named Cerbo; size every one of them.
xdotool search --name '^Cerbo' windowmove %@ 0 0 windowsize %@ "$W" "$H"
sleep "$SETTLE"
if [ -n "$CLICK" ]; then
  xdotool mousemove --window "$win" "${CLICK%,*}" "${CLICK#*,}" click 1
  sleep 1.5
fi
import -window root -crop "${W}x${H}+0+0" +repage "$OUT"
EOF
chmod +x "$work/shoot.sh"

# The flake package puts the tools on PATH; from the repo, fetch them.
tools=()
command -v xvfb-run xdotool import > /dev/null ||
  tools=(nix shell nixpkgs#xvfb-run nixpkgs#xdotool nixpkgs#imagemagick -c)
WORK=$work BIN=$bin OUT=$out W=$width H=$height CLICK=$click SETTLE=$settle \
  "${tools[@]}" xvfb-run -a -s "-screen 0 ${width}x${height}x24 -dpi 96" "$work/shoot.sh"

echo "$out"
