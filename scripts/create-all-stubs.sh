#!/usr/bin/env bash
set -euo pipefail

# Create missing crate stubs for every member listed in the workspace Cargo.toml.
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

# Extract the members block between "members =" and the matching closing bracket.
members_block=$(awk '
  BEGIN { in=0; }
  /members[[:space:]]*=/ {
    # find the opening bracket on this line (or the following lines)
    idx = index($0, "[");
    if (idx > 0) {
      # capture after the first '['
      sub(".*\\[","[");
      in = 1;
    } else {
      in = 1;
    }
  }
  in {
    print $0;
    if (index($0, "]") > 0) exit;
  }
' "$WORKSPACE_MANIFEST")

if [ -z "$members_block" ]; then
  echo "Error: failed to parse members block from $WORKSPACE_MANIFEST"
  exit 1
fi

# Find all quoted paths inside the block.
mapfile -t members < <(printf "%s" "$members_block" | grep -oP '"\K[^"]+(?=")' || true)

if [ ${#members[@]} -eq 0 ]; then
  echo "No workspace members found in Cargo.toml"
  exit 0
fi

created=0
skipped=0

for m in "${members[@]}"; do
  # Normalize path (strip trailing commas/spaces)
  member_path="$(echo "$m" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"

  # Skip obvious non-crate entries
  case "$member_path" in
    docs|docs/*|.github/*|tools/*)
      echo "skipping non-crate member: $member_path"
      ((skipped++))
      continue
      ;;
  esac

  crate_dir="$member_path"
  cargo_toml_path="$crate_dir/Cargo.toml"
  src_dir="$crate_dir/src"
  lib_rs_path="$src_dir/lib.rs"

  # Ensure directory exists
  if [ ! -d "$crate_dir" ]; then
    mkdir -p "$src_dir"
  else
    mkdir -p "$src_dir" || true
  fi

  # Derive package name from the directory basename.
  pkg_name="$(basename "$crate_dir")"

  # Create Cargo.toml if it does not exist
  if [ ! -f "$cargo_toml_path" ]; then
    cat > "$cargo_toml_path" <<EOF
[package]
name = "$pkg_name"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "Auto-generated stub crate for $pkg_name (scaffolded by scripts/create-all-stubs.sh)."
rust-version = "1.70"

[dependencies]
# Add crate-specific dependencies here when replacing the stub.
EOF
    echo "created: $cargo_toml_path"
    ((created++))
  else
    echo "exists: $cargo_toml_path (skipped)"
    ((skipped++))
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
    ((created++))
  else
    echo "exists: $lib_rs_path (skipped)"
    ((skipped++))
  fi
done

echo "done. created $created files; skipped $skipped existing files."
