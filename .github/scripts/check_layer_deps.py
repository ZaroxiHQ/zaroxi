#!/usr/bin/env python3
"""
Workspace layer dependency checker.

This script enforces the workspace dependency-direction rules described in
CONTRIBUTING.md. It parses the root Cargo.toml workspace members list and
each member's Cargo.toml to detect internal (workspace) dependencies and
ensures they follow the allowed layer rules.

If any violation is found, the script prints human-friendly errors and exits
with non-zero status so CI fails.

Notes:
- The script uses Python 3.11's tomllib to parse TOML. GitHub Actions uses a recent
  Python runtime (3.11) by default in the workflow above.
- To extend namespaces or adjust rules, edit the PREFIX_TO_LAYER and ALLOWED map below.
"""

import sys
import tomllib
from pathlib import Path
from typing import Dict, List

ROOT = Path.cwd()
ROOT_CARGO = ROOT / "Cargo.toml"


def load_toml(path: Path) -> dict:
    try:
        with path.open("rb") as f:
            return tomllib.load(f)
    except FileNotFoundError:
        print(f"Missing Cargo.toml: {path}", file=sys.stderr)
        sys.exit(2)
    except Exception as e:
        print(f"Failed to parse {path}: {e}", file=sys.stderr)
        sys.exit(2)


# Map crate-name prefix to a logical layer identifier.
PREFIX_TO_LAYER = {
    "zaroxi-kernel-": "kernel",
    "zaroxi-core-": "core",
    "zaroxi-domain-": "domain",
    "zaroxi-application-": "application",
    "zaroxi-interface-": "interface",
    "zaroxi-intelligence-": "intelligence",
    "zaroxi-security-": "security",
    "zaroxi-infrastructure-": "infrastructure",
}

# Allowed target layers for a crate in a given layer.
# The special layer "external" represents third-party crates (crates.io, git, etc.)
ALLOWED: Dict[str, List[str]] = {
    "kernel": ["kernel", "external"],
    "core": ["kernel", "core", "external"],
    "domain": ["kernel", "core", "external"],
    "application": ["kernel", "core", "domain", "external"],
    "interface": ["kernel", "core", "domain", "application", "interface", "external"],
    "intelligence": ["kernel", "core", "domain", "external"],
    "security": ["kernel", "core", "domain", "external"],
    "infrastructure": ["kernel", "core", "external"],
    # unknown layer: be conservative and allow external only (will be flagged for maintainers)
    "unknown": ["external"],
}

def detect_layer(crate_name: str) -> str:
    for prefix, layer in PREFIX_TO_LAYER.items():
        if crate_name.startswith(prefix):
            return layer
    return "unknown"


def collect_workspace_members(root_cargo: Path) -> List[str]:
    data = load_toml(root_cargo)
    workspace = data.get("workspace", {})
    members = workspace.get("members", [])
    if not isinstance(members, list):
        print("Workspace members in Cargo.toml must be an array", file=sys.stderr)
        sys.exit(2)
    return members


def read_package_name(manifest_path: Path) -> str:
    data = load_toml(manifest_path)
    package = data.get("package", {})
    name = package.get("name")
    if not name:
        print(f"No [package].name in {manifest_path}", file=sys.stderr)
        sys.exit(2)
    return name


def collect_internal_deps(manifest_path: Path) -> List[str]:
    data = load_toml(manifest_path)
    deps = []
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        sec = data.get(section, {})
        if isinstance(sec, dict):
            deps.extend(sec.keys())
    # also check target-specific deps (simple scan)
    for key in data.keys():
        if key.startswith("target"):
            sec = data.get(key, {})
            if isinstance(sec, dict):
                for subsec in ("dependencies", "dev-dependencies", "build-dependencies"):
                    sub = sec.get(subsec, {})
                    if isinstance(sub, dict):
                        deps.extend(sub.keys())
    return deps


def main():
    members = collect_workspace_members(ROOT_CARGO)

    # Build map from crate-name -> member-path
    crate_name_to_path: Dict[str, Path] = {}
    for member in members:
        member_path = (ROOT / member).resolve()
        manifest = member_path / "Cargo.toml"
        if not manifest.exists():
            # skip members that are not present in the checkout (could be intentionally absent)
            print(f"Skipping missing workspace member manifest: {manifest}", file=sys.stderr)
            continue
        name = read_package_name(manifest)
        crate_name_to_path[name] = member_path

    # Reverse mapping for quick lookup
    workspace_crates = set(crate_name_to_path.keys())

    violations = []

    for crate_name, crate_path in crate_name_to_path.items():
        manifest = crate_path / "Cargo.toml"
        internal_deps = collect_internal_deps(manifest)
        src_layer = detect_layer(crate_name)
        allowed = ALLOWED.get(src_layer, ALLOWED["unknown"])
        for dep in sorted(set(internal_deps)):
            if dep in workspace_crates:
                tgt_layer = detect_layer(dep)
                # if target layer is unknown, treat as potential violation to be reviewed
                if tgt_layer not in allowed:
                    violations.append(
                        f"{crate_name} ({src_layer}) -> {dep} ({tgt_layer}) is not allowed. "
                        f"Allowed target layers for {src_layer}: {allowed}."
                    )

    if violations:
        print("Dependency layer violations detected:", file=sys.stderr)
        for v in violations:
            print(" - " + v, file=sys.stderr)
        print("\nSee CONTRIBUTING.md and .github/scripts/check_layer_deps.py to update rules or request exceptions.", file=sys.stderr)
        sys.exit(1)

    print("No workspace dependency layer violations detected.")
    sys.exit(0)


if __name__ == "__main__":
    main()
