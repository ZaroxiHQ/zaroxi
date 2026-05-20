#!/usr/bin/env bash
# Enhanced architecture_check.sh — family-aware Zaroxi architecture linter
#
# Architectural rationale (short):
# - Make checks family-aware: the script understands real Zaroxi crate families
#   (zaroxi-interface-*, zaroxi-application-*, zaroxi-domain-*, zaroxi-core-*, etc.)
#   and maps them to canonical roles (interface, application, domain, core, kernel).
# - Preserve strict directionality: outer layers may depend inward; inner layers
#   MUST NEVER depend outward. Infrastructure/intelligence/security are special
#   supporting families with narrow exceptions.
# - Keep checks conservative (hard FAIL where deterministic), advisory WARN
#   for stylistic/intentional guidance, and readable for maintainers.
#
# Usage:
#   ./scripts/architecture_check.sh
#
# Validation suggestions (run from workspace root):
#   ./scripts/architecture_check.sh
#   cargo test -p zaroxi-core-engine-render
#
# Example outputs:
#   [PASS] family-scan: found 42 crates across known families
#   [WARN] advisory: zaroxi-interface-desktop references 'glyphon' (engine backend leakage?)
#   [FAIL] dependency-direction: zaroxi-core-engine-render (core) -> zaroxi-interface-desktop (interface) is forbidden
#
set -euo pipefail
ROOT_DIR="$(pwd)"

# Counters
PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

# Additional counters for improved summary
UNKNOWN_DEP_COUNT=0    # dependencies that could not be mapped to a family
# NOTE: REAL_ADVISORY_COUNT is derived at summary time as WARN_COUNT - UNKNOWN_DEP_COUNT

# Logging helpers
log_pass() { printf "[PASS] %s\n" "$1"; PASS_COUNT=$((PASS_COUNT+1)); }
log_warn() { printf "[WARN] %s\n" "$1"; WARN_COUNT=$((WARN_COUNT+1)); }
log_fail() { printf "[FAIL] %s\n" "$1"; FAIL_COUNT=$((FAIL_COUNT+1)); }

# ---------------------------------------------------------------------------
# FAMILY & ROLE CONFIGURATION (single source of truth)
# ---------------------------------------------------------------------------
# Define recognized family patterns and which logical family they map to.
# Order matters for matching: more specific patterns should come before broader ones.
declare -A FAMILY_PATTERNS
# Primary library-family patterns (match crate names)
FAMILY_PATTERNS["^zaroxi-interface-"]="interface"
FAMILY_PATTERNS["^zaroxi-application-"]="application"
FAMILY_PATTERNS["^zaroxi-domain-"]="domain"
FAMILY_PATTERNS["^zaroxi-core-engine-"]="core-engine"
FAMILY_PATTERNS["^zaroxi-core-editor-"]="core-editor"
FAMILY_PATTERNS["^zaroxi-core-platform-"]="core-platform"
FAMILY_PATTERNS["^zaroxi-core-workspace-"]="core-workspace"

# Core-runtime / shared primitives families (fine-grained)
FAMILY_PATTERNS["^zaroxi-core-runtime"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-state"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-task"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-sync"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-threading"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-telemetry"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-event"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-input"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-io"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-commands"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-plugin-runtime"]="core-runtime"
FAMILY_PATTERNS["^zaroxi-core-scheduler"]="core-runtime"

# Supporting families
FAMILY_PATTERNS["^zaroxi-infrastructure-"]="infrastructure"
FAMILY_PATTERNS["^zaroxi-intelligence-"]="intelligence"
FAMILY_PATTERNS["^zaroxi-security-"]="security"
FAMILY_PATTERNS["^zaroxi-kernel-"]="kernel"

