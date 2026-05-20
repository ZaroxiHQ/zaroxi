#!/usr/bin/env bash
# Enhanced architecture_check.sh
#
# Architectural rationale:
# - This script codifies the current hard rules (enforced FAILs) and the
#   future intended boundaries (WARN-only advisory rules).
# - It is intentionally conservative: hard failures are applied only where
#   automated, deterministic checks are reliable. Advisory warnings flag
#   likely architectural drift that requires human review.
# - The script is structured for readability and easy extension: role maps,
#   helper functions, and grouped outputs.
#
# Validation:
#   ./scripts/architecture_check.sh
#   cargo test -p zaroxi-core-engine-render
#
# Example outputs (shortened):
# PASS:
#   [PASS] dependency-direction: crate=zaroxi-interface-desktop has no upward deps
# WARN:
#   [WARN] engine-seam: zaroxi-interface-desktop references 'glyphon' (engine backend leakage?)
# FAIL:
#   [FAIL] dependency-direction: crates/zaroxi-domain-workspace -> zaroxi-interface-app (forbidden)
#
# NOTE: This script intentionally avoids modifying Cargo.toml files. If a
# failure occurs, follow the actionable message to fix the offending crate.
#
set -euo pipefail
ROOT_DIR="$(pwd)"

# Counters
PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

# Simple logging helpers
log_pass() { printf "[PASS] %s\n" "$1"; PASS_COUNT=$((PASS_COUNT+1)); }
log_warn() { printf "[WARN] %s\n" "$1"; WARN_COUNT=$((WARN_COUNT+1)); }
log_fail() { printf "[FAIL] %s\n" "$1"; FAIL_COUNT=$((FAIL_COUNT+1)); }

# Role hierarchy (higher number = outer layer)
declare -A ROLE_RANK
ROLE_RANK[interface]=4
ROLE_RANK[application]=3
ROLE_RANK[domain]=2
ROLE_RANK[core]=1
ROLE_RANK[kernel]=0
ROLE_RANK[app_bin]=4   # apps/harness treated like outer-layer consumers

# Explicit role map for known crates in this workspace.
# Keep this list authoritative for current enforcement; add new crates here.
declare -A ROLE_MAP

# CORE ENGINE
ROLE_MAP[zaroxi-core-engine-root]=core
ROLE_MAP[zaroxi-core-engine-runtime]=core
ROLE_MAP[zaroxi-core-engine-state]=core
ROLE_MAP[zaroxi-core-engine-window]=core
ROLE_MAP[zaroxi-core-engine-input]=core
ROLE_MAP[zaroxi-core-engine-action]=core
ROLE_MAP[zaroxi-core-engine-focus]=core
ROLE_MAP[zaroxi-core-engine-layout]=core
ROLE_MAP[zaroxi-core-engine-style]=core
ROLE_MAP[zaroxi-core-engine-element]=core
ROLE_MAP[zaroxi-core-engine-view]=core
ROLE_MAP[zaroxi-core-engine-scene]=core
ROLE_MAP[zaroxi-core-engine-text]=core
ROLE_MAP[zaroxi-core-engine-render]=core

# CORE EDITOR / WORKSPACE / PLATFORM
ROLE_MAP[zaroxi-core-editor-buffer]=core
ROLE_MAP[zaroxi-core-workspace-files]=core
ROLE_MAP[zaroxi-core-platform-syntax]=core

# KERNEL
ROLE_MAP[zaroxi-kernel-core]=kernel
ROLE_MAP[zaroxi-kernel-types]=kernel
ROLE_MAP[zaroxi-kernel-errors]=kernel

# DOMAIN
ROLE_MAP[zaroxi-domain-workspace]=domain
ROLE_MAP[zaroxi-domain-buffer]=domain
ROLE_MAP[zaroxi-domain-project]=domain

# APPLICATION
ROLE_MAP[zaroxi-application-workspace]=application
ROLE_MAP[zaroxi-application-editor]=application
ROLE_MAP[zaroxi-application-ai]=application

# INTERFACE
ROLE_MAP[zaroxi-interface-desktop]=interface
ROLE_MAP[zaroxi-interface-app]=interface
ROLE_MAP[zaroxi-interface-theme]=interface

# INFRASTRUCTURE (adapters)
ROLE_MAP[zaroxi-infrastructure-ai-mock]=application
ROLE_MAP[zaroxi-infrastructure-memory]=application

# APPS / HARNESS
ROLE_MAP[zaroxi-desktop-harness]=app_bin

# Reserved / future roles (advisory only)
# These are intentionally listed to document future architecture intent.
FUTURE_ROLES=(
  "engine-text"
  "engine-render"
  "engine-input"
  "engine-layout"
  "projection-ai"
  "presenter-projections"
)

