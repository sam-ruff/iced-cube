#!/usr/bin/env bash
# Points git at the repo's hooks. The path stays relative so it works in worktrees.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
chmod +x .githooks/*
git config core.hooksPath .githooks
echo "Hooks installed from .githooks"
