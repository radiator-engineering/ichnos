"""Live streams, lazy importers, canonical dataframe projection and online DFG."""

import asyncio
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory

import numpy as np
import pandas as pd
from harness import case
from pm4py.objects.log.obj import Event
from pm4py.streaming.algo.discovery.dfg import algorithm as dfg_algorithm
from pm4py.streaming.conversion import from_pandas
from pm4py.streaming.importer.csv import importer as csv_importer
from pm4py.streaming.importer.xes import importer as xes_importer
from pm4py.streaming.stream.live_event_stream import LiveEventStream
from pm4py.streaming.stream.live_trace_stream import LiveTraceStream
from pm4py.streaming.util.live_to_static_stream import LiveToStaticStream


def _value(value):
    if isinstance(value, np.generic):
        value = value.item()
    if isinstance(value, datetime):
        return {"date": value.astimezone(timezone.utc).isoformat(timespec="microseconds")}
    if isinstance(value, dict) and "value" in value and "children" in value:
        children = value["children"]
        if value["value"] is None:
            if isinstance(children, list):
                return {"list": [[str(k), _value(v)] for k, v in children]}
            return {"container": _attributes(children)}
        return {"value": _value(value["value"]), "meta": _attributes(children)}
    return value


def _attributes(attrs):
    return {str(k): _value(v) for k, v in attrs.items()}


def _trace(trace):
    return {"attributes": _attributes(trace.attributes), "events": [_attributes(e) for e in trace]}


