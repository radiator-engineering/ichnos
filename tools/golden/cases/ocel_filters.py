"""OCEL filters: typed tables, row order and propagation on every table.

Outputs record input row indices rather than duplicating whole tables. Tests
rebuild each expected log from those rows, including attributes and qualifiers.
Readers run with hash seed zero because pm4py iterates sets while importing.
"""

import copy
import json
import os
import subprocess
import sys
from pathlib import Path

from harness import case
import pandas as pd
import pm4py
from cases.ocel import _tables

FUNCTIONS = [
    "filter_ocel_event_attribute",
    "filter_ocel_object_attribute",
    "filter_ocel_object_types_allowed_activities",
    "filter_ocel_object_per_type_count",
    "filter_ocel_start_events_per_object_type",
    "filter_ocel_end_events_per_object_type",
    "filter_ocel_events_timestamp",
    "filter_ocel_object_types",
    "filter_ocel_objects",
    "filter_ocel_events",
    "filter_ocel_activities_connected_object_type",
    "filter_ocel_cc_object",
    "filter_ocel_cc_length",
    "filter_ocel_cc_otype",
    "filter_ocel_cc_activity",
]


def synthetic():
    t = pd.Timestamp("2020-01-01T00:00:00Z")
    events = pd.DataFrame(
        [
            {
                "ocel:eid": "e0",
                "ocel:activity": "orphan",
                "ocel:timestamp": t,
                "score": 1,
                "alternate": t,
            },
            {
                "ocel:eid": "e1",
                "ocel:activity": "create",
                "ocel:timestamp": t + pd.Timedelta(days=2),
                "score": 2,
                "alternate": t,
            },
            {
                "ocel:eid": "e2",
                "ocel:activity": "finish",
                "ocel:timestamp": t,
                "score": 3,
            },
            {
                "ocel:eid": "e3",
                "ocel:activity": "create",
                "ocel:timestamp": t + pd.Timedelta(days=1),
                "score": 1,
                "alternate": t + pd.Timedelta(days=3),
            },
            {
                "ocel:eid": "e4",
                "ocel:activity": "single",
                "ocel:timestamp": t,
                "alternate": t,
            },
        ]
    )
    objects = pd.DataFrame(
        [
            {"ocel:oid": "a", "ocel:type": "order", "size": 1},
            {"ocel:oid": "b", "ocel:type": "item", "size": 2},
            {"ocel:oid": "c", "ocel:type": "item", "size": 3},
            {"ocel:oid": "d", "ocel:type": "order", "size": 1},
            {"ocel:oid": "isolated", "ocel:type": "item", "size": 1},
            {"ocel:oid": "orphan", "ocel:type": "order"},
        ]
    )
    pairs = [
        ("e1", "a"),
        ("e1", "b"),
        ("e1", "b"),
        ("e2", "b"),
        ("e2", "c"),
        ("e3", "c"),
        ("e3", "d"),
        ("e4", "isolated"),
    ]
    ev = events.set_index("ocel:eid").to_dict("index")
    ob = objects.set_index("ocel:oid").to_dict("index")
    relations = pd.DataFrame(
        [
            {
                "ocel:eid": e,
                "ocel:oid": o,
                "ocel:activity": ev[e]["ocel:activity"],
                "ocel:timestamp": ev[e]["ocel:timestamp"],
                "ocel:type": ob[o]["ocel:type"],
                "ocel:qualifier": "primary" if i % 2 else "",
            }
            for i, (e, o) in enumerate(pairs)
        ]
    )
    return pm4py.OCEL(
        events=events,
        objects=objects,
        relations=relations,
        o2o=pd.DataFrame(
            [
                {"ocel:oid": "a", "ocel:oid_2": "b", "ocel:qualifier": "link"},
                {
                    "ocel:oid": "orphan",
                    "ocel:oid_2": "isolated",
                    "ocel:qualifier": "link",
                },
            ]
        ),
        e2e=pd.DataFrame(
            [
                {"ocel:eid": "e0", "ocel:eid_2": "e1", "ocel:qualifier": "next"},
                {"ocel:eid": "e1", "ocel:eid_2": "e2", "ocel:qualifier": "next"},
            ]
        ),
        object_changes=pd.DataFrame(
            [
                {
                    "ocel:oid": "a",
                    "ocel:type": "order",
                    "ocel:timestamp": t,
                    "ocel:field": "size",
                    "size": 4,
                },
                {
                    "ocel:oid": "orphan",
                    "ocel:type": "order",
                    "ocel:timestamp": t,
                    "ocel:field": "size",
                    "size": 5,
                },
            ]
        ),
        globals={"custom": "kept"},
    )


