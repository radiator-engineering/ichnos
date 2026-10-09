"""Object-centric discovery: pm4py's ``discover_ocdfg``.

Each case records the log as typed tables (``input``) and the OC-DFG pm4py
discovers from it (``expected``). Real logs are read in a child process with
``PYTHONHASHSEED=0``, because pm4py's readers order rows by iterating Python
sets. The synthetic log of ``ocel_stats`` adds reversed and fractional times,
an unrelated event and object, and repeated event and object ids.

``expected`` keys each part by object type, then activity or
``[source, target]`` edge, and sorts every set:

- ``activities`` and ``object_types``: sorted lists.
- ``activities_indep``: ``{activity: occurrences}``.
- ``activities_ot``, ``start_activities``, ``end_activities``:
  ``{object type: {activity: occurrences}}``.
- ``edges``: ``{object type: [[source, target, edge]]}``.

``occurrences`` is ``{"events", "unique_objects", "total_objects"}``, the
last as ``[event, object]`` pairs. ``edge`` is ``{"event_couples",
"unique_objects", "total_objects", "event_couples_durations",
"total_objects_durations"}``: event pairs, objects, ``[source event, target
event, object]`` triples and pm4py's sorted durations in seconds. Without
edge performance both duration lists are empty.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

from harness import case  # imports pm4py quietly; keep it first

import pm4py
from cases.ocel import _READERS, _tables
from cases.ocel_stats import SLOTS, Calendar, synthetic

METRICS = ("events", "unique_objects", "total_objects")
EDGE_METRICS = ("event_couples", "unique_objects", "total_objects")


def _ids(values):
    return sorted([str(x) for x in v] if isinstance(v, tuple) else str(v) for v in values)


def _occurrences(part, *keys):
    """Turns pm4py's ``{metric: {key...: set}}`` into ``{key...: {metric: list}}``."""

    def lookup(metric):
        value = part[metric]
        for k in keys:
            value = value.get(k, {})
        return value

    names = set()
    for metric in METRICS:
        names |= set(lookup(metric))
    return {
        str(name): {metric: _ids(lookup(metric).get(name, ())) for metric in METRICS}
        for name in sorted(names)
    }


def _by_type(part):
    types = set()
    for metric in METRICS:
        types |= set(part[metric])
    return {str(ot): _occurrences(part, ot) for ot in sorted(types)}


def describe(d):
    edges = {}
    types = set()
    for metric in EDGE_METRICS:
        types |= set(d["edges"][metric])
    for ot in sorted(types):
        pairs = set()
        for metric in EDGE_METRICS:
            pairs |= set(d["edges"][metric].get(ot, {}))
        rows = []
        for a, b in sorted(pairs):
            edge = {m: _ids(d["edges"][m].get(ot, {}).get((a, b), ())) for m in EDGE_METRICS}
            for m in ("event_couples", "total_objects"):
                durations = d["edges_performance"][m].get(ot, {}).get((a, b), [])
                edge[m + "_durations"] = [float(x) for x in durations]
            rows.append([str(a), str(b), edge])
        edges[str(ot)] = rows
    return {
        "activities": _ids(d["activities"]),
        "object_types": _ids(d["object_types"]),
        "activities_indep": _occurrences(d["activities_indep"]),
        "activities_ot": _by_type(d["activities_ot"]),
        "start_activities": _by_type(d["start_activities"]),
        "end_activities": _by_type(d["end_activities"]),
        "edges": edges,
    }


def tables(log):
    """The log's tables, with event times in their own offset.

    Business hours count wall-clock time, so the offset matters. A naive
    time stays in UTC, as ``_tables`` writes it.
    """
    out = _tables(log)
    for row, time in zip(out["events"], log.events[log.event_timestamp]):
        if time.tzinfo is not None:
            row["timestamp"] = time.isoformat()
    # The Rust test looks these up by id.
    for row in out["relations"]:
        for key in ("activity", "timestamp", "type"):
            row.pop(key)
    return out


def discover(log, **params):
    return {"input": tables(log), "expected": describe(pm4py.discover_ocdfg(log, **params))}


def _read_pinned(rel, path):
    """Discovers the OC-DFG of ``path`` in a child with ``PYTHONHASHSEED=0``."""
    golden_tools = str(Path(__file__).resolve().parent.parent)
    env = dict(os.environ, PYTHONHASHSEED="0")
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
    out = subprocess.run([sys.executable, __file__, rel, str(path)], env=env,
                         check=True, capture_output=True, text=True)
    return json.loads(out.stdout)


LOGS = [
    "example_log.jsonocel",
    "example_log.csv",
    "newocel.jsonocel",
    "ocel20_example.jsonocel",
    "typed.jsonocel",
    "typed.xmlocel",
    "typed.csv",
    "typed20.jsonocel",
    "typed20.xmlocel",
    "typed20.ocel.csv",
]

for _rel in LOGS:
    def _run(fixtures, _rel=_rel):
        return _read_pinned(_rel, fixtures["log"])

    case("ocdfg-" + _rel.replace(".", "-").replace("_", "-"), fixture="ocel/" + _rel,
         functions=[_READERS[_rel][0], "pm4py.discover_ocdfg"])(_run)


SYNTHETIC = {
    "ocdfg-synthetic": ({}, False),
    "ocdfg-synthetic-repeated-ids": ({}, True),
    "ocdfg-synthetic-no-performance": ({"compute_edges_performance": False}, False),
    "ocdfg-synthetic-business-hours": ({"business_hours": True}, False),
    "ocdfg-synthetic-business-slots": ({"business_hours": True, "business_hour_slots": SLOTS}, False),
}

for _id, (_params, _repeated) in SYNTHETIC.items():
    def _synthetic(fixtures, _repeated=_repeated, **params):
        return discover(synthetic(_repeated), **params)

    case(_id, functions=["pm4py.discover_ocdfg"], params=_params)(_synthetic)


@case("ocdfg-synthetic-holiday", functions=["pm4py.discover_ocdfg"],
      params={"business_hours": True})
def _holiday(fixtures, **params):
    # The work calendar is an object, so the meta block cannot record it.
    return discover(synthetic(), workcalendar=Calendar(), **params)


@case("ocdfg-empty", functions=["pm4py.discover_ocdfg"])
def _empty(fixtures):
    return discover(pm4py.OCEL())


if __name__ == "__main__":
    print(json.dumps(discover(_READERS[sys.argv[1]][1](sys.argv[2]))))
