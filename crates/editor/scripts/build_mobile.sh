#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BUILD_MODE="${2:-debug}"

case "$1" in
  ios)
    "$SCRIPT_DIR/build_ios.sh" "$BUILD_MODE"
    ;;
  android)
    "$SCRIPT_DIR/build_android.sh" "$BUILD_MODE"
    ;;
  all)
    "$SCRIPT_DIR/build_ios.sh" "$BUILD_MODE"
    "$SCRIPT_DIR/build_android.sh" "$BUILD_MODE"
    ;;
  *)
    echo "Usage: $0 {ios|android|all} [debug|release]"
    exit 1
    ;;
esac