# App / tooling / harness patterns (explicitly model non-library members)
# These help avoid treating top-level apps as "unknown".
FAMILY_PATTERNS["^apps/"]="app_bin"
FAMILY_PATTERNS["^zaroxi-desktop-harness$"]="harness"
FAMILY_PATTERNS["^workspace-daemon$"]="daemon"
FAMILY_PATTERNS["^ai-daemon$"]="daemon"
FAMILY_PATTERNS["^desktop$"]="app_bin"

# Path-based historical entry (left for compatibility)
FAMILY_PATTERNS["^crates/zaroxi-desktop-harness$"]="harness"

# Map family -> canonical role (for messaging) and numeric rank used for direction checks.
# Higher numeric rank = more outer layer (allowed to depend inward).
declare -A FAMILY_ROLE
declare -A FAMILY_RANK
# canonical roles (family -> messaging role)
FAMILY_ROLE["interface"]="interface"
FAMILY_ROLE["application"]="application"
FAMILY_ROLE["domain"]="domain"
FAMILY_ROLE["core-engine"]="core"
FAMILY_ROLE["core-editor"]="core"
FAMILY_ROLE["core-platform"]="core"
FAMILY_ROLE["core-workspace"]="core"
FAMILY_ROLE["core-runtime"]="core"
FAMILY_ROLE["infrastructure"]="infrastructure"
FAMILY_ROLE["intelligence"]="intelligence"
FAMILY_ROLE["security"]="security"
FAMILY_ROLE["kernel"]="kernel"
FAMILY_ROLE["app_bin"]="app_bin"
FAMILY_ROLE["harness"]="app_bin"
FAMILY_ROLE["daemon"]="app_bin"
FAMILY_ROLE["unknown"]="unknown"

# numeric ranks (outer -> inner). Higher = more outer.
FAMILY_RANK["interface"]=7
FAMILY_RANK["app_bin"]=7
FAMILY_RANK["harness"]=7
FAMILY_RANK["daemon"]=7
FAMILY_RANK["application"]=6
FAMILY_RANK["domain"]=5
FAMILY_RANK["core-engine"]=4
FAMILY_RANK["core-editor"]=4
FAMILY_RANK["core-platform"]=4
FAMILY_RANK["core-workspace"]=4
FAMILY_RANK["core-runtime"]=4
FAMILY_RANK["infrastructure"]=3
FAMILY_RANK["intelligence"]=3
FAMILY_RANK["security"]=3
FAMILY_RANK["kernel"]=0
FAMILY_RANK["unknown"]=-1

# Explicit exceptions where infra -> application adapters are allowed.
declare -a INFRA_TO_APP_EXCEPTIONS=(
  "zaroxi-infrastructure-ai-mock:zaroxi-application-ai"
  "zaroxi-infrastructure-memory:zaroxi-application-workspace"
)

# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------
# Trim whitespace
trim() { sed 's/^[[:space:]]*//;s/[[:space:]]*$//'; }

# Extract crate name from Cargo.toml
crate_name_from_toml() {
  local toml="$1"
  if [ -f "$toml" ]; then
    awk -F= '/^\s*name\s*=/ { gsub(/[" \t]/,"",$2); print $2; exit }' "$toml" || true
  fi
}

# Classify crate by name -> family (pattern matching)
classify_family() {
  local crate="$1"
  for pat in "${!FAMILY_PATTERNS[@]}"; do
    if [[ "$crate" =~ $pat ]]; then
      echo "${FAMILY_PATTERNS[$pat]}"
      return
    fi
  done
  # fallback: heuristic for zaroxi-core-*
  if [[ "$crate" == zaroxi-core-* ]]; then
    echo "core-runtime"
    return
  fi
  echo "unknown"
}

# Resolve family -> role and rank
family_role() {
  local fam="$1"
  echo "${FAMILY_ROLE[$fam]:-unknown}"
}
family_rank() {
  local fam="$1"
  echo "${FAMILY_RANK[$fam]:-${FAMILY_RANK["unknown"]}}"
}

