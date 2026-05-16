#!/usr/bin/env python3
"""
check_layer_deps.py - Production-grade workspace layer dependency checker.

This script recursively parses all Cargo.toml files inside the workspace,
builds an internal map of zaroxi-* crates and their internal dependencies,
and enforces the strict layer dependency matrix described in CONTRIBUTING.md.

Features:
- Parse every Cargo.toml under the repository root (skips obvious non-workspace folders)
- Determine crate layer from crate name prefix (zaroxi-kernel-*, zaroxi-core-*, ...)
- Collect only internal zaroxi-* dependencies from [dependencies], [dev-dependencies],
  [build-dependencies] and target-specific sections
- Strictly enforce the allowed layer matrix:
    kernel         -> kernel, external
    core           -> kernel, core
    domain         -> kernel, core, domain
    application    -> kernel, core, domain, application
    interface      -> kernel, core, domain, application, interface
    intelligence   -> kernel, core, domain
    security       -> kernel, core, domain
    infrastructure -> kernel, core
- --fix-suggestions: prints actionable suggestions for misplaced dependencies
- --report <path>: write a JSON report of the dependency graph + violations
- Robust error handling for missing/malformed manifests

No external Python dependencies are used (stdlib only).
Designed to run with Python 3.12 (uses tomllib).

Exit codes:
  0 - no violations / successful run
  1 - violations detected
  2 - fatal parse / IO error
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

ROOT = Path.cwd()
IGNORED_DIRS = {"target", ".git", ".github", "node_modules"}

# Layer detection prefix mapping
PREFIX_TO_LAYER: Dict[str, str] = {
    "zaroxi-kernel-": "kernel",
    "zaroxi-core-": "core",
    "zaroxi-domain-": "domain",
    "zaroxi-application-": "application",
    "zaroxi-interface-": "interface",
    "zaroxi-intelligence-": "intelligence",
    "zaroxi-security-": "security",
    "zaroxi-infrastructure-": "infrastructure",
}

# Strict allowed target layers for each source layer
ALLOWED: Dict[str, List[str]] = {
    "kernel": ["kernel", "external"],
    "core": ["kernel", "core"],
    "domain": ["kernel", "core", "domain"],
    "application": ["kernel", "core", "domain", "application"],
    "interface": ["kernel", "core", "domain", "application", "interface"],
    "intelligence": ["kernel", "core", "domain"],
    "security": ["kernel", "core", "domain"],
    "infrastructure": ["kernel", "core"],
    # Unknown crates (not matching naming) are treated conservatively as "external-only" allowed
    "unknown": ["external"],
}


@dataclass
class CrateInfo:
    name: str
    manifest: Path
    layer: str
    internal_deps: List[str]


@dataclass
class Violation:
    src: str
    src_layer: str
    tgt: str
    tgt_layer: str
    manifest: str
    reason: str
    suggestion: Optional[str] = None


def load_toml(path: Path) -> dict:
    try:
        with path.open("rb") as f:
            return tomllib.load(f)
    except FileNotFoundError:
        raise
    except Exception as exc:
        raise RuntimeError(f"Failed to parse TOML {path}: {exc}") from exc


def detect_layer(crate_name: str) -> str:
    for prefix, layer in PREFIX_TO_LAYER.items():
        if crate_name.startswith(prefix):
            return layer
    return "unknown"


def find_manifests(root: Path) -> List[Path]:
    """Recursively locate Cargo.toml files under the workspace root, skipping ignored dirs."""
    manifests: List[Path] = []
    for p in root.rglob("Cargo.toml"):
        # skip workspace root Cargo.toml itself (we'll parse it separately)
        if p.resolve() == (root / "Cargo.toml").resolve():
            continue
        # skip manifests in ignored directories
        if any(part in IGNORED_DIRS for part in p.parts):
            continue
        manifests.append(p)
    return sorted(manifests)


def read_package_name(manifest: Path) -> str:
    try:
        data = load_toml(manifest)
    except FileNotFoundError:
        raise RuntimeError(f"Missing manifest: {manifest}")
    package = data.get("package")
    if not package or "name" not in package:
        raise RuntimeError(f"Missing [package].name in {manifest}")
    return package["name"]


def collect_zaroxi_deps(manifest: Path) -> List[str]:
    """Collect declared dependency names that start with 'zaroxi-' from the manifest."""
    try:
        data = load_toml(manifest)
    except Exception as exc:
        raise RuntimeError(f"Failed to read {manifest}: {exc}") from exc

    deps: Set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        sec = data.get(section, {})
        if isinstance(sec, dict):
            deps.update(k for k in sec.keys() if k.startswith("zaroxi-"))
    # target-specific sections
    for k, v in data.items():
        if not k.startswith("target"):
            continue
        if isinstance(v, dict):
            for sub in ("dependencies", "dev-dependencies", "build-dependencies"):
                subsec = v.get(sub, {})
                if isinstance(subsec, dict):
                    deps.update(x for x in subsec.keys() if x.startswith("zaroxi-"))
    return sorted(deps)


def build_crate_map(root: Path) -> Dict[str, CrateInfo]:
    """Parse manifests and return mapping crate_name -> CrateInfo"""
    manifests = find_manifests(root)
    crate_map: Dict[str, CrateInfo] = {}
    errors: List[str] = []

    for manifest in manifests:
        try:
            name = read_package_name(manifest)
        except Exception as exc:
            errors.append(str(exc))
            continue
        layer = detect_layer(name)
        try:
            deps = collect_zaroxi_deps(manifest)
        except Exception as exc:
            errors.append(str(exc))
            deps = []
        crate_map[name] = CrateInfo(name=name, manifest=str(manifest), layer=layer, internal_deps=deps)

    if errors:
        raise RuntimeError("Errors while reading manifests:\n" + "\n".join(errors))
    return crate_map


def analyze(crate_map: Dict[str, CrateInfo]) -> Tuple[List[Violation], dict]:
    """Check each crate's internal zaroxi deps against allowed rules."""
    violations: List[Violation] = []
    nodes = {}

    workspace_names = set(crate_map.keys())

    for name, info in crate_map.items():
        nodes[name] = {"layer": info.layer, "manifest": info.manifest, "deps": info.internal_deps}
        allowed = ALLOWED.get(info.layer, ALLOWED["unknown"])
        for dep in info.internal_deps:
            # If dependency is not present in the workspace, treat as external (skip)
            if dep not in workspace_names:
                continue
            tgt_layer = crate_map[dep].layer
            if tgt_layer not in allowed:
                reason = f"{info.layer} crates may not depend on {tgt_layer} crates"
                suggestion = suggest_fix(info.layer, dep, crate_map)
                violations.append(Violation(src=name, src_layer=info.layer, tgt=dep, tgt_layer=tgt_layer,
                                            manifest=info.manifest, reason=reason, suggestion=suggestion))
    report = {"nodes": nodes, "allowed_matrix": ALLOWED}
    return violations, report


