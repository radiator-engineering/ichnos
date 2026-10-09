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
from pm4py.streaming.algo.conformance.tbr import algorithm as _tbr_algorithm
from pm4py.streaming.algo.conformance.footprints import algorithm as _fp_algorithm
from pm4py.streaming.algo.conformance.temporal import algorithm as _temporal_algorithm
from pm4py.objects.petri_net.obj import PetriNet, Marking
from pm4py.objects.petri_net.utils.petri_utils import add_arc_from_to
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
# Appended to the stream area: incremental conformance using native algorithms.


def _net(spec):
    net = PetriNet("stream")
    places = {name: PetriNet.Place(name) for name in spec["places"]}
    transitions = {name: PetriNet.Transition(name, label) for name, label in spec["transitions"]}
    net.places.update(places.values())
    net.transitions.update(transitions.values())
    for place, transition, output, weight in spec["arcs"]:
        source, target = (
            (transitions[transition], places[place])
            if output
            else (places[place], transitions[transition])
        )
        add_arc_from_to(source, target, net, weight=weight)
    return (
        net,
        Marking({places[p]: n for p, n in spec["initial"].items()}),
        Marking({places[p]: n for p, n in spec["final"].items()}),
    )


def _chain(activities):
    places = [f"p{i:03}" for i in range(len(activities) + 1)]
    transitions = [[f"t{i:03}", a] for i, a in enumerate(activities)]
    arcs = []
    for i, (transition, _) in enumerate(transitions):
        arcs += [[places[i], transition, False, 1], [places[i + 1], transition, True, 1]]
    return {
        "places": places,
        "transitions": transitions,
        "arcs": arcs,
        "initial": {places[0]: 1},
        "final": {places[-1]: 1},
    }


def _marking(marking):
    return {p.name: n for p, n in marking.items() if n}


def _conf_state(algo, kind):
    if kind in {"tbr", "footprints"}:
        diagnostics = algo.get()
        assert len(diagnostics) == len(algo.case_dict)
    if kind == "tbr":
        return {
            c: {
                "marking": _marking(algo.get_status(c)["marking"]),
                "missing": int(algo.missing[c]),
                "is_fit": int(algo.missing[c]) == 0,
            }
            for c in sorted(algo.case_dict)
        }
    if kind == "footprints":
        return {
            c: {
                "last": algo.case_dict[c],
                "deviations": int(algo.dev_dict[c]),
                "is_fit": algo.get_status(c),
            }
            for c in sorted(algo.case_dict)
        }
    return {
        c: [[d[1], d[2], d[3], "infinity" if d[4] == 2**63 - 1 else d[4]] for d in rows]
        for c, rows in sorted(algo.get().items())
    }


