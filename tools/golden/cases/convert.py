"""Conversions checked against the pinned pm4py oracle: event log to OCEL,
OCEL to NetworkX graph, event log to time intervals, and the object graphs
the OCEL graph builds on.

Attribute values are JSON scalars; a date is ``["date", <UTC ISO>]``.
pandas' missing values are left out.

``convert_log_to_ocel`` and ``convert_log_to_time_intervals`` get an
``EventLog``, so the event order is the trace order. An OCEL is its events
``[id, activity, timestamp, attributes]``, objects ``[id, type,
attributes]`` and relations ``[event, object, type]``.

A graph is its nodes ``[id, attributes]`` and its edges ``[source, target,
attributes]``, sorted. The nodes of ``ocel_to_nx`` keep NetworkX's order;
those of ``ocel_features_to_nx`` come from Python sets and are sorted.

An object graph is its sorted pairs. A time interval is ``[begin, end, case
id, source index, target index]``.
"""

import json
import numbers
from datetime import datetime, timezone

from harness import case  # imports pm4py quietly; keep it first

import pandas as pd
import pm4py

from harness.fixtures import load_log


def _missing(value):
    if value is None:
        return True
    try:
        return bool(pd.isna(value))
    except (TypeError, ValueError):
        return False


def _value(value):
    if isinstance(value, (pd.Timestamp, datetime)):
        value = pd.Timestamp(value)
        if value.tzinfo is None:
            value = value.tz_localize("UTC")
        return ["date", value.astimezone(timezone.utc).isoformat(timespec="microseconds")]
    if isinstance(value, bool):
        return value
    if isinstance(value, numbers.Integral):
        return int(value)
    if isinstance(value, numbers.Real):
        return float(value)
    return str(value)


def _attrs(row, skip=()):
    return {str(k): _value(v) for k, v in row.items() if k not in skip and not _missing(v)}


def _event_log(fixtures):
    return pm4py.convert_to_event_log(load_log(fixtures["log"]))


def _read_ocel(fixtures):
    path = str(fixtures["log"])
    return pm4py.read_ocel2(path) if "ocel20" in path else pm4py.read_ocel(path)


def log_to_ocel(fixtures, **params):
    ocel = pm4py.convert_log_to_ocel(_event_log(fixtures), **params)
    ev = ["ocel:eid", "ocel:activity", "ocel:timestamp"]
    return {
        "events": [[str(r["ocel:eid"]), str(r["ocel:activity"]), _value(r["ocel:timestamp"]), _attrs(r, ev)]
                   for r in ocel.events.to_dict("records")],
        "objects": [[str(r["ocel:oid"]), str(r["ocel:type"]), _attrs(r, ["ocel:oid", "ocel:type"])]
                    for r in ocel.objects.to_dict("records")],
        "relations": [[str(r["ocel:eid"]), str(r["ocel:oid"]), str(r["ocel:type"])]
                      for r in ocel.relations.to_dict("records")],
    }


def _sorted(rows):
    return sorted(rows, key=lambda r: json.dumps(r, sort_keys=True))


def ocel_to_networkx(fixtures, variant):
    graph = pm4py.convert_ocel_to_networkx(_read_ocel(fixtures), variant=variant)
    nodes = [[str(n), _attrs(d.get("attr", {}))] for n, d in graph.nodes(data=True)]
    edges = [[str(u), str(v), _attrs(d.get("attr", {}))] for u, v, d in graph.edges(data=True)]
    return {
        "multigraph": graph.is_multigraph(),
        "nodes": nodes if variant == "ocel_to_nx" else _sorted(nodes),
        "edges": _sorted(edges),
    }


def objects_graph(fixtures, graph_type):
    graph = pm4py.discover_objects_graph(_read_ocel(fixtures), graph_type=graph_type)
    return sorted([str(a), str(b)] for a, b in graph)


_LOG = []


def _intervals(fixtures, **params):
    # The rows name each event by its index, so keep the converted log.
    _LOG[:] = _event_log(fixtures)
    log = pm4py.objects.log.obj.EventLog(_LOG)
    if "filter_activity_couple" in params:
        params["filter_activity_couple"] = tuple(params["filter_activity_couple"])
    rows = []
    by_attrs = {id(t.attributes): t for t in _LOG}
    for begin, end, source, target, attrs in pm4py.convert_log_to_time_intervals(log, **params):
        trace = by_attrs[id(attrs)]
        i = next(k for k, e in enumerate(trace) if e is source)
        j = next(k for k, e in enumerate(trace) if e is target)
        rows.append([begin, end, str(attrs["concept:name"]), i, j])
    return rows


_OCEL = ["pm4py.convert_log_to_ocel"]
case("log-to-ocel-running-example", fixture="running-example.csv", functions=_OCEL)(log_to_ocel)
case("log-to-ocel-running-example-attributes", fixture="running-example.csv", functions=_OCEL,
     params={"additional_event_attributes": ["org:resource", "Costs", "missing"],
             "additional_object_attributes": {"case:concept:name": ["case:creator"],
                                              "org:resource": ["Costs"]}})(log_to_ocel)
case("log-to-ocel-running-example-two-types", fixture="running-example.csv", functions=_OCEL,
     params={"object_types": ["case:concept:name", "org:resource"],
             "additional_object_attributes": {"org:resource": ["concept:name"]}})(log_to_ocel)
case("log-to-ocel-running-example-separator", fixture="running-example.csv", functions=_OCEL,
     params={"object_types": ["org:resource", "case:creator"], "obj_separator": "e"})(log_to_ocel)
# Split on "e", both columns give the id "t" ("Pete", "check ticket"); a relation from
# concept:name keeps that type, while the object has type org:resource.
case("log-to-ocel-running-example-shared-ids", fixture="running-example.csv", functions=_OCEL,
     params={"object_types": ["org:resource", "concept:name"], "obj_separator": "e"})(log_to_ocel)

for _name, _file in [("example-log", "ocel/example_log.jsonocel"), ("ocel20-example", "ocel/ocel20_example.jsonocel")]:
    for _variant in ["ocel_to_nx", "ocel_features_to_nx"]:
        case(f"networkx-{_name}-{_variant.replace('_', '-')}", fixture=_file, params={"variant": _variant},
             functions=["pm4py.convert_ocel_to_networkx"])(ocel_to_networkx)
    for _graph in ["object_interaction", "object_descendants", "object_inheritance", "object_cobirth",
                   "object_codeath"]:
        case(f"objects-graph-{_name}-{_graph.replace('_', '-')}", fixture=_file, params={"graph_type": _graph},
             functions=["pm4py.discover_objects_graph"])(objects_graph)

_IV = ["pm4py.convert_log_to_time_intervals"]
case("time-intervals-running-example", fixture="running-example.csv", functions=_IV)(_intervals)
case("time-intervals-receipt-couple", fixture="receipt.csv", functions=_IV,
     params={"filter_activity_couple": ["T06 Determine necessity of stop advice", "T05 Print and send confirmation of receipt"]})(_intervals)
case("time-intervals-interval-log-couple", fixture="interval_event_log.csv", functions=_IV,
     params={"start_timestamp_key": "start_timestamp",
             "filter_activity_couple": ["send reminder", "cancel order"]})(_intervals)
