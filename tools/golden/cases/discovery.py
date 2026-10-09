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

Cases ``powl-<variant>-<log>`` run ``pm4py.discover_powl`` with one entry of
``POWL_VARIANTS`` on the ``inductive`` logs, or on a log given as ``traces``
in ``params`` (lists of activities, so traces can be empty). The brute-force
variant runs on the logs where pm4py finishes in seconds. Each case emits
``powl``: :func:`cases.powl.describe_powl` of the model, made independent of
child order by :func:`canonical_powl`.

Footprints (lane ``miner-powl``) emit each footprints dict with sets as
sorted lists, pairs as ``[a, b]`` and the DFG as sorted ``[[a, b], count]``:

- ``footprints-log-<log>``: ``entire`` is ``pm4py.discover_footprints`` of
  the DataFrame (``entire_dataframe``); ``traces`` is the same on
  ``pm4py.convert_to_event_log`` of it (``trace_by_trace``), one entry per
  distinct result with its ``count``, sorted; ``dfg`` is the same on the dict
  of ``pm4py.discover_dfg``; ``powl`` is the same on
  ``pm4py.discover_powl`` of the DataFrame.
- ``footprints-log-synthetic-emptytraces``: ``entire`` (``entire_event_log``,
  through ``footprints.algorithm.apply``) and ``traces`` of a log with empty
  traces, given as ``traces``.
- ``footprints-powl-<model>``: ``powl`` is the footprints of each POWL string
  of ``cases.powl.MODELS``, and ``frequent`` of its
  ``simplify_using_frequent_transitions()``. ``frequent`` is null when
  pm4py makes a frequent transition without an activity from two silent
  steps, which ichnos keeps as they are.

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

# miners-classic alpha and heuristics discovery cases.
def _classic_language(model, depth=3):
    """Exact executable visible prefixes up to depth, with silent closure.

    Unlike unrestricted reachability, this terminates for the visible loops
    of the supplied unsound miners. Never emit a partial language on a cap.
    """
    from collections import deque
    from pm4py.objects.petri_net import semantics
    net, im, fm = model
    places = sorted(net.places, key=lambda p: (p.name, id(p)))
    def key(m):
        return tuple(m.get(p, 0) for p in places)
    todo = deque([(im, ())])
    seen = {(key(im), ())}
    prefixes = {()}
    accepted = set()
    while todo:
        marking, word = todo.popleft()
        if marking == fm:
            accepted.add(word)
        for t in semantics.enabled_transitions(net, marking):
            next_word = word if t.label is None else (*word, t.label)
            if len(next_word) > depth:
                continue
            next_marking = semantics.execute(t, net, marking)
            state = (key(next_marking), next_word)
            prefixes.add(next_word)
            if state not in seen:
                if len(seen) >= 100000:
                    raise RuntimeError("classic miner bounded language exceeded 100000 states")
                seen.add(state)
                todo.append((next_marking, next_word))
    return {"depth": depth, "prefixes": sorted(prefixes), "accepted": sorted(accepted)}


def _classic_footprints(model):
    """Only emit full footprints after a bounded reachability preflight."""
    from collections import deque
    from pm4py.objects.petri_net import semantics
    net, im, fm = model
    places = sorted(net.places, key=lambda p: (p.name, id(p)))
    def key(m):
        return tuple(m.get(p, 0) for p in places)
    todo = deque([im])
    seen = {key(im)}
    while todo:
        m = todo.popleft()
        for t in semantics.enabled_transitions(net, m):
            nxt = semantics.execute(t, net, m)
            state = key(nxt)
            if state not in seen:
                if len(seen) >= 10000:
                    return {"status": "state_space_limit", "value": None}
                seen.add(state)
                todo.append(nxt)
    return {"status": "complete", "value": pm4py.discover_footprints(net, im, fm)}


def _classic_alpha_plus_run(path, activity_key):
    from harness import canonical
    from pm4py.algo.discovery.alpha.variants import plus
    log = pm4py.convert_to_event_log(load_log(Path(path)))
    models = {
        "alpha_plus": pm4py.discover_petri_net_alpha_plus(log, activity_key=activity_key),
        "alpha_plus_remove_unconnected": plus.apply(log, parameters={
            "pm4py:param:activity_key": activity_key, "remove_unconnected": True}),
    }
    return canonical.normalize({name: {"footprints": _classic_footprints(model),
                                       "language": _classic_language(model)}
                                for name, model in models.items()})


def _classic_alpha_plus_seeds(path, activity_key, seeds):
    # Receipt's pair merge changes behaviour with Python set order. Each
    # run is an unmodified public/native miner call in a fresh interpreter.
    groups = {}
    golden_tools = str(Path(__file__).resolve().parents[1])
    worker = ("import json,sys; from cases.discovery import _classic_alpha_plus_run; "
              "print(json.dumps(_classic_alpha_plus_run(sys.argv[1],sys.argv[2])))")
    for seed in seeds:
        env = dict(os.environ, PYTHONHASHSEED=str(seed))
        env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
        result = json.loads(subprocess.run([sys.executable, "-c", worker, str(path), activity_key],
                                          env=env, check=True, capture_output=True, text=True).stdout)
        key = json.dumps(result, sort_keys=True)
        if key not in groups:
            groups[key] = {"seeds": [], "models": result}
        groups[key]["seeds"].append(seed)
    return [groups[key] for key in sorted(groups)]


