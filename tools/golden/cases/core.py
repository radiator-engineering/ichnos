"""Area ``core``: log conversions and log helpers, ported in ``ichnos-core``.

Every case loads the CSV running example, then converts the DataFrame with
``pm4py.convert_to_event_log`` so that pm4py runs its ``EventLog`` code
paths. A Rust test builds the same log with ``format_batch`` and
``EventLog::from_arrow``. Traces are listed in log order, which is case ID
order after ``format_dataframe``. Timestamps are UTC strings.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pm4py

from harness import case
from harness.fixtures import load_log

FIXTURE = "running-example.csv"


def _log(fixtures: dict[str, Path]) -> Any:
    return pm4py.convert_to_event_log(load_log(fixtures["log"]))


def _activities(trace: Any) -> list[Any]:
    return [e["concept:name"] for e in trace]


@case(
    "convert-running-example-csv",
    fixture=FIXTURE,
    functions=[
        "pm4py.convert_to_event_log",
        "pm4py.convert_to_event_stream",
        "pm4py.convert_to_dataframe",
    ],
)
def convert(fixtures: dict[str, Path]) -> dict[str, Any]:
    log = _log(fixtures)
    stream = pm4py.convert_to_event_stream(log)
    back = pm4py.convert_to_event_log(stream)
    frame = pm4py.convert_to_dataframe(log)
    return {
        "traces": [
            {"attributes": dict(t.attributes), "activities": _activities(t)} for t in log
        ],
        "stream_case_ids": [e["case:concept:name"] for e in stream],
        "stream_first_event": dict(stream[0]),
        "round_trip_case_ids": [t.attributes["concept:name"] for t in back],
        "dataframe_columns": list(frame.columns),
        "dataframe_rows": len(frame),
    }


@case(
    "artificial-start-end-running-example-csv",
    fixture=FIXTURE,
    functions=["pm4py.insert_artificial_start_end"],
)
def artificial(fixtures: dict[str, Path]) -> list[dict[str, Any]]:
    log = pm4py.insert_artificial_start_end(_log(fixtures))
    return [
        {
            "case_id": t.attributes["concept:name"],
            "activities": _activities(t),
            "timestamps": [e.get("time:timestamp") for e in t],
        }
        for t in log
    ]


@case(
    "project-running-example-csv",
    fixture=FIXTURE,
    functions=["pm4py.project_on_event_attribute"],
)
def project(fixtures: dict[str, Path]) -> dict[str, Any]:
    log = _log(fixtures)
    return {
        key: pm4py.project_on_event_attribute(log, key)
        for key in ["concept:name", "org:resource", "Costs"]
    }


@case(
    "set-classifier-running-example-csv",
    fixture=FIXTURE,
    functions=["pm4py.set_classifier"],
)
def set_classifier(fixtures: dict[str, Path]) -> dict[str, Any]:
    # set_classifier changes the log in place, so each call gets a fresh log.
    out = {}
    for name, classifier in [
        ("activity_and_costs", ["concept:name", "Costs"]),
        ("resource", "org:resource"),
    ]:
        log = pm4py.set_classifier(_log(fixtures), classifier)
        out[name] = [[e["@@classifier"] for e in t] for t in log]
    return out


@case(
    "hof-running-example-csv",
    fixture=FIXTURE,
    functions=[
        "pm4py.hof.filter_log",
        "pm4py.hof.filter_trace",
        "pm4py.hof.sort_log",
        "pm4py.hof.sort_trace",
    ],
)
def hof(fixtures: dict[str, Path]) -> dict[str, Any]:
    log = _log(fixtures)
    case_ids = lambda log: [t.attributes["concept:name"] for t in log]  # noqa: E731
    first = log[0]
    no_pete = pm4py.hof.filter_trace(lambda e: e["org:resource"] != "Pete", first)
    by_name = pm4py.hof.sort_trace(first, key=lambda e: e["concept:name"], reverse=True)
    return {
        "filter_log_longer_than_5": case_ids(pm4py.hof.filter_log(lambda t: len(t) > 5, log)),
        "sort_log_by_length_reverse": case_ids(pm4py.hof.sort_log(log, key=len, reverse=True)),
        "sort_log_by_length": case_ids(pm4py.hof.sort_log(log, key=len)),
        "filter_trace_without_pete": {
            "attributes": dict(no_pete.attributes),
            "activities": _activities(no_pete),
        },
        "sort_trace_by_activity_reverse": {
            "attributes": dict(by_name.attributes),
            "activities": _activities(by_name),
        },
    }


@case(
    "networkx-running-example-csv",
    fixture=FIXTURE,
    functions=["pm4py.convert_log_to_networkx"],
)
def networkx(fixtures: dict[str, Path]) -> dict[str, Any]:
    graph = pm4py.convert_log_to_networkx(
        _log(fixtures),
        include_df=True,
        other_case_attributes_as_nodes=["creator"],
        event_attributes_as_nodes=["org:resource", "Costs"],
    )
    nodes = sorted(
        ({"id": str(n), "type": d["attr"]["type"]} for n, d in graph.nodes(data=True)),
        key=lambda n: (n["id"], n["type"]),
    )
    edges = sorted(
        (
            {
                "source": str(u),
                "target": str(v),
                "type": d["attr"]["type"],
                "name": d["attr"].get("name"),
            }
            for u, v, d in graph.edges(data=True)
        ),
        key=lambda e: (e["source"], e["target"]),
    )
    return {"nodes": nodes, "edges": edges}


@case(
    "rebase-running-example-csv",
    fixture=FIXTURE,
    functions=["pm4py.rebase"],
    params={"case_id": "org:resource"},
)
def rebase(fixtures: dict[str, Path], case_id: str) -> list[dict[str, Any]]:
    log = pm4py.rebase(_log(fixtures), case_id=case_id)
    return [
        {"case_id": t.attributes["concept:name"], "activities": _activities(t)} for t in log
    ]


@case(
    "sample-running-example-csv",
    fixture=FIXTURE,
    functions=["pm4py.sample_cases", "pm4py.sample_events"],
)
def sample(fixtures: dict[str, Path]) -> dict[str, Any]:
    # pm4py samples with Python's random module, so only the sizes are
    # deterministic. ichnos uses its own seeded generator.
    log = _log(fixtures)
    stream = pm4py.convert_to_event_stream(log)
    return {
        "cases_3": len(pm4py.sample_cases(log, 3)),
        "cases_100": len(pm4py.sample_cases(log, 100)),
        "events_10": len(pm4py.sample_events(stream, 10)),
        "events_100": len(pm4py.sample_events(stream, 100)),
    }


def format_rows(fixtures: dict[str, Path]) -> list[list[Any]]:
    """The ordered ``[case, activity, timestamp]`` rows after ``format_dataframe``."""
    frame = load_log(fixtures["log"])
    return [
        [case_id, activity, timestamp]
        for case_id, activity, timestamp in zip(
            frame["case:concept:name"], frame["concept:name"], frame["time:timestamp"]
        )
    ]


for case_id, fixture in [
    ("format-receipt-csv", "receipt.csv"),
    ("format-interval-event-log-csv", "interval_event_log.csv"),
]:
    case(case_id, fixture=fixture, functions=["pm4py.format_dataframe"])(format_rows)


@case(
    "to-interval-reviewing-csv",
    fixture="reviewing.csv",
    functions=["pm4py.objects.log.util.interval_lifecycle.to_interval"],
)
def to_interval(fixtures: dict[str, Path]) -> list[dict[str, Any]]:
    from pm4py.objects.log.util import interval_lifecycle

    log = interval_lifecycle.to_interval(_log(fixtures))
    return [
        {
            "case_id": t.attributes["concept:name"],
            "events": [
                [
                    e["concept:name"],
                    e["start_timestamp"],
                    e["time:timestamp"],
                    e["@@duration"],
                    e.get("@@startevent_org:resource"),
                ]
                for e in t
            ],
        }
        for t in log
    ]


@case(
    "to-lifecycle-interval-event-log-csv",
    fixture="interval_event_log.csv",
    functions=["pm4py.objects.log.util.interval_lifecycle.to_lifecycle"],
)
def to_lifecycle(fixtures: dict[str, Path]) -> list[dict[str, Any]]:
    from pm4py.objects.log.util import interval_lifecycle

    log = interval_lifecycle.to_lifecycle(_log(fixtures))
    return [
        {
            "case_id": t.attributes["concept:name"],
            "events": [
                [
                    e["concept:name"],
                    e["lifecycle:transition"],
                    e["time:timestamp"],
                    e["@@origin_ev_idx"],
                ]
                for e in t
            ],
        }
        for t in log
    ]
