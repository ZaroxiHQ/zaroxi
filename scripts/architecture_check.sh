#!/usr/bin/env bash
# Enhanced architecture_check.sh — family-aware Zaroxi architecture linter
#
# Architectural rationale (short):
# - Make checks family-aware and low-noise: the script understands Zaroxi
#   crate families and reports only deterministic FAILs and concise WARNs.
# - Composition roots (apps/daemons/harness) are explicitly modeled so they do
#   not appear as "unknown" and are allowed broader composition dependencies.
# - Keep output human-readable: inventory, violations, advisories, composition notes, summary.
#
# Usage:
#   ./scripts/architecture_check.sh
#
# Validation suggestions (run from workspace root):
#   ./scripts/architecture_check.sh
#
set -euo pipefail
ROOT_DIR="$(pwd)"

# -------------------------
# Counters & result holders
# -------------------------
declare -i PASS_COUNT=0
declare -i WARN_COUNT=0
declare -i FAIL_COUNT=0
declare -i UNKNOWN_DEP_COUNT=0

# Arrays must be initialized under set -u
declare -a CRATE_TOMLS=()
declare -A CRATE_TO_FAMILY=()
declare -A FAMILY_TO_CRATES=()
declare -a UNKNOWN_CRATES=()
declare -a UNKNOWN_DEPS=()
declare -a FAIL_LIST=()
declare -a WARN_LIST=()
declare -a COMPOSITION_ROOTS=()

# Logging helpers (also collect messages for structured sections)
log_pass() { printf "[PASS] %s\n" "$1"; PASS_COUNT=$((PASS_COUNT+1)); }
log_warn() { printf "[WARN] %s\n" "$1"; WARN_COUNT=$((WARN_COUNT+1)); WARN_LIST+=("$1"); }
log_fail() { printf "[FAIL] %s\n" "$1"; FAIL_COUNT=$((FAIL_COUNT+1)); FAIL_LIST+=("$1"); }

# -------------------------
# Family & Role configuration
# -------------------------
declare -A FAMILY_PATTERNS
FAMILY_PATTERNS["^zaroxi-interface-"]="interface"
FAMILY_PATTERNS["^zaroxi-application-"]="application"
FAMILY_PATTERNS["^zaroxi-domain-"]="domain"
FAMILY_PATTERNS["^zaroxi-core-engine-"]="core-engine"
# Explicit mapping for the new engine UI crate to ensure deterministic classification.
FAMILY_PATTERNS["^zaroxi-core-engine-ui$"]="core-engine"
FAMILY_PATTERNS["^zaroxi-core-editor-"]="core-editor"
FAMILY_PATTERNS["^zaroxi-core-platform-"]="core-platform"
FAMILY_PATTERNS["^zaroxi-core-workspace-"]="core-workspace"

# Helpful alias patterns to reduce noisy advisory warnings for known internal naming
FAMILY_PATTERNS["^zaroxi-ops"]="application"
FAMILY_PATTERNS["^zaroxi-infra"]="infrastructure"
FAMILY_PATTERNS["^zaroxi-protocol"]="kernel"
FAMILY_PATTERNS["^zaroxi-ai"]="intelligence"
FAMILY_PATTERNS["^zaroxi-engine"]="core-engine"
FAMILY_PATTERNS["^zaroxi-config"]="kernel"
FAMILY_PATTERNS["^zaroxi-app"]="application"
FAMILY_PATTERNS["^zaroxi-desktop"]="harness"

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

FAMILY_PATTERNS["^zaroxi-infrastructure-"]="infrastructure"
FAMILY_PATTERNS["^zaroxi-intelligence-"]="intelligence"
FAMILY_PATTERNS["^zaroxi-security-"]="security"
FAMILY_PATTERNS["^zaroxi-kernel-"]="kernel"
# Explicit mapping for recently added kernel crates to ensure deterministic
# classification and to avoid unknown-family warnings for new kernel members.
# This keeps the inventory/count stable and enforces kernel-family checks for
# these crates without changing existing dependency rules.
FAMILY_PATTERNS["^zaroxi-kernel-id$"]="kernel"

# explicit app/tooling patterns (avoid them being treated as unknown)
FAMILY_PATTERNS["^apps/"]="app_bin"
FAMILY_PATTERNS["^zaroxi-desktop-harness$"]="harness"
FAMILY_PATTERNS["^workspace-daemon$"]="daemon"
FAMILY_PATTERNS["^ai-daemon$"]="daemon"
FAMILY_PATTERNS["^desktop$"]="app_bin"
FAMILY_PATTERNS["^crates/zaroxi-desktop-harness$"]="harness"