def streaming_conformance(
    fixtures,
    kind,
    events=None,
    model=None,
    custom=False,
    maximum_iterations=10,
    zeta=6.0,
    interval=False,
    perturb=False,
):
    import pm4py

    rows = (
        list(xes_importer.apply(str(fixtures["log"])))
        if events is None
        else [Event(e) for e in events]
    )
    case_key, activity_key, end_key = (
        ("case", "task", "end")
        if custom
        else ("case:concept:name", "concept:name", "time:timestamp")
    )
    start_key = "start" if interval else end_key
    for row in rows:
        for key in {start_key, end_key}:
            if key in row and isinstance(row[key], str):
                row[key] = datetime.fromisoformat(row[key])
    activities = sorted({str(e[activity_key]) for e in rows if activity_key in e})
    if model is None:
        if kind == "tbr":
            model = _pnml_spec(fixtures["model"]) if "model" in fixtures else _chain(activities)
        elif kind == "footprints":
            log = pm4py.read_xes(str(fixtures["log"]), return_legacy_log_object=True)
            from pm4py.algo.discovery.footprints import algorithm as fp_discovery

            fp = fp_discovery.apply(log, variant=fp_discovery.Variants.ENTIRE_EVENT_LOG)
            model = {
                key: sorted(fp[key])
                for key in [
                    "activities",
                    "start_activities",
                    "end_activities",
                    "sequence",
                    "parallel",
                ]
            }
            if perturb:
                model["start_activities"] = []
                model["end_activities"] = []
                model["sequence"] = []
                model["parallel"] = []
        else:
            log = pm4py.read_xes(str(fixtures["log"]), return_legacy_log_object=True)
            profile = pm4py.discover_temporal_profile(log)
            model = [[a, b, mean, std] for (a, b), (mean, std) in sorted(profile.items())]
    params = {
        "pm4py:param:case_id_key": case_key,
        "pm4py:param:activity_key": activity_key,
        "pm4py:param:timestamp_key": end_key,
        "pm4py:param:start_timestamp_key": start_key,
        "maximum_iterations_invisibles": maximum_iterations,
        "zeta": zeta,
    }
    if kind == "tbr":
        algo = _tbr_algorithm.apply(*_net(model), parameters=params)
    elif kind == "footprints":
        fp = {
            key: set(tuple(p) if isinstance(p, list) else p for p in items)
            for key, items in model.items()
        }
        algo = _fp_algorithm.apply(fp, parameters=params)
    else:
        algo = _temporal_algorithm.apply(
            {(a, b): (mean, std) for a, b, mean, std in model}, parameters=params
        )
    points = sorted({0, 1, len(rows) // 2, max(0, len(rows) - 1), len(rows)})
    snapshots = [{"at": 0, "state": _conf_state(algo, kind)}]
    for i, row in enumerate(rows, 1):
        algo.receive(row)
        if i in points:
            snapshots.append({"at": i, "state": _conf_state(algo, kind)})
    terminated = {}
    if kind in {"tbr", "footprints"}:
        for c in sorted(list(algo.case_dict)):
            result = algo.terminate(c)
            if kind == "tbr":
                result["marking"] = _marking(result["marking"])
            terminated[c] = result
        # Also exercise native terminate_all, including fresh case IDs.
        for row in rows:
            algo.receive(row)
        algo.terminate_all()
    after = _conf_state(algo, kind) if kind != "temporal" else None
    if events is None and kind != "temporal":
        # Every case is checked, with complete ordered digests and typed samples.
        compact = lambda state: _summary([[c, value] for c, value in sorted(state.items())])
        for snapshot in snapshots:
            snapshot["state"] = compact(snapshot["state"])
        terminated = compact(terminated)
        after = compact(after)
    return {
        "kind": kind,
        "model": model,
        "snapshots": snapshots,
        "terminated": terminated,
        "after_termination": after,
    }


_CONF_FUNCTIONS = {
    kind: [
        f"pm4py.streaming.algo.conformance.{kind}.algorithm.apply",
        f"pm4py.streaming.algo.conformance.{kind}.variants.classic.apply",
    ]
    for kind in ["tbr", "footprints", "temporal"]
}
for _kind, _class, _methods in [
    (
        "tbr",
        "TbrStreamingConformance",
        [
            "get_paths_net",
            "verify_tbr",
            "enable_trans_with_invisibles",
            "get_status",
            "terminate",
            "terminate_all",
        ],
    ),
    (
        "footprints",
        "FootprintsStreamingConformance",
        [
            "verify_footprints",
            "verify_intra_case",
            "verify_start_case",
            "get_status",
            "terminate",
            "terminate_all",
        ],
    ),
    ("temporal", "TemporalProfileStreamingConformance", ["check_conformance"]),
]:
    _path = f"pm4py.streaming.algo.conformance.{_kind}.variants.classic.{_class}"
    _CONF_FUNCTIONS[_kind] += [_path] + [_path + "." + method for method in _methods]
for _kind in _CONF_FUNCTIONS:
    for _fixture in ["running-example", "receipt", "roadtraffic100traces"]:
        case(
            "conf-" + _kind + "-" + _fixture,
            fixture=_fixture + ".xes",
            functions=_CONF_FUNCTIONS[_kind],
            params={"kind": _kind},
        )(streaming_conformance)
for _fixture in ["running-example", "receipt", "roadtraffic100traces"]:
    case(
        "conf-footprints-deviating-" + _fixture,
        fixture=_fixture + ".xes",
        functions=_CONF_FUNCTIONS["footprints"],
        params={"kind": "footprints", "perturb": True},
    )(streaming_conformance)

_PAIR_FP = {
    "activities": ["a", "b"],
    "start_activities": ["a"],
    "end_activities": ["b"],
    "sequence": [["a", "b"]],
    "parallel": [["b", "a"]],
}
_CONF_ROWS = [
    {"case:concept:name": c, "concept:name": a}
    for c, a in [("x", "a"), ("y", "b"), ("x", "unknown"), ("x", "b"), ("y", "a"), ("x", "a")]
]
_CONF_ROWS += [{"concept:name": "missing-case"}, {"case:concept:name": "missing-activity"}]
for _kind, _model in [("tbr", _chain(["a", "b"])), ("footprints", _PAIR_FP)]:
    case(
        "conf-" + _kind + "-interleaved",
        functions=_CONF_FUNCTIONS[_kind],
        params={"kind": _kind, "events": _CONF_ROWS, "model": _model},
    )(streaming_conformance)
    case(
        "conf-" + _kind + "-empty",
        functions=_CONF_FUNCTIONS[_kind],
        params={"kind": _kind, "events": [], "model": _model},
    )(streaming_conformance)
    case(
        "conf-" + _kind + "-custom",
        functions=_CONF_FUNCTIONS[_kind],
        params={
            "kind": _kind,
            "events": [{"case": "c", "task": "a"}, {"case": "c", "task": "b"}],
            "model": _model,
            "custom": True,
        },
    )(streaming_conformance)

_SILENT_NET = {
    "places": ["p0", "p1", "p2", "p3"],
    "transitions": [["tau-start", None], ["a", "a"], ["tau-end", None]],
    "arcs": [
        ["p0", "tau-start", False, 1],
        ["p1", "tau-start", True, 1],
        ["p1", "a", False, 1],
        ["p2", "a", True, 1],
        ["p2", "tau-end", False, 1],
        ["p3", "tau-end", True, 1],
    ],
    "initial": {"p0": 1},
    "final": {"p3": 1},
}
for _maximum in [0, 1, 2, 10]:
    case(
        "conf-tbr-silent-" + str(_maximum),
        functions=_CONF_FUNCTIONS["tbr"],
        params={
            "kind": "tbr",
            "events": [{"case:concept:name": "c", "concept:name": "a"}],
            "model": _SILENT_NET,
            "maximum_iterations": _maximum,
        },
    )(streaming_conformance)
_WEIGHTED_NET = {
    "places": ["p0", "p1"],
    "transitions": [["a", "a"]],
    "arcs": [["p0", "a", False, 2], ["p1", "a", True, 2]],
    "initial": {},
    "final": {"p1": 1},
}
case(
    "conf-tbr-weighted",
    functions=_CONF_FUNCTIONS["tbr"],
    params={
        "kind": "tbr",
        "events": [{"case:concept:name": "c", "concept:name": "a"}],
        "model": _WEIGHTED_NET,
    },
)(streaming_conformance)

_TEMP_ROWS = [
    {
        "case": "c",
        "task": a,
        "start": "2024-01-01T00:00:" + s + "+00:00",
        "end": "2024-01-01T00:00:" + e + "+00:00",
    }
    for a, s, e in [
        ("a", "00", "03"),
        ("b", "02", "04"),
        ("b", "05", "07"),
        ("a", "06", "08"),
        ("b", "20", "21"),
    ]
]
_TEMP_ROWS.insert(
    2,
    {
        "case": "other",
        "task": "a",
        "start": "2024-01-01T00:00:00+00:00",
        "end": "2024-01-01T00:00:01+00:00",
    },
)
_TEMP_ROWS += [{"case": "c", "task": "b"}]
for _zeta in [0.0, 1.0, 6.0]:
    case(
        "conf-temporal-interval-" + str(int(_zeta)),
        functions=_CONF_FUNCTIONS["temporal"],
        params={
            "kind": "temporal",
            "events": _TEMP_ROWS,
            "model": [["a", "b", 5.0, 1.0], ["b", "b", 0.0, 0.0]],
            "custom": True,
            "interval": True,
            "zeta": _zeta,
        },
    )(streaming_conformance)
case(
    "conf-temporal-empty",
    functions=_CONF_FUNCTIONS["temporal"],
    params={"kind": "temporal", "events": [], "model": []},
)(streaming_conformance)


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


def _pnml_spec(path):
    import pm4py

    net, initial, final = pm4py.read_pnml(str(path))
    arcs = []
    for arc in net.arcs:
        output = isinstance(arc.source, PetriNet.Transition)
        place, transition = (arc.target, arc.source) if output else (arc.source, arc.target)
        arcs.append([place.name, transition.name, output, arc.weight])
    return {
        "places": sorted(p.name for p in net.places),
        "transitions": sorted([[t.name, t.label] for t in net.transitions]),
        "arcs": sorted(arcs),
        "initial": _marking(initial),
        "final": _marking(final),
    }


case(
    "conf-tbr-running-example-pnml",
    fixtures={"log": "running-example.xes", "model": "running-example.pnml"},
    functions=_CONF_FUNCTIONS["tbr"],
    params={"kind": "tbr"},
)(streaming_conformance)

_TWO_PATHS = {
    "places": ["short", "long", "middle", "ready", "done"],
    "transitions": [["short-tau", None], ["long-tau-1", None], ["long-tau-2", None], ["A", "A"]],
    "arcs": [
        ["short", "short-tau", False, 1],
        ["ready", "short-tau", True, 1],
        ["long", "long-tau-1", False, 1],
        ["middle", "long-tau-1", True, 1],
        ["middle", "long-tau-2", False, 1],
        ["ready", "long-tau-2", True, 1],
        ["ready", "A", False, 1],
        ["done", "A", True, 1],
    ],
    "initial": {"short": 1, "long": 1},
    "final": {"done": 1, "long": 1},
}
case(
    "conf-tbr-two-silent-paths",
    functions=_CONF_FUNCTIONS["tbr"],
    params={
        "kind": "tbr",
        "model": _TWO_PATHS,
        "events": [{"case:concept:name": "c", "concept:name": "A"}],
    },
)(streaming_conformance)

_DUPLICATE_LABELS = {
    "places": ["first", "second", "done"],
    "transitions": [["first-A", "A"], ["second-A", "A"]],
    "arcs": [
        ["first", "first-A", False, 1],
        ["second", "first-A", True, 1],
        ["second", "second-A", False, 1],
        ["done", "second-A", True, 1],
    ],
    "initial": {"first": 1},
    "final": {"done": 1},
}
case(
    "conf-tbr-duplicate-labels",
    functions=_CONF_FUNCTIONS["tbr"],
    params={
        "kind": "tbr",
        "model": _DUPLICATE_LABELS,
        "events": [{"case:concept:name": "c", "concept:name": "A"}] * 2,
    },
)(streaming_conformance)

# Group three: proxy-trie online alignments and per-object stream distribution.
from pm4py.streaming.algo.conformance.alignments import algorithm as _iws_algorithm
from pm4py.streaming.algo.conformance.alignments.variants import approx_iws as _iws
from pm4py.streaming.conversion.ocel_flatts_distributor import OcelFlattsDistributor
from pm4py.objects.log.obj import Trace, EventLog

_IWS_FUNCTIONS = [
    "pm4py.streaming.algo.conformance.alignments.algorithm.apply",
    "pm4py.streaming.algo.conformance.alignments.variants.approx_iws.apply",
    "pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments",
    "pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments.finish",
]


def _iws_result(result):
    return {k: v for k, v in result.items() if k not in {
        "last_event_runtime", "total_runtime", "runtime",
    }}


def streaming_iws(fixtures, events=None, model=None, proxy=None, custom=False,
                  look_ahead=3, decay_time=10, discount_factor=0.9, max_states=20,
                  automatic=False):
    rows = list(xes_importer.apply(str(fixtures["log"]))) if events is None else [Event(e) for e in events]
    case_key, activity_key = ("case", "task") if custom else ("case:concept:name", "concept:name")
    if model is None:
        model = _pnml_spec(fixtures["model"]) if "model" in fixtures else _chain(sorted({str(e[activity_key]) for e in rows}))
    if proxy is None:
        if "model" in fixtures:
            cases = {}
            for row in rows:
                cases.setdefault(str(row[case_key]), []).append(row[activity_key])
            proxy = list(cases.values())
        else:
            proxy = [[label for _, label in model["transitions"] if label is not None]]
    params = {
        "pm4py:param:case_id_key": case_key,
        "pm4py:param:activity_key": activity_key,
        "look_ahead": look_ahead, "decay_time": decay_time,
        "discount_factor": discount_factor, "max_states": max_states,
        "ret_tuple_as_trans_desc": True,
    }
    if not automatic:
        params["proxy_log"] = EventLog([Trace([Event({activity_key: a}) for a in trace]) for trace in proxy])
    net, im, fm = _net(model)
    sequences = _iws._proxy_transition_sequences(net, im, fm, params)
    # Record actual native-prepared proxy runs; duplicate-label/model-search
    # ties are explicit inputs to the Rust replay rather than hidden choices.
    proxy_sequences = [[t.name for t in s] for s in sequences]
    algo = _iws_algorithm.apply(net, im, fm, parameters=params)
    compact = lambda state: _summary([[c, _iws_result(v)] for c, v in sorted(state.items())]) if events is None else {c: _iws_result(v) for c, v in state.items()}
    points = sorted({0, 1, len(rows) // 2, max(0, len(rows)-1), len(rows)})
    snapshots = [{"at": 0, "state": compact(algo.get())}]
    for i, row in enumerate(rows, 1):
        algo.receive(row)
        if i in points:
            snapshots.append({"at": i, "state": compact(algo.get())})
    finished = {c: _iws_result(algo.finish(c)) for c in sorted(algo.get())}
    after = compact(algo.get())
    if events is None:
        finished = _summary([[c, v] for c, v in sorted(finished.items())])
    # Native live delivery is also checked against every final prefix.
    live = _iws_algorithm.apply(*_net(model), parameters=params)
    stream = LiveEventStream(parameters={"thread_pool_size": 1})
    stream.register(live)
    for row in rows[:1]:
        stream.append(row)
    stream.start()
    for row in rows[1:]:
        stream.append(row)
    stream.stop()
    assert compact(live.get()) == snapshots[-1]["state"]
    return {"model": model, "proxy_sequences": proxy_sequences, "snapshots": snapshots,
            "finished": finished, "after_finish": after}


for _fixture in ["running-example", "receipt", "roadtraffic100traces"]:
    case("iws-" + _fixture, fixture=_fixture + ".xes", functions=_IWS_FUNCTIONS)(streaming_iws)

_IWS_CHAIN = {
    "places": ["p0", "p1", "p2", "p3", "p4"],
    "transitions": [["start", None], ["A", "A"], ["B", "B"], ["end", None]],
    "arcs": [["p0", "start", False, 1], ["p1", "start", True, 1],
             ["p1", "A", False, 1], ["p2", "A", True, 1],
             ["p2", "B", False, 1], ["p3", "B", True, 1],
             ["p3", "end", False, 1], ["p4", "end", True, 1]],
    "initial": {"p0": 1}, "final": {"p4": 1},
}
_IWS_ROWS = [{"case:concept:name": c, "concept:name": a} for c, a in
             [("c", "B"), ("d", "A"), ("c", "X"), ("d", "B"), ("c", "A"), ("c", "X")]]
for _name, _settings in [
    ("silent-lookahead", {}), ("lookahead-one", {"look_ahead": 1}),
    ("decay-fallback", {"decay_time": 1, "discount_factor": 0.25}),
    ("state-cap", {"max_states": 1}),
    ("automatic-chain", {"automatic": True}),
]:
    case("iws-" + _name, functions=_IWS_FUNCTIONS,
         params={"events": _IWS_ROWS, "model": _IWS_CHAIN, "proxy": [["A", "B"]], **_settings})(streaming_iws)
case("iws-completion-custom", functions=_IWS_FUNCTIONS,
     params={"events": [{"case": "c", "task": "A", "@@complete": True},
                        {"case": "c", "task": "B"}, {"case": "d", "task": "X"}],
             "model": _IWS_CHAIN, "proxy": [["A", "B"]], "custom": True})(streaming_iws)
case("iws-empty", functions=_IWS_FUNCTIONS,
     params={"events": [], "model": _IWS_CHAIN, "proxy": [["A", "B"]]})(streaming_iws)
case("iws-duplicate-labels", functions=_IWS_FUNCTIONS,
     params={"events": [{"case:concept:name": "c", "concept:name": "A"}] * 2,
             "model": _DUPLICATE_LABELS, "proxy": [["A", "A"]]})(streaming_iws)

# Two complete branches share a prefix and have different invisible suffixes.
_IWS_BRANCH = {
    "places": ["p0", "p1", "p2", "p3", "end"],
    "transitions": [["A", "A"], ["B", "B"], ["C", "C"], ["tau", None]],
    "arcs": [["p0", "A", False, 1], ["p1", "A", True, 1],
             ["p1", "B", False, 1], ["end", "B", True, 1],
             ["p1", "C", False, 1], ["p2", "C", True, 1],
             ["p2", "tau", False, 1], ["end", "tau", True, 1]],
    "initial": {"p0": 1}, "final": {"end": 1},
}
case("iws-branching-proxy", functions=_IWS_FUNCTIONS,
     params={"events": [{"case:concept:name": "c", "concept:name": "A"},
                        {"case:concept:name": "d", "concept:name": "C"}],
             "model": _IWS_BRANCH, "proxy": [["A", "B"], ["A", "C"]]})(streaming_iws)

_OCEL_FUNCTIONS = ["pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor" + s
                   for s in ["", ".register", ".append"]]


def streaming_ocel(fixtures, events=None, types=None, custom=False, duplicate=False):
    if events is None:
        source = json.loads(fixtures["log"].read_text())
        types = sorted({o["ocel:type"] for o in source["ocel:objects"].values()})
        events = []
        for identifier, e in source["ocel:events"].items():
            row = {"ocel:eid": identifier, "ocel:activity": e["ocel:activity"],
                   "ocel:timestamp": datetime.fromisoformat(e["ocel:timestamp"]).replace(tzinfo=timezone.utc), **e["ocel:vmap"]}
            for ot in types:
                row["ocel:type:" + ot] = [oid for oid in e["ocel:omap"] if source["ocel:objects"][oid]["ocel:type"] == ot]
            events.append(row)
    else:
        events = [dict(e) for e in events]
        for e in events:
            key = "when" if custom else "ocel:timestamp"
            if key in e:
                e[key] = datetime.fromisoformat(e[key])
    params = {} if not custom else {
        "pm4py:param:activity_key": "task", "pm4py:param:case_id_key": "case",
        "pm4py:param:timestamp_key": "end",
        "param:event:activity": "act", "param:event:timestamp": "when",
        "param:object:type:prefix:extended": "objects:",
    }
    distributor = OcelFlattsDistributor(parameters=params)
    collectors = {}
    streams = []
    for ot in types:
        stream = LiveEventStream(parameters={"thread_pool_size": 1})
        collector = LiveToStaticStream()
        stream.register(collector)
        distributor.register(ot, stream)
        if duplicate:
            distributor.register(ot, stream)
        stream.start()
        collectors[ot] = collector
        streams.append(stream)
    for e in events:
        distributor.append(e)
    for s in streams:
        s.stop()
    return {"events": [_attributes(e) for e in events], "types": types,
            "flattened": {ot: [_attributes(e) for e in collector.get()] for ot, collector in collectors.items()}}


case("ocel-example", fixture="ocel/example_log.jsonocel", functions=_OCEL_FUNCTIONS)(streaming_ocel)
_OCEL_ROWS = [{"ocel:activity": "A", "ocel:timestamp": "2024-01-01T00:00:00+00:00",
               "payload": 42, "case:concept:name": "overwritten", "ocel:type:order": ["o1", "o1", "o2"],
               "ocel:type:item": ["i1"], "ocel:type:unregistered": ["u1"]},
              {"ocel:activity": "B", "ocel:timestamp": "2024-01-02T00:00:00+00:00",
               "ocel:type:order": [], "ocel:type:item": ["i2"]}]
case("ocel-duplicates", functions=_OCEL_FUNCTIONS,
     params={"events": _OCEL_ROWS, "types": ["order", "item", "empty"], "duplicate": True})(streaming_ocel)
case("ocel-custom", functions=_OCEL_FUNCTIONS,
     params={"events": [{"act": "A", "when": "2024-01-01T00:00:00+00:00",
                         "objects:order": ["o1", "o2"], "payload": "kept"}],
             "types": ["order"], "custom": True})(streaming_ocel)
case("ocel-empty", functions=_OCEL_FUNCTIONS,
     params={"events": [], "types": ["order"]})(streaming_ocel)

case("iws-running-example-pnml", fixtures={"log":"running-example.xes","model":"running-example.pnml"}, functions=_IWS_FUNCTIONS)(streaming_iws)

# Prefix-only Declare automata: immediate, absorbing violations; no end checks.
from pm4py.streaming.algo.conformance.declare import algorithm as _declare_algorithm

_DECLARE_STREAM_FUNCTIONS = [
    "pm4py.streaming.algo.conformance.declare.algorithm.apply",
    "pm4py.streaming.algo.conformance.declare.variants.automata.apply",
    "pm4py.streaming.algo.conformance.declare.variants.automata.DeclareStreamingConformance",
]
_DECLARE_UNARY = ["existence", "absence", "exactly_one", "init"]
_DECLARE_BINARY = ["responded_existence", "coexistence", "response", "precedence",
                   "succession", "altresponse", "altprecedence", "altsuccession",
                   "chainresponse", "chainprecedence", "chainsuccession",
                   "noncoexistence", "nonsuccession", "nonchainsuccession"]
_DECLARE_ALL = [[t, ["A"]] for t in _DECLARE_UNARY] + [[t, ["A", "B"]] for t in _DECLARE_BINARY]


def _declare_state(algo, compact=False):
    result = algo.get()
    cases = {}
    for case_id, data in algo._cases.items():
        constraints = sorted([[template, list(args), state]
                              for (template, args), (state, _) in data["constraints_state"].items()])
        cases[str(case_id)] = {"events": data["events"], "deviations": data["deviations"],
                              "constraints_state": constraints}
    history = [[_value(timestamp), count] for timestamp, count in result["deviations_per_time"]]
    return {
        "total_events_processed": result["total_events_processed"],
        "total_deviations": result["total_deviations"],
        "deviations_per_time": _summary(history) if compact else history,
        "cases": _summary([[c, v] for c, v in sorted(cases.items())]) if compact else cases,
    }


def streaming_declare(fixtures, events=None, model=None):
    import pm4py

    rows = list(xes_importer.apply(str(fixtures["log"]))) if events is None else [Event(e) for e in events]
    for row in rows:
        if isinstance(row.get("time:timestamp"), str):
            row["time:timestamp"] = datetime.fromisoformat(row["time:timestamp"])
    if model is None:
        # Discover rules on the real log for two representative activities.
        # Use zero selection thresholds to exercise all eighteen templates.
        log = pm4py.read_xes(str(fixtures["log"]), return_legacy_log_object=True)
        activities = sorted({str(e["concept:name"]) for e in rows})[:2]
        native_model = pm4py.discover_declare(log, considered_activities=set(activities),
                                             min_support_ratio=0, min_confidence_ratio=0)
        model = sorted([[template, list(args) if isinstance(args, tuple) else [args]]
                        for template, rules in native_model.items() for args in rules])
    else:
        native_model = {}
        for template, args in model:
            key = args[0] if len(args) == 1 else tuple(args)
            native_model.setdefault(template, {})[key] = {"support": 123, "confidence": 456}
    algo = _declare_algorithm.apply(native_model)
    points = sorted({0, 1, len(rows)//2, max(0,len(rows)-1),len(rows)})
    snapshots = [{"at":0,"state":_declare_state(algo,events is None)}]
    for i,row in enumerate(rows,1):
        algo.receive(row)
        if i in points:
            snapshots.append({"at":i,"state":_declare_state(algo,events is None)})
    live_algo = _declare_algorithm.apply(native_model)
    stream = LiveEventStream(parameters={"thread_pool_size":1})
    stream.register(live_algo)
    for row in rows[:1]:
        stream.append(row)
    stream.start()
    for row in rows[1:]:
        stream.append(row)
    stream.stop()
    assert _declare_state(live_algo,events is None) == snapshots[-1]["state"]
    return {"model":model,"snapshots":snapshots,"live":_declare_state(live_algo,events is None)}


for _fixture in ["running-example", "receipt", "roadtraffic100traces"]:
    case("declare-" + _fixture,fixture=_fixture + ".xes",functions=_DECLARE_STREAM_FUNCTIONS)(streaming_declare)


def _declare_events(cases):
    return [{"case:concept:name":case_id,"concept:name":activity}
            for case_id,trace in cases for activity in trace]


for _name,_rows in [
    ("all-templates",_declare_events([("a",["A","A","B","B","X","A","B"]),
                                     ("b",["B","A","X","B"]),
                                     ("c",["X","X"])])),
    ("pending",_declare_events([("a",["A"]),("b",["X"]),("c",["A","A","B"])])),
    ("interleaved",_declare_events([("a",["A"]),("b",["B"]),("a",["B"]),
                                   ("b",["A"]),("a",["A"]),("b",["B"])])),
    ("missing",[{}, {"concept:name":"A"}, {"case:concept:name":"a"},
                {"concept:name":"B"}, {"case:concept:name":"a","concept:name":"A"}]),
    ("timestamps",[{"case:concept:name":"a","concept:name":"A","time:timestamp":"2024-01-01T00:00:00+00:00"},
                   {"case:concept:name":"a","concept:name":"B"},
                   {"case:concept:name":"a","concept:name":"A","time:timestamp":42}]),
    ("empty",[]),
]:
    case("declare-" + _name,functions=_DECLARE_STREAM_FUNCTIONS,
         params={"events":_rows,"model":_DECLARE_ALL})(streaming_declare)

case("declare-self-pairs",functions=_DECLARE_STREAM_FUNCTIONS,
     params={"events":_declare_events([("a",["A","A","X","A"])]),
             "model":[[t,["A","A"]] for t in _DECLARE_BINARY]})(streaming_declare)
case("declare-empty-model",functions=_DECLARE_STREAM_FUNCTIONS,
     params={"events":_declare_events([("a",["A","B"])]),"model":[]})(streaming_declare)
case("declare-special-labels",functions=_DECLARE_STREAM_FUNCTIONS,
     params={"events":_declare_events([("case ' \\ ☃",["a,'\\☃","b)\n","a,'\\☃"])]),
             "model":[["precedence",["a,'\\☃","b)\n"]], ["chainresponse",["a,'\\☃","b)\n"]]]})(streaming_declare)
