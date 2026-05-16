# justfile for Zaroxi workspace
# Local developer shortcuts. Requires 'just' to be installed.
# See: https://just.systems/ for installation instructions.

# Default recipe: run full local CI
default:
    just validate

# Run clippy and fmt checks
check:
    @echo "Running clippy and fmt checks..."
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check

# Run all tests
test:
    @echo "Running test suite..."
    cargo test --workspace

# Run cargo-deny checks
deny:
    @echo "Running cargo-deny..."
    cargo deny check

# Run the same sequence CI runs (fast local sanity before pushing)
ci:
    @echo "Running full local CI sequence..."
    just check
    cargo check --workspace --all-targets
    cargo test --workspace
    just deny
    python3 .github/scripts/check_layer_deps.py

# ARCHITECTURE / enforcement helpers
arch:
    @echo "Running architecture enforcement scripts..."
    python3 .github/scripts/check_layer_deps.py --report layer-deps-report.json
    python3 .github/scripts/check_crate_naming.py
    python3 .github/scripts/check_circular_deps.py
    python3 .github/scripts/check_crate_size.py

arch-report:
    @echo "Generating architecture report and formatting with jq (if available)..."
    python3 .github/scripts/check_layer_deps.py --report layer-deps-report.json
    @if command -v jq >/dev/null 2>&1; then jq . layer-deps-report.json || true; else cat layer-deps-report.json; fi

# Run just the size check with full output
size:
    @echo "Running crate size analysis..."
    python3 .github/scripts/check_crate_size.py

# Scaffold a new crate under crates/ and register it in workspace Cargo.toml
# Usage: just new-crate NAME
new-crate NAME:
    @if [ -z "{{NAME}}" ]; then echo "Usage: just new-crate NAME"; exit 1; fi
    @CRATE_NAME="{{NAME}}"
    @CRATE_DIR="crates/${CRATE_NAME}"
    @echo "Creating ${CRATE_DIR}..."
    @mkdir -p ${CRATE_DIR}/src
    @cat > ${CRATE_DIR}/Cargo.toml <<'CARGO'
[package]
name = "{{NAME}}"
version = "0.1.0"
edition = "2024"
description = "new Zaroxi crate {{NAME}}"

[dependencies]
CARGO
    @cat > ${CRATE_DIR}/src/lib.rs <<'RS'
#![allow(unused)]
// Library entry for {{NAME}}

pub fn hello() -> &'static str {
    "hello"
}
RS
    @echo "Registering ${CRATE_DIR} in workspace Cargo.toml..."
    @python3 - <<'PY'
import re,sys
from pathlib import Path
name = "{{NAME}}"
cargo = Path("Cargo.toml")
text = cargo.read_text()
# Find the members = [ ... ] block and insert the new member if not present
m = re.search(r"(members\\s*=\\s*\\[)(.*?)(\\])", text, flags=re.S)
if not m:
    print("Failed to locate workspace members array in Cargo.toml", file=sys.stderr)
    sys.exit(1)
prefix, body, suffix = m.group(1), m.group(2), m.group(3)
entry = f'  "crates/{name}",\\n'
if f'crates/{name}' in body:
    print("Entry already present in Cargo.toml; skipping addition.")
else:
    # insert before closing bracket, keep a trailing newline style
    new_body = body + entry
    new_text = text[:m.start()] + prefix + new_body + suffix + text[m.end():]
    cargo.write_text(new_text)
    print(f"Added crates/{name} to workspace members.")
PY
    @echo "Scaffolded crates/{{NAME}}. Run 'just check' and 'just test' locally."

# Combined validate: architecture + deny + fmt + clippy
validate:
    @echo "Running full local validation (architecture, deny, fmt, clippy)..."
    just arch
    just deny
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
