#!/usr/bin/env bash
set -euo pipefail

# Robust stub creator for workspace members.
# - Uses Python to reliably extract the members array from Cargo.toml (handles multi-line, comments, etc).
# - Creates <member>/Cargo.toml and <member>/src/lib.rs only if they do not already exist.
# - Cargo.toml contains a short description field.
# - Safe to run multiple times; will not overwrite existing files.
#
# Usage:
#   bash scripts/create-all-stubs.sh

WORKSPACE_MANIFEST="Cargo.toml"

if [ ! -f "$WORKSPACE_MANIFEST" ]; then
  echo "Error: workspace Cargo.toml not found in repo root."
  exit 1
fi

# Extract members using a small Python snippet (more portable than complex awk on different awk implementations)
members_raw=$(python3 - <<'PY'
import re, sys
s = open("Cargo.toml", "r", encoding="utf-8").read()
m = re.search(r'members\s*=\s*\[(.*?)\]', s, re.S)
if not m:
    # No members block found; exit cleanly
    sys.exit(0)
block = m.group(1)
# Extract all double-quoted strings inside the members block
items = re.findall(r'"([^"]+)"', block)
for it in items:
    print(it)
PY
)

if [ -z "$members_raw" ]; then
  echo "No workspace members found in Cargo.toml (or Python failed to extract them)."
  exit 0
fi

created=0
skipped=0

# Read members_raw line by line
while IFS= read -r member_path; do
  member_path="${member_path%%,}" # strip trailing comma if any
  member_path="$(printf "%s" "$member_path" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  [ -z "$member_path" ] && continue

  # Skip obvious non-crate entries
  case "$member_path" in
    docs|docs/*|.github/*|tools/*)
      echo "skipping non-crate member: $member_path"
      skipped=$((skipped+1))
      continue
      ;;
  esac

  crate_dir="$member_path"
  cargo_toml_path="$crate_dir/Cargo.toml"
  src_dir="$crate_dir/src"
  lib_rs_path="$src_dir/lib.rs"

  # Ensure directory exists
  mkdir -p "$src_dir"

  # Derive package name from the directory basename.
  pkg_name="$(basename "$crate_dir")"

  # Create Cargo.toml if it does not exist
  if [ ! -f "$cargo_toml_path" ]; then
    cat > "$cargo_toml_path" <<EOF
[package]
name = "${pkg_name}"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "Auto-generated stub crate for ${pkg_name} (created by scripts/create-all-stubs.sh)."
rust-version = "1.70"

[dependencies]
# add crate-specific deps when replacing the stub
EOF
    echo "created: $cargo_toml_path"
    created=$((created+1))
  else
    echo "exists: $cargo_toml_path"
    skipped=$((skipped+1))
  fi

  # Create src/lib.rs if it does not exist
  if [ ! -f "$lib_rs_path" ]; then
    cat > "$lib_rs_path" <<'EOF'
// Auto-generated stub.
// Replace this file with the crate implementation.

#![allow(dead_code)]
#![doc = "Auto-generated crate stub. Replace with real implementation."]

/// The crate package name.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// A small sanity helper.
pub fn info() -> &'static str {
    CRATE_NAME
}
EOF
    echo "created: $lib_rs_path"
    created=$((created+1))
  else
    echo "exists: $lib_rs_path"
    skipped=$((skipped+1))
  fi
done <<EOF
$members_raw
EOF

echo "done. created $created files; skipped $skipped existing files."
