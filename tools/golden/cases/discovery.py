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

Logs are CSV only until ichnos can read XES in the discovery tests.
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


def _inductive_seeds(path: Path, variant: str) -> dict[str, Any]:
    """Runs :func:`_inductive_run` once per seed in ``IMF_SEEDS`` and groups
    the runs by tree."""
    golden_tools = str(Path(__file__).resolve().parents[1])
    runs: dict[str, dict[str, Any]] = {}
    for seed in IMF_SEEDS:
        env = dict(os.environ, PYTHONHASHSEED=str(seed))
        env["PYTHONPATH"] = os.pathsep.join(filter(None, [golden_tools, env.get("PYTHONPATH")]))
        out = subprocess.run(
            [sys.executable, __file__, str(path), variant],
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


if __name__ == "__main__":
    # One seeded run for _inductive_seeds: prints the result as one JSON line.
    from harness import canonical

    print(json.dumps(canonical.normalize(_inductive_run(Path(sys.argv[1]), sys.argv[2]))))
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
