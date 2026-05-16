#!/usr/bin/env python3
"""
check_crate_naming.py - Validate crate naming conventions for the Zaroxi workspace.

Rules:
- All crate names must start with "zaroxi-".
- Pattern: zaroxi-{layer}-{sublayer?}-{concern}
  - Valid layers: kernel, core, domain, application, interface, intelligence, security, infrastructure
  - For "core" crates a sublayer is required and must be one of the allowed core sublayers
  - For non-core crates sublayer must be omitted (concern may contain additional dashes)
- Duplicate concern names within the same (layer, sublayer) are flagged.

Outputs clear violations and exits with code 1 on any violation.

No external deps (stdlib only). Uses tomllib (Python 3.12+).
"""

from __future__ import annotations

import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Set

ROOT = Path.cwd()
IGNORED_DIRS = {"target", ".git", ".github", "node_modules"}

VALID_LAYERS = {
    "kernel",
    "core",
    "domain",
    "application",
    "interface",
    "intelligence",
    "security",
    "infrastructure",
}

# Valid core sublayers as requested
VALID_CORE_SUBLAYERS = {
    "editor",
    "engine",
    "platform",
    "workspace",
    "event",
    "input",
    "io",
    "runtime",
    "scheduler",
    "state",
    "sync",
    "task",
    "telemetry",
    "threading",
    "commands",
    "plugin",
}

@dataclass
class NamingViolation:
    manifest: Path
    crate: Optional[str]
    message: str


def find_manifests(root: Path) -> List[Path]:
    manifests = []
    for p in root.rglob("Cargo.toml"):
        if p.resolve() == (root / "Cargo.toml").resolve():
            continue
        if any(part in IGNORED_DIRS for part in p.parts):
            continue
        manifests.append(p)
    return sorted(manifests)


def load_package_name(manifest: Path) -> Optional[str]:
    try:
        with manifest.open("rb") as f:
            data = tomllib.load(f)
    except Exception:
        return None
    pkg = data.get("package", {})
    return pkg.get("name")


def analyze() -> Tuple[List[NamingViolation], Dict[Tuple[str, Optional[str]], Set[str]]]:
    manifests = find_manifests(ROOT)
    violations: List[NamingViolation] = []
    seen: Dict[Tuple[str, Optional[str]], Set[str]] = {}

    for m in manifests:
        name = load_package_name(m)
        if not name:
            violations.append(NamingViolation(manifest=m, crate=None, message="Missing or malformed [package].name"))
            continue

        if not name.startswith("zaroxi-"):
            violations.append(NamingViolation(manifest=m, crate=name, message="Crate name must start with 'zaroxi-'"))
            continue

        parts = name.split("-")
        # parts: ["zaroxi", layer, ...rest]
        if len(parts) < 3:
            violations.append(NamingViolation(manifest=m, crate=name, message="Crate name too short; expected 'zaroxi-{layer}-{concern}'"))
            continue

        _, layer, *rest = parts
        if layer not in VALID_LAYERS:
            violations.append(NamingViolation(manifest=m, crate=name, message=f"Invalid layer '{layer}'. Valid: {', '.join(sorted(VALID_LAYERS))}"))
            continue

        if layer == "core":
            # core crates must have a sublayer and a concern
            if len(rest) < 2:
                violations.append(NamingViolation(manifest=m, crate=name, message="Core crates must include a sublayer and concern: zaroxi-core-{sublayer}-{concern}"))
                continue
            sublayer = rest[0]
            concern = "-".join(rest[1:])
            if sublayer not in VALID_CORE_SUBLAYERS:
                violations.append(NamingViolation(manifest=m, crate=name, message=f"Invalid core sublayer '{sublayer}'. Valid core sublayers: {', '.join(sorted(VALID_CORE_SUBLAYERS))}"))
                continue
            key = ("core", sublayer)
        else:
            # non-core: rest is the concern; sublayer must not be present as a separate required token.
            sublayer = None
            concern = "-".join(rest)
            key = (layer, sublayer)

        # ensure concern is non-empty
        if not concern or concern.strip() == "":
            violations.append(NamingViolation(manifest=m, crate=name, message="Empty concern segment in crate name"))
            continue

        # check duplicate concerns within same (layer, sublayer)
        seen.setdefault(key, set())
        if concern in seen[key]:
            violations.append(NamingViolation(manifest=m, crate=name, message=f"Duplicate concern '{concern}' within layer {key}"))
        else:
            seen[key].add(concern)

    return violations, seen


def main() -> int:
    violations, _ = analyze()
    if not violations:
        print("All crate names follow the workspace naming conventions.")
        return 0

    for v in violations:
        crate_display = v.crate or "<unknown>"
        print(f"[NAMING VIOLATION] {crate_display}")
        print(f"  {v.message}")
        print(f"  File: {v.manifest}")
        print()

    print(f"Total naming violations: {len(violations)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
