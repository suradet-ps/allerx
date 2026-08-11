#!/usr/bin/env sh
# gen-icons.sh — regenerate all Tauri app icons from icon-master.svg.
#
# `cargo tauri icon` accepts a squared SVG with transparency directly and
# renders every platform size itself (macOS .icns, Windows .ico, iOS,
# Android, Store logos) into src-tauri/icons/.
#
# Prerequisite: tauri-cli (`cargo install tauri-cli --locked`).
#
# Usage:
#   script/gen-icons.sh          # Normal mode
#
# Rebuild the app afterwards to apply the new icons:
#   cargo tauri build

set -eu

cd "$(dirname "$0")/.."

if [ ! -f icon-master.svg ]; then
  echo "error: icon-master.svg not found at the repository root" >&2
  exit 1
fi

cargo tauri icon icon-master.svg

echo "All icons generated in src-tauri/icons/."