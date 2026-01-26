#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_DIR="$(cd "$CRATE_DIR/../.." && pwd)"

copy_icu_data() {
  ICU_DATA_SRC="$CRATE_DIR/pkg/icu_data.postcard"
  ICU_DATA_DST="$WORKSPACE_DIR/apps/mobile/assets/native"

  if [ -f "$ICU_DATA_SRC" ]; then
    mkdir -p "$ICU_DATA_DST"
    cp "$ICU_DATA_SRC" "$ICU_DATA_DST/"
    echo "Copied ICU data to $ICU_DATA_DST/"
  else
    echo "Warning: ICU data not found at $ICU_DATA_SRC"
    echo "Run 'bun run wasm:build && bun run assets' first to generate ICU data"
  fi
}

BUILD_MODE="${2:-debug}"

case "$1" in
  ios)
    "$SCRIPT_DIR/build_ios.sh" "$BUILD_MODE"
    copy_icu_data
    ;;
  android)
    "$SCRIPT_DIR/build_android.sh" "$BUILD_MODE"
    copy_icu_data
    ;;
  all)
    "$SCRIPT_DIR/build_ios.sh" "$BUILD_MODE"
    "$SCRIPT_DIR/build_android.sh" "$BUILD_MODE"
    copy_icu_data
    ;;
  *)
    echo "Usage: $0 {ios|android|all} [debug|release]"
    exit 1
    ;;
esac
