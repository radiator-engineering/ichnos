"""Area ``discovery``: process discovery, ported in ``ichnos-discovery``.

Each miner lane keeps its cases in its own clearly named functions below.

Inductive miner (lane ``miner-inductive``), cases ``inductive-<variant>-<log>``:

- ``im``: ``pm4py.discover_process_tree_inductive(log)`` (noise 0, so IM).
- ``imf``: the same with ``noise_threshold=0.2`` (so IMf).
- ``imd``: the same on ``pm4py.objects.dfg.obj.DFG(*pm4py.discover_dfg(log))``
  (a DFG, so IMd).

Each ``im`` and ``imd`` case emits:

- ``tree``: the discovered tree in pm4py's string syntax.
- ``tree_behaviour``: :func:`harness.behaviour.model_behaviour` of the tree.
- ``petri_net_behaviour``: the same for ``pm4py.discover_petri_net_inductive``
  with the same arguments.

Cases ``inductive-im-<option>-<log>`` run IM with one option set, through
``pm4py.algo.discovery.inductive.algorithm.apply``, because
``disable_strict_sequence_cut`` is not a ``pm4py.discover_*`` argument. The
net is ``pm4py.convert_to_petri_net`` of the tree, as in
``pm4py.discover_petri_net_inductive``:

- ``nofallthrough``: ``disable_fallthroughs=True``, so only the empty-traces
  and flower-model fall-throughs run.
- ``plainsequence``: ``disable_strict_sequence_cut=True``.

The six logs reach the activity-concurrent, strict tau loop and tau loop
fall-throughs, but never activity once per trace. The case
``inductive-im-fallthroughs-synthetic`` runs IM on a small log, given as
trace strings in ``params`` and read with ``pm4py.parse_event_log_string``.
It reaches activity once per trace, strict tau loop and tau loop. These cases
emit the same three fields as ``im``.

pm4py's IMf result can depend on Python's hash seed: the order of its
exclusive-choice groups follows set order, and IMf breaks ties by that order.
So each ``imf`` case mines the log once per seed in ``IMF_SEEDS``, each in a
fresh interpreter, and emits ``runs``: one entry per distinct tree, with the
three fields above and the ``seeds`` that produced it, sorted by tree.

Cases ``bpmn-inductive-<variant>-<log>`` run
``pm4py.discover_bpmn_inductive`` with the same arguments and logs as the
``inductive`` cases. Each emits ``tree`` (the tree
``pm4py.discover_process_tree_inductive`` gives for the same arguments) and
``bpmn``: :func:`cases.bpmn.canonical_bpmn` of the diagram, which a Rust
test compares up to isomorphism. The ``imf`` cases emit ``runs`` over
``IMF_SEEDS``, grouped by tree, as above.

Logs are CSV only until ichnos can read XES in the discovery tests.

Temporal profile (lane ``miner-temporal-profile``), cases
``temporal-profile-<log>``. Each case runs both
variants in ``pm4py.algo.discovery.temporal_profile.variants``: ``log`` on ``pm4py.convert_to_event_log`` of the log, and ``dataframe`` on the
log as loaded. Each variant runs once with elapsed time and once with
``business_hours=True`` (pm4py's default slots). The interval log passes
``start_timestamp_key="start_timestamp"``; the other logs use the completion
timestamp as the start. Each profile is a list of
``[activity, activity, mean, stdev]`` rows sorted by the activity pair.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

import pm4py
from pm4py.objects.dfg.obj import DFG

from harness import case
from harness.behaviour import FUNCTIONS as BEHAVIOUR_FUNCTIONS
from harness.behaviour import model_behaviour
from harness.fixtures import load_log

INDUCTIVE_LOGS = {
    "running-example-csv": "running-example.csv",
    "receipt-csv": "receipt.csv",
    "roadtraffic100traces-csv": "roadtraffic100traces.csv",
    "interleavings-receipt_even-csv": "interleavings/receipt_even.csv",
    "interleavings-receipt_odd-csv": "interleavings/receipt_odd.csv",
    "reviewing-csv": "reviewing.csv",
}

INDUCTIVE_FUNCTIONS = [
    "pm4py.discover_process_tree_inductive",
    "pm4py.discover_petri_net_inductive",
    *BEHAVIOUR_FUNCTIONS,
]

# Variant name -> (keyword arguments, whether the miner gets a DFG).
INDUCTIVE_VARIANTS = {
    "im": ({}, False),
    "imf": ({"noise_threshold": 0.2}, False),
    "imd": ({}, True),
}


# Option name -> (algorithm.apply parameters, log id), for IM.
INDUCTIVE_OPTIONS = {
    "nofallthrough": ({"disable_fallthroughs": True}, "receipt-csv"),
    "plainsequence": ({"disable_strict_sequence_cut": True}, "receipt-csv"),
}

# A small log that reaches the activity-once-per-trace fall-through.
INDUCTIVE_SYNTHETIC_TRACES = ["a,b,d,c,d", "b,a", "b,d,c,b,a"]

INDUCTIVE_OPTION_FUNCTIONS = [
    "pm4py.algo.discovery.inductive.algorithm.apply",
    "pm4py.convert_to_petri_net",
    *BEHAVIOUR_FUNCTIONS,
]

# Hash seeds for the IMf runs.
IMF_SEEDS = list(range(8))


def inductive(fixtures: dict[str, Path], variant: str) -> dict[str, Any]:
    """Runs one inductive miner variant on the log in ``fixtures``."""
    if variant == "imf":
        return _inductive_seeds(fixtures["log"], variant)
    return _inductive_run(fixtures["log"], variant)


def _inductive_run(path: Path, variant: str) -> dict[str, Any]:
    kwargs, on_dfg = INDUCTIVE_VARIANTS[variant]
    log = load_log(path)
    source = DFG(*pm4py.discover_dfg(log)) if on_dfg else log
    tree = pm4py.discover_process_tree_inductive(source, **kwargs)
    net = pm4py.discover_petri_net_inductive(source, **kwargs)
    return {
        "tree": str(tree),
        "tree_behaviour": model_behaviour(log, tree),
        "petri_net_behaviour": model_behaviour(log, net),
    }


def _inductive_seeds(path: Path, variant: str, kind: str = "tree") -> dict[str, Any]:
    """Runs ``SEEDED_RUNS[kind]`` once per seed in ``IMF_SEEDS`` and groups
    the runs by tree."""
    golden_tools = str(Path(__file__).resolve().parents[1])
    runs: dict[str, dict[str, Any]] = {}
    for seed in IMF_SEEDS:
        env = dict(os.environ, PYTHONHASHSEED=str(seed))
        env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
        out = subprocess.run(
            [sys.executable, __file__, str(path), variant, kind],
            env=env,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        run = json.loads(out.strip().splitlines()[-1])
        runs.setdefault(run["tree"], {**run, "seeds": []})["seeds"].append(seed)
    return {"runs": [runs[tree] for tree in sorted(runs)]}


def inductive_options(
    fixtures: dict[str, Path],
    parameters: dict[str, Any] | None = None,
    traces: list[str] | None = None,
) -> dict[str, Any]:
    """Runs IM with ``parameters`` on the log in ``fixtures``, or on ``traces``."""
    from pm4py.algo.discovery.inductive import algorithm as inductive_miner

    log = load_log(fixtures["log"]) if traces is None else pm4py.parse_event_log_string(traces, ",")
    tree = inductive_miner.apply(
        log,
        variant=inductive_miner.Variants.IM,
        parameters={"noise_threshold": 0.0, **(parameters or {})},
    )
    return {
        "tree": str(tree),
        "tree_behaviour": model_behaviour(log, tree),
        "petri_net_behaviour": model_behaviour(log, pm4py.convert_to_petri_net(tree)),
    }


def _register_inductive() -> None:
    for variant in INDUCTIVE_VARIANTS:
        for log_id, rel in INDUCTIVE_LOGS.items():
            case(
                f"inductive-{variant}-{log_id}",
                fixture=rel,
                functions=INDUCTIVE_FUNCTIONS,
                params={"variant": variant},
            )(inductive)
    for option, (parameters, log_id) in INDUCTIVE_OPTIONS.items():
        case(
            f"inductive-im-{option}-{log_id}",
            fixture=INDUCTIVE_LOGS[log_id],
            functions=INDUCTIVE_OPTION_FUNCTIONS,
            params={"parameters": parameters},
        )(inductive_options)
    case(
        "inductive-im-fallthroughs-synthetic",
        functions=["pm4py.parse_event_log_string", *INDUCTIVE_OPTION_FUNCTIONS],
        params={"traces": INDUCTIVE_SYNTHETIC_TRACES},
    )(inductive_options)


_register_inductive()


# miners-classic DFG discovery cases. EventLog is the canonical oracle path.
def dfg_mining(fixtures: dict[str, Path], traces=None, activity_key="concept:name",
               timestamp_key="time:timestamp", start_key=None, excluded_dates=None):
    from datetime import datetime, date
    from pm4py.algo.discovery.dfg.variants import native, performance
    from pm4py.statistics.eventually_follows.log import get as eventually
    from pm4py.util import constants
    if traces is None:
        frame = load_log(fixtures["log"])
        log = pm4py.convert_to_event_log(frame, stream_postprocessing=True)
    else:
        from pm4py.objects.log.obj import EventLog, Trace, Event
        log = EventLog([Trace([Event({activity_key: e[0], timestamp_key: datetime.fromisoformat(e[1]),
                                     **({start_key: datetime.fromisoformat(e[2])} if start_key else {})})
                               for e in trace]) for trace in traces])
        frame = None
        # The reference's public performance/EFG wrappers leave the start key
        # at time:timestamp even with a custom completion key, unless configured.
        log.properties["pm4py:param:start_timestamp_key"] = timestamp_key
    params = {"pm4py:param:activity_key": activity_key, "pm4py:param:timestamp_key": timestamp_key}
    def edges(mapping):
        return [[a,b,value] for (a,b), value in sorted(mapping.items())]
    graph, starts, ends = pm4py.discover_dfg(log, activity_key=activity_key, timestamp_key=timestamp_key)
    public_performance, ps, pe = pm4py.discover_performance_dfg(log, activity_key=activity_key, timestamp_key=timestamp_key)
    result = {
        "dfg": {"graph": edges(graph), "start_activities": starts, "end_activities": ends},
        "alias": edges(pm4py.discover_directly_follows_graph(log, activity_key=activity_key, timestamp_key=timestamp_key)[0]),
        "frequency_options": [{"window": w, "keep_once_per_case": once,
                               "graph": edges(native.apply(log, {**params, "window": w, "keep_once_per_case": once}))}
                              for w in [0,1,2,99] for once in [False,True]],
        "minimum_self_distance": pm4py.derive_minimum_self_distance(log, activity_key=activity_key),
        "eventually": edges(pm4py.discover_eventually_follows_graph(log, activity_key=activity_key, timestamp_key=timestamp_key)),
        "performance": {"graph": edges(public_performance), "start_activities": ps, "end_activities": pe},
    }
    typed = pm4py.discover_dfg_typed(frame if frame is not None else log,
                                    activity_key=activity_key, timestamp_key=timestamp_key)
    result["typed"] = {"graph": edges(typed.graph), "start_activities": typed.start_activities,
                       "end_activities": typed.end_activities}
    variants = []
    for interval in ([False, True] if start_key else [False]):
        time_params = {**params, "pm4py:param:start_timestamp_key": start_key if interval else timestamp_key}
        for first in [False, True]:
            variants.append({"interval": interval, "first": first,
                             "graph": edges(eventually.apply(log, {**time_params, "keep_first_following": first}))})
    result["eventually_options"] = variants
    performance_options = []
    class Calendar:
        def is_working_day(self, day):
            return day not in {date.fromisoformat(d) for d in excluded_dates or []}
    for interval in ([False, True] if start_key else [False]):
        for business in [False, True]:
            p = {**params, "pm4py:param:start_timestamp_key": start_key if interval else timestamp_key,
                 "aggregationMeasure": "all", "business_hours": business,
                 "business_hour_slots": constants.DEFAULT_BUSINESS_HOUR_SLOTS,
                 "workcalendar": Calendar()}
            row = {"interval": interval, "business": business, "graph": edges(performance.apply(log, p))}
            if traces is not None:
                row["raw_values"] = edges(performance.apply(log, {**p, "aggregationMeasure": "raw_values"}))
            performance_options.append(row)
    result["performance_options"] = performance_options
    return result

DFG_MINING_FUNCTIONS = [
    "pm4py.discover_dfg", "pm4py.discover_directly_follows_graph", "pm4py.discover_dfg_typed",
    "pm4py.discover_performance_dfg", "pm4py.derive_minimum_self_distance",
    "pm4py.discover_eventually_follows_graph", "pm4py.algo.discovery.dfg.variants.native.apply",
    "pm4py.algo.discovery.dfg.variants.performance.apply", "pm4py.statistics.eventually_follows.log.get.apply",
]
for _dfg_rel in ["running-example.xes", "receipt.xes", "roadtraffic100traces.xes",
                 "interleavings/receipt_even.csv", "interleavings/receipt_odd.csv"]:
    case("dfg-mining-" + _dfg_rel.replace("/", "-").replace(".", "-"), fixture=_dfg_rel,
         functions=DFG_MINING_FUNCTIONS)(dfg_mining)

case("dfg-mining-empty", functions=DFG_MINING_FUNCTIONS, params={"traces": [[], []]})(dfg_mining)
case("dfg-mining-intervals", functions=DFG_MINING_FUNCTIONS, params={
    "activity_key": "act", "timestamp_key": "end", "start_key": "start",
    "excluded_dates": ["2024-01-02"],
    "traces": [[], [["alone", "2024-01-01T08:00:00+00:00", "2024-01-01T08:00:00+00:00"]],
        [["A", "2024-01-01T09:00:00+00:00", "2024-01-01T08:00:00+00:00"],
         ["B", "2024-01-01T10:00:00+00:00", "2024-01-01T08:30:00+00:00"],
         ["A", "2024-01-01T10:00:00.250000+00:00", "2024-01-01T10:00:00+00:00"],
         ["B", "2024-01-02T10:00:00+00:00", "2024-01-02T08:00:00+00:00"],
         ["A", "2024-01-03T08:00:00+00:00", "2024-01-03T08:00:00+00:00"]],
        [["C", "2024-01-01T12:00:00+02:00", "2024-01-01T12:00:00+02:00"],
         ["D", "2024-01-01T08:00:00+00:00", "2024-01-01T08:00:00+00:00"],
         ["C", "2024-01-01T12:00:00+02:00", "2024-01-01T12:00:00+02:00"],
         ["D", "2024-01-01T10:00:00+00:00", "2024-01-01T10:00:00+00:00"],
         ["C", "2024-01-01T10:00:00.000001+00:00", "2024-01-01T10:00:00.000001+00:00"]],
        [["repeat", "2024-01-01T07:00:00+00:00", "2024-01-01T07:00:00+00:00"],
         ["repeat", "2024-01-01T07:00:00+00:00", "2024-01-01T07:00:00+00:00"]]],
})(dfg_mining)


TEMPORAL_PROFILE_LOGS = {
    "running-example": "running-example.csv",
    "receipt": "receipt.csv",
    "roadtraffic100traces": "roadtraffic100traces.csv",
    "interval-event-log": "interval_event_log.csv",
}

TEMPORAL_PROFILE_FUNCTIONS = [
    "pm4py.discover_temporal_profile",
    "pm4py.algo.discovery.temporal_profile.variants.log.apply",
    "pm4py.algo.discovery.temporal_profile.variants.dataframe.apply",
]


def temporal_profile(fixtures: dict[str, Path], start_timestamp_key: str | None = None) -> dict[str, Any]:
    """Temporal profiles of the log in ``fixtures``, per variant and time measure."""
    from pm4py.algo.discovery.temporal_profile.variants import dataframe as dataframe_variant
    from pm4py.algo.discovery.temporal_profile.variants import log as log_variant

    df = load_log(fixtures["log"])
    inputs = {
        "log": (pm4py.convert_to_event_log(df), log_variant),
        "dataframe": (df, dataframe_variant),
    }
    out: dict[str, Any] = {}
    for name, (log, variant) in inputs.items():
        base = {} if start_timestamp_key is None else {variant.Parameters.START_TIMESTAMP_KEY: start_timestamp_key}
        for measure, extra in [("elapsed", {}), ("business", {variant.Parameters.BUSINESS_HOURS: True})]:
            profile = variant.apply(log, parameters={**base, **extra})
            out[f"{name}_{measure}"] = [
                [a, b, float(mean), float(std)] for (a, b), (mean, std) in sorted(profile.items())
            ]
    return out


for _log_id, _rel in TEMPORAL_PROFILE_LOGS.items():
    case(
        f"temporal-profile-{_log_id}",
        fixture=_rel,
        functions=TEMPORAL_PROFILE_FUNCTIONS,
        params={"start_timestamp_key": "start_timestamp"} if _log_id == "interval-event-log" else {},
    )(temporal_profile)


BPMN_INDUCTIVE_FUNCTIONS = [
    "pm4py.discover_bpmn_inductive",
    "pm4py.discover_process_tree_inductive",
]


def bpmn_inductive(fixtures: dict[str, Path], variant: str) -> dict[str, Any]:
    """Runs ``pm4py.discover_bpmn_inductive`` with one inductive variant on
    the log in ``fixtures``."""
    if variant == "imf":
        return _inductive_seeds(fixtures["log"], variant, "bpmn")
    return _bpmn_inductive_run(fixtures["log"], variant)


def _bpmn_inductive_run(path: Path, variant: str) -> dict[str, Any]:
    from cases.bpmn import canonical_bpmn

    kwargs, on_dfg = INDUCTIVE_VARIANTS[variant]
    log = load_log(path)
    source = DFG(*pm4py.discover_dfg(log)) if on_dfg else log
    return {
        "tree": str(pm4py.discover_process_tree_inductive(source, **kwargs)),
        "bpmn": canonical_bpmn(pm4py.discover_bpmn_inductive(source, **kwargs)),
    }


def _register_bpmn_inductive() -> None:
    for variant in INDUCTIVE_VARIANTS:
        for log_id, rel in INDUCTIVE_LOGS.items():
            case(
                f"bpmn-inductive-{variant}-{log_id}",
                fixture=rel,
                functions=BPMN_INDUCTIVE_FUNCTIONS,
                params={"variant": variant},
            )(bpmn_inductive)


_register_bpmn_inductive()

# What one seeded run of _inductive_seeds computes, by kind.
SEEDED_RUNS = {"tree": _inductive_run, "bpmn": _bpmn_inductive_run}


if __name__ == "__main__":
    # One seeded run for _inductive_seeds: prints the result as one JSON line.
    from harness import canonical

    path, variant, kind = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    print(json.dumps(canonical.normalize(SEEDED_RUNS[kind](path, variant))))