declare -A FAMILY_ROLE
declare -A FAMILY_RANK
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

declare -a INFRA_EXCEPTIONS=(
  "zaroxi-infrastructure-ai-mock:zaroxi-application-ai"
  "zaroxi-infrastructure-ai-mock:zaroxi-application"
  "zaroxi-infrastructure-memory:zaroxi-application-workspace"
  "zaroxi-infrastructure-memory:zaroxi-application"
  "zaroxi-infrastructure-memory:zaroxi-domain-workspace"
  "zaroxi-infrastructure-memory:zaroxi-core-editor-buffer"
)

# -------------------------
# Utilities
# -------------------------
trim() { sed 's/^[[:space:]]*//;s/[[:space:]]*$//'; }

crate_name_from_toml() {
  local toml="$1"
  if [ -f "$toml" ]; then
    awk -F= '/^\s*name\s*=/ { gsub(/[" \t]/,"",$2); print $2; exit }' "$toml" || true
  fi
}

classify_family() {
  local crate="$1"
  for pat in "${!FAMILY_PATTERNS[@]}"; do
    if [[ "$crate" =~ $pat ]]; then
      echo "${FAMILY_PATTERNS[$pat]}"
      return
    fi
  done
  if [[ "$crate" == zaroxi-core-* ]]; then
    echo "core-runtime"
    return
  fi
  echo "unknown"
}

family_role() { local fam="$1"; echo "${FAMILY_ROLE[$fam]:-unknown}"; }
family_rank() { local fam="$1"; echo "${FAMILY_RANK[$fam]:-${FAMILY_RANK["unknown"]}}"; }

is_infra_exception() {
  local from="$1" to="$2"
  for ex in "${INFRA_EXCEPTIONS[@]}"; do
    # direct pair match (crate:crate)
    if [[ "$from:$to" == "$ex" ]]; then
      return 0
    fi
    # split "from:to" pattern and allow matching by-from or by-from:to
    IFS=':' read -r ex_from ex_to <<< "$ex"
    # If the exception lists only the source crate (ex_from matches and ex_to empty),
    # treat it as "allow all higher-layer deps from this infra crate".
    if [[ -n "$ex_from" && -z "$ex_to" && "$from" == "$ex_from" ]]; then
      return 0
    fi
    # If both sides provided, allow when both match (covered above) or when ex_to matches to.
    if [[ -n "$ex_from" && -n "$ex_to" && "$from" == "$ex_from" && "$to" == "$ex_to" ]]; then
      return 0
    fi
  done
  return 1
}

# -------------------------
# Structured printers
# -------------------------
print_header() {
  printf "\n=== %s ===\n" "$1"
}

