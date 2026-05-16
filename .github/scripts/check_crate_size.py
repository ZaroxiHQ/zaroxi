#!/usr/bin/env python3
"""
check_crate_size.py - Count lines of Rust code per crate src/ and warn/fail on thresholds.

Behavior:
- Recursively finds all Cargo.toml manifests in the workspace
- For each crate, counts total lines across all .rs files under its src/ directory
- Emits:
  - [WARN] if lines > 1500 (non-fatal)
  - [FAIL] and exit 1 if lines > 3000
- Prints a summary table sorted by size (descending)

No external deps (stdlib only). Uses tomllib (Python 3.12+).
"""

from __future__ import annotations

import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Tuple

ROOT = Path.cwd()
IGNORED_DIRS = {"target", ".git", ".github", "node_modules"}

WARN_THRESHOLD = 1500
FAIL_THRESHOLD = 3000


@dataclass
class SizeInfo:
    crate: str
    manifest: Path
    src_dir: Path
    lines: int


def find_manifests(root: Path) -> List[Path]:
    manifests = []
    for p in root.rglob("Cargo.toml"):
        if p.resolve() == (root / "Cargo.toml").resolve():
            continue
        if any(part in IGNORED_DIRS for part in p.parts):
            continue
        manifests.append(p)
    return sorted(manifests)


def read_package_name(manifest: Path) -> str:
    try:
        with manifest.open("rb") as f:
            data = tomllib.load(f)
    except Exception as e:
        raise RuntimeError(f"Failed to parse {manifest}: {e}")
    pkg = data.get("package", {})
    if "name" not in pkg:
        raise RuntimeError(f"Missing [package].name in {manifest}")
    return pkg["name"]


def count_lines_in_src(manifest: Path) -> Tuple[Path, int]:
    crate_dir = manifest.parent
    src_dir = crate_dir / "src"
    total = 0
    if not src_dir.exists():
        return src_dir, 0
    for rs in src_dir.rglob("*.rs"):
        try:
            with rs.open("r", encoding="utf8") as fh:
                for _ in fh:
                    total += 1
        except Exception:
            # ignore unreadable files but continue
            continue
    return src_dir, total


def main() -> int:
    manifests = find_manifests(ROOT)
    size_infos: List[SizeInfo] = []
    errors: List[str] = []

    for m in manifests:
        try:
            name = read_package_name(m)
        except Exception as e:
            errors.append(str(e))
            continue
        src_dir, lines = count_lines_in_src(m)
        size_infos.append(SizeInfo(crate=name, manifest=m, src_dir=src_dir, lines=lines))

    if errors:
        print("Errors while reading manifests:", file=sys.stderr)
        for e in errors:
            print("  " + e, file=sys.stderr)

    # sort by lines descending
    size_infos.sort(key=lambda x: x.lines, reverse=True)

    # Print table header
    print(f"{'CRATE':<40} {'LINES':>8}  {'SRC_DIR'}")
    print("-" * 80)
    fail = False
    for s in size_infos:
        line = f"{s.crate:<40} {s.lines:>8}  {str(s.src_dir)}"
        if s.lines >= FAIL_THRESHOLD:
            print(f"[FAIL] {line}")
            fail = True
        elif s.lines >= WARN_THRESHOLD:
            print(f"[WARN] {line}")
        else:
            print(f"       {line}")

    # summary
    total_crates = len(size_infos)
    total_lines = sum(s.lines for s in size_infos)
    print("-" * 80)
    print(f"Scanned {total_crates} crates, {total_lines} total lines")

    if fail:
        print("One or more crates exceed the fail threshold. Split crates before merging.", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
