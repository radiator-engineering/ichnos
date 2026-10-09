"""Area ``log``: basic statistics of event logs.

These cases need no ichnos code beyond JSON loading. The core and stats lanes
assert their outputs against them.

Expected value (``ichnos_golden::log::LogSummary`` on the Rust side):

- ``n_cases``, ``n_events``: integers;
- ``activities``, ``start_activities``, ``end_activities``: activity -> count;
- ``variants``: ``[{"activities": [...], "count": n}]``, sorted by activities;
- ``dfg``: ``[{"source": a, "target": b, "count": n}]``, sorted by (source, target).
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pm4py

from harness import case
from harness.fixtures import load_log

FUNCTIONS = [
    "pm4py.get_event_attribute_values",
    "pm4py.get_start_activities",
    "pm4py.get_end_activities",
    "pm4py.get_variants",
    "pm4py.discover_dfg",
]


def summarize(fixtures: dict[str, Path]) -> dict[str, Any]:
    log = load_log(fixtures["log"])
    variants = pm4py.get_variants(log)
    dfg, _, _ = pm4py.discover_dfg(log)
    return {
        # pm4py has no case-count function for a DataFrame; this is the
        # number of distinct case ids after format_dataframe.
        "n_cases": int(log["case:concept:name"].nunique()),
        "n_events": len(log),
        "activities": pm4py.get_event_attribute_values(log, "concept:name"),
        "start_activities": pm4py.get_start_activities(log),
        "end_activities": pm4py.get_end_activities(log),
        "variants": sorted(
            ({"activities": list(v), "count": c} for v, c in variants.items()),
            key=lambda r: r["activities"],
        ),
        "dfg": [
            {"source": a, "target": b, "count": c} for (a, b), c in sorted(dfg.items())
        ],
    }


for case_id, fixture in [
    ("running-example-xes", "running-example.xes"),
    ("running-example-csv", "running-example.csv"),
    ("receipt-xes", "receipt.xes"),
    ("roadtraffic100traces-xes", "roadtraffic100traces.xes"),
]:
    case(case_id, fixture=fixture, functions=FUNCTIONS)(summarize)
