"""Simulation, process cubes and SaCoFa/PRIPEL entry points.

Randomized cases store distribution summaries or verified invariants, not raw
Python draws. Every sampler is seeded; diffprivlib's private SystemRandom is
replaced with a seeded RandomState for reproducible mechanism observations.
"""

import random
import warnings
from collections import Counter
from datetime import datetime, timedelta, timezone
import numpy as np
import pandas as pd
import pm4py
import pm4py.privacy
from harness import case
from harness.fixtures import load_log

SEED = 1729
N = 12000


def seed():
    random.seed(SEED)
    np.random.seed(SEED)


def variants(log):
    return Counter(tuple(e["concept:name"] for e in t) for t in log)


TREES = {
    "sequence": "->( 'a', tau, 'b' )",
    "xor": "X( 'a', 'b' )",
    "parallel": "+( ->( 'a', 'b' ), 'c' )",
    "interleaving": "<>( ->( 'a', 'b' ), 'c' )",
    "loop": "*( 'a', 'b' )",
    "tau": "tau",
}


def tree_playout(fixtures, name):
    seed()
    tree = pm4py.parse_process_tree(TREES[name])
    log = pm4py.play_out(tree, parameters={"num_traces": N})
    counts = variants(log)
    return {
        "tree": TREES[name],
        "traces": N,
        "mean_length": sum(len(t) for t in log) / N,
        "rates": [[list(v), n / N] for v, n in sorted(counts.items()) if len(v) <= 7],
        "case_attributes_empty": all(not t.attributes for t in log),
        "event_keys": sorted({k for t in log for e in t for k in e}),
    }


for name in TREES:
    case("playout-tree-" + name, functions=["pm4py.play_out"], params={"name": name})(
        tree_playout
    )


@case("parse-trees", functions=["pm4py.parse_process_tree"])
def parse_trees(fixtures):
    texts = list(TREES.values()) + ["O( 'a', 'b' )", "->( 'é', X( 'a b', tau ) )"]
    return [
        {
            "input": s,
            "display": str(pm4py.parse_process_tree(s)),
            "leaves": sorted(
                [l.label for l in pm4py.parse_process_tree(s)._get_leaves()],
                key=lambda x: (x is not None, x or ""),
            ),
        }
        for s in texts
    ]


def generator(fixtures, operator):
    seed()
    params = {
        "min": 5,
        "mode": 10,
        "max": 15,
        "sequence": 0.0,
        "choice": 0.0,
        "parallel": 0.0,
        "loop": 0.0,
        "or": 0.0,
        "silent": 0.2,
        "duplicate": 0.0,
    }
    if operator == "mixed":
        for key in ["sequence", "choice", "parallel", "loop"]:
            params[key] = 0.25
    else:
        params[operator] = 1.0
    trees = pm4py.generate_process_tree(parameters={**params, "no_models": 1000})
    visible = [sum(l.label is not None for l in t._get_leaves()) for t in trees]
    operators = Counter()

    def walk(t):
        if t.operator is not None:
            operators[str(t.operator)] += 1
        for c in t.children:
            walk(c)

    for tree in trees:
        walk(tree)
    return {
        "params": params,
        "models": len(trees),
        "visible_min": min(visible),
        "visible_max": max(visible),
        "visible_mean": float(np.mean(visible)),
        "operator_rates": {
            op: n / sum(operators.values()) for op, n in sorted(operators.items())
        },
        "binary": all(
            all(len(n.children) == 2 for n in _nodes(t) if n.operator is not None)
            for t in trees
        ),
    }


def _nodes(tree):
    yield tree
    for c in tree.children:
        yield from _nodes(c)


for operator in ["sequence", "choice", "parallel", "loop", "or", "mixed"]:
    case(
        "generate-tree-" + operator,
        functions=["pm4py.generate_process_tree"],
        params={"operator": operator},
    )(generator)


