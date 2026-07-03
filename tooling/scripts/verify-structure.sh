#!/usr/bin/env bash
#
# verify-structure.sh — sanity-check the Zaroxi (pure-Rust) monorepo layout.
#
# Zaroxi is a crate-first, pure-Rust editor/IDE runtime (winit + wgpu + vello +
# cosmic-text). There is NO Tauri, no web frontend, and no separate daemon
# services — every layer is a Rust crate under `crates/`, composed by the
# desktop harness under `apps/`.
#
# This script verifies the on-disk structure matches that architecture and that
# the workspace compiles. It is intended for contributors setting up the repo.

set -euo pipefail

echo "Verifying Zaroxi repository structure..."

# ── Required directories ─────────────────────────────────────────────────────
# Layered Rust crates live in crates/; the desktop harness (composition root)
# lives in apps/; bundled assets (fonts) in assets/; docs, tests and CI tooling
# in their respective folders.
required_dirs=(
  "crates"
  "apps"
  "apps/zaroxi-desktop-harness"
  "assets/fonts"
  "docs"
  "tests"
  "tooling/scripts"
  "scripts"
  ".github/scripts"
)

missing_dirs=()
for dir in "${required_dirs[@]}"; do
  if [ ! -d "$dir" ]; then
    missing_dirs+=("$dir")
  fi
done

if [ ${#missing_dirs[@]} -gt 0 ]; then
  echo "Missing directories:"
  for dir in "${missing_dirs[@]}"; do
    echo "  - $dir"
  done
  exit 1
fi

echo "✓ Directory structure looks good"

# ── Required files ───────────────────────────────────────────────────────────
required_files=(
  "Cargo.toml"
  "apps/zaroxi-desktop-harness/Cargo.toml"
  "assets/fonts/JetBrainsMonoNerdFont-Regular.ttf"
)

missing_files=()
for file in "${required_files[@]}"; do
  if [ ! -f "$file" ]; then
    missing_files+=("$file")
  fi
done

if [ ${#missing_files[@]} -gt 0 ]; then
  echo "Missing files:"
  for file in "${missing_files[@]}"; do
    echo "  - $file"
  done
  if [ ! -f "assets/fonts/JetBrainsMonoNerdFont-Regular.ttf" ]; then
    echo ""
    echo "Fonts are missing. Run ./scripts/download-fonts.sh to fetch them."
  fi
  exit 1
fi

echo "✓ Required files exist"

# ── Architecture layers ──────────────────────────────────────────────────────
# Every layer prefix in the strict dependency chain should be represented by at
# least one crate under crates/ (Kernel → Core → Domain → Application →
# Interface, plus Infrastructure / Intelligence / Security families).
required_layers=(
  "zaroxi-kernel-"
  "zaroxi-core-"
  "zaroxi-domain-"
  "zaroxi-application-"
  "zaroxi-interface-"
  "zaroxi-infrastructure-"
  "zaroxi-intelligence-"
  "zaroxi-security-"
)

missing_layers=()
for layer in "${required_layers[@]}"; do
  if ! ls -d crates/"${layer}"* >/dev/null 2>&1; then
    missing_layers+=("$layer")
  fi
done

if [ ${#missing_layers[@]} -gt 0 ]; then
  echo "Missing architecture layers under crates/:"
  for layer in "${missing_layers[@]}"; do
    echo "  - ${layer}*"
  done
  exit 1
fi

echo "✓ All architecture layers are present"

# ── Compilation ──────────────────────────────────────────────────────────────
echo "Checking compilation..."
if cargo check --workspace --quiet; then
  echo "✓ Workspace compiles successfully"
else
  echo "✗ Compilation failed. Run 'cargo check --workspace' for details"
  exit 1
fi

echo ""
echo "✅ Repository structure verification passed!"
echo ""
echo "Next steps:"
echo "1. Fetch bundled fonts (if needed): ./scripts/download-fonts.sh"
echo "2. Run the tests:                    cargo test --workspace"
echo "3. Launch the desktop GUI:           cargo run -p zaroxi-interface-desktop --bin gui_shell"
echo "4. Or run the composition harness:   cargo run -p zaroxi-desktop-harness"