print_inventory() {
  print_header "Workspace inventory (compact)"
  local total=0
  for fam in "${!FAMILY_TO_CRATES[@]}"; do
    crates_list="${FAMILY_TO_CRATES[$fam]}"
    count=$(echo "$crates_list" | wc -w | tr -d ' ')
    printf "  - %-14s : %3d crates\n" "$fam" "$count"
    total=$((total+count))
  done
  printf "  Composition roots (apps/daemons/harness): %d\n" "${#COMPOSITION_ROOTS[@]}"
  if [ ${#UNKNOWN_CRATES[@]} -gt 0 ]; then
    echo
    echo "Unknown/unclassified crates (actionable):"
    for c in "${UNKNOWN_CRATES[@]}"; do
      printf "  - %s\n" "$c"
    done
  fi
  echo
}

print_violations() {
  print_header "Dependency violations (hard FAILs)"
  if [ ${#FAIL_LIST[@]} -eq 0 ]; then
    log_pass "no hard dependency-direction violations found"
  else
    for msg in "${FAIL_LIST[@]}"; do
      printf "  %s\n" "$msg"
    done
  fi
  echo
}

print_advisories() {
  print_header "Advisory warnings"
  if [ ${#WARN_LIST[@]} -eq 0 ]; then
    log_pass "no advisory warnings"
  else
    for msg in "${WARN_LIST[@]}"; do
      printf "  %s\n" "$msg"
    done
  fi
  echo
}

print_composition_notes() {
  print_header "Composition-root notes"
  if [ ${#COMPOSITION_ROOTS[@]} -eq 0 ]; then
    echo "  (no composition roots detected)"
  else
    for n in "${COMPOSITION_ROOTS[@]}"; do
      printf "  WARN: %s is a composition root; allowed broader composition dependencies but should not become a library dependency\n" "$n"
    done
  fi
  echo
}

print_summary() {
  print_header "Summary"
  printf "  PASS: %d\n" "$PASS_COUNT"
  printf "  WARN: %d\n" "$WARN_COUNT"
  printf "  FAIL: %d\n" "$FAIL_COUNT"
  printf "  Unknown crates: %d\n" "${#UNKNOWN_CRATES[@]}"
  printf "  Unknown dependencies: %d\n" "$UNKNOWN_DEP_COUNT"
  real_advisory_count=$((WARN_COUNT - UNKNOWN_DEP_COUNT))
  if (( real_advisory_count < 0 )); then real_advisory_count=0; fi
  printf "  Advisory (likely real) warnings: %d\n" "$real_advisory_count"
  echo
}

# -------------------------
# Scan workspace Cargo.toml files
# -------------------------
echo "Scanning workspace crates for Cargo.toml..."
while IFS= read -r -d '' f; do
  CRATE_TOMLS+=("$f")
done < <(find crates apps -maxdepth 2 -type f -name Cargo.toml -print0 2>/dev/null || true)

# Populate CRATE->FAMILY mappings
for toml in "${CRATE_TOMLS[@]}"; do
  crate=$(crate_name_from_toml "$toml")
  crate="${crate:-$(basename "$(dirname "$toml")")}"
  fam=$(classify_family "$crate")
  CRATE_TO_FAMILY["$crate"]="$fam"
  FAMILY_TO_CRATES["$fam"]+="$crate "
  # collect composition roots explicitly
  if [[ "$fam" == "app_bin" || "$fam" == "daemon" || "$fam" == "harness" ]]; then
    COMPOSITION_ROOTS+=("$crate")
  fi
done

# Build UNKNOWN_CRATES list
for crate in "${!CRATE_TO_FAMILY[@]}"; do
  if [[ "${CRATE_TO_FAMILY[$crate]}" == "unknown" ]]; then
    UNKNOWN_CRATES+=("$crate")
  fi
done

# -------------------------
# Print compact inventory
# -------------------------
print_inventory

# -------------------------
# Dependency-direction checks
# -------------------------
echo "Running dependency-direction checks..."
for toml in "${CRATE_TOMLS[@]}"; do
  crate_dir=$(dirname "$toml")
  crate_name=$(crate_name_from_toml "$toml")
  crate_name="${crate_name:-$(basename "$crate_dir")}"
  from_family="${CRATE_TO_FAMILY[$crate_name]:-unknown}"
  from_rank=$(family_rank "$from_family")

  deps=$(grep -E "zaroxi[-_][a-z0-9\-_/]+" "$toml" || true)
  if [ -z "$deps" ]; then
    log_pass "cargo-deps: $crate_name declares no zaroxi-* deps"
    continue
  fi

  while IFS= read -r line; do
    for token in $(echo "$line" | grep -oE "zaroxi[-_][a-z0-9\-_/]+" || true); do
      # normalize token to dashed form and canonical prefix if possible
      dep_crate="${token//_/-}"
      dep_crate="${dep_crate#*/}"
      if [[ "$dep_crate" == zaroxi-* ]]; then
        dep_crate="${dep_crate#zaroxi-}"
        dep_crate="zaroxi-${dep_crate}"
      fi

      # Prefer workspace-known crate mapping first
      if [[ -n "${CRATE_TO_FAMILY[$dep_crate]:-}" ]]; then
        to_family="${CRATE_TO_FAMILY[$dep_crate]}"
      else
        # map common umbrella tokens to families to avoid noisy warnings
        case "$dep_crate" in
          zaroxi-core) to_family="core-runtime" ;;
          zaroxi-kernel) to_family="kernel" ;;
          zaroxi-interface) to_family="interface" ;;
          zaroxi-application) to_family="application" ;;
          zaroxi-domain) to_family="domain" ;;
          zaroxi-infrastructure) to_family="infrastructure" ;;
          zaroxi-intelligence) to_family="intelligence" ;;
          zaroxi-security) to_family="security" ;;
          # last resort: pattern classify the token (may still be 'unknown')
          *) to_family=$(classify_family "$dep_crate") ;;
        esac
      fi

      to_rank=$(family_rank "$to_family")

      # infra explicit allowed exceptions for listed higher-layer deps (application/domain/core)
      if [[ "$from_family" == "infrastructure" && ( "$to_family" == "application" || "$to_family" == "domain" || "$to_family" == "core-runtime" ) ]]; then
        if is_infra_exception "$crate_name" "$dep_crate"; then
          log_pass "allowed infra adapter exception: $crate_name -> $dep_crate"
          continue
        fi
      fi

      # if either side unknown, count and warn (but do not fail)
      if [[ "$from_family" == "unknown" || "$to_family" == "unknown" ]]; then
        UNKNOWN_DEP_COUNT=$((UNKNOWN_DEP_COUNT+1))
        UNKNOWN_DEPS+=("$crate_name -> $dep_crate")
        log_warn "dependency-direction: $crate_name -> $dep_crate (unknown family; add FAMILY_PATTERNS if valid) origin=$toml"
        continue
      fi

      # Allowed direction: to_rank <= from_rank
      if (( to_rank <= from_rank )); then
        log_pass "dependency-direction: $crate_name ($from_family) -> $dep_crate ($to_family) allowed"
      else
        msg="FAIL: $crate_name ($from_family) -> $dep_crate ($to_family) is forbidden (origin=$toml)"
        log_fail "$msg"
      fi
    done
  done <<< "$deps"
