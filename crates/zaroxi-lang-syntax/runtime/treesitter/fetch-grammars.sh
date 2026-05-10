#!/usr/bin/env bash
set -euo pipefail

# Enhanced script to prepare tree-sitter artifacts for both native and WebAssembly.
# This script will:
#  - detect/build native grammar libraries (existing behavior)
#  - attempt to build per-language WASM parsers using tree-sitter-cli (if available)
#  - as a last resort, print actionable instructions for obtaining prebuilt wasm files
#
# Usage: Called from build.rs with TARGET environment variable set, or run manually.

RUNTIME_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Determine platform from TARGET (if not set, guess)
TARGET=${TARGET:-}
if [[ -z "$TARGET" ]]; then
    # Guess current system
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    if [[ "$OS" == "darwin" ]]; then
        OS="macos"
    fi
    if [[ "$ARCH" == "x86_64" ]]; then
        ARCH="x86_64"
    elif [[ "$ARCH" == "arm64" || "$ARCH" == "aarch64" ]]; then
        ARCH="aarch64"
    else
        ARCH="x86_64"
    fi
    TARGET="${OS}-${ARCH}"
else
    # Convert cargo target triple to our directory naming
    case "$TARGET" in
        *linux*)
            OS="linux"
            ;;
        *darwin*)
            OS="macos"
            ;;
        *windows*)
            OS="windows"
            ;;
        *)
            OS="linux"
            ;;
    esac
    case "$TARGET" in
        *x86_64*)
            ARCH="x86_64"
            ;;
        *aarch64*|*arm64*)
            ARCH="aarch64"
            ;;
        *)
            ARCH="x86_64"
            ;;
    esac
    TARGET="${OS}-${ARCH}"
fi

GRAMMAR_DIR="${RUNTIME_ROOT}/grammars/${TARGET}"
mkdir -p "${GRAMMAR_DIR}"
echo "Target platform: ${TARGET}"
echo "Grammar directory: ${GRAMMAR_DIR}"

# Determine library extension
case "$OS" in
    linux)
        EXT=".so"
        PREFIX="lib"
        ;;
    macos)
        EXT=".dylib"
        PREFIX="lib"
        ;;
    windows)
        EXT=".dll"
        PREFIX=""
        ;;
    *)
        EXT=".so"
        PREFIX="lib"
        ;;
esac

# Languages to consider (repo, branch and optional subdir)
LANGUAGES=(
    "bash|https://github.com/tree-sitter/tree-sitter-bash"
    "c|https://github.com/tree-sitter/tree-sitter-c"
    "cpp|https://github.com/tree-sitter/tree-sitter-cpp"
    "c-sharp|https://github.com/tree-sitter/tree-sitter-c-sharp"
    "css|https://github.com/tree-sitter/tree-sitter-css"
    "go|https://github.com/tree-sitter/tree-sitter-go"
    "html|https://github.com/tree-sitter/tree-sitter-html"
    "java|https://github.com/tree-sitter/tree-sitter-java"
    "javascript|https://github.com/tree-sitter/tree-sitter-javascript"
    "json|https://github.com/tree-sitter/tree-sitter-json"
    "python|https://github.com/tree-sitter/tree-sitter-python"
    "ruby|https://github.com/tree-sitter/tree-sitter-ruby"
    "rust|https://github.com/tree-sitter/tree-sitter-rust"
    "typescript|https://github.com/tree-sitter/tree-sitter-typescript||typescript/src"
    "tsx|https://github.com/tree-sitter/tree-sitter-typescript||tsx/src"
    "lua|https://github.com/tree-sitter-grammars/tree-sitter-lua"
    "toml|https://github.com/tree-sitter-grammars/tree-sitter-toml"
    "yaml|https://github.com/tree-sitter-grammars/tree-sitter-yaml"
    "zig|https://github.com/tree-sitter-grammars/tree-sitter-zig"
    "cmake|https://github.com/uyha/tree-sitter-cmake"
    "dockerfile|https://github.com/camdencheek/tree-sitter-dockerfile"
    "elixir|https://github.com/elixir-lang/tree-sitter-elixir"
    "nix|https://github.com/nix-community/tree-sitter-nix"
    "markdown|https://github.com/tree-sitter-grammars/tree-sitter-markdown|split_parser|tree-sitter-markdown/src"
)