def _summary(rows):
    encoded = json.dumps(rows, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    indices = sorted({0, 1, len(rows) - 1} & set(range(len(rows))))
    return {
        "count": len(rows),
        "sha256": hashlib.sha256(encoded).hexdigest(),
        "samples": [[i, rows[i]] for i in indices],
    }


def _state(algo):
    graph, activities, starts, ends = algo.get()
    return {
        "edges": [[a, b, n] for (a, b), n in sorted(graph.items())],
        "activities": activities,
        "starts": starts,
        "ends": ends,
    }


def live_dfg(fixtures, events=None, activity_key="concept:name", case_key="case:concept:name"):
    rows = (
        list(xes_importer.apply(str(fixtures["log"])))
        if events is None
        else [Event(e) for e in events]
    )
    params = {"pm4py:param:activity_key": activity_key, "pm4py:param:case_id_key": case_key}
    algo = dfg_algorithm.apply(parameters=params)
    points = sorted(
        {0, 1, len(rows) // 2, max(0, len(rows) - 1), len(rows)} & set(range(len(rows) + 1))
    )
    snapshots = []
    if 0 in points:
        snapshots.append({"at": 0, "model": _state(algo)})
    for i, event in enumerate(rows, 1):
        algo.receive(event)
        if i in points:
            snapshots.append({"at": i, "model": _state(algo)})
    live_algo = dfg_algorithm.apply(parameters=params)
    collector = LiveToStaticStream()
    removed = LiveToStaticStream()
    stream = LiveEventStream(parameters={"thread_pool_size": 1})
    stream.register(live_algo)
    stream.register(collector)
    stream.register(collector)  # Native observers are a set: no duplicate delivery.
    stream.register(removed)
    for event in rows[:2]:
        stream.append(event)
    asyncio.run(stream.deregister(removed))
    inactive = stream.state.name.lower()
    stream.start()
    active = stream.state.name.lower()
    for event in rows[2:]:
        stream.append(event)
    stream.stop()
    stream.append(Event({activity_key: "ignored-after-stop", case_key: "late"}))
    assert _state(live_algo) == _state(algo)
    return {
        "snapshots": snapshots,
        "live_model": _state(live_algo),
        "states": [inactive, active, stream.state.name.lower()],
        "collected": _summary([_attributes(e) for e in collector.get()]),
        "removed_count": len(removed.get()),
    }


_LIVE_FUNCTIONS = [
    "pm4py.streaming.algo.discovery.dfg.algorithm.apply",
    "pm4py.streaming.algo.discovery.dfg.variants.frequency.apply",
    "pm4py.streaming.algo.interface.StreamingAlgorithm.receive",
    "pm4py.streaming.algo.interface.StreamingAlgorithm.get",
    "pm4py.streaming.stream.live_event_stream.LiveEventStream",
    "pm4py.streaming.util.live_to_static_stream.LiveToStaticStream",
]


def import_stream(
    fixtures, kind, xml=None, csv_text=None, filter_activity=None, filter_case=None, transform=False
):
    def run(path):
        params = {}
        if transform:
            params["transformation_function"] = lambda event: {**event, "copied": "yes"}
        if filter_activity is not None:
            params["acceptance_condition"] = (
                lambda event: event.get("concept:name") != filter_activity
            )
        if filter_case is not None:
            params["acceptance_condition"] = (
                lambda trace: trace.attributes.get("concept:name") != filter_case
            )
        reader = (
            csv_importer.apply(str(path), parameters=params)
            if kind == "csv"
            else xes_importer.apply(
                str(path),
                parameters=params,
                variant=(
                    xes_importer.Variants.XES_TRACE_STREAM
                    if kind == "traces"
                    else xes_importer.Variants.XES_EVENT_STREAM
                ),
            )
        )
        project = _trace if kind == "traces" else _attributes
        first = [project(item) for item in reader]
        reader.reset()
        second = [project(item) for item in reader]
        reader.reset()
        collector = LiveToStaticStream()
        stream = (
            LiveTraceStream(parameters={"thread_pool_size": 1})
            if kind == "traces"
            else LiveEventStream(parameters={"thread_pool_size": 1})
        )
        stream.register(collector)
        stream.start()
        if kind == "traces":
            reader.to_trace_stream(stream)
        else:
            reader.to_event_stream(stream)
        stream.stop()
        assert first == second == [project(item) for item in collector.get()]
        return {
            "items": _summary(first),
            "reset": _summary(second),
            "forwarded": _summary([project(item) for item in collector.get()]),
        }

    if xml is not None or csv_text is not None:
        with TemporaryDirectory() as directory:
            path = Path(directory) / ("input.xes" if xml is not None else "input.csv")
            path.write_text(xml if xml is not None else csv_text)
            return run(path)
    return run(fixtures["log"])


def dataframe_traces(fixtures, rows=None, custom=False, interleaved=False):
    case_key, activity_key, timestamp_key = (
        ("case", "task", "stamp")
        if custom
        else ("case:concept:name", "concept:name", "time:timestamp")
    )
    if rows is None:
        events = list(xes_importer.apply(str(fixtures["log"])))
        frame = pd.DataFrame([dict(event) for event in events])
    else:
        frame = pd.DataFrame(
            [
                {case_key: c, activity_key: a, timestamp_key: datetime.fromisoformat(t)}
                for c, a, t in rows
            ],
            columns=[case_key, activity_key, timestamp_key],
        )
    params = {
        "pm4py:param:case_id_key": case_key,
        "pm4py:param:activity_key": activity_key,
        "pm4py:param:timestamp_key": timestamp_key,
    }
    raw = [_trace(trace) for trace in from_pandas.apply(frame, parameters=params)]
    # Canonical grouping deliberately fixes contiguous-slice assignment for
    # interleaved input. Keep the raw native result to demonstrate the change.
    if interleaved:
        frame = frame.sort_values(case_key, kind="stable")
    reader = from_pandas.apply(frame, parameters=params)
    traces = list(reader)
    reader.reset()
    assert [_trace(t) for t in reader] == [_trace(t) for t in traces]
    reader.reset()
    collector = LiveToStaticStream()
    stream = LiveTraceStream(parameters={"thread_pool_size": 1})
    stream.register(collector)
    stream.register(collector)
    if traces:
        stream.append(reader.read_trace())
    inactive = stream.state.name.lower()
    stream.start()
    active = stream.state.name.lower()
    reader.to_trace_stream(stream)
    stream.stop()
    projected = [_trace(t) for t in traces]
    assert projected == [_trace(t) for t in collector.get()]
    return {
        "traces": _summary(projected),
        "raw": _summary(raw),
        "forwarded": _summary([_trace(t) for t in collector.get()]),
        "grouped_input": interleaved,
        "states": [inactive, active, stream.state.name.lower()],
    }


for _name in ["running-example", "receipt", "roadtraffic100traces"]:
    case("dfg-" + _name, fixture=_name + ".xes", functions=_LIVE_FUNCTIONS)(live_dfg)
    for _kind in ["events", "traces"]:
        case(
            "xes-" + _kind + "-" + _name,
            fixture=_name + ".xes",
            functions=[
                "pm4py.streaming.importer.xes.importer.apply",
                "pm4py.streaming.importer.xes.variants.xes_"
                + ("trace" if _kind == "traces" else "event")
                + "_stream.apply",
            ],
            params={"kind": _kind},
        )(import_stream)
    case(
        "dataframe-" + _name,
        fixture=_name + ".xes",
        functions=[
            "pm4py.streaming.conversion.from_pandas.apply",
            "pm4py.streaming.stream.live_trace_stream.LiveTraceStream",
        ],
        params={},
    )(dataframe_traces)

for _name in ["running-example", "receipt"]:
    case(
        "csv-" + _name,
        fixture=_name + ".csv",
        functions=[
            "pm4py.streaming.importer.csv.importer.apply",
            "pm4py.streaming.importer.csv.variants.csv_event_stream.apply",
        ],
        params={"kind": "csv"},
    )(import_stream)

case(
    "csv-quoted",
    functions=["pm4py.streaming.importer.csv.importer.apply"],
    params={
        "kind": "csv",
        "csv_text": 'case:concept:name,concept:name,extra\nc1,"a,b","line one\nline two"\nc1,"a""b",\n',
    },
)(import_stream)

for _name, _events in {
    "empty": [],
    "interleaved": [
        {"case:concept:name": c, "concept:name": a}
        for c, a in [("x", "a"), ("y", "b"), ("x", "a"), ("y", "c"), ("x", "b")]
    ],
    "missing": [
        {"concept:name": "ignored"},
        {"case:concept:name": "c"},
        {"case:concept:name": "c", "concept:name": ""},
        {"case:concept:name": "c", "concept:name": "a'\\λ"},
    ],
    "custom": [{"case": "1", "task": "α"}, {"case": "2", "task": "β"}, {"case": "1", "task": "β"}],
}.items():
    case(
        "dfg-" + _name,
        functions=_LIVE_FUNCTIONS,
        params={
            "events": _events,
            "case_key": "case" if _name == "custom" else "case:concept:name",
            "activity_key": "task" if _name == "custom" else "concept:name",
        },
    )(live_dfg)

for _name, _rows in {
    "empty": [],
    "unsorted": [
        ["b", "x", "2024-01-01T00:00:02+00:00"],
        ["b", "y", "2024-01-01T00:00:01+00:00"],
        ["a", "z", "2024-01-01T00:00:00+00:00"],
    ],
    "interleaved": [
        ["b", "x", "2024-01-01T00:00:02+00:00"],
        ["a", "z", "2024-01-01T00:00:00+00:00"],
        ["b", "y", "2024-01-01T00:00:01+00:00"],
    ],
    "custom": [["c", "λ", "2024-01-01T00:00:00+00:00"]],
}.items():
    case(
        "dataframe-" + _name,
        functions=["pm4py.streaming.conversion.from_pandas.apply"],
        params={"rows": _rows, "custom": _name == "custom", "interleaved": _name == "interleaved"},
    )(dataframe_traces)

_TYPED_XES = """<log><trace><string key="concept:name" value="c"/><string key="owner" value="λ"/>
<event><string key="concept:name" value="a"><string key="nested" value="v"/></string>
<string key="case:concept:name" value="overwritten"/><int key="n" value="3"/><float key="f" value="2.5"/>
<boolean key="b" value="true"/><date key="time:timestamp" value="2024-01-01T02:00:00+02:00"/>
<id key="id" value="α"/><list key="items"><values><string key="k" value="v"/><int key="k" value="2"/></values></list>
</event></trace><trace><string key="concept:name" value="empty"/></trace></log>"""
for _kind in ["events", "traces"]:
    case(
        "xes-" + _kind + "-typed",
        functions=["pm4py.streaming.importer.xes.importer.apply"],
        params={"kind": _kind, "xml": _TYPED_XES},
    )(import_stream)

case(
    "csv-transformed-filtered",
    functions=["pm4py.streaming.importer.csv.importer.apply"],
    params={
        "kind": "csv",
        "csv_text": "concept:name,case:concept:name\na,c\nb,c\n",
        "transform": True,
        "filter_activity": "a",
    },
)(import_stream)
case(
    "xes-events-filtered",
    fixture="running-example.xes",
    functions=["pm4py.streaming.importer.xes.importer.apply"],
    params={"kind": "events", "filter_activity": "register request"},
)(import_stream)
case(
    "xes-traces-filtered",
    fixture="running-example.xes",
    functions=["pm4py.streaming.importer.xes.importer.apply"],
    params={"kind": "traces", "filter_case": "3"},
)(import_stream)

for _name, _ids in [
    ("numeric-integers", [10, 2]),
    ("numeric-floats", [10.5, 2.5]),
    ("large-integers", [9007199254740993, 9007199254740992]),
]:
    case(
        "dataframe-" + _name,
        functions=["pm4py.streaming.conversion.from_pandas.apply"],
        params={"rows": [[c, a, "2024-01-01T00:00:00+00:00"] for c in _ids for a in ["a", "b"]]},
    )(dataframe_traces)
