#!/usr/bin/env bash
# cmfd.sh — one command for a drive: release build + live CMFD glass + full OBD capture.
#
# Default (drive mode):
#   ./cmfd.sh
#     1) cargo build --release (mfd-demo + mfd-obd-capture)
#     2) open captures/drive-TIMESTAMP/
#     3) run mfd-demo with BT ELM + crush poll + capture to that dir
#
# One Bluetooth adapter can serve only one RFCOMM client. Glass and capture
# therefore share one process (ObdFeed writes frames.ndjson while drawing).
#
# Headless maximal capture (no glass; includes DID range scan):
#   ./cmfd.sh capture [--seconds N]
#
# Glass only (no capture files):
#   ./cmfd.sh glass
#
# Env overrides:
#   MFD_OBD_BT          default 00:04:3E:96:B8:F1
#   MFD_OBD_BT_CHANNEL  default 1
#   MFD_OBD_PORT        serial instead of BT
#   MFD_OBD_REPLAY      replay a prior capture dir
#   MFD_CAPTURE_DIR     fixed capture output path
#   MFD_HZ              display rate (default 30)
#   MFD_CAMERA          /dev/videoN or auto
#   MFD_SKIP_BUILD=1    skip cargo build
#   MFD_CAPTURE_SECONDS capture-only duration (default 7200)
#
# Display-only: never writes the vehicle.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

export MFD_OBD_BT="${MFD_OBD_BT:-00:04:3E:96:B8:F1}"
export MFD_OBD_BT_CHANNEL="${MFD_OBD_BT_CHANNEL:-1}"
export MFD_HZ="${MFD_HZ:-30}"
export MFD_OBD_CRUSH="${MFD_OBD_CRUSH:-1}"

MODE="${1:-drive}"
if [[ "${1:-}" == "capture" || "${1:-}" == "glass" || "${1:-}" == "drive" || "${1:-}" == "build" ]]; then
  shift || true
fi

DEMO_BIN="$ROOT/target/release/mfd-demo"
CAP_BIN="$ROOT/target/release/mfd-obd-capture"

build_release() {
  if [[ "${MFD_SKIP_BUILD:-0}" == "1" ]]; then
    echo "cmfd: skip build (MFD_SKIP_BUILD=1)"
    return 0
  fi
  echo "cmfd: cargo build --release --bins …"
  cargo build --release --bins
  echo "cmfd: release bins ready"
}

stamp_dir() {
  local ts
  ts="$(date +%Y%m%d-%H%M%S)"
  echo "${MFD_CAPTURE_DIR:-$ROOT/captures/drive-$ts}"
}

print_env() {
  echo "cmfd: env"
  echo "  MFD_OBD_BT=${MFD_OBD_BT:-}"
  echo "  MFD_OBD_BT_CHANNEL=${MFD_OBD_BT_CHANNEL:-}"
  echo "  MFD_OBD_PORT=${MFD_OBD_PORT:-}"
  echo "  MFD_OBD_REPLAY=${MFD_OBD_REPLAY:-}"
  echo "  MFD_OBD_CRUSH=${MFD_OBD_CRUSH:-}"
  echo "  MFD_OBD_CAPTURE=${MFD_OBD_CAPTURE:-}"
  echo "  MFD_HZ=${MFD_HZ:-}"
  echo "  MFD_CAMERA=${MFD_CAMERA:-}"
}

case "$MODE" in
  build)
    build_release
    ls -la "$DEMO_BIN" "$CAP_BIN" 2>/dev/null || true
    ;;

  glass)
    build_release
    unset MFD_OBD_CAPTURE || true
    print_env
    echo "cmfd: glass only → $DEMO_BIN"
    exec "$DEMO_BIN" "$@"
    ;;

  capture)
    build_release
    CAP="$(stamp_dir)"
    mkdir -p "$CAP"
    SECS="${MFD_CAPTURE_SECONDS:-7200}"
    # Allow --seconds on the CLI after mode
    EXTRA=()
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --seconds)
          SECS="$2"
          shift 2
          ;;
        *)
          EXTRA+=("$1")
          shift
          ;;
      esac
    done
    print_env
    echo "cmfd: headless crush capture → $CAP  (${SECS}s)"
    echo "cmfd: files: frames.ndjson  signals.csv  meta.toml  session.json"
    ARGS=(--crush --seconds "$SECS" -o "$CAP")
    if [[ -n "${MFD_OBD_REPLAY:-}" ]]; then
      ARGS+=(--replay "$MFD_OBD_REPLAY")
    elif [[ -n "${MFD_OBD_PORT:-}" ]]; then
      ARGS+=(--port "$MFD_OBD_PORT")
    else
      ARGS+=(--bt "$MFD_OBD_BT" --channel "${MFD_OBD_BT_CHANNEL}")
    fi
    if [[ ${#EXTRA[@]} -gt 0 ]]; then
      exec "$CAP_BIN" "${ARGS[@]}" "${EXTRA[@]}"
    else
      exec "$CAP_BIN" "${ARGS[@]}"
    fi
    ;;

  drive|*)
    build_release
    CAP="$(stamp_dir)"
    mkdir -p "$CAP"
    export MFD_OBD_CAPTURE="$CAP"
    export MFD_OBD_CRUSH=1
    print_env
    echo "cmfd: DRIVE MODE"
    echo "  glass:   $DEMO_BIN"
    echo "  capture: $CAP"
    echo "  crush:   Mode 01 discover + multi-module UDS + continuous poll"
    echo "  note:    one BT adapter — capture is inside mfd-demo (not a second process)"
    echo "  quit:    Esc  →  capture files finalize on exit"
    echo ""
    # Write a small pointer for post-drive parse
    {
      echo "started_at=$(date -Iseconds 2>/dev/null || date)"
      echo "mode=drive"
      echo "bt=${MFD_OBD_BT}"
      echo "crush=1"
      echo "bin=$DEMO_BIN"
    } >"$CAP/cmfd-run.txt"
    # Ensure finish on signals if shell dies around demo
    trap 'echo "cmfd: stopped — parse $CAP"' EXIT
    exec "$DEMO_BIN" "$@"
    ;;
esac
