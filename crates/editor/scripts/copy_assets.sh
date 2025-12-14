#!/bin/bash
SRC="assets/Noto-Phantom.ttf"
DEST="pkg/Noto-Phantom.ttf"

if [ ! -f "$DEST" ] || ! cmp -s "$SRC" "$DEST"; then
  cp "$SRC" "$DEST"
  echo "Copied $SRC to $DEST"
else
  echo "$DEST is up to date."
fi
