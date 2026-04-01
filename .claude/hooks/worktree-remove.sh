#!/bin/bash

INPUT=$(cat)
WORKTREE_PATH=$(echo "$INPUT" | jq -r '.worktree_path')
NAME=$(basename "$WORKTREE_PATH")
MAIN_REPO="${WORKTREE_PATH%/.claude/worktrees/*}"

git worktree remove --force "$WORKTREE_PATH" > /dev/null 2>&1
git -C "$MAIN_REPO" branch -D "worktree/$NAME" > /dev/null 2>&1 || true
