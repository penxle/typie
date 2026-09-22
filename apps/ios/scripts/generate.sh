#!/usr/bin/env bash
set -euo pipefail
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/Library/pnpm/bin:$HOME/.local/share/mise/shims:$PATH"
cd "$(dirname "$0")/.."
just graphql icons