def classic_miners(fixtures, traces=None, variants=None, activity_key="concept:name", alpha_plus_seeds=None):
    """Public alpha/alpha+ and classic heuristics, with option sweeps."""
    from pm4py.objects.log.obj import EventLog, Trace, Event
    from cases.model import describe_heuristics_net, _matrix
    from pm4py.algo.discovery.alpha.variants import plus
    from pm4py.algo.discovery.heuristics.variants import classic
    if traces is None:
        log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    else:
        log = EventLog([Trace([Event({activity_key: a}) for a in t]) for t in traces])
    kwargs = {"activity_key": activity_key}
    models = {
        "alpha": pm4py.discover_petri_net_alpha(log, **kwargs),
    }
    # The pinned alpha+ implementation indexes a missing boundary on an
    # empty log. Record the exception; Rust deliberately returns a shell.
    try:
        if alpha_plus_seeds is None:
            models["alpha_plus"] = pm4py.discover_petri_net_alpha_plus(log, **kwargs)
            models["alpha_plus_remove_unconnected"] = plus.apply(log, parameters={
                "pm4py:param:activity_key": activity_key, "remove_unconnected": True})
        alpha_plus_error = None
    except IndexError as exc:
        alpha_plus_error = type(exc).__name__
    options = variants or [
        {},
        {"dependency_thresh": 0.8, "and_measure_thresh": 0.5},
        {"dependency_thresh": 0.0, "and_measure_thresh": 1.0,
         "min_act_count": 2, "min_dfg_occurrences": 2,
         "dfg_pre_cleaning_noise_thresh": 0.2, "loop_length_two_thresh": 0.9},
    ]
    nets = []
    for i, parameters in enumerate(options):
        params = {"pm4py:param:activity_key": activity_key, **parameters}
        h = classic.apply_heu(log, parameters=params)
        description = describe_heuristics_net(h)
        description["nodes"].sort(key=lambda n: n["name"])
        for node in description["nodes"]:
            for field in ["inputs", "outputs"]:
                node[field].sort(key=lambda e: (e["target"], e["dependency"], e["frequency"]))
        description["dfg_window_2_matrix"] = _matrix(h.dfg_window_2_matrix)
        description["dfg"] = sorted([a,b,v] for (a,b),v in h.dfg.items())
        models[f"heuristics_{i}"] = pm4py.convert_to_petri_net(h)
        nets.append({"options": parameters, "net": description})
    # Public defaults are covered independently of the native option sweep.
    public_h = pm4py.discover_heuristics_net(log, **kwargs)
    assert public_h.dfg_matrix == classic.apply_heu(log, parameters={"pm4py:param:activity_key": activity_key}).dfg_matrix
    models["heuristics_public"] = pm4py.discover_petri_net_heuristics(log, **kwargs)
    return {
        "traces": traces, "activity_key": activity_key,
        "alpha_plus_error": alpha_plus_error,
        "alpha_plus_runs": (_classic_alpha_plus_seeds(fixtures["log"], activity_key, alpha_plus_seeds)
                            if alpha_plus_seeds is not None else None),
        "heuristics": nets,
        "models": {name: {"footprints": _classic_footprints(model),
                           "language": _classic_language(model)}
                   for name,model in models.items()},
    }


CLASSIC_MINER_FUNCTIONS = [
    "pm4py.discover_petri_net_alpha", "pm4py.discover_petri_net_alpha_plus",
    "pm4py.discover_heuristics_net", "pm4py.discover_petri_net_heuristics",
    "pm4py.algo.discovery.alpha.variants.plus.apply",
    "pm4py.algo.discovery.heuristics.variants.classic.apply_heu",
    "pm4py.convert_to_petri_net", "pm4py.discover_footprints",
    "pm4py.objects.petri_net.semantics.enabled_transitions",
    "pm4py.objects.petri_net.semantics.execute",
]
for _id, _fixture in {
    "running-example-xes": "running-example.xes",
    "receipt-xes": "receipt.xes",
    "roadtraffic100traces-xes": "roadtraffic100traces.xes",
    "interleavings-receipt_even-csv": "interleavings/receipt_even.csv",
    "interleavings-receipt_odd-csv": "interleavings/receipt_odd.csv",
}.items():
    case(f"classic-miners-{_id}", fixture=_fixture, functions=CLASSIC_MINER_FUNCTIONS,
         params={"alpha_plus_seeds": list(range(8))} if _id == "receipt-xes" else {})(classic_miners)

for _id, _traces in {
    "empty": [],
    "empty-trace": [[]],
    "self-loop-only": [["a", "a"], ["a"]],
    "boundaries": [[], ["a"], ["b"], ["a", "b"], ["a", "a"]],
    "loops": [["s", "a", "a", "b", "e"], ["s", "b", "a", "b", "a", "e"],
              ["s", "a", "b", "a", "b", "e"], ["s", "a", "e"]],
    "parallel": [["s", "a", "b", "e"]] * 4 + [["s", "b", "a", "e"]] * 4,
    "custom-key": [["α", "β", "γ"], ["α", "β", "β", "γ"]],
}.items():
    case(f"classic-miners-{_id}", functions=CLASSIC_MINER_FUNCTIONS,
         params={"traces": _traces, "activity_key": "task" if _id == "custom-key" else "concept:name"})(classic_miners)


case("classic-miners-cleaned-loop",functions=CLASSIC_MINER_FUNCTIONS,params={"traces":[["a","c","b"]]*20+[["a","b","a","c","b"]],"variants":[{"min_dfg_occurrences":0}]})(classic_miners)


# miners-classic log-skeleton and DECLARE discovery cases.
SKELETON_DECLARE_FUNCTIONS = [
    "pm4py.discover_log_skeleton", "pm4py.discover_declare",
    "pm4py.algo.discovery.declare.variants.classic.apply",
]


