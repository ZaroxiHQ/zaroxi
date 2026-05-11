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
    # Auto-detect host OS/ARCH and normalize to our directory names.
    UNAME_S=$(uname -s)
    UNAME_M=$(uname -m)

    case "$UNAME_S" in
        Linux*|linux*) OS="linux" ;;
        Darwin*|darwin*) OS="macos" ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT) OS="windows" ;;
        *) OS=$(echo "$UNAME_S" | tr '[:upper:]' '[:lower:]') ;;
    esac

    case "$UNAME_M" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        i386|i686) ARCH="x86_32" ;;
        *) ARCH="x86_64" ;;
    esac

    TARGET="${OS}-${ARCH}"
    echo "[fetch-grammars] detected host target: ${TARGET}"
else
    # Normalize provided TARGET into our expected form
    case "$TARGET" in
        *linux*) OS="linux" ;;
        *darwin*|*macos*) OS="macos" ;;
        *windows*) OS="windows" ;;
        *) OS="linux" ;;
    esac

    case "$TARGET" in
        *x86_64*|*amd64*) ARCH="x86_64" ;;
        *aarch64*|*arm64*) ARCH="aarch64" ;;
        *i386*|*i686*) ARCH="x86_32" ;;
        *) ARCH="x86_64" ;;
    esac

    TARGET="${OS}-${ARCH}"
    echo "[fetch-grammars] normalized target: ${TARGET}"
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

# Helper: try to run a command via npx if local binary is missing.
# This wrapper prefers a direct binary if present, otherwise will invoke npx.
# Special-case mapping: when callers request the `tree-sitter` binary via npx,
# use the `tree-sitter-cli` npm package name (npx expects package name).
#
# Usage: maybe_run_npx <cmd> [args...]
maybe_run_npx() {
  cmd="$1"; shift || true

  # If the requested binary is available on PATH, run it directly.
  if command -v "$cmd" >/dev/null 2>&1; then
    "$cmd" "$@"
    return $?
  fi

  # If npx is available, use it. Map common package names if necessary.
  if command -v npx >/dev/null 2>&1; then
    # When callers ask for "tree-sitter", prefer npx package "tree-sitter-cli".
    if [[ "$cmd" == "tree-sitter" ]]; then
      npx --yes tree-sitter-cli "$@"
      return $?
    fi

    # Otherwise try to npx the requested command as-is.
    npx --yes "$cmd" "$@"
    return $?
  fi

  # No candidate available
  return 127
}

# Step A — Build native libraries (existing behavior) ------------------------------------------------
echo "Attempting to build native grammar libraries (if toolchain available)..."

# Only run if tree-sitter CLI is present (native build uses tree-sitter too)
if ! maybe_run_npx tree-sitter --version >/dev/null 2>&1; then
  echo "tree-sitter CLI not available via PATH or npx. Native builds will be skipped."