done

# -------------------------
# Source advisory checks (engine seam leakage)
# -------------------------
echo
echo "Running source-advisory checks..."
for crate in "${!CRATE_TO_FAMILY[@]}"; do
  fam="${CRATE_TO_FAMILY[$crate]}"
  if [[ "$fam" == "interface" || "$fam" == "app_bin" || "$fam" == "harness" ]]; then
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
      log_warn "advisory: $crate (family=$fam) references engine backend internals; prefer engine seams (text_seam) and engine-facing primitives (RenderIntent/Transcript)"
    else
      log_pass "advisory: $crate (family=$fam) shows no obvious engine backend references"
    fi
  fi
done

# Additional advisory for interface-desktop specifically
if [ -d "$ROOT_DIR/crates/zaroxi-interface-desktop" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi-core-engine-text|glyphon" "crates/zaroxi-interface-desktop" >/dev/null 2>&1; then
    log_warn "advisory: crates/zaroxi-interface-desktop references engine-text or glyphon; interface should use render intents/transcripts"
  else
    log_pass "advisory: interface-desktop appears to use engine-facing primitives"
  fi
fi

# -------------------------
# Legacy deterministic checks (kept for strict rules)
# -------------------------
echo
echo "Running legacy deterministic checks..."
if [ -d "$ROOT_DIR/crates/zaroxi-domain-workspace" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_interface_|zaroxi-interface-|zaroxi_application_|zaroxi-application-|zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-domain-workspace" >/dev/null 2>&1; then
    log_fail "domain-workspace imports interface/application/infrastructure (forbidden)."
  else
    log_pass "legacy-check: domain-workspace has no upward imports"
  fi
fi

if [ -d "$ROOT_DIR/crates/zaroxi-application-workspace" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-application-workspace" >/dev/null 2>&1; then
    log_fail "application-workspace imports interface (forbidden)."
  else
    log_pass "legacy-check: application-workspace has no interface imports"
  fi
fi

if [ -d "$ROOT_DIR/crates/zaroxi-interface-desktop" ]; then
  if grep -R --line-number --exclude-dir=target -E "zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-interface-desktop" >/dev/null 2>&1; then
    log_fail "interface-desktop imports infrastructure (forbidden)."
  else
    log_pass "legacy-check: interface-desktop does not import infrastructure"
  fi
fi

# -------------------------
# Final structured output
# -------------------------
print_violations
print_advisories
print_composition_notes
print_summary

# Exit logic: FAILs are fatal; WARNs do not fail CI, but unknown deps are highlighted.
if (( FAIL_COUNT > 0 )); then
  echo "Fix the above FAILs to restore architectural integrity."
  exit 1
fi

if (( UNKNOWN_DEP_COUNT > 0 )); then
  echo "Some dependencies could not be mapped to known families (see advisory WARNs)."
  echo "Consider adding FAMILY_PATTERNS entries for new crates or normalizing Cargo.toml dependency tokens."
  exit 0
fi

if (( WARN_COUNT > 0 )); then
  echo "Advisory warnings were emitted; review WARN lines above."
  exit 0
fi

echo "Architecture checks passed (no FAILs, no WARNs)."
exit 0
