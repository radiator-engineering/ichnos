"""Area ``dfg``: pm4py's DFG filters on the DFGs of fixture logs.

Each case discovers the DFG of a log with ``pm4py.discover_dfg`` and counts
activities with ``pm4py.get_event_attribute_values``. It then runs every
filter in ``algo/filtering/dfg/dfg_filtering.py`` over a parameter grid.

Expected value:

- ``input``: the DFG the filters start from;
- ``results``: ``[{"filter": name, "param": value, "keep_all_activities":
  bool or null, "output": dfg}]`` in a fixed order;
- ``petri_net``: ``pm4py.convert_to_petri_net`` of the input DFG with its
  start and end activities, described as ``model.py`` does.

A DFG is ``{"edges": {"a -> b": n}, "start_activities": {a: n},
"end_activities": {a: n}, "activities_count": {a: n}}``. Edges are keyed by
``"<a> -> <b>"`` to keep the files small. The to, from and contain
filters run on every activity of a log with at most ten activities, and on
five activities spread over the frequency order otherwise. ``clean_noise`` results have no
start, end or count entries, because pm4py's
``clean_dfg_based_on_noise_thresh`` returns only edges.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pm4py
from pm4py.algo.filtering.dfg import dfg_filtering as f

from harness import case
from harness.fixtures import load_log

FUNCTIONS = [
    "pm4py.discover_dfg",
    "pm4py.get_event_attribute_values",
    "pm4py.algo.filtering.dfg.dfg_filtering.filter_dfg_on_activities_percentage",
    "pm4py.algo.filtering.dfg.dfg_filtering.filter_dfg_on_paths_percentage",
    "pm4py.algo.filtering.dfg.dfg_filtering.filter_dfg_keep_connected",
    "pm4py.algo.filtering.dfg.dfg_filtering.filter_dfg_to_activity",
    "pm4py.algo.filtering.dfg.dfg_filtering.filter_dfg_from_activity",
    "pm4py.algo.filtering.dfg.dfg_filtering.filter_dfg_contain_activity",
    "pm4py.algo.filtering.dfg.dfg_filtering.clean_dfg_based_on_noise_thresh",
    "pm4py.convert_to_petri_net",
]

PERCENTAGES = [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0]
CONNECTED_THRESHOLDS = [0.0, 0.2, 0.5, 0.8, 0.95]
NOISE_THRESHOLDS = [0.0, 0.05, 0.2, 0.5]
MAX_ACTIVITY_FILTERS = 10


def edges(dfg: dict[tuple[str, str], int]) -> dict[str, int]:
    return {f"{a} -> {b}": n for (a, b), n in dfg.items()}


def describe(dfg: Any, sa: Any, ea: Any, ac: Any) -> dict[str, Any]:
    return {
        "edges": edges(dfg),
        "start_activities": dict(sa),
        "end_activities": dict(ea),
        "activities_count": dict(ac),
    }


def sample_activities(ac: dict[str, int]) -> list[str]:
    """All activities of a small log; otherwise five spread over the
    frequency order, to keep the file small."""
    acts = sorted(ac, key=lambda a: (-ac[a], a))
    if len(acts) <= MAX_ACTIVITY_FILTERS:
        return acts
    n = len(acts)
    return [acts[i] for i in sorted({0, n // 4, n // 2, 3 * n // 4, n - 1})]


def filters(fixtures: dict[str, Path]) -> dict[str, Any]:
    log = load_log(fixtures["log"])
    dfg, sa, ea = pm4py.discover_dfg(log)
    ac = pm4py.get_event_attribute_values(log, "concept:name")
    results: list[dict[str, Any]] = []

    def add(name: str, param: Any, keep_all: bool | None, out: Any) -> None:
        results.append(
            {"filter": name, "param": param, "keep_all_activities": keep_all, "output": out}
        )

    for p in PERCENTAGES:
        add("activities_percentage", p, None,
            describe(*f.filter_dfg_on_activities_percentage(dfg, sa, ea, ac, p)))
    for keep_all in [False, True]:
        for p in PERCENTAGES:
            add("paths_percentage", p, keep_all,
                describe(*f.filter_dfg_on_paths_percentage(dfg, sa, ea, ac, p, keep_all)))
        for t in CONNECTED_THRESHOLDS:
            add("keep_connected", t, keep_all,
                describe(*f.filter_dfg_keep_connected(dfg, sa, ea, ac, t, keep_all)))
    for act in sample_activities(ac):
        add("to_activity", act, None, describe(*f.filter_dfg_to_activity(dfg, sa, ea, ac, act)))
        add("from_activity", act, None, describe(*f.filter_dfg_from_activity(dfg, sa, ea, ac, act)))
        add("contain_activity", act, None,
            describe(*f.filter_dfg_contain_activity(dfg, sa, ea, ac, act)))
    for t in NOISE_THRESHOLDS:
        out = f.clean_dfg_based_on_noise_thresh(dfg, list(ac), t)
        add("clean_noise", t, None, {"edges": edges(out)})
    from cases.model import describe_net

    return {
        "input": describe(dfg, sa, ea, ac),
        "results": results,
        "petri_net": describe_net(*pm4py.convert_to_petri_net(dfg, sa, ea)),
    }


for case_id, fixture in [
    ("filters-running-example", "running-example.xes"),
    ("filters-receipt", "receipt.xes"),
]:
    case(case_id, fixture=fixture, functions=FUNCTIONS)(filters)
