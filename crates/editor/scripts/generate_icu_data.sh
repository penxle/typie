#!/usr/bin/env bash
# Generate ICU data blob for WASM

set -e

echo "Generating ICU data blob..."

# Install icu_datagen if not already installed
if ! command -v icu4x-datagen &> /dev/null; then
  echo "Installing icu4x-datagen..."
  cargo install icu4x-datagen
fi

path="pkg/icu_data.postcard"
tmp_path="icu_data.postcard.tmp"
log_path="icu_data.log"

# Generate blob to a temporary file, capturing output
rm -f "$tmp_path"
CLICOLOR_FORCE=1 icu4x-datagen \
  --markers-for-bin pkg/editor_bg.wasm \
  --format blob \
  --out $tmp_path > "$log_path" 2>&1

if [ -f "$path" ] && cmp -s "$tmp_path" "$path"; then
  echo "ICU data is up to date."
  rm "$tmp_path"
  rm "$log_path"
else
  mv "$tmp_path" "$path"
  cat "$log_path"
  rm "$log_path"
  echo "ICU data blob generated at: $path"
  echo "Size: $(du -h $path | cut -f1)"
fi
