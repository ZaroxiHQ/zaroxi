#!/usr/bin/env bash
#
# run-ci-local.sh — run the important CI gates locally, in CI order, with the
# same commands/tools the GitHub workflows use. This lets you reproduce the
# pipeline before pushing.
#
# Mirrors:
#   .github/workflows/linux.yml         (fmt, clippy, build, test + grammar prep)
#   .github/workflows/architecture.yml  (naming, cycles, architecture_check.sh)
#   .github/workflows/security-audit.yml(cargo deny + cargo audit)
#   .github/workflows/docs-link-check.yml (pinned markdown-link-check)
#
# Usage:
#   tooling/scripts/run-ci-local.sh [--fast]
#     --fast   Skip the two clippy passes and the docs link check (quicker loop).
#
# The script runs every gate (it does NOT stop at the first failure) and prints a
# summary at the end, exiting non-zero if any gate failed.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

# Pinned markdown-link-check version (see docs-link-check.yml).
MLC_VERSION="3.11.2"

PASSED=()
FAILED=()
SKIPPED=()

run() {
  # run "<label>" <command...>
  local label="$1"; shift
  echo ""
  echo "════════════════════════════════════════════════════════════════════"
  echo "▶ $label"
  echo "  \$ $*"
  echo "════════════════════════════════════════════════════════════════════"
  if "$@"; then
    PASSED+=("$label")
  else
    FAILED+=("$label")
    echo "✗ FAILED: $label"
  fi
}

have() { command -v "$1" >/dev/null 2>&1; }

# ── 1. Format ────────────────────────────────────────────────────────────────
run "fmt --check" cargo fmt --all -- --check

# ── 2. Check / build ─────────────────────────────────────────────────────────
run "check --workspace --all-targets" cargo check --workspace --all-targets

# ── 3. Clippy (default + all-features) ───────────────────────────────────────
if [[ "$FAST" -eq 0 ]]; then
  run "clippy (default)" cargo clippy --workspace --all-targets -- -D warnings
  run "clippy (all-features)" cargo clippy --workspace --all-targets --all-features -- -D warnings
else
  SKIPPED+=("clippy (default)" "clippy (all-features)")
fi

run "build --workspace" cargo build --workspace

# ── 4. Tree-sitter grammar prep (before syntax tests) ────────────────────────
run "prepare tree-sitter grammars" bash tooling/scripts/prepare-treesitter.sh

# ── 5. Tests ─────────────────────────────────────────────────────────────────
run "test --workspace" cargo test --workspace
run "test syntax highlight_spans" cargo test -p zaroxi-core-platform-syntax --test highlight_spans

# ── 6. Architecture gates ────────────────────────────────────────────────────
run "check_circular_deps.py" python3 .github/scripts/check_circular_deps.py
run "check_crate_naming.py" python3 .github/scripts/check_crate_naming.py
run "architecture_check.sh" bash scripts/architecture_check.sh

# ── 7. Security ──────────────────────────────────────────────────────────────
if have cargo-audit; then
  run "cargo audit" cargo audit
else
  echo "… cargo-audit not installed (cargo install cargo-audit --locked) — skipping"
  SKIPPED+=("cargo audit")
fi
if have cargo-deny; then
  run "cargo deny check" cargo deny check
else
  echo "… cargo-deny not installed (cargo install cargo-deny --locked) — skipping"
  SKIPPED+=("cargo deny check")
fi

# ── 8. Docs link check (pinned tool) ─────────────────────────────────────────
if [[ "$FAST" -eq 0 ]]; then
  if have npx; then
    md_status=0
    files=$(find docs -name '*.md' -print 2>/dev/null; echo README.md)
    for f in $files; do
      echo "  markdown-link-check: $f"
      npx --yes "markdown-link-check@${MLC_VERSION}" -c .github/markdown-link-check.json "$f" || md_status=1
    done
    if [[ "$md_status" -eq 0 ]]; then PASSED+=("docs link check"); else FAILED+=("docs link check"); fi
  else
    echo "… npx not available — skipping docs link check"
    SKIPPED+=("docs link check")
  fi
else
  SKIPPED+=("docs link check")
fi

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════════════"
echo "CI-local summary"
echo "════════════════════════════════════════════════════════════════════"
printf '  ✓ PASS  %s\n' "${PASSED[@]:-}"
[[ ${#SKIPPED[@]} -gt 0 ]] && printf '  ○ SKIP  %s\n' "${SKIPPED[@]}"
[[ ${#FAILED[@]} -gt 0 ]] && printf '  ✗ FAIL  %s\n' "${FAILED[@]}"
echo "────────────────────────────────────────────────────────────────────"
echo "  ${#PASSED[@]} passed, ${#FAILED[@]} failed, ${#SKIPPED[@]} skipped"

# Windows follow-up: this script targets Unix. The Windows-specific gates
# (MSVC grammar build, .dll loading, filesystem read-only semantics) can only be
# validated on Windows. Point the developer at the PowerShell equivalent.
case "$(uname -s 2>/dev/null || echo unknown)" in
  MINGW* | MSYS* | CYGWIN* | Windows_NT) ;;
  *)
    echo ""
    echo "ℹ Windows gates are NOT covered here. On a Windows host/runner run:"
    echo "    pwsh -File tooling/scripts/run-ci-windows.ps1"
    echo "  (covers explorer_integration, resolve_dirty_close, highlight_spans,"
    echo "   MSVC grammar build, and the shared fmt/clippy/test/audit/deny gates)"
    ;;
esac

[[ ${#FAILED[@]} -eq 0 ]]
