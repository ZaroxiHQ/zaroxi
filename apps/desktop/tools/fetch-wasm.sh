#!/usr/bin/env bash
set -euo pipefail

# Fetch/copy the web-tree-sitter engine wasm and attempt to build per-language wasm
# from grammars under crates/zaroxi-lang-syntax/runtime/treesitter/{grammars,languages}.
#
# Usage (from repo root):
#   cd apps/desktop && npm run prepare-wasm
#
# This script:
# - Ensures the target runtime dir exists
# - Copies node_modules/web-tree-sitter/tree-sitter.wasm if present
# - Otherwise downloads the engine wasm from unpkg
# - Attempts to run `npx tree-sitter-cli generate` and `npx tree-sitter-cli build-wasm`
#   inside each grammar dir (best-effort; may fail if your toolchain is missing)

RUNTIME_DIR="$(pwd)/../crates/zaroxi-lang-syntax/runtime/treesitter"
echo "[prepare-wasm] runtime target: $RUNTIME_DIR"
mkdir -p "$RUNTIME_DIR"

# 1) Copy engine wasm from node_modules if available
if [ -f node_modules/web-tree-sitter/tree-sitter.wasm ]; then
  echo "[prepare-wasm] copying tree-sitter.wasm from node_modules"
  cp node_modules/web-tree-sitter/tree-sitter.wasm "$RUNTIME_DIR/tree-sitter.wasm"
else
  # 2) Download engine wasm from unpkg as a fallback
  echo "[prepare-wasm] node_modules/web-tree-sitter/tree-sitter.wasm not found; attempting download from unpkg"
  if command -v curl >/dev/null 2>&1; then
    if curl -fsSL -o "$RUNTIME_DIR/tree-sitter.wasm" "https://unpkg.com/web-tree-sitter/tree-sitter.wasm"; then
      echo "[prepare-wasm] downloaded engine wasm to $RUNTIME_DIR/tree-sitter.wasm"
    else
      echo "[prepare-wasm] failed to download engine wasm from unpkg"
    fi
  else
    echo "[prepare-wasm] curl not available; cannot download engine wasm"
  fi
fi

# 3) Attempt to build per-language .wasm files by iterating grammar dirs.
GRAMMAR_ROOT="$(pwd)/../crates/zaroxi-lang-syntax/runtime/treesitter"
echo "[prepare-wasm] scanning grammar directories under $GRAMMAR_ROOT/grammars and $GRAMMAR_ROOT/languages"

shopt -s nullglob 2>/dev/null || true

for d in "$GRAMMAR_ROOT"/grammars/* "$GRAMMAR_ROOT"/languages/*; do
  if [ -d "$d" ]; then
    echo "[prepare-wasm] processing grammar: $d"
    (
      cd "$d" || exit 0
      # Try generate + build-wasm via tree-sitter-cli (npx --yes to prefer local install)
      if command -v npx >/dev/null 2>&1; then
        echo "[prepare-wasm] running: npx --yes tree-sitter-cli generate"
        npx --yes tree-sitter-cli generate || echo "[prepare-wasm] tree-sitter generate failed for $d (continuing)"
        echo "[prepare-wasm] running: npx --yes tree-sitter-cli build-wasm"
        if npx --yes tree-sitter-cli build-wasm; then
          echo "[prepare-wasm] build-wasm succeeded in $d"
          # Move produced wasm(s) to the runtime root
          for w in *.wasm; do
            if [ -f "$w" ]; then
              mv -v "$w" "$GRAMMAR_ROOT"/
            fi
          done
        else
          echo "[prepare-wasm] build-wasm failed in $d - please ensure a C toolchain (gcc/clang, make) and python are installed"
        fi
      else
        echo "[prepare-wasm] npx not available; skipping build for $d"
      fi
    )
  fi
done

echo "[prepare-wasm] finished. Check $RUNTIME_DIR for tree-sitter.wasm and language .wasm files."