def dfg(fixtures, name):
    if name == "loop":
        graph = {("a", "a"): 2, ("a", "b"): 3, ("b", "a"): 1}
        starts, ends = {"a": 4, "b": 1}, {"b": 4, "a": 1}
    else:
        log = load_log(fixtures["log"])
        graph, starts, ends = pm4py.discover_dfg(log)
    # Python dictionaries have insertion order; Rust's typed graph has lexical order.
    graph = dict(sorted(graph.items()))
    starts = dict(sorted(starts.items()))
    ends = dict(sorted(ends.items()))
    params = {"max_no_variants": 20, "max_no_occ_per_activitiy": 2}
    result = pm4py.play_out(graph, starts, ends, parameters=params)
    return {
        "graph": [[a, b, n] for (a, b), n in graph.items()],
        "starts": starts,
        "ends": ends,
        "variants": [
            {
                "activities": [e["concept:name"] for e in t],
                "probability": t.attributes["probability"],
            }
            for t in result
        ],
    }


case("playout-dfg-loop", functions=["pm4py.play_out"], params={"name": "loop"})(dfg)
for log in ["running-example", "receipt", "roadtraffic100traces"]:
    case(
        "playout-dfg-" + log,
        fixture=log + ".xes",
        functions=["pm4py.play_out"],
        params={"name": log},
    )(dfg)


def stable_semantics():
    """Sort enabled transitions, preserving pm4py firing and stop semantics.

    Object-address set iteration otherwise changes fixed-seed observations.
    """
    from pm4py.objects.petri_net.semantics import ClassicSemantics

    class StableTransitions(set):
        def __iter__(self):
            return iter(
                sorted(
                    super().__iter__(),
                    key=lambda t: (t is None, t.name if t is not None else ""),
                )
            )

        def union(self, *others):
            return StableTransitions(super().union(*others))

    class StableSemantics(ClassicSemantics):
        def enabled_transitions(self, net, marking):
            return StableTransitions(super().enabled_transitions(net, marking))

    return StableSemantics()


def petri(fixtures):
    from harness.fixtures import load_model

    seed()
    net, im, fm = load_model(fixtures["model"])
    # Stabilize transition iteration before random choice, avoiding object-address order.
    net._PetriNet__transitions = sorted(net.transitions, key=lambda t: t.name)
    log = pm4py.play_out(
        net,
        im,
        fm,
        parameters={
            "noTraces": 5000,
            "maxTraceLength": 100,
            "petri_semantics": stable_semantics(),
        },
    )
    counts = variants(log)
    return {
        "traces": len(log),
        "mean_length": sum(len(t) for t in log) / len(log),
        "rates": [
            [list(v), n / len(log)] for v, n in sorted(counts.items()) if n >= 30
        ],
        "activities": sorted({a for v in counts for a in v}),
        "timestamp_step": all(
            (b["time:timestamp"] - a["time:timestamp"]).total_seconds() == 1
            for t in log
            for a, b in zip(t, t[1:])
        ),
    }


case(
    "playout-petri-running-example",
    fixtures={"model": "running-example.pnml"},
    functions=["pm4py.play_out"],
)(petri)