# Check whether pair matches infra->app exception list
is_infra_app_exception() {
  local from="$1"
  local to="$2"
  for ex in "${INFRA_TO_APP_EXCEPTIONS[@]}"; do
    if [[ "$from:$to" == "$ex" || "$from" == "$ex" || "$from:$to" == "${ex//:/}" ]]; then
      return 0
    fi
  done
  return 1
}

# Simple grep helper that returns true if no forbidden tokens are found or allowed tokens are present.
check_grep_excluding() {
  local pattern="$1"
  local dir="$2"
  local allow_pattern="${3:-}"
  if ! grep -R --line-number --exclude-dir=target -E "$pattern" "$dir" >/dev/null 2>&1; then
    return 0
  fi
  if [ -n "$allow_pattern" ] && grep -R --line-number --exclude-dir=target -E "$allow_pattern" "$dir" >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

# ---------------------------------------------------------------------------
# Scanning workspace crates and classifying families
# ---------------------------------------------------------------------------
echo "Scanning workspace crates for Cargo.toml..."
CRATE_TOMLS=()
while IFS= read -r -d '' f; do
  CRATE_TOMLS+=("$f")
done < <(find crates apps -maxdepth 2 -type f -name Cargo.toml -print0 2>/dev/null || true)

declare -A CRATE_TO_FAMILY
declare -A FAMILY_TO_CRATES
declare -a UNKNOWN_CRATES

for toml in "${CRATE_TOMLS[@]}"; do
  crate=$(crate_name_from_toml "$toml")
  crate="${crate:-$(basename "$(dirname "$toml")")}"
  fam=$(classify_family "$crate")
  CRATE_TO_FAMILY["$crate"]="$fam"
  FAMILY_TO_CRATES["$fam"]+="$crate "
done

# Report known families and unknowns
echo "Repository family inventory:"
total=0
for fam in "${!FAMILY_TO_CRATES[@]}"; do
  crates_list="${FAMILY_TO_CRATES[$fam]}"
  count=$(echo "$crates_list" | wc -w | tr -d ' ')
  printf "  - %-14s : %3d crates\n" "$fam" "$count"
  total=$((total+count))
done

# Find crates not matched (unknown)
for crate in "${!CRATE_TO_FAMILY[@]}"; do
  if [[ "${CRATE_TO_FAMILY[$crate]}" == "unknown" ]]; then
    UNKNOWN_CRATES+=("$crate")
  fi
done

if [ ${#UNKNOWN_CRATES[@]} -gt 0 ]; then
  echo "Unknown/unclassified crates:"
  for c in "${UNKNOWN_CRATES[@]}"; do
    echo "  - $c"
  done
  log_warn "found ${#UNKNOWN_CRATES[@]} unknown crates; consider adding family patterns to FAMILY_PATTERNS"
else
  log_pass "family-scan: found $total crates across known families"
fi

# ---------------------------------------------------------------------------
# Dependency checks: parse each Cargo.toml for zaroxi-* dependencies and evaluate direction
# ---------------------------------------------------------------------------
echo
echo "Running dependency-direction checks..."

for toml in "${CRATE_TOMLS[@]}"; do
  crate_dir=$(dirname "$toml")
  crate_name=$(crate_name_from_toml "$toml")
  crate_name="${crate_name:-$(basename "$crate_dir")}"
  from_family="${CRATE_TO_FAMILY[$crate_name]:-unknown}"
  from_role=$(family_role "$from_family")
  from_rank=$(family_rank "$from_family")

  # find lines referencing zaroxi crate tokens
  deps=$(grep -E "zaroxi[-_][a-z0-9\-_/]+" "$toml" || true)
  if [ -z "$deps" ]; then
    log_pass "cargo-deps: $crate_name declares no zaroxi-* deps"
    continue
  fi

  while IFS= read -r line; do
    for token in $(echo "$line" | grep -oE "zaroxi[-_][a-z0-9\-_/]+" || true); do
      # normalize
      dep_crate="${token//_/-}"
      dep_crate="${dep_crate#*/}"
      # Normalize common forms into canonical crate names where possible.
      # Attempt to preserve the full crate name (e.g. zaroxi-core-engine-render).
      # If the token already looks like a zaroxi crate leave it; otherwise try to
      # restore a reasonable canonical form.
      if [[ "$dep_crate" == zaroxi-* ]]; then
        # ensure we have dashed form and canonical prefix
        dep_crate="${dep_crate#zaroxi-}"
        dep_crate="zaroxi-${dep_crate}"
      fi

      # Prefer workspace-known crate mapping to avoid umbrella-prefix false positives.
      if [[ -n "${CRATE_TO_FAMILY[$dep_crate]:-}" ]]; then
        to_family="${CRATE_TO_FAMILY[$dep_crate]}"
      else
        # Handle umbrella shortnames introduced by some manifests or comments,
        # e.g. "zaroxi-core", "zaroxi-kernel", etc. Map them to sensible families
        # instead of leaving them 'unknown' and producing noisy warnings.
        case "$dep_crate" in
          zaroxi-core) to_family="core-runtime" ;;
          zaroxi-kernel) to_family="kernel" ;;
          zaroxi-interface) to_family="interface" ;;
          zaroxi-application) to_family="application" ;;
          zaroxi-domain) to_family="domain" ;;
          zaroxi-infrastructure) to_family="infrastructure" ;;
          zaroxi-intelligence) to_family="intelligence" ;;
          zaroxi-security) to_family="security" ;;
          # If nothing matched, fall back to pattern-based classification.
          *) to_family=$(classify_family "$dep_crate") ;;
        esac
      fi

      to_role=$(family_role "$to_family")
      to_rank=$(family_rank "$to_family")

      # infra->app exceptions
      if [[ "$from_family" == "infrastructure" ]] && [[ "$to_family" == "application" ]]; then
        if is_infra_app_exception "$crate_name" "$dep_crate"; then
          log_pass "allowed infra->app adapter: $crate_name -> $dep_crate"
          continue
        fi
      fi

      # unknown roles -> warn (count and continue). These are actionable: either
      # add a FAMILY_PATTERNS entry or verify the dependency token.
      if [[ "$from_family" == "unknown" || "$to_family" == "unknown" ]]; then
        UNKNOWN_DEP_COUNT=$((UNKNOWN_DEP_COUNT+1))
        log_warn "dependency-direction: $crate_name -> $dep_crate (unknown family; update FAMILY_PATTERNS) origin=$toml"
        continue
      fi

      # Allowed if to_rank <= from_rank (outer -> inner)
      if (( to_rank <= from_rank )); then
        log_pass "dependency-direction: $crate_name ($from_family) -> $dep_crate ($to_family) allowed"
      else
        log_fail "dependency-direction: $crate_name ($from_family) -> $dep_crate ($to_family) is forbidden (origin=$toml)"
      fi
    done
  done <<< "$deps"