# Allowed infra->application exceptions (explicit, narrow)
# Keep this list very small and explicit.
ALLOWED_INFRA_TO_APP_EXCEPTIONS=(
  "zaroxi-infrastructure-ai-mock:zaroxi-application-ai"
  "zaroxi-infrastructure-memory:zaroxi-application-workspace"
)

# Utility: get role for a crate name (fall back to 'unknown')
role_of() {
  local crate="$1"
  if [[ -n "${ROLE_MAP[$crate]:-}" ]]; then
    echo "${ROLE_MAP[$crate]}"
  else
    echo "unknown"
  fi
}

# Utility: numeric rank (unknown -> -1)
rank_of() {
  local role
  role=$(role_of "$1")
  echo "${ROLE_RANK[$role]:- -1}"
}

# Parse a crate name from a Cargo.toml file
crate_name_from_toml() {
  local toml="$1"
  # look for name = "..." in [package]
  if [ -f "$toml" ]; then
    awk -F= '/^\s*name\s*=/ { gsub(/[" \t]/,"",$2); print $2; exit }' "$toml" || true
  fi
}

# Check a crate's declared dependencies in Cargo.toml for zaroxi-* path deps
check_cargo_deps() {
  local crate_dir="$1"
  local toml="$crate_dir/Cargo.toml"
  if [ ! -f "$toml" ]; then
    log_warn "no Cargo.toml for $crate_dir; skipping cargo-deps check"
    return
  fi

  local crate_name
  crate_name="$(crate_name_from_toml "$toml")"
  crate_name="${crate_name:-$(basename "$crate_dir")}"

  # gather referenced zaroxi crate names found in the Cargo.toml (both dashed and underscored)
  local deps
  deps=$(grep -E "zaroxi[-_][a-z0-9\-_/]+" "$toml" || true)
  if [ -z "$deps" ]; then
    log_pass "cargo-deps: $crate_name declares no zaroxi-* deps"
    return
  fi

  # For each match, extract the crate token and check role direction.
  while IFS= read -r line; do
    # extract tokens like zaroxi-core-engine-text or zaroxi_core_engine_text
    for token in $(echo "$line" | grep -oE "zaroxi[-_][a-z0-9\-_/]+" || true); do
      # normalize underscored to dashed crate name (common source forms)
      dep_crate="${token//_/-}"
      dep_crate="${dep_crate#*/}" # in case of path = "../zaroxi-..." capture trailing
      dep_crate="${dep_crate#zaroxi-}"
      dep_crate="zaroxi-${dep_crate}"
      evaluate_dependency "$crate_name" "$dep_crate" "$crate_dir" "Cargo.toml"
    done
  done <<< "$deps"
}

# Check source usages: grep for 'use zaroxi_' or 'zaroxi-' in source files (advisory)
check_source_usages() {
  local crate_dir="$1"
  local crate_name
  crate_name="$(crate_name_from_toml "$crate_dir/Cargo.toml")"
  crate_name="${crate_name:-$(basename "$crate_dir")}"

  # Search Rust sources for direct references to engine backend types (e.g. glyphon, GlyphonBackend)
  if grep -R --line-number --exclude-dir=target -E "glyphon|GlyphonBackend|Glyphon::|zaroxi_core_engine_text::|zaroxi-core-engine-text" "$crate_dir" >/dev/null 2>&1; then
    log_warn "engine-seam: $crate_name references engine backends or glyphon; ensure seams (text_seam) are used instead"
  else
    log_pass "engine-seam: $crate_name shows no obvious backend leakage"
  fi
}

# Evaluate one dependency occurrence; decide PASS/WARN/FAIL
evaluate_dependency() {
  local from_crate="$1"
  local to_crate="$2"
  local crate_dir="$3"
  local origin_detail="$4"

  # canonicalize crate names (strip potential path prefixes)
  local from_role
  from_role=$(role_of "$from_crate")
  local to_role
  to_role=$(role_of "$to_crate")

  # Apply special-case infra->app exceptions
  local pair="${crate_dir##*/}:$to_crate"
  for ex in "${ALLOWED_INFRA_TO_APP_EXCEPTIONS[@]}"; do
    if [[ "$pair" == "$ex" || "$from_crate:$to_crate" == "$ex" ]]; then
      log_pass "allowed infra->app adapter: $from_crate -> $to_crate ($origin_detail)"
      return
    fi
  done

  # If either role is unknown, emit a WARN (requires human review).
  if [[ "$from_role" == "unknown" || "$to_role" == "unknown" ]]; then
    log_warn "dependency-direction: $from_crate -> $to_crate (unknown role; please update ROLE_MAP if this is valid) origin=$origin_detail"
    return
  fi

  # numeric ranks
  local from_rank=${ROLE_RANK[$from_role]:- -1}
  local to_rank=${ROLE_RANK[$to_role]:- -1}

  # Allowed if to_rank <= from_rank (outer -> inner allowed)
  if (( to_rank <= from_rank )); then
    log_pass "dependency-direction: $from_crate ($from_role) -> $to_crate ($to_role) allowed"
    return
  fi

  # Otherwise this is an upward dependency (forbidden)
  log_fail "dependency-direction: $from_crate ($from_role) -> $to_crate ($to_role) is forbidden (origin=$origin_detail)"
}

