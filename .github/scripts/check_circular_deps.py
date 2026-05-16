#!/usr/bin/env python3
"""
check_circular_deps.py - Detect cycles among zaroxi-* crates in the workspace.

- Parses all Cargo.toml manifests (stdlib tomllib)
- Builds a directed graph of internal dependencies (edges only to other zaroxi-* crates present in the workspace)
- Runs DFS to detect cycles and prints full cycle paths in a human friendly format.

Exit codes:
  0 - no cycles
  1 - cycles found
  2 - fatal parse error
"""

from __future__ import annotations

import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Set, Tuple

ROOT = Path.cwd()
IGNORED_DIRS = {"target", ".git", ".github", "node_modules"}


@dataclass
class CrateNode:
    name: str
    manifest: Path
    deps: List[str]


def find_manifests(root: Path) -> List[Path]:
    manifests = []
    for p in root.rglob("Cargo.toml"):
        if p.resolve() == (root / "Cargo.toml").resolve():
            continue
        if any(part in IGNORED_DIRS for part in p.parts):
            continue
        manifests.append(p)
    return sorted(manifests)


def load_package_name(manifest: Path) -> str:
    try:
        with manifest.open("rb") as f:
            data = tomllib.load(f)
    except Exception as e:
        raise RuntimeError(f"Failed to parse {manifest}: {e}")
    pkg = data.get("package", {})
    if "name" not in pkg:
        raise RuntimeError(f"Missing [package].name in {manifest}")
    return pkg["name"]


def collect_zaroxi_deps(manifest: Path) -> List[str]:
    try:
        with manifest.open("rb") as f:
            data = tomllib.load(f)
    except Exception as e:
        raise RuntimeError(f"Failed to parse {manifest}: {e}")

    deps = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        sec = data.get(section, {})
        if isinstance(sec, dict):
            deps.update(k for k in sec.keys() if k.startswith("zaroxi-"))
    for k, v in data.items():
        if not k.startswith("target"):
            continue
        if isinstance(v, dict):
            for sub in ("dependencies", "dev-dependencies", "build-dependencies"):
                subsec = v.get(sub, {})
                if isinstance(subsec, dict):
                    deps.update(k for k in subsec.keys() if k.startswith("zaroxi-"))
    return sorted(deps)


def build_graph() -> Dict[str, CrateNode]:
    nodes: Dict[str, CrateNode] = {}
    manifests = find_manifests(ROOT)
    errors: List[str] = []
    for m in manifests:
        try:
            name = load_package_name(m)
            deps = collect_zaroxi_deps(m)
            nodes[name] = CrateNode(name=name, manifest=m, deps=deps)
        except Exception as e:
            errors.append(str(e))
    if errors:
        raise RuntimeError("Errors while reading manifests:\n" + "\n".join(errors))
    return nodes


def detect_cycles(nodes: Dict[str, CrateNode]) -> List[List[str]]:
    graph = {n: [d for d in nodes[n].deps if d in nodes] for n in nodes}
    visited: Set[str] = set()
    stack: List[str] = []
    on_stack: Set[str] = set()
    cycles: List[List[str]] = []

    def dfs(u: str):
        visited.add(u)
        stack.append(u)
        on_stack.add(u)
        for v in graph.get(u, []):
            if v not in visited:
                dfs(v)
            elif v in on_stack:
                # found a cycle: extract the cycle path starting from v
                try:
                    idx = stack.index(v)
                    cycle = stack[idx:] + [v]
                except ValueError:
                    cycle = stack[:] + [v]
                cycles.append(cycle)
        stack.pop()
        on_stack.remove(u)

    for node in sorted(nodes.keys()):
        if node not in visited:
            dfs(node)
    # Deduplicate cycles (canonicalize)
    unique_cycles: List[List[str]] = []
    seen_signatures: Set[Tuple[str, ...]] = set()
    for c in cycles:
        # produce a rotation-normalized tuple signature
        if not c:
            continue
        # drop the duplicate closing node for signature
        base = c[:-1]
        # normalize rotation by choosing minimal element index
        rotations = [tuple(base[i:] + base[:i]) for i in range(len(base))]
        sig = min(rotations)
        if sig not in seen_signatures:
            seen_signatures.add(sig)
            unique_cycles.append(list(sig) + [sig[0]])
    return unique_cycles


def main() -> int:
    try:
        nodes = build_graph()
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2

    cycles = detect_cycles(nodes)
    if not cycles:
        print("No circular dependencies detected among zaroxi-* workspace crates.")
        return 0

    for cyc in cycles:
        start = cyc[0]
        print(f"[CIRCULAR] {start}")
        for i in range(len(cyc) - 1):
            print(f"  → {cyc[i+1]}")
        # emphasize cycle closure
        print(f"  ← cycle here (back to {cyc[0]})")
        print()
    print(f"Total cycles detected: {len(cycles)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
