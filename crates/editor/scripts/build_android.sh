#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_DIR="$(cd "$CRATE_DIR/../.." && pwd)"
OUTPUT_DIR="$CRATE_DIR/target/android/jniLibs"

BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" = "debug" ]; then
  PROFILE="dev"
  echo "Building editor for Android (debug)..."
else
  PROFILE="release-native"
  echo "Building editor for Android (release)..."
fi

if ! command -v cargo-ndk &> /dev/null; then
  echo "Installing cargo-ndk..."
  cargo install cargo-ndk
fi

rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android 2> /dev/null || true

mkdir -p "$OUTPUT_DIR"

echo "Building for Android targets..."
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -t x86 \
  -o "$OUTPUT_DIR" \
  build --manifest-path "$CRATE_DIR/Cargo.toml" --profile "$PROFILE" --features native --no-default-features

echo "Android libraries created at $OUTPUT_DIR"

MOBILE_ANDROID_DIR="$WORKSPACE_DIR/apps/mobile/android/app/src/main/jniLibs"
rm -rf "$MOBILE_ANDROID_DIR"
cp -R "$OUTPUT_DIR" "$MOBILE_ANDROID_DIR"
echo "Copied to $MOBILE_ANDROID_DIR"
echo ""
echo "Directory structure:"
ls -la "$MOBILE_ANDROID_DIR"