done

# ---------------------------------------------------------------------------
# Source advisory checks: ensure interface crates don't reference engine internals
# ---------------------------------------------------------------------------
echo
echo "Running source-advisory checks..."

# Interface and app crates should not reference engine backend implementation names (glyphon, glyphon types)
for crate in "${!CRATE_TO_FAMILY[@]}"; do
  fam="${CRATE_TO_FAMILY[$crate]}"
  if [[ "$fam" == "interface" || "$fam" == "app_bin" ]]; then
    # find crate dir
    crate_dir=""
    for toml in "${CRATE_TOMLS[@]}"; do
      name=$(crate_name_from_toml "$toml")
      name="${name:-$(basename "$(dirname "$toml")")}"
      if [[ "$name" == "$crate" ]]; then
        crate_dir="$(dirname "$toml")"
        break
      fi
    done
    if [ -z "$crate_dir" ]; then
      continue
    fi
    if grep -R --line-number --exclude-dir=target -E "glyphon|GlyphonBackend|zaroxi_core_engine_text::|zaroxi-core-engine-text" "$crate_dir" >/dev/null 2>&1; then
      log_warn "advisory: $crate (family=$fam) appears to reference engine backend internals; prefer engine seams (text_seam) and engine-facing primitives (RenderIntent/Transcript)"
    else
      log_pass "advisory: $crate (family=$fam) shows no obvious engine backend references"
    fi
  fi
