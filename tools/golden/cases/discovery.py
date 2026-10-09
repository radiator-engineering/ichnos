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
