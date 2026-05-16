#!/usr/bin/env bash
set -euo pipefail

# Script to build Tree‑sitter grammar shared libraries from source.
# Supports Linux, macOS, and Windows (if compiled).
# Usage: Called from build.rs with TARGET environment variable set.

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

# Language definitions: language|git‑repo|branch|subdir (optional fields)
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

# Check for tree‑sitter CLI
if ! command -v tree-sitter >/dev/null 2>&1; then
    echo "tree‑sitter CLI is not installed."
    echo "Skipping grammar builds. For static linking (Rust, TOML) this is fine."
    echo "If you need grammars for other languages, install tree-sitter-cli via:"
    echo "  cargo install tree-sitter-cli"
    exit 0
fi

echo "Building grammars from source (using tree‑sitter CLI)..."

# Temporary directory for all builds (will be cleaned up on exit)
BUILD_ROOT="$(mktemp -d)"
trap 'rm -rf "$BUILD_ROOT"' EXIT

for lang_spec in "${LANGUAGES[@]}"; do
    IFS='|' read -r lang repo branch subdir <<< "$lang_spec"
    # Set default branch to master if not specified
    branch="${branch:-master}"
    echo "Building ${lang}…"

    # Unique temporary directory for this language
    lang_tmp="$(mktemp -d -p "$BUILD_ROOT")"
    # Clone the repository
    if git clone --depth 1 --branch "$branch" "$repo" "$lang_tmp" 2>/dev/null; then
        pushd "$lang_tmp" > /dev/null
        # Enter subdirectory if specified
        if [[ -n "$subdir" && -d "$subdir" ]]; then
            cd "$subdir"
        fi
        # Generate parser (if grammar.js exists)
        if [[ -f "grammar.js" ]]; then
            tree-sitter generate 2>/dev/null || true
        fi
        # Build the library
        if tree-sitter build 2>/dev/null; then
            # Find the built library
            # It may be placed in the current directory or in target/release
            built_lib=""
            for pattern in "${PREFIX}tree-sitter-${lang}${EXT}" \
                          "target/release/${PREFIX}tree-sitter-${lang}${EXT}" \
                          "target/debug/${PREFIX}tree-sitter-${lang}${EXT}" \
                          "*.${EXT}"; do
                # Only match the specific prefix
                matches=($pattern)
                if [[ ${#matches[@]} -gt 0 && -f "${matches[0]}" ]]; then
                    built_lib="${matches[0]}"
                    break
                fi
            done
            if [[ -n "$built_lib" ]]; then
                cp "$built_lib" "${GRAMMAR_DIR}/"
                echo "  → ${lang} built successfully"
            else
                echo "  → ${lang}: could not locate built library"
            fi
        else
            echo "  → ${lang}: build failed"
        fi
        popd > /dev/null
    else
        echo "  → ${lang}: failed to clone repository"
    fi
    # Clean up this language's source directory (optional, but we are in a temp that will be removed later)
done

# Special handling for markdown (already included as separate entry)
echo "All grammars processed."
echo "Built libraries are in ${GRAMMAR_DIR}"
ls -la "${GRAMMAR_DIR}/"