# Helper: try to run a command via npx if local binary is missing
maybe_run_npx() {
  if command -v "$1" >/dev/null 2>&1; then
    "$@"
    return $?
  elif command -v npx >/dev/null 2>&1; then
    npx --yes "$@"
    return $?
  else
    return 127
  fi
}

# Step A — Build native libraries (existing behavior) ------------------------------------------------
echo "Attempting to build native grammar libraries (if toolchain available)..."

# Only run if tree-sitter CLI is present (native build uses tree-sitter too)
if ! maybe_run_npx tree-sitter --version >/dev/null 2>&1; then
  echo "tree-sitter CLI not available via PATH or npx. Native builds will be skipped."
else
  # Temporary directory for all builds (will be cleaned up on exit)
  BUILD_ROOT="$(mktemp -d)"
  trap 'rm -rf "$BUILD_ROOT"' EXIT

  for lang_spec in "${LANGUAGES[@]}"; do
      IFS='|' read -r lang repo branch subdir <<< "$lang_spec"
      branch="${branch:-master}"
      echo "Building native ${lang}…"

      lang_tmp="$(mktemp -d -p "$BUILD_ROOT")"
      if git clone --depth 1 --branch "$branch" "$repo" "$lang_tmp" 2>/dev/null; then
          pushd "$lang_tmp" > /dev/null
          if [[ -n "$subdir" && -d "$subdir" ]]; then
              cd "$subdir"
          fi
          if [[ -f "grammar.js" ]]; then
              maybe_run_npx tree-sitter generate || true
          fi
          if maybe_run_npx tree-sitter build >/dev/null 2>&1; then
              built_lib=""
              for pattern in "${PREFIX}tree-sitter-${lang}${EXT}" \
                            "target/release/${PREFIX}tree-sitter-${lang}${EXT}" \
                            "target/debug/${PREFIX}tree-sitter-${lang}${EXT}" \
                            "*.${EXT}"; do
                  matches=($pattern)
                  if [[ ${#matches[@]} -gt 0 && -f "${matches[0]}" ]]; then
                      built_lib="${matches[0]}"
                      break
                  fi
              done
              if [[ -n "$built_lib" ]]; then
                  cp "$built_lib" "${GRAMMAR_DIR}/"
                  echo "  → ${lang} native library copied to ${GRAMMAR_DIR}"
              else
                  echo "  → ${lang}: built but could not locate native library"
              fi
          else
              echo "  → ${lang}: native build failed (continuing)"
          fi
          popd > /dev/null
      else
          echo "  → ${lang}: failed to clone repository (continuing)"
      fi
  done

  echo "Native grammar pass complete. Built libs (if any) are in ${GRAMMAR_DIR}"
  ls -la "${GRAMMAR_DIR}" || true
fi

# Step B — Attempt to produce per-language WASM parsers ------------------------------------------------
# web-tree-sitter requires:
#  - engine runtime: tree-sitter.wasm (already handled elsewhere)
#  - per-language parser wasm files: tree-sitter-<lang>.wasm
#
# If tree-sitter CLI (node tree-sitter-cli) is available, attempt to build wasm for grammar dirs
echo
echo "Attempting to build per-language WASM parsers using tree-sitter-cli (if available)..."

WASM_BUILT=()
if command -v npx >/dev/null 2>&1; then
  for lang_dir in "${RUNTIME_ROOT}/languages/"*; do
    if [[ -d "$lang_dir" ]]; then
      # If the grammar directory contains a grammar.js or can generate via tree-sitter, try building wasm.
      # Some language packs in 'languages' are only query sets; skip those without build metadata.
      if [[ -f "${lang_dir}/grammar.js" || -f "${lang_dir}/src/parser.c" || -f "${lang_dir}/binding.gyp" ]]; then
        echo "[wasm] attempting build in ${lang_dir}"
        pushd "$lang_dir" > /dev/null
        # generate + build-wasm using npx tree-sitter-cli where possible
        if npx --yes tree-sitter-cli generate >/dev/null 2>&1 || true; then
          if npx --yes tree-sitter-cli build-wasm >/dev/null 2>&1; then
            for w in *.wasm; do
              if [[ -f "$w" ]]; then
                mv -v "$w" "${RUNTIME_ROOT}/"
                WASM_BUILT+=("$w")
              fi
            done
          else
            echo "[wasm] build-wasm failed in ${lang_dir} (toolchain may be missing)"
          fi
        fi
        popd > /dev/null
      fi
    fi
  done
else
  echo "[wasm] npx not available; skipping automatic wasm builds in languages/"
fi

# Also try building from any 'grammars' source directories (those may contain full repos)
if command -v npx >/dev/null 2>&1; then
  for g in "${RUNTIME_ROOT}/grammars/"*; do
    if [[ -d "$g" ]]; then
      echo "[wasm] checking grammar repo: ${g}"
      # Try to find grammar.js in nested dirs
      if find "$g" -maxdepth 3 -type f -name 'grammar.js' | grep -q '.'; then
        # Attempt build in the directory that contains grammar.js
        for gm in $(find "$g" -maxdepth 3 -type f -name 'grammar.js' -printf '%h\n' | sort -u); do
          echo "[wasm] attempting build in ${gm}"
          pushd "${gm}" > /dev/null
          if npx --yes tree-sitter-cli generate >/dev/null 2>&1 || true; then
            if npx --yes tree-sitter-cli build-wasm >/dev/null 2>&1; then
              for w in *.wasm; do
                if [[ -f "$w" ]]; then
                  mv -v "$w" "${RUNTIME_ROOT}/"
                  WASM_BUILT+=("$w")
                fi
              done
            else
              echo "[wasm] build-wasm failed in ${gm} (toolchain may be missing)"
            fi
          fi
          popd > /dev/null
        done
      fi
    fi
  done
fi

# Step C — Report results and provide next steps --------------------------------------------------------
echo
if [[ ${#WASM_BUILT[@]} -gt 0 ]]; then
  echo "[wasm] Built and moved the following .wasm files into ${RUNTIME_ROOT}:"
  for f in "${WASM_BUILT[@]}"; do
    echo "  - $f"
  done
else
  echo "[wasm] No per-language .wasm parsers were produced by this script."
  echo "[wasm] To enable web-tree-sitter parsing in the browser you must provide per-language wasm files"
  echo "       (e.g. tree-sitter-rust.wasm, tree-sitter-toml.wasm) in:"
  echo "         ${RUNTIME_ROOT}"
  echo
  echo "Options:"
  echo "  1) Install a native toolchain and tree-sitter CLI, then re-run this script to build wasm:"
  echo "       sudo apt install build-essential clang pkg-config python3"
  echo "       cd apps/desktop && npm install"
  echo "       cd apps/desktop && npm run prepare-wasm"
  echo
  echo "  2) Obtain prebuilt parser wasm artifacts and place them into the runtime dir. For example,"
  echo "       crates/zaroxi-lang-syntax/runtime/treesitter/tree-sitter-rust.wasm"
  echo "       crates/zaroxi-lang-syntax/runtime/treesitter/tree-sitter-toml.wasm"
  echo
  echo "  3) If you have CI that builds wasm artifacts, copy the produced .wasm files into the runtime dir"
  echo "     as part of your developer onboarding or packaging step."
fi

echo
echo "[fetch-grammars] done. Current runtime dir contents:"
ls -la "${RUNTIME_ROOT}"
