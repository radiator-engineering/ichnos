"""OCEL summaries from ``pm4py/ocel.py``: object types, attribute names,
flattening, activities and object counts per type, and the temporal, object
and interaction summaries.

Each case records the input tables (see ``cases.ocel``) and every result.
Sets are sorted lists. A function that raises records its error type and
message instead. Real logs are read in a child process with
``PYTHONHASHSEED=0``, because some pm4py readers order rows by set
iteration.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

from harness import case  # imports pm4py quietly; keep it first

import pandas as pd
import pm4py
from cases.ocel import _READERS, _date, _missing, _tables, _value

FUNCTIONS = [
    "pm4py.ocel_get_object_types",
    "pm4py.ocel_get_attribute_names",
    "pm4py.ocel_flattening",
    "pm4py.ocel_object_type_activities",
    "pm4py.ocel_objects_ot_count",
    "pm4py.ocel_temporal_summary",
    "pm4py.ocel_objects_summary",
    "pm4py.ocel_objects_interactions_summary",
]


def synthetic():
    """Repeated relations, shared timestamps, typed attributes, an object
    without relations and an event without relations."""
    times = pd.to_datetime(
        [
            "2024-01-08T08:00:00.250Z",
            "2024-01-05T16:30:00Z",
            "2024-01-08T08:00:00.250Z",
            "2024-01-08T19:00:00.500Z",
            "2024-01-10T00:00:00Z",
        ],
        format="ISO8601",
    )
    events = pd.DataFrame(
        {
            "ocel:eid": ["e3", "e1", "e4", "e2", "unused"],
            "ocel:activity": ["finish", "start", "middle", "finish", "unrelated"],
            "ocel:timestamp": times,
            "cost": [3.5, None, 1.0, 2.25, None],
            "clerk": ["ann", "bob", None, "ann", None],
        }
    )
    objects = pd.DataFrame(
        {
            "ocel:oid": ["z", "a", "item", "lonely", "b"],
            "ocel:type": ["order", "order", "item", "unrelated", "order"],
            "weight": [None, 2, 5, None, 7],
            "colour": ["red", None, "blue", "green", None],
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
        ("e2", "b"),
        ("e2", "z"),
    ]
    ev = events.set_index("ocel:eid")
    ot = objects.set_index("ocel:oid")
    relations = pd.DataFrame(
        {
            "ocel:eid": [e for e, _ in pairs],
            "ocel:oid": [o for _, o in pairs],
            "ocel:activity": [ev.loc[e, "ocel:activity"] for e, _ in pairs],
            "ocel:timestamp": [ev.loc[e, "ocel:timestamp"] for e, _ in pairs],
            "ocel:type": [ot.loc[o, "ocel:type"] for _, o in pairs],
        }
    )
    return pm4py.OCEL(events=events, objects=objects, relations=relations)


def _guard(fn):
    try:
        return {"ok": fn()}
    except Exception as e:  # noqa: BLE001 - pm4py's error is the expected result
        return {"error": type(e).__name__, "message": str(e)}


def _row(record):
    return {str(k): _value(v) for k, v in record.items() if not _missing(v)}


def _flattening(log, object_type):
    return [_row(r) for r in pm4py.ocel_flattening(log, object_type).to_dict("records")]


def _temporal(log):
    table = pm4py.ocel_temporal_summary(log)
    return [
        {
            "timestamp": _date(r[log.event_timestamp]),
            "activities": [str(a) for a in r[log.event_activity]],
            "objects": [str(o) for o in r[log.object_id_column]],
        }
        for r in table.to_dict("records")
    ]


def _objects(log):
    table = pm4py.ocel_objects_summary(log)
    rows = []
    for r in table.to_dict("records"):
        interacting = r["interacting_objects"]
        rows.append(
            {
                "object": str(r[log.object_id_column]),
                "activities": [str(a) for a in r["activities_lifecycle"]],
                "start": _date(r["lifecycle_start"]),
                "end": _date(r["lifecycle_end"]),
                "duration": float(r["lifecycle_duration"]),
                "interacting": None
                if not isinstance(interacting, set)
                else sorted(str(o) for o in interacting),
            }
        )
    return rows


def _interactions(log):
    table = pm4py.ocel_objects_interactions_summary(log)
    eid, oid, otype = log.event_id_column, log.object_id_column, log.object_type_column
    return [
        {
            "event": str(r[eid]),
            "activity": str(r[log.event_activity]),
            "object": str(r[oid]),
            "type": str(r[otype]),
            "object_2": str(r[oid + "_2"]),
            "type_2": str(r[otype + "_2"]),
        }
        for r in table.to_dict("records")
    ]


def summarize(log):
    types = [str(t) for t in pm4py.ocel_get_object_types(log)]
    return {
        "input": _tables(log),
        "object_types": _guard(lambda: types),
        "attribute_names": _guard(lambda: [str(a) for a in pm4py.ocel_get_attribute_names(log)]),
        "flattening": {
            t: _guard(lambda t=t: _flattening(log, t)) for t in types + ["no-such-type"]
        },
        "object_type_activities": _guard(
            lambda: {
                str(t): sorted(str(a) for a in acts)
                for t, acts in sorted(pm4py.ocel_object_type_activities(log).items())
            }
        ),
        "objects_ot_count": _guard(
            lambda: {
                str(e): {str(t): int(n) for t, n in sorted(c.items())}
                for e, c in sorted(pm4py.ocel_objects_ot_count(log).items())
            }
        ),
        "temporal_summary": _guard(lambda: _temporal(log)),
        "objects_summary": _guard(lambda: _objects(log)),
        "objects_interactions_summary": _guard(lambda: _interactions(log)),
    }


@case("synthetic", functions=FUNCTIONS)
def _synthetic(fixtures):
    return summarize(synthetic())


@case("empty", functions=FUNCTIONS)
def _empty(fixtures):
    return summarize(pm4py.OCEL())


def _read_pinned(rel, path):
    golden_tools = str(Path(__file__).resolve().parent.parent)
    env = dict(os.environ, PYTHONHASHSEED="0")
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
    out = subprocess.run(
        [sys.executable, __file__, rel, str(path)],
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(out.stdout)


for _rel in [
    "example_log.jsonocel",
    "newocel.jsonocel",
    "ocel20_example.jsonocel",
    "typed.jsonocel",
    "typed20.jsonocel",
]:

    def _run(fixtures, _rel=_rel):
        return _read_pinned(_rel, fixtures["log"])

    case(
        "real-" + _rel.replace(".", "-").replace("_", "-"),
        fixture="ocel/" + _rel,
        functions=[_READERS[_rel][0]] + FUNCTIONS,
    )(_run)


if __name__ == "__main__":
    print(json.dumps(summarize(_READERS[sys.argv[1]][1](sys.argv[2]))))