else
  # Pre-scan languages and skip ones that already have native or wasm artifacts.
  # Use exact per-language checks (canonical wasm filenames and canonical native lib names)
  # instead of merely testing for the presence of a containing directory.
  TO_BUILD=()
  SKIPPED=()

  # Helper: return success (0) only if BOTH canonical per-language wasm and a native artifact exist.
  # Use the actual runtime/grammar directories (RUNTIME_DIR and GRAMMAR_DIR) when probing the FS.
  # Many earlier failures were caused by checking an undefined variable (RUNTIME_ROOT). Use the
  # concrete paths that are defined at the top of this script.
  language_has_artifact() {
    local lang="$1"
    local wasm_found=1
    local native_found=1

    # Canonical wasm filenames to check (ordered). Check both the runtime root and the platform-specific
    # grammar directory because packaging sometimes places wasm next to native libs.
    local wasm_names=(
      "${RUNTIME_DIR}/tree-sitter-${lang}.wasm"
      "${RUNTIME_DIR}/${lang}.wasm"
      "${RUNTIME_DIR}/language-${lang}.wasm"
      "${GRAMMAR_DIR}/tree-sitter-${lang}.wasm"
      "${GRAMMAR_DIR}/${lang}.wasm"
      "${GRAMMAR_DIR}/tree-sitter-${lang}.wasm"
    )

    for p in "${wasm_names[@]}"; do
      if [ -f "$p" ]; then
        echo "[fetch-grammars] found wasm for ${lang}: $p"
        wasm_found=0
        break
      fi
    done

    # Check for native artifacts in platform grammar dir (exact per-language names/patterns).
    # Native artifacts are expected under GRAMMAR_DIR; do not rely on runtime-root for native libs.
    if [ -d "${GRAMMAR_DIR}" ]; then
      # canonical library names
      if [ -f "${GRAMMAR_DIR}/${PREFIX}tree-sitter-${lang}${EXT}" ] || [ -f "${GRAMMAR_DIR}/libtree-sitter-${lang}${EXT}" ] || [ -f "${GRAMMAR_DIR}/tree-sitter-${lang}${EXT}" ]; then
        echo "[fetch-grammars] found native lib for ${lang} in ${GRAMMAR_DIR}"
        native_found=0
      fi

      # Node addon / .node named for that language (exact-match patterns)
      if ls "${GRAMMAR_DIR}/${lang}.node" 1> /dev/null 2>&1 || ls "${GRAMMAR_DIR}"/*"${lang}"*.node 1> /dev/null 2>&1; then
        echo "[fetch-grammars] found .node addon for ${lang} in ${GRAMMAR_DIR}"
        native_found=0
      fi

      # Broad check: first-class canonical patterns (libtree-sitter-<lang>* or tree-sitter-<lang>*).
      if ls "${GRAMMAR_DIR}"/libtree-sitter-"${lang}"* 1> /dev/null 2>&1 || ls "${GRAMMAR_DIR}"/tree-sitter-"${lang}"* 1> /dev/null 2>&1; then
        echo "[fetch-grammars] found native lib pattern for ${lang} in ${GRAMMAR_DIR}"
        native_found=0
      fi
    fi

    # Only consider the language "complete" if both wasm and native were found.
    if [ "$wasm_found" -eq 0 ] && [ "$native_found" -eq 0 ]; then
      return 0
    fi

    # Diagnostic messages to indicate what is missing for this language.
    if [ "$wasm_found" -ne 0 ] && [ "$native_found" -ne 0 ]; then
      echo "[fetch-grammars] missing both wasm and native for ${lang}"
    elif [ "$wasm_found" -ne 0 ]; then
      echo "[fetch-grammars] missing wasm for ${lang}"
    else
      echo "[fetch-grammars] missing native for ${lang}"
    fi

    return 1
  }

  for lang_spec in "${LANGUAGES[@]}"; do
      IFS='|' read -r lang repo branch subdir <<< "$lang_spec"
      # Normalize branch default
      branch="${branch:-master}"

      if language_has_artifact "${lang}"; then
        SKIPPED+=("${lang}")
        continue
      fi

      # If we reached here, neither exact wasm nor native artifact is present -> schedule build
      TO_BUILD+=("${lang_spec}")
  done

  if [ "${#TO_BUILD[@]}" -eq 0 ]; then
    echo "All requested languages already have both wasm and native artifacts under ${RUNTIME_DIR} and ${GRAMMAR_DIR}; skipping native build pass."
  else
    echo "Languages to build (missing wasm and/or native artifacts):"
    for s in "${TO_BUILD[@]}"; do
      IFS='|' read -r _lang _repo _branch _subdir <<< "$s"
      echo "  - ${_lang}"
    done

    if [ "${#SKIPPED[@]}" -gt 0 ]; then
      echo "Languages skipped (already complete): ${SKIPPED[*]}"
    fi

    # Temporary directory for all builds (will be cleaned up on exit)
    BUILD_ROOT="$(mktemp -d)"
    trap 'rm -rf "$BUILD_ROOT"' EXIT

    for lang_spec in "${TO_BUILD[@]}"; do
        IFS='|' read -r lang repo branch subdir <<< "$lang_spec"
        branch="${branch:-master}"
        echo "Building native ${lang}…"

        lang_tmp="$(mktemp -d -p "$BUILD_ROOT")"
        if git clone --depth 1 --branch "$branch" "$repo" "$lang_tmp" 2>/dev/null; then
            pushd "$lang_tmp" > /dev/null
            if [[ -n "$subdir" && -d "$subdir" ]]; then
                cd "$subdir"
            fi

            # If the grammar exposes JS dependencies (package.json) try installing them so
            # grammar.js can require modules like `tree-sitter-c/grammar`.
            if [[ -f "package.json" ]]; then
                echo "[fetch-grammars] npm install in $(pwd) to satisfy grammar.js dependencies"
                if command -v npm >/dev/null 2>&1; then
                    npm install --no-audit --no-fund --silent || echo "[fetch-grammars] npm install failed (continuing)"
                else
                    echo "[fetch-grammars] npm not available; skipping npm install"
                fi
            fi

            # If there's a grammar.js, attempt generate. Capture output so we can detect
            # "Cannot find module" errors and attempt to install the missing npm package.
            if [[ -f "grammar.js" ]]; then
                gen_out=""
                gen_status=0
                gen_out=$(maybe_run_npx tree-sitter generate 2>&1) || gen_status=$?
                if [[ $gen_status -ne 0 ]] && echo "$gen_out" | grep -q "Cannot find module"; then
                    missing_pkg=$(echo "$gen_out" | sed -n "s/.*Cannot find module '\([^']*\)'.*/\1/p" | head -n1)
                    if [[ -n "$missing_pkg" && "$(command -v npm >/dev/null && echo yes || true)" == "yes" ]]; then
                        echo "[fetch-grammars] Detected missing JS module in grammar.js: $missing_pkg"
                        echo "[fetch-grammars] Attempting npm install $missing_pkg in $(pwd)"
                        npm install --no-audit --no-fund --silent "$missing_pkg" || echo "[fetch-grammars] npm install $missing_pkg failed (continuing)"
                        # Retry generate once
                        gen_out=$(maybe_run_npx tree-sitter generate 2>&1) || true
                    fi
                fi
            fi

            # Try to build native artifacts. Capture output to detect MODULE_NOT_FOUND and retry npm install where appropriate.
            build_status=0
            build_out=""
            build_out=$(maybe_run_npx tree-sitter build 2>&1) || build_status=$?

            if [[ $build_status -ne 0 ]] && echo "$build_out" | grep -q "Cannot find module"; then
                # Extract the first missing module name and attempt to npm install it, then retry build once.
                missing_pkg=$(echo "$build_out" | sed -n "s/.*Cannot find module '\([^']*\)'.*/\1/p" | head -n1)
                if [[ -n "$missing_pkg" && "$(command -v npm >/dev/null && echo yes || true)" == "yes" ]]; then
                    echo "[fetch-grammars] Detected missing JS module during build: $missing_pkg"
                    echo "[fetch-grammars] Attempting npm install $missing_pkg in $(pwd)"
                    npm install --no-audit --no-fund --silent "$missing_pkg" || echo "[fetch-grammars] npm install $missing_pkg failed (continuing)"
                    # Retry build
                    build_status=0
                    build_out=$(maybe_run_npx tree-sitter build 2>&1) || build_status=$?
                fi
            fi

            if [[ $build_status -eq 0 ]]; then
                built_lib=""
                # Broaden the search to prefer platform-native library names like libtree-sitter-<lang>.<ext>,
                # but also accept other common locations and Node addons (.node).
                for pattern in "libtree-sitter-${lang}${EXT}" \
                              "${PREFIX}tree-sitter-${lang}${EXT}" \
                              "parser${EXT}" \
                              "target/release/${PREFIX}tree-sitter-${lang}${EXT}" \
                              "target/debug/${PREFIX}tree-sitter-${lang}${EXT}" \
                              "target/release/*.so" \
                              "target/debug/*.so" \
                              "*${EXT}" \
                              "*.node" \
                              "Release/*.node" \
                              "build/Release/*.node" \
                              "target/*/${PREFIX}tree-sitter-${lang}${EXT}" \
                              "target/*/*/${PREFIX}tree-sitter-${lang}${EXT}"; do
                  matches=($pattern)
                  if [[ ${#matches[@]} -gt 0 && -f "${matches[0]}" ]]; then
                      built_lib="${matches[0]}"
                      break
                  fi
                done

                if [[ -n "$built_lib" ]]; then
                    mkdir -p "${GRAMMAR_DIR}"
                    # Normalize destination filename for native libraries: prefer libtree-sitter-<lang><EXT>
                    base="$(basename "$built_lib")"
                    dest="${GRAMMAR_DIR}/$base"

                    # If the artifact is a generic "parser" binary (common in some repos),
                    # rename it to the canonical libtree-sitter-<lang><EXT> so downstream
                    # runtime discovery works consistently.
                    if [[ "${base}" == "parser${EXT}" || "${base}" == "parser" || "${base}" == "parser.so" || "${base}" == "parser.dylib" ]]; then
                      dest="${GRAMMAR_DIR}/${PREFIX}tree-sitter-${lang}${EXT}"
                    fi

                    # Some bindings produce names like tree_sitter_<lang>_binding.so or tree_sitter_<lang>.so
                    # map those to libtree-sitter-<lang><EXT> as well.
                    if [[ "${base}" == tree_sitter* || "${base}" == *"tree_sitter"* ]]; then
                      dest="${GRAMMAR_DIR}/${PREFIX}tree-sitter-${lang}${EXT}"
                    fi

                    # If file already uses canonical libtree-sitter name, keep it.
                    if [[ "${base}" == "${PREFIX}tree-sitter-${lang}${EXT}" || "${base}" == "libtree-sitter-${lang}${EXT}" || "${base}" == "${PREFIX}tree-sitter-${lang}${EXT}" ]]; then
                      dest="${GRAMMAR_DIR}/${base}"
                    fi

                    # Avoid overwriting a .node addon when the artifact is a Node addon; copy as-is.
                    if [[ "${base}" == *.node ]]; then
                      dest="${GRAMMAR_DIR}/${base}"
                    fi

                    cp -v "$built_lib" "$dest"
                    echo "  → ${lang} native artifact copied to ${GRAMMAR_DIR} (source: ${built_lib} -> dest: ${dest})"
                else
                    # If the build succeeded but we couldn't locate a canonical native lib, attempt a more thorough search
                    # for libtree-sitter-* (or tree-sitter-*) anywhere under the repo clone and copy first match.
                    found=""
                    if command -v find >/dev/null 2>&1; then
                      while IFS= read -r f; do
                        if [[ -z "$found" ]]; then
                          found="$f"
                          break
                        fi
                      done < <(find . -type f -regextype posix-extended -regex '.*(libtree-sitter-|tree-sitter-).*('"${EXT#"."}"'|node)$' 2>/dev/null || true)
                    fi

                    if [[ -n "$found" ]]; then
                        mkdir -p "${GRAMMAR_DIR}"
                        base="$(basename "$found")"
                        dest="${GRAMMAR_DIR}/$base"

                        if [[ "${base}" == "parser${EXT}" || "${base}" == "parser" || "${base}" == "parser.so" || "${base}" == "parser.dylib" ]]; then
                          dest="${GRAMMAR_DIR}/${PREFIX}tree-sitter-${lang}${EXT}"
                        fi
                        if [[ "${base}" == tree_sitter* || "${base}" == *"tree_sitter"* ]]; then
                          dest="${GRAMMAR_DIR}/${PREFIX}tree-sitter-${lang}${EXT}"
                        fi
                        if [[ "${base}" == *.node ]]; then
                          dest="${GRAMMAR_DIR}/${base}"
                        fi

                        cp -v "$found" "$dest"
                        echo "  → ${lang} native artifact copied to ${GRAMMAR_DIR} (discovered: ${found} -> dest: ${dest})"
                    else
                        # If still nothing, print helpful diagnostics.
                        echo "  → ${lang}: build succeeded (exit code 0) but no native artifact (.${EXT} or .node) was found"
                        echo "     Search output (first 200 chars):"
                        echo "     ${build_out:0:200}"
                    fi
                fi

                # Additionally, attempt to produce per-language WASM parser files (tree-sitter CLI supports `build-wasm`).
                # Try multiple strategies (best‑effort) because different grammars expose different build flows:
                # 1) maybe_run_npx tree-sitter build-wasm (preferred mapping to tree-sitter-cli)
                # 2) explicit npx --yes tree-sitter-cli build-wasm
                # 3) npm run build-wasm (if a package.json script is present)
                # 4) best-effort emcc compile of parser.c (only if emscripten is installed)
                wasm_built=false

                # Strategy 1: preferred wrapper (maps "tree-sitter" -> "tree-sitter-cli" via maybe_run_npx)
                if maybe_run_npx tree-sitter build-wasm >/dev/null 2>&1; then
                  echo "  → ${lang}: tree-sitter build-wasm invoked (strategy: maybe_run_npx)"
                  wasm_built=true
                else
                  # Strategy 2: try explicit npx invocation of tree-sitter-cli
                  if command -v npx >/dev/null 2>&1; then
                    if npx --yes tree-sitter-cli build-wasm >/dev/null 2>&1; then
                      echo "  → ${lang}: tree-sitter-cli build-wasm invoked via npx"
                      wasm_built=true
                    fi
                  fi
                fi

                # Strategy 3: npm script provided by grammar repository (common in some repos)
                if ! $wasm_built && [ -f "package.json" ]; then
                  if command -v npm >/dev/null 2>&1; then
                    # Detect a build-wasm script in package.json using a simple grep (avoid jq dependency).
                    if grep -q "\"build-wasm\"" package.json 2>/dev/null || grep -q "\"buildWasm\"" package.json 2>/dev/null || grep -q "\"build:wasm\"" package.json 2>/dev/null; then
                      echo "  → ${lang}: running npm run build-wasm (detected script)"
                      if npm run --silent build-wasm >/dev/null 2>&1 || npm run --silent build:wasm >/dev/null 2>&1; then
                        echo "  → ${lang}: npm build-wasm script succeeded"
                        wasm_built=true
                      else
                        echo "  → ${lang}: npm build-wasm script failed (continuing)"
                      fi
                    fi
                  fi
                fi

                # Strategy 4: emscripten fallback (best-effort)
                if ! $wasm_built && command -v emcc >/dev/null 2>&1; then
                  # Look for common parser sources
                  local_src=""
                  if [[ -f "src/parser.c" ]]; then
                    local_src="src/parser.c"
                  elif [[ -f "parser.c" ]]; then
                    local_src="parser.c"
                  fi

                  if [[ -n "$local_src" ]]; then
                    outname="tree-sitter-${lang}.wasm"
                    echo "  → ${lang}: attempting emcc compile of ${local_src} -> ${outname} (best-effort)"
                    # Minimal flags -- may not produce a fully usable runtime for all grammars but can succeed on some.
                    if emcc "$local_src" -O3 -s WASM=1 -s SIDE_MODULE=1 -o "$outname" >/dev/null 2>&1; then
                      mv -v "$outname" "${RUNTIME_ROOT}/" || true
                      echo "  → ${lang}: emcc produced wasm -> moved to ${RUNTIME_ROOT}/${outname}"
                      wasm_built=true
                    else
                      echo "  → ${lang}: emcc compile failed (continuing)"
                    fi
                  fi
                fi

                # Regardless of whether build-wasm reported success, some tools (npm installs, tree-sitter-cli, or bindings)
                # may have produced .wasm artifacts under node_modules/ or nested build dirs. Do a robust scan and move any
                # discovered .wasm into the OS-specific GRAMMAR_DIR alongside the native library and normalize the name to
                # tree-sitter-<lang>.wasm so the frontend loader can find it predictably.
                mkdir -p "${GRAMMAR_DIR}"
                moved_any=false

                # We'll look for wasm in several likely locations: current tree, node_modules, build/target dirs.
                if command -v find >/dev/null 2>&1; then
                  # Discover both files and (rare) directories that may contain wasm artifacts,
                  # but ensure we ultimately move/copy only a real .wasm file (not a directory).
                  while IFS= read -r candidate; do
                    wasmfile="$candidate"

                    # If candidate is a directory (some builds produce a dir named *.wasm), try to
                    # locate the first .wasm file inside it and use that.
                    if [[ -d "$wasmfile" ]]; then
                      inner="$(find "$wasmfile" -maxdepth 2 -type f -name '*.wasm' -print -quit 2>/dev/null || true)"
                      if [[ -n "$inner" && -f "$inner" ]]; then
                        wasmfile="$inner"
                      else
                        echo "  → ${lang}: discovered directory ${candidate} but no .wasm inside; skipping"
                        continue
                      fi
                    fi

                    # Skip non-files (safety)
                    if [[ ! -f "$wasmfile" ]]; then
                      continue
                    fi

                    # canonical destination filename
                    destname="tree-sitter-${lang}.wasm"
                    destpath="${GRAMMAR_DIR}/${destname}"

                    # If the file already is the canonical path under GRAMMAR_DIR, just mark moved and continue.
                    if [[ "$(realpath -m "$wasmfile")" == "$(realpath -m "$destpath")" ]]; then
                      moved_any=true
                      continue
                    fi

                    # If destination exists and identical, skip; otherwise back it up and copy/move.
                    if [[ -f "${destpath}" ]]; then
                      if cmp -s "$wasmfile" "${destpath}"; then
                        echo "  → ${lang}: wasm ${destname} already present in ${GRAMMAR_DIR} and identical; skipping"
                        moved_any=true
                        continue
                      else
                        echo "  → ${lang}: existing ${destname} differs in ${GRAMMAR_DIR}; backing up"
                        mv -v "${destpath}" "${destpath}.bak" || true
                      fi
                    fi

                    # Prefer moving when wasmfile is within the temporary clone dir; otherwise copy.
                    if mv -v "$wasmfile" "${destpath}" 2>/dev/null; then
                      echo "  → ${lang}: moved wasm $(basename "${destpath}") -> ${destpath}"
                    elif cp -v "$wasmfile" "${destpath}" 2>/dev/null; then
                      echo "  → ${lang}: copied wasm $(basename "$wasmfile") -> ${destpath}"
                    else
                      echo "  → ${lang}: failed to relocate wasm $wasmfile (continuing)"
                      continue
                    fi

                    moved_any=true
                    wasm_built=true
                  done < <(find . \( -type f -name '*.wasm' -o -type d -name '*.wasm' \) -o -path './node_modules/*' -prune 2>/dev/null | sort -u)
                else
                  # Fallback: simple glob checks in a few known dirs. Handle directories as well.
                  candidates=(./*.wasm ./node_modules/*/*.wasm ./build/*.wasm ./target/*/*.wasm)
                  for w in "${candidates[@]}"; do
                    for f in $w; do
                      # If candidate is a directory, attempt to find a .wasm file inside it.
                      actual=""
                      if [[ -d "$f" ]]; then
                        actual="$(find "$f" -maxdepth 2 -type f -name '*.wasm' -print -quit 2>/dev/null || true)"
                        if [[ -z "$actual" ]]; then
                          echo "  → ${lang}: candidate directory $f contains no .wasm; skipping"
                          continue
                        fi
                      else
                        actual="$f"
                      fi

                      if [[ -f "$actual" ]]; then
                        destname="tree-sitter-${lang}.wasm"
                        destpath="${GRAMMAR_DIR}/${destname}"
                        if [[ -f "${destpath}" ]]; then
                          if cmp -s "$actual" "${destpath}"; then
                            echo "  → ${lang}: wasm ${destname} already present in ${GRAMMAR_DIR}; skipping"
                            moved_any=true
                            wasm_built=true
                            continue
                          fi
                        fi
                        cp -v "$actual" "${destpath}" || true
                        echo "  → ${lang}: copied wasm $(basename "$actual") -> ${destpath}"
                        moved_any=true
                        wasm_built=true
                      fi
                    done
                  done
                fi

                if ! $wasm_built; then
                  echo "  → ${lang}: tree-sitter build-wasm unavailable or failed for this grammar (no wasm discovered)"
                else
                  echo "  → ${lang}: wasm artifact ensured at ${GRAMMAR_DIR}/tree-sitter-${lang}.wasm"
                fi
            else
                # Build failed; print a concise hint including captured stderr to help diagnosis.
                echo "  → ${lang}: native build failed (continuing)"
                echo "     Build stderr preview:"
                echo "     ${build_out:0:400}"
                # If build failed with MODULE_NOT_FOUND, try scanning for wasm outputs anyway (some repos emit wasm even on partial failures)
                if echo "${build_out}" | grep -q "Cannot find module" || echo "${build_out}" | grep -q "MODULE_NOT_FOUND"; then
                  if command -v find >/dev/null 2>&1; then
                    while IFS= read -r wasmfile; do
                      mkdir -p "${RUNTIME_ROOT}"
                      base="$(basename "$wasmfile")"
                      destname="$base"
                      if [[ "$base" != tree-sitter-* && "$base" != "${lang}.wasm" ]]; then
                        destname="tree-sitter-${lang}.wasm"
                      fi
                      cp -v "$wasmfile" "${RUNTIME_ROOT}/${destname}" || true
                      echo "  → ${lang}: copied wasm $(basename "$wasmfile") to ${RUNTIME_ROOT}/ (partial build -> ${destname})"
                    done < <(find . -maxdepth 4 -type f -name '*.wasm' 2>/dev/null || true)
                  fi
                fi
            fi

            popd > /dev/null
        else
            echo "  → ${lang}: failed to clone repository (continuing)"
        fi
    done

    echo "Native grammar pass complete. Built libs (if any) are in ${GRAMMAR_DIR}"
    ls -la "${GRAMMAR_DIR}" || true
  fi
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
            # Keep produced .wasm inside the grammar directory and normalize filename to tree-sitter-<lang>.wasm.
            shopt -s nullglob
            langname="$(basename "$lang_dir")"
            for w in *.wasm; do
              if [[ -f "$w" ]]; then
                base="$(basename "$w")"
                destname="$base"
                if [[ "$base" != tree-sitter-* && "$base" != "${langname}.wasm" ]]; then
                  destname="tree-sitter-${langname}.wasm"
                fi
                destpath="${lang_dir}/${destname}"
                if [[ -f "${destpath}" ]]; then
                  if cmp -s "$w" "${destpath}"; then
                    echo "[wasm] ${destname} already present in ${lang_dir}; skipping"
                    WASM_BUILT+=("$(basename "${destpath}")")
                    continue
                  else
                    mv -v "${destpath}" "${destpath}.bak" || true
                  fi
                fi
                mv -v "$w" "${destpath}" || cp -v "$w" "${destpath}" || true
                WASM_BUILT+=("$(basename "${destpath}")")
              fi
            done
            shopt -u nullglob
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
              shopt -s nullglob
              langname="$(basename "$gm")"
              for w in *.wasm; do
                if [[ -f "$w" ]]; then
                  base="$(basename "$w")"
                  destname="$base"
                  if [[ "$base" != tree-sitter-* && "$base" != "${langname}.wasm" ]]; then
                    destname="tree-sitter-${langname}.wasm"
                  fi
                  destpath="${gm}/${destname}"
                  if [[ -f "${destpath}" ]]; then
                    if cmp -s "$w" "${destpath}"; then
                      echo "[wasm] ${destname} already present in ${gm}; skipping"
                      WASM_BUILT+=("$(basename "${destpath}")")
                      continue
                    else
                      mv -v "${destpath}" "${destpath}.bak" || true
                    fi
                  fi
                  mv -v "$w" "${destpath}" || cp -v "$w" "${destpath}" || true
                  WASM_BUILT+=("$(basename "${destpath}")")
                fi
              done
              shopt -u nullglob
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
#!/usr/bin/env bash
set -euo pipefail

# fetch-grammars.sh
#
# Purpose:
# - Ensure engine runtime `tree-sitter.wasm` is present under this runtime directory.
# - Optionally attempt a best-effort build of per-language WASM parsers from local grammar sources.
# - Allow downloading specific prebuilt language wasm artifacts.
#
# Important policy:
# - This script will NOT clone remote repositories. It only works with sources already present
#   under the `languages/` or `grammars/` directories inside this runtime directory.
# - If a grammar's `grammar.js` requires npm packages, the script will attempt `npm install`
#   inside that grammar directory (best-effort) before running `tree-sitter-cli build-wasm`.
#
# Usage:
#   ./fetch-grammars.sh                # check engine + list existing wasm files
#   ./fetch-grammars.sh --build        # attempt to build per-language wasm from local grammar sources
#   ./fetch-grammars.sh --fetch rust=https://.../tree-sitter-rust.wasm toml=...  # download given wasm files
#
# Location: crates/zaroxi-lang-syntax/runtime/treesitter
# Example expected per-language filenames: tree-sitter-rust.wasm, tree-sitter-toml.wasm

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNTIME_DIR="$SCRIPT_DIR"
LANGUAGES_DIR="$RUNTIME_DIR/languages"
GRAMMARS_DIR="$RUNTIME_DIR/grammars"

print_help() {
  cat <<EOF
fetch-grammars.sh - prepare web-tree-sitter runtime

Usage:
  $0 [--build] [--fetch lang=url ...] [--help]

Options:
  --build         Attempt best-effort build of per-language .wasm files from local grammar sources.
  --fetch k=URL   Download a prebuilt wasm for language key (repeatable).
  --help          Show this help and exit.

Notes:
  - Building wasm requires a C toolchain (gcc/clang, make), python3 and tree-sitter CLI (node tree-sitter-cli).
  - The script will NOT clone remote repos. Put grammar sources under 'languages/' or 'grammars/'.
EOF
}

# Helpers
run_tree_sitter_cli() {
  # Prefer npx tree-sitter-cli if available, otherwise use tree-sitter if present.
  if command -v npx >/dev/null 2>&1; then
    npx --yes tree-sitter-cli "$@"
    return $?
  elif command -v tree-sitter >/dev/null 2>&1; then
    tree-sitter "$@"
    return $?
  else
    return 127
  fi
}

download_url_to() {
  local url="$1"
  local out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$out" "$url"
    return $?
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$out" "$url"
    return $?
  else
    echo "[fetch-grammars] no curl or wget available to download $url"
    return 2
  fi
}

# Parse args
DO_BUILD=false
declare -a FETCH_PAIRS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build) DO_BUILD=true; shift ;;
    --fetch) shift; while [[ $# -gt 0 && "$1" != --* ]]; do FETCH_PAIRS+=("$1"); shift; done ;;
    --help) print_help; exit 0 ;;
    *) echo "[fetch-grammars] unknown arg: $1"; print_help; exit 1 ;;
  esac
done

echo "[fetch-grammars] runtime dir: $RUNTIME_DIR"

# 1) Ensure engine runtime wasm exists
if [ -f "$RUNTIME_DIR/tree-sitter.wasm" ]; then
  echo "[fetch-grammars] engine runtime present: $RUNTIME_DIR/tree-sitter.wasm"
else
  # Try copying from likely node_modules locations relative to repo
  NODE_CANDIDATES=(
    "$PWD/node_modules/web-tree-sitter/tree-sitter.wasm"
    "$PWD/../node_modules/web-tree-sitter/tree-sitter.wasm"
    "$PWD/../../node_modules/web-tree-sitter/tree-sitter.wasm"
  )
  copied=false
  for c in "${NODE_CANDIDATES[@]}"; do
    if [ -f "$c" ]; then
      echo "[fetch-grammars] copying engine wasm from $c"
      cp -v "$c" "$RUNTIME_DIR/tree-sitter.wasm"
      copied=true
      break
    fi
  done
  if ! $copied; then
    # fallback: try download from unpkg CDN
    if command -v curl >/dev/null 2>&1; then
      echo "[fetch-grammars] attempting to download engine wasm from unpkg"
      if curl -fsSL -o "$RUNTIME_DIR/tree-sitter.wasm" "https://unpkg.com/web-tree-sitter/tree-sitter.wasm"; then
        echo "[fetch-grammars] downloaded engine wasm to $RUNTIME_DIR/tree-sitter.wasm"
      else
        echo "[fetch-grammars] failed to obtain engine wasm. Place web-tree-sitter's tree-sitter.wasm in $RUNTIME_DIR"
      fi
    else
      echo "[fetch-grammars] engine wasm missing and curl not available. Place tree-sitter.wasm into $RUNTIME_DIR"
    fi
  fi
fi

# Brief header check if present
if [ -f "$RUNTIME_DIR/tree-sitter.wasm" ] && command -v hexdump >/dev/null 2>&1; then
  hdr=$(hexdump -n 4 -v -e '1/1 "%02x "' "$RUNTIME_DIR/tree-sitter.wasm" || true)
  echo "[fetch-grammars] engine header bytes: $hdr"
fi

# 2) Process explicit fetch pairs (lang=url)
if [ "${#FETCH_PAIRS[@]}" -gt 0 ]; then
  echo "[fetch-grammars] downloading requested wasm files..."
  for pair in "${FETCH_PAIRS[@]}"; do
    # support "lang=url" form
    if [[ "$pair" =~ ^([^=]+)=(.+)$ ]]; then
      lang="${BASH_REMATCH[1]}"
      url="${BASH_REMATCH[2]}"
      fname="tree-sitter-${lang}.wasm"
      out="$RUNTIME_DIR/$fname"
      echo "[fetch-grammars] fetching $lang -> $fname from $url"
      if download_url_to "$url" "$out"; then
        echo "[fetch-grammars] saved $out"
      else
        echo "[fetch-grammars] failed to download $url"
      fi
    else
      echo "[fetch-grammars] invalid fetch pair: $pair (expected lang=url)"
    fi
  done
fi

# 3) Optionally attempt to build per-language wasm files from local grammar sources
WASM_BUILT=()

if $DO_BUILD; then
  echo "[fetch-grammars] build mode enabled: attempting best-effort builds from local grammar sources"
  echo "[fetch-grammars] Note: will perform per-language checks and skip only the specific languages that already have exact artifacts. No global short-circuit will be applied."

  try_build() {
    local dir="$1"
    echo "[fetch-grammars] attempting build in: $dir"

    # Determine language name (directory basename)
    langname="$(basename "$dir")"

    # 0) If canonical wasm already exists in the runtime dir, skip entirely.
    if [ -f "${RUNTIME_DIR}/tree-sitter-${langname}.wasm" ]; then
      echo "[fetch-grammars] skipping ${langname}: ${RUNTIME_DIR}/tree-sitter-${langname}.wasm already present"
      return 0
    fi

    # 1) If this grammar directory already contains .wasm artifacts, move them and skip build.
    shopt -s nullglob
    found_wasm=false
    for w in "$dir"/*.wasm; do
      if [ -f "$w" ]; then
        echo "[fetch-grammars] found existing wasm in ${dir}: ${w}; relocating to runtime"
        if mv -v "$w" "$RUNTIME_DIR"/ 2>/dev/null; then
          WASM_BUILT+=("$(basename "$w")")
        else
          cp -v "$w" "$RUNTIME_DIR"/ || true
          WASM_BUILT+=("$(basename "$w")")
        fi
        found_wasm=true
      fi
    done
    shopt -u nullglob

    if $found_wasm; then
      echo "[fetch-grammars] existing wasm moved for ${langname}; skipping build"
      return 0
    fi

    # 2) If a native artifact already exists in the platform grammar dir, skip native build.
    if [ -d "${GRAMMAR_DIR}" ]; then
      if [ -f "${GRAMMAR_DIR}/${PREFIX}tree-sitter-${langname}${EXT}" ] || [ -f "${GRAMMAR_DIR}/libtree-sitter-${langname}${EXT}" ] || [ -f "${GRAMMAR_DIR}/tree-sitter-${langname}${EXT}" ] || ls "${GRAMMAR_DIR}"/*"${langname}"*.node 1> /dev/null 2>&1; then
        echo "[fetch-grammars] native artifact for ${langname} already present in ${GRAMMAR_DIR}; skipping native build"
        return 0
      fi
    fi

    # 3) Locate the best directory inside the cloned repo to run tree-sitter commands.
    #      - Prefer the provided directory itself if it contains grammar.js / grammar.json / parser.c / binding.gyp
    #      - Otherwise search up to a modest depth for any of these indicators and use the containing directory.
    build_dir=""
    if [ -d "$dir" ]; then
      # If the top-level dir looks buildable, use it
      if [ -f "$dir/grammar.js" ] || [ -f "$dir/grammar.json" ] || [ -f "$dir/src/parser.c" ] || [ -f "$dir/binding.gyp" ]; then
        build_dir="$dir"
      else
        # Search for plausible grammar roots (grammar.json, grammar.js, parser.c, binding.gyp)
        # prefer shallower matches and those whose path includes common subdir names (typescript, tsx, src, split_parser)
        if command -v find >/dev/null 2>&1; then
          while IFS= read -r candidate; do
            # candidate is the file path; take its directory
            candidate_dir="$(dirname "$candidate")"
            build_dir="$candidate_dir"
            break
          done < <(find "$dir" -maxdepth 4 -type f \( -name 'grammar.json' -o -name 'grammar.js' -o -name 'parser.c' -o -name 'binding.gyp' \) -print 2>/dev/null)
        fi

        # Fallback: if there's a 'src' directory that looks promising, use it
        if [ -z "$build_dir" ] && [ -d "$dir/src" ]; then
          build_dir="$dir/src"
        fi

        # As a last resort use the top-level dir
        if [ -z "$build_dir" ]; then
          build_dir="$dir"
        fi
      fi
    else
      # If the provided path isn't a directory (shouldn't happen), bail.
      echo "[fetch-grammars] build dir $dir does not exist; skipping"
      return 1
    fi

    echo "[fetch-grammars] chosen build directory for ${langname}: ${build_dir}"

    # 4) If the grammar exposes JS dependencies (package.json) try installing them so grammar.js can require modules.
    if command -v npm >/dev/null 2>&1 && [ -f "${build_dir}/package.json" ]; then
      echo "[fetch-grammars] npm install in ${build_dir} to satisfy grammar.js dependencies"
      (cd "$build_dir" && npm install --no-audit --no-fund --silent) || echo "[fetch-grammars] npm install failed in ${build_dir} (continuing)"
    fi

    # 5) Attempt native build (tree-sitter build) in the chosen build_dir.
    pushd "$build_dir" > /dev/null || return 1

    build_status=0
    build_out=""
    build_out=$(maybe_run_npx tree-sitter build 2>&1) || build_status=$?

    # If MODULE_NOT_FOUND errors appear, attempt to npm install the missing package then retry once.
    if [[ $build_status -ne 0 ]] && echo "$build_out" | grep -q "Cannot find module"; then
      missing_pkg=$(echo "$build_out" | sed -n "s/.*Cannot find module '\([^']*\)'.*/\1/p" | head -n1)
      if [[ -n "$missing_pkg" && "$(command -v npm >/dev/null && echo yes || true)" == "yes" ]]; then
        echo "[fetch-grammars] Detected missing JS module during build: $missing_pkg"
        echo "[fetch-grammars] Attempting npm install $missing_pkg in $(pwd)"
        npm install --no-audit --no-fund --silent "$missing_pkg" || echo "[fetch-grammars] npm install $missing_pkg failed (continuing)"
        build_status=0
        build_out=$(maybe_run_npx tree-sitter build 2>&1) || build_status=$?
      fi
    fi

    if [[ $build_status -eq 0 ]]; then
      # Try to locate any produced native artifact under the build_dir (or nearby target dirs)
      built_lib=""
      search_patterns=(
        "${build_dir}/libtree-sitter-${langname}${EXT}"
        "${build_dir}/${PREFIX}tree-sitter-${langname}${EXT}"
        "${build_dir}/parser${EXT}"
        "${build_dir}/target/release/${PREFIX}tree-sitter-${langname}${EXT}"
        "${build_dir}/target/debug/${PREFIX}tree-sitter-${langname}${EXT}"
        "${build_dir}/target/release/*${EXT}"
        "${build_dir}/target/debug/*${EXT}"
        "${build_dir}/*${EXT}"
        "${build_dir}/*.node"
        "${build_dir}/Release/*.node"
        "${build_dir}/build/Release/*.node"
      )

      for pattern in "${search_patterns[@]}"; do
        matches=( $pattern )
        if [[ ${#matches[@]} -gt 0 && -f "${matches[0]}" ]]; then
          built_lib="${matches[0]}"
          break
        fi
      done

      # If nothing found, perform a broader repo-wide search for libtree-sitter-* or tree-sitter-* artifacts.
      if [[ -z "$built_lib" ]] && command -v find >/dev/null 2>&1; then
        found="$(find "$dir" -maxdepth 5 -type f -regextype posix-extended -regex '.*(libtree-sitter-|tree-sitter-).*('"${EXT#"."}"'|node)$' -print -quit 2>/dev/null || true)"
        if [[ -n "$found" ]]; then
          built_lib="$found"
        fi
      fi

      if [[ -n "$built_lib" ]]; then
        mkdir -p "${GRAMMAR_DIR}"
        base="$(basename "$built_lib")"
        dest="${GRAMMAR_DIR}/$base"

        # Normalize common names to canonical libtree-sitter-<lang><EXT>
        if [[ "${base}" == "parser${EXT}" || "${base}" == "parser" || "${base}" == "parser.so" || "${base}" == "parser.dylib" ]]; then
          dest="${GRAMMAR_DIR}/${PREFIX}tree-sitter-${langname}${EXT}"
        fi
        if [[ "${base}" == tree_sitter* || "${base}" == *"tree_sitter"* ]]; then
          dest="${GRAMMAR_DIR}/${PREFIX}tree-sitter-${langname}${EXT}"
        fi
        if [[ "${base}" == *.node ]]; then
          dest="${GRAMMAR_DIR}/${base}"
        fi
        if cp -v "$built_lib" "$dest"; then
          echo "  → ${langname} native artifact copied to ${GRAMMAR_DIR} (source: ${built_lib} -> dest: ${dest})"
        else
          echo "  → ${langname}: failed to copy detected native artifact ${built_lib} (continuing)"
        fi
      else
        echo "  → ${langname}: build succeeded (exit code 0) but no native artifact (.${EXT} or .node) was found"
        echo "     Build stderr preview:"
        echo "     ${build_out:0:200}"
      fi

      # After native build try to produce/move any wasm artifacts produced in the build_dir or repo
      wasm_built=false
      if maybe_run_npx tree-sitter build-wasm >/dev/null 2>&1; then
        wasm_built=true
      elif command -v npx >/dev/null 2>&1; then
        if npx --yes tree-sitter-cli build-wasm >/dev/null 2>&1; then
          wasm_built=true
        fi
      fi

      # Scan for any .wasm files produced anywhere under the cloned repo and relocate them.
      shopt -s nullglob
      if command -v find >/dev/null 2>&1; then
        while IFS= read -r wasmfile; do
          if [[ -f "$wasmfile" ]]; then
            base="$(basename "$wasmfile")"
            destname="$base"
            # Normalize non-canonical names
            if [[ "$base" != tree-sitter-* && "$base" != "${langname}.wasm" ]]; then
              destname="tree-sitter-${langname}.wasm"
            fi
            destpath="${RUNTIME_DIR}/${destname}"
            if [[ -f "${destpath}" ]]; then
              if cmp -s "$wasmfile" "${destpath}"; then
                echo "  → ${langname}: wasm ${destname} already present in ${RUNTIME_DIR}; skipping"
                WASM_BUILT+=("$(basename "${destpath}")")
                continue
              else
                mv -v "${destpath}" "${destpath}.bak" || true
              fi
            fi
            if mv -v "$wasmfile" "${destpath}" 2>/dev/null || cp -v "$wasmfile" "${destpath}" 2>/dev/null; then
              echo "  → ${langname}: relocated wasm -> ${destpath}"
              WASM_BUILT+=("$(basename "${destpath}")")
              wasm_built=true
            fi
          fi
        done < <(find "$dir" -maxdepth 6 -type f -name '*.wasm' 2>/dev/null | sort -u)
      fi
      shopt -u nullglob

      if ! $wasm_built; then
        echo "  → ${langname}: tree-sitter build-wasm unavailable or produced no wasm for this grammar"
      else
        echo "  → ${langname}: wasm artifact ensured (if produced) at ${RUNTIME_DIR}/tree-sitter-${langname}.wasm"
      fi
    else
      # Build failed; print a concise hint including captured stderr to help diagnosis.
      echo "  → ${langname}: native build failed (continuing)"
      echo "     Build stderr preview:"
      echo "     ${build_out:0:400}"
      # If build failed with MODULE_NOT_FOUND, try scanning for wasm outputs anyway (some repos emit wasm even on partial failures)
      if echo "${build_out}" | grep -q "Cannot find module" || echo "${build_out}" | grep -q "MODULE_NOT_FOUND"; then
        if command -v find >/dev/null 2>&1; then
          while IFS= read -r wasmfile; do
            mkdir -p "${RUNTIME_DIR}"
            base="$(basename "$wasmfile")"
            destname="$base"
            if [[ "$base" != tree-sitter-* && "$base" != "${langname}.wasm" ]]; then
              destname="tree-sitter-${langname}.wasm"
            fi
            cp -v "$wasmfile" "${RUNTIME_DIR}/${destname}" || true
            echo "  → ${langname}: copied wasm $(basename "$wasmfile") to ${RUNTIME_DIR}/ (partial build -> ${destname})"
          done < <(find "$dir" -maxdepth 6 -type f -name '*.wasm' 2>/dev/null || true)
        fi
      fi
    fi

    popd > /dev/null
  }

  # Search canonical local grammar dirs and attempt builds only where sensible.
  # Many grammar repositories place buildable sources in nested subdirectories
  # (e.g. typescript/src, tsx/src, tree-sitter-markdown/src, split_parser/, etc).
  # To handle those layouts we inspect the top-level dir first and then scan
  # nested subdirs up to depth 3 for indicators of buildable grammar sources.
  if [ -d "$LANGUAGES_DIR" ]; then
    for d in "$LANGUAGES_DIR"/*; do
      if [ -d "$d" ]; then
        # Prefer immediate/top-level grammar sources
        if [ -f "$d/grammar.js" ] || [ -f "$d/src/parser.c" ] || [ -f "$d/binding.gyp" ]; then
          try_build "$d"
          continue
        fi

        # Otherwise search nested directories (depth 3) for grammar.js, parser.c or binding.gyp.
        # This will discover repos that group language subfolders (typescript/tsx) or use split parsers.
        found_any=false
        while IFS= read -r subdir; do
          if [ -n "$subdir" ]; then
            found_any=true
            try_build "$subdir"
          fi
        done < <(find "$d" -maxdepth 3 -type f \( -name 'grammar.js' -o -name 'parser.c' -o -name 'binding.gyp' \) -printf '%h\n' 2>/dev/null | sort -u)

        if ! $found_any; then
          echo "[fetch-grammars] skipping $d (no buildable sources found)"
        fi
      fi
    done
  fi

  if [ -d "$GRAMMARS_DIR" ]; then
    # look for nested grammar.js up to depth 3
    find "$GRAMMARS_DIR" -maxdepth 3 -type f -name 'grammar.js' -printf '%h\n' | sort -u | while read -r gm; do
      try_build "$gm"
    done
  fi
fi

# 4) Report results & actionable next steps
echo
if [ "${#WASM_BUILT[@]}" -gt 0 ]; then
  echo "[fetch-grammars] Built/installed the following .wasm files into $RUNTIME_DIR:"
  for f in "${WASM_BUILT[@]}"; do
    echo "  - $f"
  done
fi

echo "[fetch-grammars] current runtime dir contents:"
ls -la "$RUNTIME_DIR" || true
echo

# Detect missing common language wasm for quick guidance
MISSING=()
for l in rust toml javascript typescript markdown; do
  if [ ! -f "$RUNTIME_DIR/tree-sitter-${l}.wasm" ]; then
    MISSING+=("$l")
  fi
done

if [ "${#MISSING[@]}" -gt 0 ]; then
  echo "[fetch-grammars] common missing per-language wasm: ${MISSING[*]}"
  echo "If you need web parsing, provide the per-language wasm files (examples):"
  echo "  $RUNTIME_DIR/tree-sitter-rust.wasm"
  echo "  $RUNTIME_DIR/tree-sitter-toml.wasm"
  echo
  echo "To build locally (best-effort):"
  echo "  - Install native toolchain: sudo apt install build-essential clang pkg-config python3"
  echo "  - Ensure tree-sitter CLI is available (npm install --save-dev tree-sitter-cli) or have 'tree-sitter' installed"
  echo "  - Run: (from repo) cd crates/zaroxi-lang-syntax/runtime/treesitter && ./fetch-grammars.sh --build"
  echo
  echo "If builds fail with 'MODULE_NOT_FOUND' from grammar.js, enter the grammar directory and run:"
  echo "  npm install"
  echo "then re-run the build for that grammar directory."
fi

echo "[fetch-grammars] done."
