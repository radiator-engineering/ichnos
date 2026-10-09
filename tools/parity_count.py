#!/usr/bin/env python3
"""Validate parity table rows and recompute status totals."""

import argparse
from collections import Counter
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", type=Path,
                        default=Path(__file__).resolve().parents[1] / "docs/parity.md")
    args = parser.parse_args()
    counts = Counter()
    names = set()
    for number, line in enumerate(args.path.read_text().splitlines(), 1):
        if not line.startswith("| `pm4py."):
            continue
        cells = [cell.strip() for cell in line.split("|")[1:-1]]
        if len(cells) != 6:
            parser.error(f"line {number}: expected six columns")
        name, source, rust, crate, status, notes = cells
        if name in names:
            parser.error(f"line {number}: duplicate {name}")
        if status not in ("todo", "ported", "dropped"):
            parser.error(f"line {number}: invalid status {status!r}")
        if not all((source, rust, crate, notes)):
            parser.error(f"line {number}: empty required cell")
        names.add(name)
        counts[status] += 1
    if not names:
        parser.error("no parity rows found")
    for status in ("todo", "ported", "dropped"):
        print(f"{status}: {counts[status]}")
    print(f"total: {sum(counts.values())}")


if __name__ == "__main__":
    main()