def skeleton_declare(fixtures, traces=None, activity_key="concept:name", declare_options=None):
    """Lossless label-index encoding of public skeleton/DECLARE outputs."""
    from pm4py.objects.log.obj import EventLog, Trace, Event
    from pm4py.algo.discovery.declare.variants import classic as declare_classic
    if traces is None:
        log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    else:
        log = EventLog([Trace([Event({activity_key: a}) for a in t]) for t in traces])
    options = declare_options or [
        {},
        {"min_support_ratio": 0.75, "min_confidence_ratio": 0.95},
        {"min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
    ]
    labels = sorted({e[activity_key] for t in log for e in t} |
                    {a for opts in options for a in opts.get("considered_activities", [])})
    index = {a: i for i, a in enumerate(labels)}
    skeletons = []
    for noise in [0.0, 0.2, 0.5, 0.9, 1.0]:
        model = pm4py.discover_log_skeleton(log, noise_threshold=noise, activity_key=activity_key)
        encoded = {name: sorted([index[a], index[b]] for a,b in value)
                   for name,value in model.items() if name != "activ_freq"}
        encoded["activ_freq"] = sorted([index[a], sorted(freqs)] for a,freqs in model["activ_freq"].items())
        skeletons.append({"noise": noise, "model": encoded})
    models = []
    for opts in options:
        kwargs = dict(opts)
        for key in ["allowed_templates", "considered_activities"]:
            if key in kwargs:
                kwargs[key] = set(kwargs[key])
        if "auto_selection_multiplier" in kwargs:
            model = declare_classic.apply(log, parameters={"pm4py:param:activity_key": activity_key, **kwargs})
        else:
            model = pm4py.discover_declare(log, activity_key=activity_key, **kwargs)
        encoded = {template: sorted([
            [*[index[a] for a in (args if isinstance(args, tuple) else (args,))],
             values["support"], values["confidence"]]
            for args,values in rules.items()]) for template,rules in model.items()}
        models.append({"options": opts, "model": encoded})
    return {"labels": labels, "skeleton": skeletons, "declare": models}


for _id, _fixture in {
    "running-example-xes": "running-example.xes",
    "receipt-xes": "receipt.xes",
    "roadtraffic100traces-xes": "roadtraffic100traces.xes",
    "interleavings-receipt_even-csv": "interleavings/receipt_even.csv",
    "interleavings-receipt_odd-csv": "interleavings/receipt_odd.csv",
}.items():
    case(f"skeleton-declare-{_id}", fixture=_fixture, functions=SKELETON_DECLARE_FUNCTIONS)(skeleton_declare)

SKELETON_DECLARE_SYNTHETIC_OPTIONS = [
    {},
    {"min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
    {"min_support_ratio": 1.0, "min_confidence_ratio": 1.0},
    {"min_support_ratio": 0.5},
    {"min_confidence_ratio": 0.5},
    {"auto_selection_multiplier": 0.0},
    {"auto_selection_multiplier": 1.0},
    {"considered_activities": ["a", "b", "ghost"], "min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
    {"considered_activities": []},
    {"allowed_templates": []},
    {"allowed_templates": ["absence", "succession", "coexistence", "noncoexistence", "altsuccession", "chainsuccession", "nonsuccession", "nonchainsuccession"], "min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
    {"allowed_templates": ["response", "precedence", "succession", "nonsuccession"], "min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
    {"allowed_templates": ["altresponse", "altprecedence", "altsuccession", "chainresponse", "chainprecedence", "chainsuccession", "nonchainsuccession"], "min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
    {"allowed_templates": ["responded_existence", "coexistence", "noncoexistence", "existence", "absence"], "min_support_ratio": 0.0, "min_confidence_ratio": 0.0},
]

for _id, _traces in {
    "empty": [],
    "empty-traces": [[], []],
    "repeated": [["a", "b", "a", "b"], ["a", "a", "b"], ["b", "a", "b", "b"], ["a"], ["b"], []],
    "weighted": [["a", "b"]] * 4 + [["a"]] * 2 + [["b"]] * 3 + [[]],
    "projection": [["a", "x", "b", "a", "b"], ["x", "b", "x", "a"], ["x"], []],
    "frequency-ties": [["a"], []],
    "frequency-ties-reversed": [[], ["a"]],
    "custom-key": [["α", "β", "α", "β"], ["β", "α"], []],
}.items():
    case(f"skeleton-declare-{_id}", functions=SKELETON_DECLARE_FUNCTIONS,
         params={"traces": _traces, "activity_key": "task" if _id == "custom-key" else "concept:name",
                 "declare_options": SKELETON_DECLARE_SYNTHETIC_OPTIONS})(skeleton_declare)

# miners-classic batch and correlation discovery cases.
def batches_correlation(fixtures, traces=None, activity_key="concept:name", interval=False, fill_missing_resource=None, oracle_solver="default"):
    from datetime import datetime
    from pm4py.objects.log.obj import EventLog, Trace, Event
    from pm4py.algo.discovery.correlation_mining.variants import classic
    from pm4py.algo.discovery.correlation_mining import util as cm_util
    from pm4py.algo.discovery.batches.variants import log as batch_log
    from pm4py.util.lp import solver
    if traces is None:
        log = pm4py.convert_to_event_log(load_log(fixtures["log"]), stream_postprocessing=True)
        # Resource-less fixtures exercise an explicit single-resource projection.
        for trace in log:
            for event in trace:
                if "org:resource" not in event:
                    event["org:resource"] = "unassigned"
    else:
        log = EventLog([Trace([Event({activity_key: row[0], "org:resource": row[1],
            "time:timestamp": datetime.fromisoformat(row[3]), "start_timestamp": datetime.fromisoformat(row[2])})
            for row in events], attributes={"concept:name": case_id}) for case_id, events in traces])
    params = {"pm4py:param:activity_key": activity_key, "pm4py:param:timestamp_key": "time:timestamp",
              "pm4py:param:start_timestamp_key": "start_timestamp" if interval else "time:timestamp"}
    def edges(mapping):
        return [[a, b, value] for (a,b), value in sorted(mapping.items())]
    def normalize_batches(groups):
        return [{"activity": pair[0], "resource": pair[1], "count": count,
                 "batches": {kind: sorted([[start,end,sorted([list(ev) for ev in events])] for start,end,events in batches])
                             for kind,batches in kinds.items()}} for pair,count,kinds in groups]
    def compact_batches(model):
        import copy
        import hashlib
        micro = copy.deepcopy(model)
        samples = []
        for group in micro:
            for kind,batches in group["batches"].items():
                for batch in batches:
                    batch[0],batch[1] = round(batch[0]*1e6),round(batch[1]*1e6)
                    for event in batch[2]:
                        event[0],event[1] = round(event[0]*1e6),round(event[1]*1e6)
                        samples.append([group["activity"],group["resource"],kind,batch[0],batch[1],*event])
        encoded = json.dumps(micro,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()
        return {"format":"batch-identities-microseconds-sha256","groups":len(micro),
                "batches":sum(group["count"] for group in micro),"events":len(samples),
                "sha256":hashlib.sha256(encoded).hexdigest(),"samples":sorted(samples)[:3]}
    batch_runs = []
    for distance,size in [(900,2),(0,2),(60,3),(0,1)]:
        options = {**params,"merge_distance":distance,"min_batch_size":size}
        actual = batch_log.apply(log,parameters=options)
        if not interval:
            public = pm4py.discover_batches(log, merge_distance=distance, min_batch_size=size, activity_key=activity_key)
            assert normalize_batches(public) == normalize_batches(actual)
        batch_runs.append({"distance":distance,"size":size,"model":compact_batches(normalize_batches(actual)) if sum(map(len,log))>1000 else normalize_batches(actual)})
    correlation_runs = []
    if sum(map(len,log)):
        frame = pm4py.convert_to_dataframe(log)
        for exact in ([False, True] if traces is not None or "running-example" in str(fixtures.get("log","")) else [False]):
            options = {**params,"exact_time_matching":exact}
            _,grouped,activities = classic.preprocess_log(log,parameters=options)
            ps,duration = classic.get_PS_dur_matrix(grouped,activities,parameters=options)
            counts = {a:len(events) for a,events in grouped.items()}
            cost = cm_util.get_c_matrix(ps,duration,activities,counts)
            default_freq,default_perf = classic.apply(log,parameters=options)
            freq,perf = default_freq,default_perf
            if oracle_solver == "highs":
                from unittest.mock import patch
                from pm4py.util.lp.variants import scipy_solver
                original_apply = scipy_solver.apply
                def highs_apply(*args, **kwargs):
                    return original_apply(*args, parameters={"method":"highs"})
                with patch.object(solver,"DEFAULT_LP_SOLVER_VARIANT",solver.SCIPY), patch.dict(solver.VERSIONS_APPLY,{solver.SCIPY:highs_apply}):
                    freq,perf = classic.apply(log,parameters=options)
            uniform_cost = bool((cost == cost[0,0]).all())
            selected_freq = {(a,a):counts[a] for a in activities} if uniform_cost else freq
            selected_perf = {(a,b):float(duration[activities.index(a),activities.index(b)]) for a,b in selected_freq}
            if not exact and not interval:
                public,starts,ends = pm4py.correlation_miner(frame,activity_key=activity_key)
                assert public == default_freq
                public_perf,psa,pea = pm4py.correlation_miner(frame,annotation="performance",activity_key=activity_key)
                assert public_perf == default_perf and psa == starts and pea == ends
            else:
                input_labels = [ev[activity_key] for trace in log for ev in trace]
                first,last = input_labels[0],input_labels[-1]
                starts,ends = {first:counts[first]}, {last:counts[last]}
            stats = [[a,b,float(ps[i,j]),float(duration[i,j]),float(cost[i,j])] for i,a in enumerate(activities) for j,b in enumerate(activities)]
            objective = sum(count * cost[activities.index(a),activities.index(b)] for (a,b),count in freq.items())
            correlation_runs.append({"exact":exact,"frequency":edges(selected_freq),"performance":edges(selected_perf),"uniform_cost":uniform_cost,
                                     "default_frequency":edges(default_freq),"default_performance":edges(default_perf),"native_frequency":edges(freq),"native_performance":edges(perf),"starts":starts,"ends":ends,
                                     "counts":counts,"statistics":stats,"objective":float(objective),"solver": "scipy/highs" if oracle_solver == "highs" else solver.DEFAULT_LP_SOLVER_VARIANT})
    return {"batches":batch_runs,"correlation":correlation_runs}

BATCHES_CORRELATION_FUNCTIONS = ["pm4py.discover_batches", "pm4py.correlation_miner",
                   "pm4py.algo.discovery.batches.variants.log.apply",
                   "pm4py.algo.discovery.correlation_mining.variants.classic.apply"]
for _fixture in ["running-example.xes","receipt.xes","roadtraffic100traces.xes", "interleavings/receipt_even.csv","interleavings/receipt_odd.csv"]:
    case("batches-correlation-"+_fixture.replace("/","-").replace(".","-"),
         functions=BATCHES_CORRELATION_FUNCTIONS,fixtures={"log":_fixture},params={"fill_missing_resource":"unassigned","oracle_solver":"highs" if _fixture=="receipt.xes" else "default"})(batches_correlation)

def _batches_correlation_rows(rows):
    from datetime import datetime, timedelta, timezone
    epoch=datetime(2024,1,1,tzinfo=timezone.utc)
    return [[case_id, [[activity,resource,(epoch+timedelta(seconds=start)).isoformat(),(epoch+timedelta(seconds=end)).isoformat()]
                      for activity,resource,start,end in events]] for case_id,events in rows]

BATCHES_CORRELATION_SYNTHETIC = {
    "equal-endpoints": ([[f"case-{i % 32:02}", [("a", "r", 0, 10)]] for i in range(40)], True),
    "microseconds": ([["c1", [("a", "r", 0.000001, 0.000001)]], ["c2", [("a", "r", 1.000001, 1.000001)]]], False),
    "empty": ([],False),
    "empty-traces": ([["empty",[]]],False),
    "five-types": ([["c1",[("sim","r",0,10),("start","r",0,10),("end","r",0,20),("seq","r",0,10),("conc","r",0,10)]],
                    ["c2",[("sim","r",0,10),("start","r",0,20),("end","r",10,20),("seq","r",10,20),("conc","r",5,15)]]],True),
    "duplicates": ([["same",[("a","r",0,0),("a","r",0,0),("b","r",1,1)]],["same",[("a","r",0,0)]],["other",[("a","r",0,0),("b","r",2,2)]]],False),
    "heap-order": ([["c",[("a","r",s,s+1) for s in [90,0,60,30,120,15,45,75,105]]]],True),
    "intervals": ([["c1",[("a","r",0,20),("b","r",5,10),("a","r",10,11),("c","s",30,40)]],
                   ["c2",[("b","r",20,21),("c","s",21,22),("a","r",25,24)]]],True),
    "custom-key": ([["c1",[("α","r",0,0),("β","s",10,10)]],["c2",[("α","r",1,1),("β","s",12,12)]]],False),
}
for _name,(_rows,_interval) in BATCHES_CORRELATION_SYNTHETIC.items():
    _params={"traces":_batches_correlation_rows(_rows),"interval":_interval,"activity_key":"task" if _name=="custom-key" else "concept:name"}
    case("batches-correlation-"+_name,functions=BATCHES_CORRELATION_FUNCTIONS,params=_params)(batches_correlation)
# miners-classic ILP region discovery cases.
def _ilp_language(model, depth=3):
    """Exact executable visible prefixes up to depth, with silent closure.

    Unlike unrestricted reachability, this terminates for the visible loops
    of the supplied unsound miners. Never emit a partial language on a cap.
    """
    from collections import deque
    from pm4py.objects.petri_net import semantics
    net, im, fm = model
    places = sorted(net.places, key=lambda p: (p.name, id(p)))
    def key(m):
        return tuple(m.get(p, 0) for p in places)
    todo = deque([(im, ())])
    seen = {(key(im), ())}
    prefixes = {()}
    accepted = set()
    while todo:
        marking, word = todo.popleft()
        if marking == fm:
            accepted.add(word)
        for t in semantics.enabled_transitions(net, marking):
            next_word = word if t.label is None else (*word, t.label)
            if len(next_word) > depth:
                continue
            next_marking = semantics.execute(t, net, marking)
            state = (key(next_marking), next_word)
            prefixes.add(next_word)
            if state not in seen:
                if len(seen) >= 100000:
                    raise RuntimeError("classic miner bounded language exceeded 100000 states")
                seen.add(state)
                todo.append((next_marking, next_word))
    return {"depth": depth, "prefixes": sorted(prefixes), "accepted": sorted(accepted)}


def _ilp_footprints(model):
    """Only emit full footprints after a bounded reachability preflight."""
    from collections import deque
    from pm4py.objects.petri_net import semantics
    net, im, fm = model
    places = sorted(net.places, key=lambda p: (p.name, id(p)))
    def key(m):
        return tuple(m.get(p, 0) for p in places)
    todo = deque([im])
    seen = {key(im)}
    while todo:
        m = todo.popleft()
        for t in semantics.enabled_transitions(net, m):
            nxt = semantics.execute(t, net, m)
            state = key(nxt)
            if state not in seen:
                if len(seen) >= 10000:
                    return {"status": "state_space_limit", "value": None}
                seen.add(state)
                todo.append(nxt)
    return {"status": "complete", "value": pm4py.discover_footprints(net, im, fm)}


def ilp_miner(fixtures, traces=None, activity_key="concept:name", alphas=None, causal=None):
    from pm4py.objects.log.obj import EventLog, Trace, Event
    from pm4py.algo.discovery.ilp.variants import classic
    from pm4py.util.lp.variants import scipy_solver
    from pm4py.util.lp import solver
    from unittest.mock import patch
    if traces is None:
        log=pm4py.convert_to_event_log(load_log(fixtures["log"]),stream_postprocessing=True)
    else:
        log=EventLog([Trace([Event({activity_key:a}) for a in trace]) for trace in traces])
    original=scipy_solver.apply
    def integer_highs(c,Aub,bub,Aeq,beq,parameters=None):
        import numpy as np
        options={**(parameters or {}),"method":"highs"}
        solution=original(c,Aub,bub,Aeq,beq,parameters=options)
        # Only region problems end with m<=0; implicit-place reduction uses
        # an unbounded integer feasibility problem and needs no tie policy.
        if not solution.success or np.asarray(Aub)[-1,-1] != 1:
            return solution
        eq=np.asarray(Aeq).reshape((-1,len(c))).tolist()+[list(c)]
        rhs=np.asarray(beq).ravel().tolist()+[round(solution.fun)]
        for i in range(len(c)):
            row=[0]*len(c);row[i]=1
            if round(solution.x[i]):
                candidate=original(c,Aub,bub,eq+[row],rhs+[0],parameters=options)
                if candidate.success:
                    solution=candidate
                    value=0
                else:
                    value=1
            else:
                value=0
            eq.append(row);rhs.append(value)
        return solution
    result=[]
    for alpha in (alphas or [1.0,0.2,0.0]):
        params={"pm4py:param:activity_key":activity_key,"show_progress_bar":False,"alpha":alpha}
        if causal is not None: params["causal_relation"]={tuple(pair):1 for pair in causal}
        with patch.dict(solver.VERSIONS_APPLY,{solver.SCIPY:integer_highs}):
            try:
                model=pm4py.discover_petri_net_ilp(log,alpha=alpha,activity_key=activity_key) if causal is None else classic.apply(log,parameters=params)
            except Exception as error:
                if traces != []: raise
                result.append({"alpha":alpha,"error":type(error).__name__})
                continue
        result.append({"alpha":alpha,"model":{"language":_ilp_language(model),"footprints":_ilp_footprints(model)}})
    return {"runs":result,"solver":"scipy/highs/integer/lexicographic"}

ILP_FUNCTIONS=["pm4py.discover_petri_net_ilp","pm4py.algo.discovery.ilp.variants.classic.apply","pm4py.util.lp.variants.scipy_solver.apply"]
for fixture in ["running-example.xes","receipt.xes","roadtraffic100traces.xes","interleavings/receipt_even.csv","interleavings/receipt_odd.csv"]:
    case("ilp-miner-"+fixture.replace("/","-").replace(".","-"),fixture=fixture,functions=ILP_FUNCTIONS)(ilp_miner)
for name,traces in {
 "empty":[],"empty-traces":[[],[]],"sequence":[["a","b","c"],["a","b","c"]],
 "parallel":[["a","b","c","d"],["a","c","b","d"]],
 "loops":[["a","b","a","b","c"],["a","c"],["a","a","c"]],
 "weighted":[["a","b","d"]]*8+[["a","c","d"],["a","b","c","d"]],
 "custom-key":[["α","β"],["α","γ","β"],[]]
}.items():
    case("ilp-miner-"+name,functions=ILP_FUNCTIONS,params={"traces":traces,"activity_key":"task" if name=="custom-key" else "concept:name"})(ilp_miner)
case("ilp-miner-causal",functions=ILP_FUNCTIONS,params={"traces":[["a","b"]],"causal":[["▶","a"],["a","b"],["b","■"]]})(ilp_miner)

# Genetic fixed-matrix, public discovery and controlled pm4py crossover cases.
def _genetic_language(model, depth=3):
    """Exact executable visible prefixes up to depth, with silent closure.

    Unlike unrestricted reachability, this terminates for the visible loops
    of the supplied unsound miners. Never emit a partial language on a cap.
    """
    from collections import deque
    from pm4py.objects.petri_net import semantics

    net, im, fm = model
    places = sorted(net.places, key=lambda p: (p.name, id(p)))

    def key(m):
        return tuple((m.get(p, 0) for p in places))

    todo = deque([(im, ())])
    seen = {(key(im), ())}
    prefixes = {()}
    accepted = set()
    while todo:
        marking, word = todo.popleft()
        if marking == fm:
            accepted.add(word)
        for t in semantics.enabled_transitions(net, marking):
            next_word = word if t.label is None else (*word, t.label)
            if len(next_word) > depth:
                continue
            next_marking = semantics.execute(t, net, marking)
            state = (key(next_marking), next_word)
            prefixes.add(next_word)
            if state not in seen:
                if len(seen) >= 100000:
                    raise RuntimeError("classic miner bounded language exceeded 100000 states")
                seen.add(state)
                todo.append((next_marking, next_word))
    return {"depth": depth, "prefixes": sorted(prefixes), "accepted": sorted(accepted)}


def _genetic_footprints(model):
    """Only emit full footprints after a bounded reachability preflight."""
    from collections import deque
    from pm4py.objects.petri_net import semantics

    net, im, fm = model
    places = sorted(net.places, key=lambda p: (p.name, id(p)))

    def key(m):
        return tuple((m.get(p, 0) for p in places))

    todo = deque([im])
    seen = {key(im)}
    while todo:
        m = todo.popleft()
        for t in semantics.enabled_transitions(net, m):
            nxt = semantics.execute(t, net, m)
            state = key(nxt)
            if state not in seen:
                if len(seen) >= 10000:
                    return {"status": "state_space_limit", "value": None}
                seen.add(state)
                todo.append(nxt)
    return {"status": "complete", "value": pm4py.discover_footprints(net, im, fm)}


def _genetic_named_transitions(model, labels):
    # Native conversion gives every silent transition the empty name.
    # pm4py's eventually-enabled traversal keys visited nodes by repr, causing
    # collisions. Normalize names to the Rust causal-matrix IDs before comparing
    # these name-sensitive diagnostics; labels, arcs and markings are unchanged.
    net = model[0]
    for transition in net.transitions:
        if transition.label is not None:
            transition.name = "t" + str(labels.index(transition.label))
        else:
            producers = {
                arc.source.label for edge in transition.in_arcs for arc in edge.source.in_arcs
            }
            consumers = {
                arc.target.label for edge in transition.out_arcs for arc in edge.target.out_arcs
            }
            assert len(producers) == len(consumers) == 1
            transition.name = (
                "tau-"
                + str(labels.index(next(iter(producers))))
                + "-"
                + str(labels.index(next(iter(consumers))))
            )


def genetic_matrix_case(
    fixtures, traces=None, activity_key="concept:name", matrix=None, depth=3, assert_silent=False
):
    from datetime import datetime, timedelta, timezone
    from collections import defaultdict
    from pm4py.objects.log.obj import EventLog, Trace, Event
    from pm4py.objects.genetic_matrix.obj import GeneticMatrix
    from pm4py.objects.conversion.genetic_matrix.variants.to_petri_net import apply

    if traces is None:
        log = pm4py.convert_to_event_log(load_log(fixtures["log"]), stream_postprocessing=True)
    else:
        log = EventLog(
            [
                Trace(
                    [
                        Event(
                            {
                                activity_key: a,
                                "time:timestamp": datetime(2020, 1, 1, tzinfo=timezone.utc)
                                + timedelta(seconds=i),
                            }
                        )
                        for i, a in enumerate(trace)
                    ],
                    attributes={"concept:name": str(ti)},
                )
                for ti, trace in enumerate(traces)
            ]
        )
    if matrix is None:
        labels = list(dict.fromkeys((event[activity_key] for trace in log for event in trace)))
        edges = {
            (a[activity_key], b[activity_key]) for trace in log for a, b in zip(trace, trace[1:])
        }
        inputs = {t: [[a] for a, b in sorted(edges) if b == t] for t in labels}
        outputs = {t: [[b] for a, b in sorted(edges) if a == t] for t in labels}
    else:
        labels = matrix["activities"]
        inputs = matrix["inputs"]
        outputs = matrix["outputs"]
    I = defaultdict(list, {t: [frozenset(s) for s in inputs.get(t, [])] for t in labels})
    O = defaultdict(list, {t: [frozenset(s) for s in outputs.get(t, [])] for t in labels})
    model = apply(GeneticMatrix(I, O, labels))
    if assert_silent:
        _genetic_named_transitions(model, labels)
    metrics = pm4py.fitness_token_based_replay(log, *model, activity_key=activity_key)
    result = {
        "matrix": {"activities": labels, "inputs": inputs, "outputs": outputs},
        "model": {
            "language": _genetic_language(model, depth),
            "footprints": _genetic_footprints(model),
        },
        "fitness": 0.4 * metrics["average_trace_fitness"]
        + 0.6 * metrics["percentage_of_fitting_traces"] / 100,
    }
    if assert_silent:
        result["model"]["silent_transitions"] = sum((t.label is None for t in model[0].transitions))
    return result


def genetic_public_case(fixtures, traces, activity_key="concept:name"):
    import random
    from datetime import datetime, timedelta, timezone
    from pm4py.objects.log.obj import EventLog, Trace, Event

    log = EventLog(
        [
            Trace(
                [
                    Event(
                        {
                            activity_key: a,
                            "time:timestamp": datetime(2020, 1, 1, tzinfo=timezone.utc)
                            + timedelta(seconds=i),
                        }
                    )
                    for i, a in enumerate(trace)
                ],
                attributes={"concept:name": str(ti)},
            )
            for ti, trace in enumerate(traces)
        ]
    )
    random.seed(0)
    model = pm4py.discover_petri_net_genetic(
        log, population_size=4, generations=1, activity_key=activity_key
    )
    metrics = pm4py.fitness_token_based_replay(log, *model, activity_key=activity_key)
    return {
        "model": {"language": _genetic_language(model), "footprints": _genetic_footprints(model)},
        "fitness": 0.4 * metrics["average_trace_fitness"]
        + 0.6 * metrics["percentage_of_fitting_traces"] / 100,
    }


_GENETIC_FUNCTIONS = [
    "pm4py.objects.conversion.genetic_matrix.variants.to_petri_net.apply",
    "pm4py.fitness_token_based_replay",
]
_GENETIC_PUBLIC_FUNCTIONS = ["pm4py.discover_petri_net_genetic", "pm4py.fitness_token_based_replay"]
for _fixture in [
    "running-example.xes",
    "receipt.xes",
    "roadtraffic100traces.xes",
    "interleavings/receipt_even.csv",
    "interleavings/receipt_odd.csv",
]:
    case(
        "genetic-matrix-" + _fixture.replace("/", "-").replace(".", "-"),
        fixture=_fixture,
        functions=_GENETIC_FUNCTIONS,
    )(genetic_matrix_case)
for _name, _traces in {
    "sequence": [["a", "b", "c"]],
    "parallel": [["a", "b", "c", "d"], ["a", "c", "b", "d"]],
    "loop": [["a", "b", "a"]],
    "silent": [["a", "b", "d"], ["a", "c", "d"]],
}.items():
    case("genetic-matrix-" + _name, functions=_GENETIC_FUNCTIONS, params={"traces": _traces})(
        genetic_matrix_case
    )
case(
    "genetic-matrix-custom-key",
    functions=_GENETIC_FUNCTIONS,
    params={"traces": [["λ", "", "終"]], "activity_key": "work"},
)(genetic_matrix_case)
case(
    "genetic-matrix-grouped",
    functions=_GENETIC_FUNCTIONS,
    params={
        "traces": [["a", "b", "c", "d"], ["a", "c", "b", "d"]],
        "matrix": {
            "activities": ["a", "b", "c", "d"],
            "inputs": {"a": [], "b": [["a"]], "c": [["a"]], "d": [["b", "c"]]},
            "outputs": {"a": [["b", "c"]], "b": [["d"]], "c": [["d"]], "d": []},
        },
    },
)(genetic_matrix_case)
for _name, _traces in {
    "sequence": [["a", "b", "c"], ["a", "b", "c"]],
    "single": [["a"]],
    "loop": [["a", "b", "a", "b"]],
}.items():
    case(
        "genetic-public-" + _name, functions=_GENETIC_PUBLIC_FUNCTIONS, params={"traces": _traces}
    )(genetic_public_case)


def genetic_operators_case(fixtures):
    from unittest.mock import patch
    from collections import defaultdict
    from pm4py.algo.discovery.genetic.variants import classic

    labels = ["a", "b", "c", "d"]

    def individual(edges, grouped=False):
        I = defaultdict(list, {t: [] for t in labels})
        O = defaultdict(list, {t: [] for t in labels})
        for a, b in edges:
            I[b].append(frozenset([a]))
            O[a].append(frozenset([b]))
        if grouped:
            I = defaultdict(list, {t: [frozenset().union(*v)] if v else [] for t, v in I.items()})
            O = defaultdict(list, {t: [frozenset().union(*v)] if v else [] for t, v in O.items()})
        return (I, O)

    def serial(ind):
        return {
            "inputs": [
                [sorted((labels.index(t) for t in s)) for s in ind[0][label]] for label in labels
            ],
            "outputs": [
                [sorted((labels.index(t) for t in s)) for s in ind[1][label]] for label in labels
            ],
        }

    a = individual([("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")], True)
    b = individual([("a", "b"), ("b", "c"), ("c", "d")])
    crosses = []
    for t in labels:
        with patch.object(classic.random, "choice", return_value=t), patch.object(
            classic.random, "randrange", return_value=0
        ):
            crosses.append([serial(child) for child in classic.crossover(a, b, labels)])
    return {"parents": [serial(a), serial(b)], "crossovers": crosses}


case("genetic-operators", functions=["pm4py.algo.discovery.genetic.variants.classic.crossover"])(
    genetic_operators_case
)
_non_simple = {
    "activities": ["a", "x", "b", "c"],
    "inputs": {"b": [["a"]], "c": [["a", "x"]]},
    "outputs": {"a": [["b", "c"]], "x": [["c"]]},
}
_non_simple_exit = {
    "activities": ["a", "x", "b", "c", "d"],
    "inputs": {"b": [["a"]], "c": [["a", "x"]], "d": [["b"], ["c"]]},
    "outputs": {"a": [["b", "c"]], "x": [["c"]], "b": [["d"]], "c": [["d"]]},
}
_overlapping_simple = {
    "activities": ["a", "x", "b", "c", "d"],
    "inputs": {"b": [["a"]], "c": [["a", "x"]], "d": [["a"]]},
    "outputs": {"a": [["b", "c"], ["d"]], "x": [["c"]]},
}
for _name, _matrix, _traces in [
    ("non-simple", _non_simple, [["a", "b"], ["a", "c"], ["x", "c"]]),
    ("non-simple-exit", _non_simple_exit, [["a", "b", "d"], ["a", "c", "d"], ["x", "c", "d"]]),
    ("overlapping-simple", _overlapping_simple, [["a", "b"], ["a", "d"], ["x", "c"]]),
]:
    case(
        "genetic-matrix-" + _name,
        functions=_GENETIC_FUNCTIONS,
        params={"matrix": _matrix, "traces": _traces, "depth": 4, "assert_silent": True},
    )(genetic_matrix_case)


def genetic_operators_prefix_case(fixtures):
    from collections import defaultdict
    from itertools import product
    from unittest.mock import patch
    from pm4py.algo.discovery.genetic.variants import classic

    labels = ["u", "v", "w", "t", "x", "y", "z"]

    def parent(grouped):
        inputs = defaultdict(list, {a: [] for a in labels})
        outputs = defaultdict(list, {a: [] for a in labels})
        for a in ["u", "v", "w"]:
            outputs[a] = [frozenset(["t"])]
        for a in ["x", "y", "z"]:
            inputs[a] = [frozenset(["t"])]
        inputs["t"] = (
            [frozenset(["v", "w"]), frozenset(["u"])]
            if grouped
            else [frozenset([a]) for a in ["u", "v", "w"]]
        )
        outputs["t"] = (
            [frozenset(["y", "z"]), frozenset(["x"])]
            if grouped
            else [frozenset([a]) for a in ["x", "y", "z"]]
        )
        return (inputs, outputs)

    def serial(ind):
        return {
            "inputs": [
                [sorted((labels.index(t) for t in binding)) for binding in ind[0][a]]
                for a in labels
            ],
            "outputs": [
                [sorted((labels.index(t) for t in binding)) for binding in ind[1][a]]
                for a in labels
            ],
        }

    a, b = (parent(False), parent(True))
    crossovers = []
    choices = []
    for t in labels:
        input_range = range(min(len(a[0][t]), len(b[0][t]))) if a[0][t] and b[0][t] else [None]
        output_range = range(min(len(a[1][t]), len(b[1][t]))) if a[1][t] and b[1][t] else [None]
        for ip, op in product(input_range, output_range):
            points = [point for point in [ip, op] if point is not None]
            with patch.object(classic.random, "choice", return_value=t), patch.object(
                classic.random, "randrange", side_effect=points
            ):
                crossovers.append([serial(child) for child in classic.crossover(a, b, labels)])
            choices.append({"activity": t, "input_point": ip, "output_point": op})
    return {"parents": [serial(a), serial(b)], "choices": choices, "crossovers": crossovers}


case(
    "genetic-operators-prefix",
    functions=["pm4py.algo.discovery.genetic.variants.classic.crossover"],
)(genetic_operators_prefix_case)


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


# Variant name -> (POWLDiscoveryVariant member, discover_powl keyword arguments).
POWL_VARIANTS = {
    "tree": ("TREE", {}),
    "maximal": ("MAXIMAL", {}),
    "bruteforce": ("BRUTE_FORCE", {}),
    "dynamic": ("DYNAMIC_CLUSTERING", {}),
    "dynamic-ratio08": ("DYNAMIC_CLUSTERING", {"order_graph_filtering_threshold": 0.8}),
    "maximal-filter03": ("MAXIMAL", {"filtering_weight_factor": 0.3}),
    "tree-filter03": ("TREE", {"filtering_weight_factor": 0.3}),
}

# Logs the brute-force variant runs on; pm4py takes too long on the others.
POWL_BRUTE_FORCE_LOGS = [
    "running-example-csv",
    "roadtraffic100traces-csv",
    "interleavings-receipt_odd-csv",
    "reviewing-csv",
]

# Small logs, by name, for the cases the six logs miss.
POWL_SYNTHETIC_LOGS = {
    "fallthroughs": [list(t.split(",")) for t in INDUCTIVE_SYNTHETIC_TRACES],
    "emptytraces": [[], ["a", "b"], ["b", "a"], ["a"], []],
    "allempty": [[], []],
}


def powl(
    fixtures: dict[str, Path], variant: str, traces: list[list[str]] | None = None
) -> dict[str, Any]:
    """Runs ``pm4py.discover_powl`` with one variant on the log in
    ``fixtures``, or on ``traces``."""
    from pm4py.algo.discovery.powl.inductive.variants.powl_discovery_varaints import (
        POWLDiscoveryVariant,
    )
    from pm4py.objects.log.obj import Event, EventLog, Trace

    from cases.powl import describe_powl

    if traces is None:
        log = load_log(fixtures["log"])
    else:
        log = EventLog()
        for i, activities in enumerate(traces):
            trace = Trace(attributes={"concept:name": str(i)})
            for a in activities:
                trace.append(Event({"concept:name": a}))
            log.append(trace)
    member, kwargs = POWL_VARIANTS[variant]
    model = pm4py.discover_powl(log, variant=POWLDiscoveryVariant[member], **kwargs)
    return {"powl": canonical_powl(describe_powl(model))}


def canonical_powl(d: dict[str, Any]) -> dict[str, Any]:
    """A described POWL model with XOR children sorted, every order closed
    under transitivity, and partial-order children sorted by their own form
    and the forms of their predecessors and successors."""
    if "children" not in d:
        return d
    children = [canonical_powl(c) for c in d["children"]]
    key = lambda c: json.dumps(c, sort_keys=True)  # noqa: E731
    if d["kind"] == "loop":
        return {**d, "children": children}
    if d["kind"] == "xor":
        return {**d, "children": sorted(children, key=key)}
    n = len(children)
    order = {(i, j) for i, j in d["order"]}
    while True:
        more = {(i, k) for i, j in order for j2, k in order if j == j2} - order
        if not more:
            break
        order |= more
    keys = [key(c) for c in children]
    rank = sorted(
        range(n),
        key=lambda i: (
            keys[i],
            sorted(keys[p] for p in range(n) if (p, i) in order),
            sorted(keys[q] for q in range(n) if (i, q) in order),
        ),
    )
    at = {old: new for new, old in enumerate(rank)}
    return {
        **d,
        "children": [children[i] for i in rank],
        "order": sorted([at[i], at[j]] for i, j in order),
    }


def _register_powl() -> None:
    for variant in POWL_VARIANTS:
        for log_id, rel in INDUCTIVE_LOGS.items():
            if variant == "bruteforce" and log_id not in POWL_BRUTE_FORCE_LOGS:
                continue
            case(
                f"powl-{variant}-{log_id}",
                fixture=rel,
                functions=["pm4py.discover_powl"],
                params={"variant": variant},
            )(powl)
        for name, traces in POWL_SYNTHETIC_LOGS.items():
            case(
                f"powl-{variant}-synthetic-{name}",
                functions=["pm4py.discover_powl"],
                params={"variant": variant, "traces": traces},
            )(powl)


_register_powl()


FOOTPRINTS_FUNCTIONS = [
    "pm4py.discover_footprints",
    "pm4py.convert_to_event_log",
    "pm4py.discover_dfg",
    "pm4py.discover_powl",
]


def footprints_json(fp: dict[str, Any]) -> dict[str, Any]:
    """A footprints dict with sets as sorted lists."""
    out: dict[str, Any] = {}
    for key, value in fp.items():
        if key == "dfg":
            out[key] = sorted([[a, b], int(n)] for (a, b), n in value.items())
        elif key in ("sequence", "parallel"):
            out[key] = sorted([a, b] for a, b in value)
        elif isinstance(value, (set, frozenset)):
            out[key] = sorted(value)
        elif key == "trace":
            out[key] = list(value)
        elif isinstance(value, bool):
            out[key] = value
        else:
            out[key] = int(value)
    return out


def _traces_footprints(log: Any) -> list[dict[str, Any]]:
    groups: dict[str, dict[str, Any]] = {}
    for fp in pm4py.discover_footprints(log):
        j = footprints_json(fp)
        groups.setdefault(json.dumps(j, sort_keys=True), {**j, "count": 0})["count"] += 1
    return [groups[k] for k in sorted(groups)]


def footprints_log(
    fixtures: dict[str, Path], traces: list[list[str]] | None = None
) -> dict[str, Any]:
    """Footprints of the log in ``fixtures`` and of its DFG and POWL model,
    or of the log ``traces``."""
    if traces is not None:
        from pm4py.algo.discovery.footprints import algorithm as footprints_algorithm
        from pm4py.objects.log.obj import Event, EventLog, Trace

        log = EventLog()
        for i, activities in enumerate(traces):
            trace = Trace(attributes={"concept:name": str(i)})
            for a in activities:
                trace.append(Event({"concept:name": a}))
            log.append(trace)
        entire = footprints_algorithm.apply(
            log, variant=footprints_algorithm.Variants.ENTIRE_EVENT_LOG
        )
        return {"entire": footprints_json(entire), "traces": _traces_footprints(log)}
    df = load_log(fixtures["log"])
    dfg, _, _ = pm4py.discover_dfg(df)
    return {
        "entire": footprints_json(pm4py.discover_footprints(df)),
        "traces": _traces_footprints(pm4py.convert_to_event_log(df)),
        "dfg": footprints_json(pm4py.discover_footprints(dfg)),
        "powl": footprints_json(pm4py.discover_footprints(pm4py.discover_powl(df))),
    }


def _has_silent_frequent(model: Any) -> bool:
    """Whether ``model`` holds a frequent transition without an activity:
    pm4py makes one from a choice or loop of two silent steps, which
    ichnos keeps as it is."""
    if getattr(model, "activity", "") is None:
        return True
    return any(_has_silent_frequent(c) for c in getattr(model, "children", []))


def footprints_powl(fixtures: dict[str, Path], text: str) -> dict[str, Any]:
    """Footprints of the POWL string ``text``, before and after
    ``simplify_using_frequent_transitions()``. ``frequent`` is null when
    the simplified model differs from ichnos's (see
    ``_has_silent_frequent``)."""
    model = pm4py.parse_powl_model_string(text)
    frequent = model.simplify_using_frequent_transitions()
    return {
        "powl": footprints_json(pm4py.discover_footprints(model)),
        "frequent": None
        if _has_silent_frequent(frequent)
        else footprints_json(pm4py.discover_footprints(frequent)),
    }


def _register_footprints() -> None:
    from cases.powl import MODELS

    for log_id, rel in INDUCTIVE_LOGS.items():
        case(f"footprints-log-{log_id}", fixture=rel, functions=FOOTPRINTS_FUNCTIONS)(
            footprints_log
        )
    case(
        "footprints-log-synthetic-emptytraces",
        functions=["pm4py.algo.discovery.footprints.algorithm.apply"],
        params={"traces": POWL_SYNTHETIC_LOGS["emptytraces"]},
    )(footprints_log)
    for name, text in MODELS:
        case(
            f"footprints-powl-{name}",
            functions=[
                "pm4py.parse_powl_model_string",
                "pm4py.discover_footprints",
                "pm4py.objects.powl.obj.POWL.simplify_using_frequent_transitions",
            ],
            params={"text": text},
        )(footprints_powl)


_register_footprints()

# What one seeded run of _inductive_seeds computes, by kind.
SEEDED_RUNS = {"tree": _inductive_run, "bpmn": _bpmn_inductive_run}


if __name__ == "__main__":
    # One seeded run for _inductive_seeds: prints the result as one JSON line.
    from harness import canonical

    path, variant, kind = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    print(json.dumps(canonical.normalize(SEEDED_RUNS[kind](path, variant))))
