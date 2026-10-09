"""The case registry.

A case module is ``tools/golden/cases/<area>.py``. The module name is the area,
so a case can only register into the area of the file it lives in. That keeps
each lane inside its own file.
"""

from __future__ import annotations

import inspect
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

CASE_ID = re.compile(r"^[a-z0-9][a-z0-9_-]*$")

# Filled as case modules are imported: area -> case id -> Case.
REGISTRY: dict[str, dict[str, Case]] = {}


@dataclass(frozen=True)
class Case:
    """One golden case: what pm4py computes, from which fixtures, with which parameters."""

    area: str
    id: str
    # Role -> path relative to fixtures/logs, for example {"log": "running-example.xes"}.
    fixtures: dict[str, str]
    # The pm4py functions the case calls, for the meta block.
    functions: list[str]
    # Keyword arguments passed to ``compute`` and recorded in the meta block.
    params: dict[str, Any] = field(default_factory=dict)
    # compute(fixture_paths: dict[str, Path], **params) -> JSON-able value.
    compute: Callable[..., Any] | None = None


def case(
    id: str,
    *,
    functions: list[str],
    fixture: str | None = None,
    fixtures: dict[str, str] | None = None,
    params: dict[str, Any] | None = None,
) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    """Register a golden case. Use it as a decorator, or call it on a shared function.

    ``fixture="x.xes"`` is shorthand for ``fixtures={"log": "x.xes"}``. The
    decorated function receives the fixture paths as a ``{role: Path}`` dict,
    then ``params`` as keyword arguments, and returns the expected value.
    """
    if fixture is not None and fixtures is not None:
        raise ValueError(f"case {id!r}: give fixture or fixtures, not both")
    if not CASE_ID.match(id):
        raise ValueError(f"case id {id!r} must match {CASE_ID.pattern}")
    roles = {"log": fixture} if fixture is not None else dict(fixtures or {})
    area = _calling_area()

    def register(fn: Callable[..., Any]) -> Callable[..., Any]:
        cases = REGISTRY.setdefault(area, {})
        if id in cases:
            raise ValueError(f"duplicate case {area}/{id}")
        cases[id] = Case(
            area=area,
            id=id,
            fixtures=roles,
            functions=list(functions),
            params=dict(params or {}),
            compute=fn,
        )
        return fn

    return register


def _calling_area() -> str:
    """The area is the name of the ``cases/<area>.py`` module that called :func:`case`."""
    frame = inspect.currentframe()
    assert frame is not None
    # Skip this function and case() itself.
    caller = frame.f_back.f_back  # type: ignore[union-attr]
    path = Path(caller.f_globals.get("__file__", ""))  # type: ignore[union-attr]
    if path.parent.name != "cases":
        raise RuntimeError(f"cases must be registered from tools/golden/cases/<area>.py, not {path}")
    return path.stem
