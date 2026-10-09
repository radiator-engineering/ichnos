"""Fixture loading.

Every case loads its inputs through these functions, so that all goldens for a
given file see the same pm4py object. :func:`describe_loader` records in each
golden's meta block how the fixture was loaded.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pandas as pd
import pm4py

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURES = REPO_ROOT / "fixtures" / "logs"

# Pinned so a later install of a Rust XES backend cannot change the goldens.
XES_VARIANT = "iterparse"


def fixture(rel: str) -> Path:
    """Absolute path of a fixture under ``fixtures/logs``."""
    path = FIXTURES / rel
    if not path.is_file():
        raise FileNotFoundError(f"fixture not found: {path}")
    return path


def load_log(path: Path) -> pd.DataFrame:
    """Load an event log as a pm4py-formatted pandas DataFrame.

    XES goes through ``pm4py.read_xes``. CSV goes through ``pandas.read_csv``
    and ``pm4py.format_dataframe`` with pm4py's default column names
    (``case:concept:name``, ``concept:name``, ``time:timestamp``), as pm4py's
    own tests do. Parquet goes through ``pandas.read_parquet`` then the same
    formatting.
    """
    name = path.name
    if name.endswith((".xes", ".xes.gz")):
        return pm4py.read_xes(str(path), variant=XES_VARIANT)
    if name.endswith(".csv"):
        return pm4py.format_dataframe(pd.read_csv(path))
    if name.endswith(".parquet"):
        return pm4py.format_dataframe(pd.read_parquet(path))
    raise ValueError(f"no log loader for {path}")


def load_model(path: Path) -> Any:
    """Load a model: ``(net, im, fm)`` for PNML, a ``ProcessTree`` for PTML, a BPMN graph for BPMN."""
    name = path.name
    if name.endswith(".pnml"):
        return pm4py.read_pnml(str(path))
    if name.endswith(".ptml"):
        return pm4py.read_ptml(str(path))
    if name.endswith(".bpmn"):
        return pm4py.read_bpmn(str(path))
    raise ValueError(f"no model loader for {path}")


def describe_loader(rel: str) -> dict[str, Any]:
    """The loader call recorded in the meta block for fixture ``rel``."""
    if rel.endswith((".xes", ".xes.gz")):
        return {"function": "pm4py.read_xes", "params": {"variant": XES_VARIANT}}
    if rel.endswith(".csv"):
        return {"function": "pandas.read_csv + pm4py.format_dataframe", "params": {}}
    if rel.endswith(".parquet"):
        return {"function": "pandas.read_parquet + pm4py.format_dataframe", "params": {}}
    if rel.endswith(".pnml"):
        return {"function": "pm4py.read_pnml", "params": {}}
    if rel.endswith(".ptml"):
        return {"function": "pm4py.read_ptml", "params": {}}
    if rel.endswith(".bpmn"):
        return {"function": "pm4py.read_bpmn", "params": {}}
    return {"function": None, "params": {}}