# Top-level checks
echo "Running enhanced architecture checks..."

# Iterate over crate directories under crates/ and apps/
CRATE_DIRS=()
while IFS= read -r -d '' dir; do
  CRATE_DIRS+=("$dir")
done < <(find crates apps -maxdepth 2 -type f -name Cargo.toml -print0 2>/dev/null)

if [ ${#CRATE_DIRS[@]} -eq 0 ]; then
  log_warn "No crates found under crates/ or apps/ (unexpected); falling back to legacy checks"
fi

# Examine each crate Cargo.toml dependencies for zaroxi-* tokens
for toml in "${CRATE_DIRS[@]}"; do
  crate_dir=$(dirname "$toml")
  check_cargo_deps "$crate_dir"
  check_source_usages "$crate_dir"
done

# Legacy targeted checks preserved for compatibility (fail-fast)
# 1) interface MUST NOT import infrastructure (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-interface-desktop" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-interface-desktop" >/dev/null 2>&1; then
    log_fail "interface-desktop imports infrastructure (forbidden). Inspect crates/zaroxi-interface-desktop source for 'zaroxi-infrastructure-' usages."
  else
    log_pass "legacy-check: interface-desktop does not import infrastructure"
  fi
fi

# 2) domain MUST NOT import interface, application, or infrastructure (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-domain-workspace" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_interface_|zaroxi-interface-|zaroxi_application_|zaroxi-application-|zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-domain-workspace" >/dev/null 2>&1; then
    log_fail "domain-workspace imports interface/application/infrastructure (forbidden)."
  else
    log_pass "legacy-check: domain-workspace has no upward imports"
  fi
fi

# 3) application MUST NOT import interface (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-application-workspace" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-application-workspace" >/dev/null 2>&1; then
    log_fail "application-workspace imports interface (forbidden)."
  else
    log_pass "legacy-check: application-workspace has no interface imports"
  fi
fi

# 4) infrastructure narrow exceptions (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-infrastructure-memory" ]; then
  if ! check_grep_excluding "zaroxi_application_|zaroxi-application-|zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-infrastructure-memory" "zaroxi_application_workspace|zaroxi-application-workspace"; then
    log_fail "infrastructure-memory imports forbidden application or interface crates (only 'zaroxi-application-workspace' allowed)"
  else
    log_pass "legacy-check: infra-memory imports only allowed app port"
  fi
fi

if [ -d "$ROOT_DIR/crates/zaroxi-infrastructure-ai-mock" ]; then
  if ! check_grep_excluding "zaroxi_application_|zaroxi-application-|zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-infrastructure-ai-mock" "zaroxi_application_ai|zaroxi-application-ai"; then
    log_fail "infrastructure-ai-mock imports forbidden application/interface crates (only 'zaroxi-application-ai' allowed)"
  else
    log_pass "legacy-check: infra-ai-mock imports only allowed app port"
  fi
fi

# Future / advisory rules
echo "Advisory checks (future architecture intent):"
for future in "${FUTURE_ROLES[@]}"; do
  echo " - Reserved future role: $future (advisory only)"
done

# Advisory: ensure interface crates do not reference engine backend implementation names
INTERFACE_DIRS=( "crates/zaroxi-interface-desktop" "crates/zaroxi-interface-app" )
for d in "${INTERFACE_DIRS[@]}"; do
  if [ -d "$ROOT_DIR/$d" ]; then
    if grep -R --line-number --exclude-dir=target -E "glyphon|GlyphonBackend|zaroxi_core_engine_text::|zaroxi-core-engine-text" "$ROOT_DIR/$d" >/dev/null 2>&1; then
      log_warn "advisory: $d appears to reference engine backend types (e.g. glyphon). Prefer engine seams (text_seam) and engine-facing primitives (RenderIntent/Transcript)."
    else
      log_pass "advisory: $d shows no obvious engine backend references"
    fi
  fi
done

# Summary
echo
echo "Architecture check summary:"
echo "  PASS: $PASS_COUNT"
echo "  WARN: $WARN_COUNT"
echo "  FAIL: $FAIL_COUNT"
if (( FAIL_COUNT > 0 )); then
  echo
  echo "One or more hard architectural violations detected. See messages above for actionable locations and offending crates."
  exit 1
fi

if (( WARN_COUNT > 0 )); then
  echo
  echo "Advisory warnings were emitted. These do not fail CI but should be reviewed:"
  exit 0
fi

echo
echo "Architecture checks passed (no FAILs, no WARNs)."
exit 0
