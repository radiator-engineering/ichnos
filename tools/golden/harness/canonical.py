"""Deterministic JSON output.

Running the generator twice must give byte-identical files. :func:`normalize`
turns pm4py and pandas values into plain JSON values, and :func:`dumps` writes
them in one fixed layout:

- object keys sorted by code point, two-space indent, UTF-8, trailing newline;
- sets and frozensets become lists sorted by their canonical JSON text;
- tuples become lists;
- floats use Python's shortest round-trip form (``repr``); ``-0.0`` becomes
  ``0.0``; NaN and infinities become the strings ``"NaN"``, ``"Infinity"`` and
  ``"-Infinity"``, because JSON has no literal for them;
- datetimes become RFC 3339 strings in UTC with a ``Z`` suffix; fractional
  seconds appear only when non-zero (6 digits, or 9 when there are nanoseconds);
  naive datetimes are taken as UTC;
- timedeltas become float seconds;
- ``None``, ``pandas.NA`` and ``NaT`` become ``null``.

Mappings must have string keys. A case whose result is keyed by tuples (a DFG,
variants) must turn it into a list of records itself; the error names the key.
"""

from __future__ import annotations

import datetime as dt
import json
import math
from collections.abc import Mapping
from typing import Any

import numpy as np
import pandas as pd


def normalize(value: Any, path: str = "$") -> Any:
    """Convert ``value`` to plain JSON types, applying the rules in the module doc."""
    if value is None or value is pd.NaT:
        return None
    if isinstance(value, (bool, np.bool_)):
        return bool(value)
    if isinstance(value, (int, np.integer)):
        return int(value)
    if isinstance(value, (float, np.floating)):
        return _float(float(value))
    if isinstance(value, str):
        return value
    if isinstance(value, pd.Timestamp):
        return _timestamp(value)
    if isinstance(value, dt.datetime):
        return _timestamp(pd.Timestamp(value))
    if isinstance(value, dt.date):
        return value.isoformat()
    if isinstance(value, (pd.Timedelta, dt.timedelta, np.timedelta64)):
        return _float(pd.Timedelta(value).total_seconds())
    if isinstance(value, Mapping):
        out = {}
        for key, item in value.items():
            if not isinstance(key, str):
                raise TypeError(
                    f"{path}: mapping key {key!r} is {type(key).__name__}, not str; "
                    "convert it to a list of records in the case"
                )
            out[key] = normalize(item, f"{path}.{key}")
        return out
    if isinstance(value, (set, frozenset)):
        items = [normalize(item, f"{path}[]") for item in value]
        return sorted(items, key=_sort_key)
    if isinstance(value, (list, tuple, np.ndarray)):
        return [normalize(item, f"{path}[{i}]") for i, item in enumerate(value)]
    if value is pd.NA:
        return None
    raise TypeError(f"{path}: cannot serialize {type(value).__name__}: {value!r}")


def dumps(value: Any) -> str:
    """Canonical JSON text for an already-normalized value."""
    return json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False) + "\n"


def _sort_key(value: Any) -> str:
    return json.dumps(value, sort_keys=True, ensure_ascii=False, allow_nan=False)


def _float(x: float) -> float | str:
    if math.isnan(x):
        return "NaN"
    if math.isinf(x):
        return "Infinity" if x > 0 else "-Infinity"
    return 0.0 if x == 0.0 else x


def _timestamp(ts: pd.Timestamp) -> str:
    ts = ts.tz_localize("UTC") if ts.tzinfo is None else ts.tz_convert("UTC")
    text = ts.strftime("%Y-%m-%dT%H:%M:%S")
    if ts.nanosecond:
        text += f".{ts.microsecond * 1000 + ts.nanosecond:09d}"
    elif ts.microsecond:
        text += f".{ts.microsecond:06d}"
    return text + "Z"
