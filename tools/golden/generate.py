#!/usr/bin/env python3
"""Generate the pm4py golden files under fixtures/golden/<area>/<case>.json.

Usage:
    generate.py [--area A] [--case C] [--check]

--check regenerates in memory, writes nothing, and exits 1 if any file would
change, is missing, or is stale (on disk with no matching case).
"""

from __future__ import annotations

import argparse
import importlib
import os
import pkgutil
import subprocess
import sys
import traceback
from pathlib import Path

# Quiet pm4py before it is imported anywhere.
os.environ.setdefault("PM4PY_SHOW_PROGRESS_BAR", "False")
os.environ.setdefault("PM4PY_SHOW_INTERNAL_WARNINGS", "False")

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

# harness imports pm4py first; pm4py prints a banner when __main__ imports it.
from harness import canonical  # noqa: E402
from harness.fixtures import REPO_ROOT, describe_loader, fixture  # noqa: E402
from harness.registry import REGISTRY, Case  # noqa: E402

import pm4py  # noqa: E402
from pm4py import meta as pm4py_meta  # noqa: E402

GOLDEN = REPO_ROOT / "fixtures" / "golden"
PM4PY_VERSION = "2.7.23.8"
PM4PY_COMMIT_PREFIX = "24a3bf6"


def pm4py_commit() -> str:
    """The commit of the pm4py checkout in use: $PM4PY_SRC, else the checkout pm4py was imported from."""
    src = Path(os.environ.get("PM4PY_SRC") or Path(pm4py.__file__).resolve().parents[1])
    try:
        out = subprocess.run(
            ["git", "-C", str(src), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as err:
        raise SystemExit(f"cannot read the pm4py commit from {src}: {err}")
    return out.stdout.strip()


def check_oracle(commit: str) -> None:
    """Refuse to write goldens from any pm4py but the pinned one."""
    if pm4py_meta.__version__ != PM4PY_VERSION or not commit.startswith(PM4PY_COMMIT_PREFIX):
        raise SystemExit(
            f"goldens come from pm4py {PM4PY_VERSION} (commit {PM4PY_COMMIT_PREFIX}); "
            f"this interpreter has pm4py {pm4py_meta.__version__} at commit {commit[:7]}"
        )


def load_areas(only: str | None) -> list[str]:
    import cases

    names = sorted(m.name for m in pkgutil.iter_modules(cases.__path__))
    if only is not None:
        if only not in names:
            raise SystemExit(f"unknown area {only!r}; known: {', '.join(names)}")
        names = [only]
    for name in names:
        importlib.import_module(f"cases.{name}")
    return names


def render(c: Case, commit: str) -> str:
    paths = {role: fixture(rel) for role, rel in c.fixtures.items()}
    expected = c.compute(paths, **c.params)
    doc = {
        "meta": {
            "area": c.area,
            "case": c.id,
            "pm4py_version": pm4py_meta.__version__,
            "pm4py_commit": commit,
            "functions": c.functions,
            "fixtures": {role: f"fixtures/logs/{rel}" for role, rel in c.fixtures.items()},
            "loaders": {role: describe_loader(rel) for role, rel in c.fixtures.items()},
            "params": c.params,
        },
        "expected": expected,
    }
    return canonical.dumps(canonical.normalize(doc))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--area", help="only this area")
    parser.add_argument("--case", help="only this case id (within --area, or in any area)")
    parser.add_argument("--check", action="store_true", help="write nothing; exit 1 if any file would change")
    args = parser.parse_args(argv)

    areas = load_areas(args.area)
    commit = pm4py_commit()
    check_oracle(commit)
    selected = [
        c
        for area in areas
        for c in REGISTRY.get(area, {}).values()
        if args.case is None or c.id == args.case
    ]
    if args.case is not None and not selected:
        raise SystemExit(f"no case {args.case!r} in {', '.join(areas)}")

    failures = 0
    for c in sorted(selected, key=lambda c: (c.area, c.id)):
        target = GOLDEN / c.area / f"{c.id}.json"
        label = f"{c.area}/{c.id}"
        try:
            text = render(c, commit)
        except Exception:
            failures += 1
            print(f"FAIL   {label}", file=sys.stderr)
            traceback.print_exc()
            continue
        old = target.read_bytes() if target.exists() else None
        new = text.encode("utf-8")
        if old == new:
            print(f"same   {label}")
        elif args.check:
            failures += 1
            print(f"{'differs' if old else 'missing'} {label}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(new)
            print(f"wrote  {label}")

    # Stale files only make sense when a whole area was regenerated.
    if args.case is None:
        for area in areas:
            known = {f"{cid}.json" for cid in REGISTRY.get(area, {})}
            for path in sorted((GOLDEN / area).glob("*.json")):
                if path.name not in known:
                    failures += 1
                    print(f"stale  {area}/{path.stem} (no case registers it; delete the file)")

        if args.area is None:
            for path in sorted(p for p in GOLDEN.glob("*") if p.is_dir()):
                if path.name not in areas:
                    failures += 1
                    print(f"stale  {path.name}/ (no cases/{path.name}.py)")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