def suggest_fix(src_layer: str, dep: str, crate_map: Dict[str, CrateInfo]) -> Optional[str]:
    """Return a human-friendly suggestion for where the dependency should live instead."""
    allowed = ALLOWED.get(src_layer, ALLOWED["unknown"])
    dep_layer = crate_map.get(dep).layer if dep in crate_map else "unknown"
    # If the dep is higher-level than allowed, recommend moving shared functionality down to the highest
    # layer that the src_layer is allowed to depend on (prefer core over kernel when available).
    allowed_non_external = [l for l in allowed if l != "external"]
    if not allowed_non_external:
        return None
    preferred = allowed_non_external[-1] if allowed_non_external else allowed_non_external[0]
    return (f"Consider moving the functionality in '{dep}' to a crate under the '{preferred}' layer "
            f"(e.g. rename to 'zaroxi-{preferred}-...') or refactor to expose a small interface in an allowed layer.")


def print_violations(violations: List[Violation], fix_suggestions: bool = False) -> None:
    for v in violations:
        print(f"[VIOLATION] {v.src} → {v.tgt}")
        print(f"  {v.reason}")
        print(f"  File: {v.manifest}")
        if fix_suggestions and v.suggestion:
            print(f"  Suggestion: {v.suggestion}")
        print()


def write_report(report: dict, violations: List[Violation], path: Path) -> None:
    out = {"report": report, "violations": [asdict(v) for v in violations]}
    path.write_text(json.dumps(out, indent=2, ensure_ascii=False))


def parse_args() -> argparse.Namespace:
    ap = argparse.ArgumentParser(description="Enforce Zaroxi workspace layer dependency rules.")
    ap.add_argument("--fix-suggestions", action="store_true", help="Print fix suggestions alongside violations.")
    ap.add_argument("--report", type=Path, default=None, help="Path to write JSON report of dependency graph and violations.")
    return ap.parse_args()


def main() -> int:
    args = parse_args()
    try:
        crate_map = build_crate_map(ROOT)
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    violations, report = analyze(crate_map)

    if args.report:
        try:
            write_report(report, violations, args.report)
            print(f"Wrote layer dependency report to {args.report}")
        except Exception as exc:
            print(f"Failed to write report: {exc}", file=sys.stderr)

    if violations:
        print_violations(violations, fix_suggestions=args.fix_suggestions)
        print(f"Total violations: {len(violations)}", file=sys.stderr)
        return 1

    print("No workspace dependency layer violations detected.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
