"""OCEL activity and edge statistics from pm4py's typed tables.

Relation-order endpoints, event-order edges, duplicate rows, reversed and
fractional times, pair/triple observations, and business calendars are explicit.
Real-log readers run in a child with a fixed hash seed to stabilize set order.
"""

import json
import os
from pathlib import Path
import subprocess
import sys

from harness import case
from cases.ocel import _tables, _READERS
import pandas as pd
import pm4py
from pm4py.statistics.ocel import (
    act_utils,
    act_ot_dependent,
    edge_metrics,
    objects_ot_count,
    ot_activities,
)

MODULES = {
    "act-utils": act_utils,
    "act-ot-dependent": act_ot_dependent,
    "edge-metrics": edge_metrics,
    "objects-ot-count": objects_ot_count,
    "ot-activities": ot_activities,
}
FUNCTIONS = {
    "act-utils": [
        "aggregate_events",
        "aggregate_unique_objects",
        "aggregate_total_objects",
        "find_associations_from_relations_df",
        "find_associations_from_ocel",
    ],
    "act-ot-dependent": [
        "aggregate_events",
        "aggregate_unique_objects",
        "aggregate_total_objects",
        "find_associations_from_ocel",
    ],
    "edge-metrics": [
        "aggregate_ev_couples",
        "aggregate_unique_objects",
        "aggregate_total_objects",
        "find_associations_per_edge",
        "performance_calculation_ocel_aggregation",
    ],
    "objects-ot-count": ["get_objects_ot_count"],
    "ot-activities": ["get_object_type_activities"],
}
ENTRIES = [(m, f) for m, fs in FUNCTIONS.items() for f in fs]


def function_name(m, f):
    return f"pm4py.statistics.ocel.{m.replace('-', '_')}.{f}"


def synthetic(duplicate_ids=False):
    events = pd.DataFrame(
        {
            "ocel:eid": ["e3", "e1", "e4", "e2", "unused"],
            "ocel:activity": ["finish", "start", "middle", "finish", "unrelated"],
            "ocel:timestamp": pd.to_datetime(
                [
                    "2024-01-08T08:00:00.250Z",
                    "2024-01-05T16:30:00Z",
                    "2024-01-08T07:00:00Z",
                    "2024-01-08T19:00:00.500Z",
                    "2024-01-10T00:00:00Z",
                ],
                format="ISO8601",
            ),
        }
    )
    objects = pd.DataFrame(
        {
            "ocel:oid": ["z", "a", "item", "lonely"],
            "ocel:type": ["order", "order", "item", "unrelated"],
        }
    )
    pairs = [
        ("e4", "a"),
        ("e1", "z"),
        ("e3", "item"),
        ("e1", "a"),
        ("e3", "z"),
        ("e4", "a"),
        ("e2", "item"),
        ("e3", "a"),
        ("e2", "a"),
        ("e2", "z"),
    ]
    ev = events.set_index("ocel:eid")
    ot = objects.set_index("ocel:oid")
    relations = pd.DataFrame(
        {
            "ocel:eid": [e for e, o in pairs],
            "ocel:oid": [o for e, o in pairs],
            "ocel:activity": [ev.loc[e, "ocel:activity"] for e, o in pairs],
            "ocel:timestamp": [ev.loc[e, "ocel:timestamp"] for e, o in pairs],
            "ocel:type": [ot.loc[o, "ocel:type"] for e, o in pairs],
            "ocel:qualifier": [
                "first",
                None,
                "i",
                "a",
                None,
                "second",
                None,
                None,
                None,
                None,
            ],
        }
    )
    if duplicate_ids:
        second = events.iloc[[1]].copy()
        second["ocel:activity"] = "ignored-second-activity"
        second["ocel:timestamp"] = pd.Timestamp("2025-01-01T00:00:00Z")
        events = pd.concat([events, second], ignore_index=True)
        second_object = objects.iloc[[0]].copy()
        second_object["ocel:type"] = "ignored-second-type"
        objects = pd.concat([objects, second_object], ignore_index=True)
    return pm4py.OCEL(events=events, objects=objects, relations=relations)


class Calendar:
    def is_working_day(self, day):
        return day.isoformat() != "2024-01-08"


SLOTS = [(7 * 3600, 12 * 3600), (12 * 3600 + 1, 17 * 3600), (10 * 3600, 13 * 3600)]


def normalized(value):
    if isinstance(value, dict):
        if value and all(isinstance(k, tuple) for k in value):
            return [[list(k), normalized(v)] for k, v in sorted(value.items())]
        return {str(k): normalized(v) for k, v in sorted(value.items())}
    if isinstance(value, set):
        return [normalized(v) for v in sorted(value)]
    if isinstance(value, (list, tuple)):
        return [normalized(v) for v in value]
    return value


