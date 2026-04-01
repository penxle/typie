#!/bin/bash

INPUT=$(cat)
NAME=$(echo "$INPUT" | jq -r '.name')
CWD=$(echo "$INPUT" | jq -r '.cwd')
WORKTREE_PATH="$CWD/.claude/worktrees/$NAME"

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

git worktree add -b "worktree/$NAME" "$WORKTREE_PATH" "$CURRENT_BRANCH" > /dev/null 2>&1

echo "$WORKTREE_PATH"