def cube(fixtures, kind, agg):
    data = {
        "case:concept:name": ["a", "b", "c", "d", "e", "f"],
        "nx": [0.0, 1.0, 2.0, 3.0, None, 4.0],
        "ny": [0.0, 1.0, 2.0, 3.0, 4.0, 4.0],
        "px_b": [0.0, 1.0, 1.0, 0.0, 1.0, 0.0],
        "px_a": [1.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        "py_u": [1.0, 0.0, 1.0, 0.0, 1.0, 1.0],
        "py_v": [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
        "value": [1.0, 3.0, None, 7.0, 9.0, 11.0],
    }
    x = "nx" if kind[0] == "n" else "px"
    y = "ny" if kind[1] == "n" else "py"
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", FutureWarning)
        pivot, cells = pm4py.get_process_cube(
            pd.DataFrame(data),
            x,
            y,
            "value",
            parameters={
                "max_divisions_x": 2,
                "max_divisions_y": 2,
                "aggregation_function": agg,
            },
        )
    # Normalize presentation to [y][x], and compare bin assignments instead of
    # pandas' precision-rounded textual interval labels.
    values = [
        [
            None if pd.isna(pivot.loc[xb, yb]) else float(pivot.loc[xb, yb])
            for xb in pivot.index
        ]
        for yb in pivot.columns
    ]
    membership = [
        [sorted(cells.get((xb, yb), set())) for xb in pivot.index]
        for yb in pivot.columns
    ]
    return {
        "data": data,
        "x": x,
        "y": y,
        "aggregation": agg,
        "shape": [len(pivot.columns), len(pivot.index)],
        "values": values,
        "cases": membership,
    }


for kind in ["nn", "np", "pn", "pp"]:
    for agg in ["mean", "sum", "min", "max"]:
        case(
            "cube-" + kind + "-" + agg,
            functions=["pm4py.get_process_cube"],
            params={"kind": kind, "agg": agg},
        )(cube)


@case(
    "privacy-mechanisms",
    functions=[
        "pm4py.algo.anonymization.trace_variant_query.util.exp_mech.exp_mech",
        "diffprivlib.mechanisms.LaplaceBoundedDomain",
        "diffprivlib.mechanisms.Binary",
        "diffprivlib.mechanisms.ExponentialCategorical",
    ],
)
def mechanisms(fixtures, epsilon=1.0):
    import diffprivlib
    from diffprivlib.mechanisms import (
        Binary,
        LaplaceBoundedDomain,
        ExponentialCategorical,
    )
    from pm4py.algo.anonymization.trace_variant_query.util import exp_mech

    seed()
    numeric = LaplaceBoundedDomain(
        epsilon=epsilon, sensitivity=10.0, lower=0.0, upper=10.0, random_state=SEED
    )
    samples = [numeric.randomise(2.0) for _ in range(N)]
    binary = Binary(epsilon=epsilon, value0="True", value1="False", random_state=SEED)
    categorical = ExponentialCategorical(
        epsilon=epsilon,
        utility_list=[(a, b, 1) for a in ["a", "b", "c"] for b in ["a", "b", "c"]],
        random_state=SEED,
    )
    return {
        "diffprivlib_version": diffprivlib.__version__,
        "samples": N,
        "epsilon": epsilon,
        "numeric_mean": float(np.mean(samples)),
        "numeric_bounds": [0.0, 10.0],
        "boolean_keep": sum(binary.randomise("True") == "True" for _ in range(N)) / N,
        "categorical_keep": sum(categorical.randomise("a") == "a" for _ in range(N))
        / N,
        "integer_laplace_zero": sum(
            int(np.random.laplace(scale=1.0 / epsilon)) == 0 for _ in range(N)
        )
        / N,
        "universe_zero": sum(
            exp_mech.exp_mech(np.arange(4), epsilon) == 0 for _ in range(N)
        )
        / N,
    }


def privacy(fixtures, name):
    import diffprivlib.mechanisms as mechanisms
    from pm4py.objects.log.obj import EventLog, Trace, Event
    from pm4py.algo.anonymization.trace_variant_query import algorithm as tvq
    from pm4py.objects.conversion.log import converter as log_converter

    seed()
    if name == "synthetic":
        sequences = [["a", "b", "c"], ["a", "c"]] * 25
    else:
        raw = pm4py.convert_to_event_log(load_log(fixtures["log"]))
        # Short real-log projections keep the prefix universe and matching bounded.
        sequences = [[e["concept:name"] for e in t][:3] for t in list(raw)[:30]]
    log = EventLog()
    base = datetime(2020, 1, 1, tzinfo=timezone.utc)
    for i, sequence in enumerate(sequences):
        trace = Trace(attributes={"concept:name": str(i), "private": "source-id"})
        for j, a in enumerate(sequence):
            trace.append(
                Event(
                    {
                        "concept:name": a,
                        "time:timestamp": base + timedelta(seconds=100 * i + j),
                        "cost": float(i % 11),
                        "flag": i % 2 == 0,
                        "category": "red" if i % 2 == 0 else "blue",
                    }
                )
            )
        log.append(trace)
    params = {"epsilon": 10.0, "k": 3, "p": 2}
    query = tvq.apply(log, parameters=params)
    # diffprivlib otherwise uses secrets.SystemRandom, outside numpy's seed.
    original = mechanisms.LaplaceBoundedDomain.__init__

    def seeded(self, *args, **kwargs):
        kwargs["random_state"] = SEED
        original(self, *args, **kwargs)

    mechanisms.LaplaceBoundedDomain.__init__ = seeded
    binary_original = mechanisms.Binary.__init__

    def binary_seeded(self, *args, **kwargs):
        kwargs["random_state"] = SEED
        binary_original(self, *args, **kwargs)

    mechanisms.Binary.__init__ = binary_seeded
    try:
        # Execute the public wrapper, not just the phases. Reset before calling
        # so control-flow counts repeat the separate query observation.
        seed()
        result = pm4py.privacy.anonymize_differential_privacy(
            log, epsilon=10.0, k=3, p=2
        )
    finally:
        mechanisms.LaplaceBoundedDomain.__init__ = original
        mechanisms.Binary.__init__ = binary_original
    output = pm4py.convert_to_event_log(result)
    count = variants(output)
    return {
        "input": sequences,
        "epsilon": 10.0,
        "k": 3,
        "p": 2,
        "query_variants": [
            [list(v), n]
            for v, n in sorted(
                variants(
                    log_converter.apply(
                        query, variant=log_converter.Variants.TO_EVENT_LOG
                    )
                ).items()
            )
        ],
        "output_variants": [[list(v), n] for v, n in sorted(count.items())],
        "invariants": {
            "traces": len(output),
            "maximum_length": max(map(len, output)),
            "case_ids_unique": len({t.attributes["concept:name"] for t in output})
            == len(output),
            "private_trace_attribute_removed": all(
                "private" not in t.attributes for t in output
            ),
            "cost_in_domain": all(0.0 <= e["cost"] <= 10.0 for t in output for e in t),
            "categories_in_domain": all(
                e["category"] in ["red", "blue"] for t in output for e in t
            ),
            "flags_boolean": all(
                isinstance(e["flag"], bool) for t in output for e in t
            ),
            "timestamps_ordered": all(
                a["time:timestamp"] <= b["time:timestamp"]
                for t in output
                for a, b in zip(t, t[1:])
            ),
        },
    }


case(
    "privacy-synthetic",
    functions=["pm4py.privacy.anonymize_differential_privacy"],
    params={"name": "synthetic"},
)(privacy)
for name in ["running-example", "receipt", "roadtraffic100traces"]:
    case(
        "privacy-" + name,
        fixture=name + ".xes",
        functions=["pm4py.privacy.anonymize_differential_privacy"],
        params={"name": name},
    )(privacy)


def real_cube(fixtures):
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    rows = []
    for i, trace in enumerate(list(log)[:50]):
        rows.append(
            {
                "case:concept:name": str(i),
                "nx": float(len(trace)),
                "ny": float(len({e["concept:name"] for e in trace})),
                "value": (
                    trace[-1]["time:timestamp"] - trace[0]["time:timestamp"]
                ).total_seconds(),
            }
        )
    data = {key: [row[key] for row in rows] for key in rows[0]}
    pivot, cells = pm4py.get_process_cube(
        pd.DataFrame(data),
        "nx",
        "ny",
        "value",
        parameters={"max_divisions_x": 2, "max_divisions_y": 2},
    )
    return {
        "data": data,
        "x": "nx",
        "y": "ny",
        "aggregation": "mean",
        "shape": [len(pivot.columns), len(pivot.index)],
        "values": [
            [
                None if pd.isna(pivot.loc[xb, yb]) else float(pivot.loc[xb, yb])
                for xb in pivot.index
            ]
            for yb in pivot.columns
        ],
        "cases": [
            [sorted(cells.get((xb, yb), set())) for xb in pivot.index]
            for yb in pivot.columns
        ],
    }


for name in ["running-example", "receipt", "roadtraffic100traces"]:
    case(
        "cube-real-" + name, fixture=name + ".xes", functions=["pm4py.get_process_cube"]
    )(real_cube)


@case(
    "privacy-behavioral-relations",
    functions=[
        "pm4py.algo.anonymization.trace_variant_query.util.behavioralAppropriateness.getBAViolations"
    ],
)
def behavioral_relations(fixtures):
    from itertools import product
    from pm4py.algo.anonymization.trace_variant_query.util import (
        behavioralAppropriateness as ba,
    )

    traces = [["a", "b", "a", "c"], ["a", "c"], ["b", "a", "c"]]
    activities = ["a", "b", "c"]
    f = ba.getFollowsRelations(activities, traces)
    p = ba.getPrecedesRelations(activities, traces)
    rows = []
    for size in [1, 2, 3]:
        for seq in product(activities, repeat=size):
            for complete in [False, True]:
                prefix = list(seq) + (["TRACE_END"] if complete else [])
                rows.append(
                    {
                        "prefix": list(seq),
                        "complete": complete,
                        "violations": ba.getBAViolations(
                            activities + ["TRACE_END"], f, p, prefix, "TRACE_END"
                        ),
                    }
                )
    return {"input": traces, "activities": activities, "rows": rows}


@case("playout-declare-prefixes", functions=["pm4py.play_out"])
def declare_prefixes(fixtures):
    from itertools import product
    from pm4py.algo.simulation.playout.declare.variants.classic import DeclarePlayout

    templates = [
        "existence",
        "absence",
        "exactly_one",
        "init",
        "responded_existence",
        "coexistence",
        "response",
        "precedence",
        "succession",
        "altresponse",
        "altprecedence",
        "altsuccession",
        "chainresponse",
        "chainprecedence",
        "chainsuccession",
        "noncoexistence",
        "nonsuccession",
        "nonchainsuccession",
    ]
    result = []
    for template in templates:
        unary = template in ["existence", "absence", "exactly_one", "init"]
        model = {template: {"a" if unary else ("a", "b"): {}}, "precedence": {}}
        if template == "precedence":
            model["precedence"] = {("a", "b"): {}}
        sampler = DeclarePlayout(model)
        rows = []
        for size in range(4):
            for prefix in product(["a", "b", "c"], repeat=size):
                state = sampler._new_constraints_state()
                valid = True
                for a in prefix:
                    state, violated = sampler._try_event(a, state)
                    if violated:
                        valid = False
                        break
                if valid:
                    rows.append(
                        {
                            "prefix": list(prefix),
                            "allowed": [
                                a
                                for a in ["a", "b", "c"]
                                if not sampler._try_event(a, state)[1]
                            ],
                        }
                    )
        # Exercise the public dispatch as well; presence of precedence selects DECLARE.
        seed()
        generated = pm4py.play_out(model, parameters={"n_traces": 20})
        result.append(
            {
                "template": template,
                "a": "a",
                "b": "b",
                "rows": rows,
                "traces": len(generated),
                "lengths_bounded": all(len(t) <= 15 for t in generated),
            }
        )
    return result


for epsilon in [0.5, 2.0]:
    case(
        "privacy-mechanisms-" + str(epsilon).replace(".", "-"),
        functions=[
            "diffprivlib.mechanisms.LaplaceBoundedDomain",
            "diffprivlib.mechanisms.Binary",
            "diffprivlib.mechanisms.ExponentialCategorical",
            "pm4py.algo.anonymization.trace_variant_query.util.exp_mech.exp_mech",
        ],
        params={"epsilon": epsilon},
    )(mechanisms)


@case("cube-top-edge", functions=["pm4py.get_process_cube"])
def cube_top_edge(fixtures):
    data = {
        "case:concept:name": ["minimum", "maximum"],
        "nx": [-387.2267593335082, 471.28764702305114],
        "ny": [0.0, 1.0],
        "value": [2.0, 7.0],
    }
    pivot, cells = pm4py.get_process_cube(
        pd.DataFrame(data),
        "nx",
        "ny",
        "value",
        parameters={"max_divisions_x": 5, "max_divisions_y": 2},
    )
    return {
        "data": data,
        "x": "nx",
        "y": "ny",
        "divisions_x": 5,
        "aggregation": "mean",
        "shape": [len(pivot.columns), len(pivot.index)],
        "values": [
            [
                None if pd.isna(pivot.loc[xb, yb]) else float(pivot.loc[xb, yb])
                for xb in pivot.index
            ]
            for yb in pivot.columns
        ],
        "cases": [
            [sorted(cells.get((xb, yb), set())) for xb in pivot.index]
            for yb in pivot.columns
        ],
    }


@case("playout-petri-final-stop", functions=["pm4py.play_out"])
def petri_final_stop(fixtures):
    from pm4py.objects.petri_net.obj import PetriNet, Marking
    from pm4py.objects.petri_net.utils.petri_utils import add_arc_from_to

    net = PetriNet("final with outgoing transition")
    p, q = PetriNet.Place("p"), PetriNet.Place("q")
    a = PetriNet.Transition("a", "a")
    net.places.update([p, q])
    net.transitions.add(a)
    add_arc_from_to(p, a, net)
    add_arc_from_to(a, q, net)
    rows = []
    for tokens in [1, 2]:
        for require in [False, True]:
            for leq in [False, True]:
                seed()
                log = pm4py.play_out(
                    net,
                    Marking({p: tokens}),
                    Marking({p: 1}),
                    parameters={
                        "noTraces": 5000,
                        "petri_semantics": stable_semantics(),
                        "maxTraceLength": 3,
                        "add_only_if_fm_is_reached": require,
                        "fm_leq_accepted": leq,
                    },
                )
                n = len(log)
                rows.append(
                    {
                        "tokens": tokens,
                        "require_final": require,
                        "leq": leq,
                        "traces": n,
                        "rates": (
                            [
                                [list(v), count / n]
                                for v, count in sorted(variants(log).items())
                            ]
                            if n
                            else []
                        ),
                    }
                )
    return rows


@case(
    "privacy-epsilon-one-distribution",
    functions=["pm4py.privacy.anonymize_differential_privacy"],
)
def privacy_distribution(fixtures):
    import json
    import os
    import subprocess
    import sys
    from pathlib import Path

    env = dict(os.environ, PYTHONHASHSEED="0")
    env["PYTHONPATH"] = str(Path(__file__).resolve().parent.parent)
    output = subprocess.run(
        [
            sys.executable,
            "-c",
            "import json; from cases.simulation import _privacy_distribution; print(json.dumps(_privacy_distribution({})))",
        ],
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(output.stdout)


def _privacy_distribution(fixtures):
    import diffprivlib.mechanisms as mechanisms
    from pm4py.objects.log.obj import EventLog, Trace, Event

    original = {
        name: getattr(mechanisms, name).__init__
        for name in ["LaplaceBoundedDomain", "Binary", "ExponentialCategorical"]
    }
    totals, lengths, costs, flags, categories = [], [], [], [], []
    seeds = 200
    base = datetime(2020, 1, 1, tzinfo=timezone.utc)
    for iteration in range(seeds):
        random.seed(iteration)
        np.random.seed(iteration)
        log = EventLog()
        for i in range(50):
            sequence = ["a", "b", "c"] if i % 2 == 0 else ["a", "c"]
            trace = Trace(attributes={"concept:name": str(i)})
            for j, a in enumerate(sequence):
                trace.append(
                    Event(
                        {
                            "concept:name": a,
                            "time:timestamp": base + timedelta(seconds=100 * i + j),
                            "cost": float(i % 11),
                            "flag": i % 2 == 0,
                            "category": "red" if i % 2 == 0 else "blue",
                        }
                    )
                )
            log.append(trace)
        for name, init in original.items():

            def seeded(self, *args, _init=init, **kwargs):
                kwargs["random_state"] = iteration
                _init(self, *args, **kwargs)

            getattr(mechanisms, name).__init__ = seeded
        try:
            result = pm4py.privacy.anonymize_differential_privacy(
                log, epsilon=1.0, k=3, p=2
            )
        finally:
            for name, init in original.items():
                getattr(mechanisms, name).__init__ = init
        output = pm4py.convert_to_event_log(result)
        totals.append(len(output))
        lengths.append(sum(map(len, output)) / max(1, len(output)))
        events = [e for t in output for e in t]
        costs.append(np.mean([e["cost"] for e in events]) if events else 0.0)
        flags.append(np.mean([e["flag"] for e in events]) if events else 0.0)
        categories.append(
            np.mean([e["category"] == "red" for e in events]) if events else 0.0
        )
    return {
        "seeds": seeds,
        "epsilon": 1.0,
        "mean_traces": float(np.mean(totals)),
        "std_traces": float(np.std(totals)),
        "mean_length": float(np.mean(lengths)),
        "cost_mean": float(np.mean(costs)),
        "flag_rate": float(np.mean(flags)),
        "category_red": float(np.mean(categories)),
    }
