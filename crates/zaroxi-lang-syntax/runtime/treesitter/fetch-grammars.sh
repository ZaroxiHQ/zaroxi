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
# Allow overriding RUNTIME_DIR from the environment; default to the script directory.
RUNTIME_DIR="${RUNTIME_DIR:-$SCRIPT_DIR}"
# Backwards compatibility: some older parts of the script still reference RUNTIME_ROOT.
RUNTIME_ROOT="${RUNTIME_DIR}"
LANGUAGES_DIR="${LANGUAGES_DIR:-$RUNTIME_DIR/languages}"
GRAMMARS_DIR="${GRAMMARS_DIR:-$RUNTIME_DIR/grammars}"

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
