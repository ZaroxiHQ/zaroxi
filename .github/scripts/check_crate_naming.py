#!/usr/bin/env python3
"""
check_crate_naming.py - Validate crate naming conventions for the Zaroxi workspace.

Scope (what this checker governs)
---------------------------------
The strict layered naming policy applies to *architecture layer crates*, which
live under `crates/`. Other packages in the repository serve different roles and
are NOT subject to the layer/sublayer naming rules:

- `apps/`   : composition roots / binaries (harness, daemons). They are only
              required to carry the `zaroxi-` project prefix; they intentionally
              do not encode an architecture layer in their name (e.g.
              `zaroxi-desktop-harness`).
- `tools/`  : developer tooling / build helpers (not shippable architecture
              crates). Exempt from the naming policy entirely.
- `docs/`   : documentation. Exempt entirely.

This scoping matches the authoritative workspace policy in the root Cargo.toml
(`[workspace.metadata.zaroxi]`) and the family model used by
`scripts/architecture_check.sh`, which already treats apps/ as composition roots.

Rules for `crates/` (architecture layer crates)
-----------------------------------------------
- Name must start with "zaroxi-".
- Pattern: zaroxi-{layer}-{...}
  - Valid layers: kernel, core, domain, application, interface, intelligence,
    security, infrastructure.
  - `core` crates come in two intentional shapes that both exist in this repo:
      1. Grouped subsystem:  zaroxi-core-{sublayer}-{concern}
         where {sublayer} is a grouping subsystem that owns many crates and
         therefore REQUIRES a concern suffix (e.g. zaroxi-core-editor-buffer).
      2. Flat concern:       zaroxi-core-{concern}
         where {concern} is a single top-level core concern that is its own
         crate (e.g. zaroxi-core-event, zaroxi-core-io).
  - For non-core layers the remainder is the concern (may contain dashes).
- Duplicate concern names within the same (layer, sublayer) are flagged.

No external deps (stdlib only). Uses tomllib (Python 3.12+).
Outputs clear violations and exits with code 1 on any violation.
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

# Grouping core subsystems: each owns many crates, so a concern suffix is
# REQUIRED (zaroxi-core-{sublayer}-{concern}).
GROUPING_CORE_SUBLAYERS = {
    "editor",
    "engine",
    "platform",
    "workspace",
    "plugin",
}

# Flat top-level core concerns: each is its own single crate
# (zaroxi-core-{concern}), with no further sublayer grouping.
FLAT_CORE_CONCERNS = {
    "commands",
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


def category_for(manifest: Path) -> str:
    """Classify a manifest by its top-level directory relative to the repo root.

    Returns one of: "crates" (architecture layer crate, full rules),
    "apps" (composition root, prefix-only rule), or "skip" (tools/docs/other).
    """
    try:
        rel = manifest.resolve().relative_to(ROOT.resolve())
    except ValueError:
        return "skip"
    top = rel.parts[0] if rel.parts else ""
    if top == "crates":
        return "crates"
    if top == "apps":
        return "apps"
    return "skip"


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
        category = category_for(m)
        if category == "skip":
            # tools/, docs/ and any non-architecture packages are not governed
            # by the layered naming policy.
            continue

        name = load_package_name(m)
        if not name:
            violations.append(NamingViolation(manifest=m, crate=None, message="Missing or malformed [package].name"))
            continue

        if not name.startswith("zaroxi-"):
            violations.append(NamingViolation(manifest=m, crate=name, message="Crate name must start with 'zaroxi-'"))
            continue

        if category == "apps":
            # Composition roots (harness/daemons) only need the project prefix;
            # they intentionally do not encode an architecture layer.
            continue

        # From here on: architecture layer crates under crates/.
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
            head = rest[0]
            if head in GROUPING_CORE_SUBLAYERS:
                # Grouped subsystem requires a concern suffix.
                concern = "-".join(rest[1:])
                if not concern:
                    violations.append(NamingViolation(
                        manifest=m,
                        crate=name,
                        message=(f"Grouped core subsystem '{head}' requires a concern: "
                                 f"zaroxi-core-{head}-{{concern}}"),
                    ))
                    continue
                key: Tuple[str, Optional[str]] = ("core", head)
            elif len(rest) == 1 and head in FLAT_CORE_CONCERNS:
                # Flat top-level core concern crate.
                concern = head
                key = ("core", None)
            else:
                violations.append(NamingViolation(
                    manifest=m,
                    crate=name,
                    message=(
                        "Unrecognized core crate name. Use a grouped subsystem "
                        f"(one of {', '.join(sorted(GROUPING_CORE_SUBLAYERS))}) with a concern, "
                        f"or a flat core concern (one of {', '.join(sorted(FLAT_CORE_CONCERNS))})."
                    ),
                ))
                continue
        else:
            # non-core: rest is the concern; sublayer must not be present as a separate required token.
            concern = "-".join(rest)
            key = (layer, None)

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