done

# Additional family-specific advisory: ensure core-engine text/render seam usage is respected
if [ -d "$ROOT_DIR/crates/zaroxi-interface-desktop" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi-core-engine-text|glyphon" "crates/zaroxi-interface-desktop" >/dev/null 2>&1; then
    log_warn "advisory: crates/zaroxi-interface-desktop references engine-text or glyphon; interface should use render intents/transcripts instead of engine backend types"
  else
    log_pass "advisory: interface-desktop appears to consume only engine-facing primitives"
  fi
fi

# ---------------------------------------------------------------------------
# Legacy targeted checks preserved where deterministic and actionable
# ---------------------------------------------------------------------------
echo
echo "Running legacy deterministic checks..."

# 1) domain MUST NOT import interface/application/infrastructure (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-domain-workspace" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_interface_|zaroxi-interface-|zaroxi_application_|zaroxi-application-|zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-domain-workspace" >/dev/null 2>&1; then
    log_fail "domain-workspace imports interface/application/infrastructure (forbidden)."
  else
    log_pass "legacy-check: domain-workspace has no upward imports"
  fi
fi

# 2) application MUST NOT import interface (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-application-workspace" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-application-workspace" >/dev/null 2>&1; then
    log_fail "application-workspace imports interface (forbidden)."
  else
    log_pass "legacy-check: application-workspace has no interface imports"
  fi
fi

# 3) interface MUST NOT import infrastructure (strict)
if [ -d "$ROOT_DIR/crates/zaroxi-interface-desktop" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-interface-desktop" >/dev/null 2>&1; then
    log_fail "interface-desktop imports infrastructure (forbidden). Inspect crates/zaroxi-interface-desktop source for 'zaroxi-infrastructure-' usages."
  else
    log_pass "legacy-check: interface-desktop does not import infrastructure"
  fi
fi

# ---------------------------------------------------------------------------
# Summary and final exit code
# ---------------------------------------------------------------------------
echo
echo "Architecture check summary:"
echo "  PASS: $PASS_COUNT"
echo "  WARN: $WARN_COUNT"
echo "  FAIL: $FAIL_COUNT"
echo "  Unknown crates: ${#UNKNOWN_CRATES[@]}"
echo "  Unknown dependencies: $UNKNOWN_DEP_COUNT"
real_advisory_count=$((WARN_COUNT - UNKNOWN_DEP_COUNT))
if (( real_advisory_count < 0 )); then real_advisory_count=0; fi
echo "  Advisory (likely real) warnings: $real_advisory_count"

# Actionable exit rules:
if (( FAIL_COUNT > 0 )); then
  echo
  echo "One or more hard architectural violations detected. See messages above for actionable locations and offending crates."
  exit 1
fi

if (( UNKNOWN_DEP_COUNT > 0 )); then
  echo
  echo "There are unknown dependency mappings ($UNKNOWN_DEP_COUNT). Please review the logged WARN lines and consider:"
  echo "  - Adding explicit FAMILY_PATTERNS entries for new crates"
  echo "  - Confirming dependency tokens in Cargo.toml are correct"
  echo "  - Ensuring workspace crate names are canonical (match Cargo.toml name)"
  # Unknown deps are advisory only; do not fail CI, but make them visible.
  exit 0
fi

if (( WARN_COUNT > 0 )); then
  echo
  echo "Advisory warnings were emitted. These do not fail CI but should be reviewed:"
  exit 0
fi

echo
echo "Architecture checks passed (no FAILs, no WARNs)."
exit 0
