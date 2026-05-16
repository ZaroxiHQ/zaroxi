#!/usr/bin/env bash
# Architecture check for Phase 3 (conservative, pattern-based).
# Fails fast when forbidden upward dependencies are detected for the main slice.
#
# Diagnosis (Phase 6): `crates/zaroxi-infrastructure-memory` intentionally
# depends on the application-level "zaroxi-application-workspace" port crate
# in order to implement a HistoryRepository adapter. This is a valid
# adapter-to-port dependency in the ports-and-adapters style (infrastructure
# implements an inner-layer port). The original script treated *any*
# application dependency from infra as a violation which produced a false
# positive for this intended adapter.
#
# Fix approach: keep the rule strict but allow narrowly-scoped exceptions
# for infrastructure adapters that implement specific application ports.
# We explicitly allow:
#  - zaroxi-infrastructure-ai-mock -> zaroxi-application-ai
#  - zaroxi-infrastructure-memory -> zaroxi-application-workspace
#
# Do NOT widen the whitelist beyond explicit port crate names.
#
# This file enforces the architecture checks and documents the permitted
# infra->application exceptions for clarity and auditability.

set -euo pipefail

ROOT_DIR="$(pwd)"

fail() {
  echo "ARCH CHECK FAILED: $1"
  exit 1
}

# Basic check: return 0 when no matches found, 1 when any match found.
check_grep() {
  pattern="$1"
  dir="$2"
  if grep -R --line-number --exclude-dir=target -E "$pattern" "$dir" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

# Enhanced check: allow a whitelist of excluded patterns from matches.
# Usage: check_grep_excluding "<pattern>" "<dir>" "<exclude_pattern>"
# Returns 0 if no non-excluded matches found, 1 if violation present.
check_grep_excluding() {
  pattern="$1"
  dir="$2"
  exclude="$3"
  # Find raw matches (if any)
  if ! grep -R --line-number --exclude-dir=target -E "$pattern" "$dir" >/dev/null 2>&1; then
    return 0
  fi

  # Capture matches and filter out allowed excludes; if anything remains it's a violation.
  if grep -R --line-number --exclude-dir=target -E "$pattern" "$dir" | grep -v -E "$exclude" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

echo "Running architecture checks (Phase 3)..."

# 1) interface MUST NOT import infrastructure
if [ -d "$ROOT_DIR/crates/zaroxi-interface-desktop" ]; then
  if ! check_grep "zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-interface-desktop"; then
    fail "interface-desktop imports infrastructure (forbidden)"
  fi
fi

# 2) domain MUST NOT import interface, application, or infrastructure
if [ -d "$ROOT_DIR/crates/zaroxi-domain-workspace" ]; then
  if ! check_grep "zaroxi_interface_|zaroxi-interface-|zaroxi_application_|zaroxi-application-|zaroxi_infrastructure_|zaroxi-infrastructure-" "$ROOT_DIR/crates/zaroxi-domain-workspace"; then
    fail "domain-workspace imports interface/application/infrastructure (forbidden)"
  fi
fi

# 3) application MUST NOT import interface
if [ -d "$ROOT_DIR/crates/zaroxi-application-workspace" ]; then
  if ! check_grep "zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-application-workspace"; then
    fail "application-workspace imports interface (forbidden)"
  fi
fi

# 4) infrastructure MUST NOT import application or interface (adapters only depend inward)
# Allowed exceptions (explicit, narrow):
#  - zaroxi-infrastructure-ai-mock may depend on zaroxi-application-ai (implements AiClient port)
#  - zaroxi-infrastructure-memory may depend on zaroxi-application-workspace (implements HistoryRepository port)
if [ -d "$ROOT_DIR/crates/zaroxi-infrastructure-memory" ]; then
  # Allow only the specific application port crate 'zaroxi-application-workspace' if present.
  if ! check_grep_excluding "zaroxi_application_|zaroxi-application-|zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-infrastructure-memory" "zaroxi_application_workspace|zaroxi-application-workspace"; then
    fail "infrastructure-memory imports forbidden application or interface crates (only 'zaroxi-application-workspace' is allowed)"
  fi
fi

if [ -d "$ROOT_DIR/crates/zaroxi-infrastructure-ai-mock" ]; then
  # Allow imports of the application-ai port crate only; disallow any other application/interface imports.
  if ! check_grep_excluding "zaroxi_application_|zaroxi-application-|zaroxi_interface_|zaroxi-interface-" "$ROOT_DIR/crates/zaroxi-infrastructure-ai-mock" "zaroxi_application_ai|zaroxi-application-ai"; then
    fail "infrastructure-ai-mock imports forbidden application/interface crates (only 'zaroxi-application-ai' is allowed)"
  fi
fi

echo "Architecture checks passed."
exit 0