def evaluate(log, m, f):
    module = MODULES[m]
    fn = getattr(module, f)
    if m in ["act-utils", "act-ot-dependent"]:
        result = {}
        for prefilter in ["none", "start", "end"]:
            params = {"prefiltering": prefilter}
            if f == "find_associations_from_relations_df":
                out = fn(log.relations, parameters=params)
            elif f == "find_associations_from_ocel":
                out = fn(log, parameters=params)
            else:
                associations = module.find_associations_from_ocel(
                    log, parameters=params
                )
                out = fn(associations)
            result[prefilter] = normalized(out)
        return result
    if m == "edge-metrics":
        edges = edge_metrics.find_associations_per_edge(log)
        if f == "find_associations_per_edge":
            return normalized(edges)
        if f != "performance_calculation_ocel_aggregation":
            return normalized(fn(edges))
        result = {}
        for kind, aggregate in [
            ("pairs", edge_metrics.aggregate_ev_couples),
            ("triples", edge_metrics.aggregate_total_objects),
        ]:
            for schedule, params in [
                ("elapsed", {}),
                ("business", {"business_hours": True}),
                ("slots", {"business_hours": True, "business_hour_slots": SLOTS}),
                ("holiday", {"business_hours": True, "workcalendar": Calendar()}),
            ]:
                result[f"{kind}-{schedule}"] = normalized(
                    fn(log, aggregate(edges), params)
                )
        return result
    return normalized(fn(log))


def direct(m, f):
    if f.startswith("aggregate_") and m.startswith("act-"):
        associations = {"a": [("e2", "o1"), ("e1", "o2"), ("e2", "o1")], "empty": []}
        if m == "act-ot-dependent":
            associations = {"type": associations, "empty-type": {}}
        return {
            "input": normalized(associations),
            "expected": normalized(getattr(MODULES[m], f)(associations)),
        }
    if f.startswith("aggregate_") and m == "edge-metrics":
        edges = {
            "type": {
                ("a", "b"): [
                    ("e2", "e1", "o1"),
                    ("e2", "e1", "o2"),
                    ("e2", "e1", "o1"),
                ],
                ("empty", "empty"): [],
            },
            "empty-type": {},
        }
        return {
            "input": normalized(edges),
            "expected": normalized(getattr(edge_metrics, f)(edges)),
        }
    if f == "find_associations_from_relations_df":
        rows = pd.DataFrame(
            {
                "ocel:eid": ["e1", "e1", "e2", "e3"],
                "ocel:oid": ["z", "z", "a", "z"],
                "ocel:activity": ["first", "ignored", "other", "last"],
            }
        )
        return {
            "input": rows.to_dict("records"),
            "expected": {
                p: normalized(
                    act_utils.find_associations_from_relations_df(
                        rows, {"prefiltering": p}
                    )
                )
                for p in ["none", "start", "end"]
            },
        }
    return None


def input_tables(log):
    tables = _tables(log)
    # Preserve original offsets for wall-clock business-hours comparisons.
    for row, time in zip(tables["events"], log.events[log.event_timestamp]):
        row["timestamp"] = time.isoformat()
    return tables


for _m, _f in ENTRIES:

    def run(fixtures, m=_m, f=_f):
        logs = [synthetic(), synthetic(True), pm4py.OCEL()]
        if f == "performance_calculation_ocel_aggregation":
            offset_log = synthetic()
            offset_log.events["ocel:timestamp"] = offset_log.events[
                "ocel:timestamp"
            ].dt.tz_convert("Etc/GMT-2")
            logs.append(offset_log)
            nanos_log = synthetic()
            nanos_log.events["ocel:timestamp"] += pd.to_timedelta(
                [10, 20, 30, 40, 50], unit="ns"
            )
            logs.append(nanos_log)
        out = {
            "scenarios": [
                {"input": input_tables(log), "expected": evaluate(log, m, f)}
                for log in logs
            ]
        }
        extra = direct(m, f)
        if extra is not None:
            out["direct"] = extra
        return out

    case(f"{_m}-{_f.replace('_', '-')}", functions=[function_name(_m, _f)])(run)


def all_statistics(log):
    return {
        "input": _tables(log),
        "expected": {
            f"{m}-{f.replace('_', '-')}": evaluate(log, m, f) for m, f in ENTRIES
        },
    }


def read_pinned(rel, path):
    env = dict(os.environ, PYTHONHASHSEED="0")
    golden_tools = str(Path(__file__).resolve().parent.parent)
    env["PYTHONPATH"] = os.pathsep.join(
        filter(None, [golden_tools, env.get("PYTHONPATH")])
    )
    result = subprocess.run(
        [sys.executable, __file__, rel, str(path)],
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


for _rel in ["example_log.jsonocel", "newocel.jsonocel", "ocel20_example.jsonocel"]:

    def run_real(fixtures, rel=_rel):
        return read_pinned(rel, fixtures["log"])

    case(
        "all-" + _rel.split(".")[0].replace("_", "-"),
        fixture="ocel/" + _rel,
        functions=[_READERS[_rel][0]] + [function_name(m, f) for m, f in ENTRIES],
    )(run_real)


if __name__ == "__main__":
    print(json.dumps(all_statistics(_READERS[sys.argv[1]][1](sys.argv[2]))))
