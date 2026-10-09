"""Shared machinery for the pm4py golden generator.

Case modules under ``tools/golden/cases/`` import from here:

- :func:`case` registers a golden case.
- :mod:`harness.fixtures` loads fixture files the same way every time.
- :mod:`harness.behaviour` turns a discovered model into comparable behaviour.
"""

from .registry import Case, case

__all__ = ["Case", "case"]
