#!/usr/bin/env bash
#
# prepare-treesitter.sh — ensure Tree-sitter grammar shared libraries exist for
# the CURRENT platform, so `zaroxi-core-platform-syntax` highlighting (and its
# `tests/highlight_spans.rs` integration test) can dynamically load them.
#
# Layout: grammars live under
#   crates/zaroxi-core-platform-syntax/runtime/treesitter/grammars/
# The runtime resolver (src/runtime.rs) also accepts a platform subdirectory
#   .../grammars/<os>-<arch>/   (e.g. linux-x86_64).
#
# On Linux the grammars are COMMITTED (see .gitignore), so this script is a fast
# no-op there. On other platforms it builds the missing grammars via the crate's
# `download_grammars` binary (requires git + a C compiler).
#
# The script prints the resolved runtime dir on stdout and, when running under
# GitHub Actions, exports ZAROXI_TREESITTER_RUNTIME_DIR to the job environment.
#
# Usage:
#   tooling/scripts/prepare-treesitter.sh [--check]
#     --check   Only report which grammars are present/missing (no build).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CRATE="zaroxi-core-platform-syntax"
RUNTIME_DIR="$REPO_ROOT/crates/$CRATE/runtime/treesitter"

mode="install"
if [[ "${1:-}" == "--check" ]]; then
  mode="check"
fi

echo "[prepare-treesitter] platform: $(uname -s)/$(uname -m)"
echo "[prepare-treesitter] runtime dir: $RUNTIME_DIR"

if [[ "$mode" == "check" ]]; then
  cargo run --release --quiet -p "$CRATE" --bin download_grammars -- check
else
  # `download_grammars install` only builds grammars that are missing for the
  # current platform; committed platform grammars are detected and skipped.
  cargo run --release --quiet -p "$CRATE" --bin download_grammars -- install
fi

# Emit the runtime dir for callers / CI. Tests also auto-resolve it from the
# crate manifest dir, but exporting it makes the location explicit.
echo "ZAROXI_TREESITTER_RUNTIME_DIR=$RUNTIME_DIR"
if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "ZAROXI_TREESITTER_RUNTIME_DIR=$RUNTIME_DIR" >> "$GITHUB_ENV"
fi
