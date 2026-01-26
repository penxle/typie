#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_DIR="$(cd "$CRATE_DIR/../.." && pwd)"
TARGET_DIR="$WORKSPACE_DIR/target"
OUTPUT_DIR="$CRATE_DIR/target/ios"
INCLUDE_DIR="$CRATE_DIR/include"

BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" = "debug" ]; then
  PROFILE="dev"
  PROFILE_DIR="debug"
  echo "Building editor for iOS (debug)..."
else
  PROFILE="release-native"
  PROFILE_DIR="release-native"
  echo "Building editor for iOS (release)..."
fi

rustup target add aarch64-apple-ios aarch64-apple-ios-sim 2> /dev/null || true

mkdir -p "$OUTPUT_DIR"

echo "Building for iOS device (arm64)..."
cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --profile "$PROFILE" --target aarch64-apple-ios --features native --no-default-features

echo "Building for iOS simulator (arm64)..."
cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --profile "$PROFILE" --target aarch64-apple-ios-sim --features native --no-default-features

LIBS_IOS="$TARGET_DIR/aarch64-apple-ios/$PROFILE_DIR/libeditor.a"
LIBS_SIM="$TARGET_DIR/aarch64-apple-ios-sim/$PROFILE_DIR/libeditor.a"

if [ -d "$OUTPUT_DIR/Editor.xcframework" ]; then
  rm -rf "$OUTPUT_DIR/Editor.xcframework"
fi

echo "Creating XCFramework..."
xcodebuild -create-xcframework \
  -library "$LIBS_IOS" -headers "$INCLUDE_DIR" \
  -library "$LIBS_SIM" -headers "$INCLUDE_DIR" \
  -output "$OUTPUT_DIR/Editor.xcframework"

echo "XCFramework created at $OUTPUT_DIR/Editor.xcframework"

MOBILE_IOS_DIR="$WORKSPACE_DIR/apps/mobile/ios/Frameworks/Editor"
mkdir -p "$MOBILE_IOS_DIR"
rm -rf "$MOBILE_IOS_DIR/Editor.xcframework"
cp -R "$OUTPUT_DIR/Editor.xcframework" "$MOBILE_IOS_DIR/"
echo "Copied to $MOBILE_IOS_DIR/Editor.xcframework"
