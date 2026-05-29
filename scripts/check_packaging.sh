#!/usr/bin/env bash
# Minimal packaging/check script used by Phase 13 to validate basic productization signals.
# This script is intentionally conservative and non-invasive: it runs metadata and
# the existing architecture check, then attempts a non-fatal cargo package dry-run
# for a representative crate.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "1/3: running cargo metadata..."
cargo metadata --no-deps > /dev/null

echo "2/3: running architecture check..."
./scripts/architecture_check.sh

echo "3/3: attempting packaging dry-run for 'zaroxi-interface-desktop' (non-fatal)..."
if cargo package --list -p zaroxi-interface-desktop --allow-dirty >/dev/null 2>&1; then
  echo "cargo package dry-run succeeded for zaroxi-interface-desktop"
else
  echo "cargo package dry-run failed or workspace is dirty; this check is non-fatal in this script"
fi

echo "packaging checks completed"
