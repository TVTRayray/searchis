#!/usr/bin/env bash
# Run `npm run test:e2e` on a private X display (Xvfb) with a private D-Bus session and KWin,
# so E2E window activation and key input never touch the real desktop (DISPLAY=:0).
# Usage: .specflow/bin/e2e-isolated.sh [extra args passed to npm run test:e2e]
set -euo pipefail
cd "$(dirname "$0")/../.."
log="$(mktemp -t searchis-e2e-kwin.XXXXXX.log)"
echo "e2e-isolated: Xvfb + private D-Bus + KWin (kwin log: $log)"
exec xvfb-run -a -s '-screen 0 1280x1024x24' dbus-run-session -- bash -c '
  kwin_x11 --replace >"$1" 2>&1 & wm=$!
  trap "kill $wm 2>/dev/null || true" EXIT
  shift
  npm run test:e2e -- "$@"
' _ "$log" "$@"
