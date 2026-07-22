#!/usr/bin/env bash
# Export CMFD enclosure STLs for print houses.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/mech/src/cmfd_enclosure.scad"
OUT="$ROOT/mech/print"
mkdir -p "$OUT"

export_part() {
  local part="$1"
  local file="$2"
  echo "OpenSCAD → $file ($part)"
  openscad -o "$OUT/$file" -D "PART=\"$part\"" "$SRC"
}

export_part front   cmfd-front-bezel.stl
export_part rear    cmfd-rear-shell.stl
export_part tray    cmfd-battery-tray.stl
export_part osb_cap cmfd-osb-cap.stl
export_part rocker  cmfd-rocker.stl
export_part bumper  cmfd-corner-bumper.stl

# zip for print house
(
  cd "$OUT"
  zip -q -r cmfd-print-files.zip ./*.stl
)
echo "Wrote $OUT/cmfd-print-files.zip"
ls -la "$OUT"
