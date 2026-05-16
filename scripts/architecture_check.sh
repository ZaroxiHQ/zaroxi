#!/usr/bin/env bash
# Simple architecture check for Phase 0
# Fails if:
#  - crates/zaroxi-interface-* imports crates/zaroxi-infrastructure-*
#  - crates/zaroxi-domain-* imports crates/zaroxi-interface-* or crates/zaroxi-application-* or crates/zaroxi-infrastructure-*
#  - crates/zaroxi-application-* imports crates/zaroxi-interface-*
#
# This is intentionally conservative and pattern-based for a small repo; it's a fast CI gate.
set -euo pipefail

ROOT_DIR="$(pwd)"

fail() {
  echo "ARCH CHECK FAILED: $1"
  exit 1
}

check_grep() {
  pattern="$1"
  dir="$2"
  if grep -R --line-number --exclude-dir=target -E "$pattern" "$dir" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

echo "Running architecture checks..."

# 1) interface MUST NOT import infrastructure
if ! check_grep "zaroxi_infrastructure_" "$ROOT_DIR/crates/zaroxi-interface-"; then
  fail "interface crates import infrastructure (pattern 'zaroxi_infrastructure_')"
fi

# 2) domain MUST NOT import interface or application or infrastructure
if ! check_grep "zaroxi_interface_|zaroxi_application_|zaroxi_infrastructure_" "$ROOT_DIR/crates/zaroxi-domain-"; then
  fail "domain crates import interface/application/infrastructure (forbidden)"
fi

# 3) application MUST NOT import interface
if ! check_grep "zaroxi_interface_" "$ROOT_DIR/crates/zaroxi-application-"; then
  fail "application crates import interface (forbidden)"
fi

echo "Architecture checks passed."
exit 0