def scenarios(log):
    eid, oid, typ = log.event_id_column, log.object_id_column, log.object_type_column
    act, ts = log.event_activity, log.event_timestamp
    types = sorted(str(x) for x in log.objects[typ].unique())
    activities = sorted(str(x) for x in log.events[act].unique())
    object_ids = log.objects[oid].astype(str).tolist()
    event_ids = log.events[eid].astype(str).tolist()
    type0 = types[0] if types else "missing"
    activity0 = activities[0] if activities else "missing"
    first_time = (
        log.events[ts].min()
        if len(log.events)
        else pd.Timestamp("2020-01-01T00:00:00Z")
    )
    last_time = log.events[ts].max() if len(log.events) else first_time
    out = []

    def add(fn, args, **kwargs):
        out.append({"function": fn, "args": args, "kwargs": kwargs})

    for positive in (True, False):
        add(FUNCTIONS[0], [act, [activity0]], positive=positive)
        add(FUNCTIONS[0], [eid, event_ids[:1]], positive=positive)
        add(FUNCTIONS[1], [typ, [type0]], positive=positive)
        add(FUNCTIONS[1], [oid, object_ids[:1]], positive=positive)
        add(FUNCTIONS[7], [[type0]], positive=positive)
        add(FUNCTIONS[9], [event_ids[:2]], positive=positive)
        add(FUNCTIONS[13], [type0], positive=positive)
        for level in (0, 1, 2, 3, 20):
            add(FUNCTIONS[8], [object_ids[:1]], positive=positive, level=level)
        add(FUNCTIONS[7], [[type0]], positive=positive, level=2)
    add(FUNCTIONS[0], [act, []])
    add(FUNCTIONS[1], [typ, []])
    add(FUNCTIONS[2], [{type0: activities[:2]}])
    add(FUNCTIONS[2], [{}])
    add(FUNCTIONS[3], [{type0: 2}])
    add(FUNCTIONS[3], [{type0: 0}])
    add(FUNCTIONS[3], [{"missing": 0}])
    add(FUNCTIONS[3], [{}])
    for ot in (type0, "missing"):
        add(FUNCTIONS[4], [ot])
        add(FUNCTIONS[5], [ot])
        add(FUNCTIONS[10], [ot])
    for left, right in (
        (first_time, last_time),
        (first_time, first_time),
        (last_time, first_time),
    ):
        add(FUNCTIONS[6], [left.isoformat(), right.isoformat()])
    for object_id in object_ids[:1] + ["missing"]:
        add(FUNCTIONS[11], [object_id])
    for minimum, maximum in ((1, 1), (2, 20), (1, 100000), (20, 2)):
        add(FUNCTIONS[12], [minimum, maximum])
    for activity in (activity0, "missing"):
        add(FUNCTIONS[14], [activity])
    if "score" in log.events:
        for positive in (True, False):
            add(FUNCTIONS[0], ["score", [1.0, True]], positive=positive)
            add(FUNCTIONS[1], ["size", [1.0]], positive=positive)
        add(
            FUNCTIONS[6],
            [first_time.isoformat(), first_time.isoformat()],
            timestamp_key="alternate",
        )
        add(FUNCTIONS[8], [["orphan"]])
        add(FUNCTIONS[11], ["isolated"])
        add(FUNCTIONS[11], ["orphan"])
    return out


def project(input_tables, output):
    result = {}
    for key, rows in _tables(output).items():
        positions = {}
        for index, row in enumerate(input_tables[key]):
            positions.setdefault(json.dumps(row, sort_keys=True), []).append(index)
        result[key] = []
        for row in rows:
            result[key].append(positions[json.dumps(row, sort_keys=True)].pop(0))
    return result


def compute(log, only=None):
    tables = _tables(log)
    results = []
    for scenario in scenarios(log):
        if only is not None and scenario["function"] != only:
            continue
        args = scenario["args"]
        if scenario["function"] == "filter_ocel_events_timestamp":
            args = [pd.Timestamp(value).to_pydatetime() for value in args]
        output = getattr(pm4py, scenario["function"])(
            copy.deepcopy(log), *args, **scenario["kwargs"]
        )
        assert output.globals == log.globals
        results.append(dict(scenario, rows=project(tables, output)))
    components = None
    if only is None or only == FUNCTIONS[11]:
        _, components = pm4py.filter_ocel_cc_object(
            log, "missing", return_conn_comp=True
        )
        components = sorted(sorted(c) for c in components)
    return {"input": tables, "results": results, "components": components}


for function in FUNCTIONS:
    case(
        function.removeprefix("filter_ocel_").replace("_", "-"),
        functions=["pm4py." + function],
        params={"only": function},
    )(lambda fixtures, only: compute(synthetic(), only))


for name, reader in (
    ("example_log.jsonocel", "read_ocel_json"),
    ("newocel.jsonocel", "read_ocel_json"),
    ("ocel20_example.jsonocel", "read_ocel2_json"),
):

    def run(fixtures, name=name, reader=reader):
        env = dict(os.environ, PYTHONHASHSEED="0")
        env["PYTHONPATH"] = str(Path(__file__).resolve().parent.parent)
        output = subprocess.run(
            [sys.executable, __file__, reader, str(fixtures["log"])],
            env=env,
            check=True,
            capture_output=True,
            text=True,
        )
        return json.loads(output.stdout)

    case(
        "all-" + name.replace(".", "-").replace("_", "-"),
        fixture="ocel/" + name,
        functions=["pm4py." + reader] + ["pm4py." + f for f in FUNCTIONS],
    )(run)


if __name__ == "__main__":
    print(json.dumps(compute(getattr(pm4py, sys.argv[1])(sys.argv[2]))))
