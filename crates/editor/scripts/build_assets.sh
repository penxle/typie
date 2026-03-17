#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_DIR="$(cd "$CRATE_DIR/../.." && pwd)"

ASSETS_DIR="$CRATE_DIR/assets"
PKG_DIR="$CRATE_DIR/pkg"
MOBILE_ASSETS_DIR="$WORKSPACE_DIR/apps/mobile/assets/native"

generate_icu_data() {
  if ! command -v icu4x-datagen &> /dev/null; then
    echo "Installing icu4x-datagen..."
    cargo install icu4x-datagen
  fi

  local postcard="$ASSETS_DIR/icu_data.postcard"
  local zst="$ASSETS_DIR/icu.zst"

  echo "Generating ICU data..."
  rm -f "$postcard"
  CLICOLOR_FORCE=1 icu4x-datagen \
    --markers-for-bin "$PKG_DIR/editor_bg.wasm" \
    --format blob \
    --out "$postcard"

  echo "Compressing ICU data with zstd..."
  zstd -19 -f --rm "$postcard" -o "$zst"
  echo "ICU data generated: $(du -h "$zst" | cut -f1)"
}

copy_if_changed() {
  local src="$1"
  local dst="$2"
  local name="$(basename "$src")"

  if [ ! -f "$src" ]; then
    echo "Warning: $src not found"
    return 1
  fi

  mkdir -p "$(dirname "$dst")"
  if [ ! -f "$dst" ] || ! cmp -s "$src" "$dst"; then
    cp "$src" "$dst"
    echo "Copied $name -> $dst"
  else
    echo "$name is up to date."
  fi
}

generate_icu_data

echo ""
echo "Copying assets to mobile..."
copy_if_changed "$ASSETS_DIR/icu.zst" "$MOBILE_ASSETS_DIR/icu.zst"
copy_if_changed "$ASSETS_DIR/Noto-Phantom.bin" "$MOBILE_ASSETS_DIR/Noto-Phantom.bin"
copy_if_changed "$ASSETS_DIR/Noto-Phantom-Emoji.bin" "$MOBILE_ASSETS_DIR/Noto-Phantom-Emoji.bin"
copy_if_changed "$ASSETS_DIR/fallbacks.json" "$MOBILE_ASSETS_DIR/fallbacks.json"
copy_if_changed "$ASSETS_DIR/theme.json" "$MOBILE_ASSETS_DIR/theme.json"
