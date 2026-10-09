"""Area ``analysis``: pm4py's Petri net analysis functions.

Each ``net-*`` case loads or builds one accepting Petri net, describes it as
``model.py`` does (``model``), and records:

- ``workflow_net``: ``pm4py.check_is_workflow_net``.
- ``soundness``: ``pm4py.check_soundness`` as ``{"sound", "messages"}``, the
  verdict and woflan's diagnostic messages. ``is_sound`` is
  ``pm4py.analysis.check_is_sound``, which tries a POWL conversion first.
- ``simplicity``: ``pm4py.simplicity_petri_net`` for the three variants.
  ``extended_cyclomatic`` builds the reachability graph, so it is ``null``
  for nets marked unbounded below.
- ``decomposition``: ``pm4py.maximal_decomposition``, each component
  described, sorted by its JSON form (pm4py's order depends on hashing).
- ``implicit_places``: ``pm4py.reduce_petri_net_implicit_places``.
- ``invisibles``: ``pm4py.reduce_petri_net_invisibles``.
- ``enabled``: names of ``pm4py.get_enabled_transitions`` in the initial
  marking, sorted.
- ``marking_equation``: ``pm4py.solve_marking_equation`` with unit costs.

Each ``sync-*`` case pairs a trace (``trace``, its activities) with a net and
records ``pm4py.construct_synchronous_product_net`` (``sync_net``) and
``pm4py.solve_extended_marking_equation`` with the default split points
(``extended_marking_equation``) and with ``split_points`` [1]
(``extended_marking_equation_split``). pm4py names synchronous product
places and transitions with Python tuples; ``sync_net`` writes a tuple
``(a, b)`` as the string ``"(a, b)"``, with ``None`` for a missing label.

``generate-marking`` records ``pm4py.generate_marking`` on the running
example with a place name and with a name-to-count map.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

import pm4py
from pm4py.objects.petri_net.obj import Marking, PetriNet
from pm4py.objects.petri_net.utils import petri_utils

from harness import case
from harness.fixtures import load_log, load_model

# Woflan takes minutes on this net in pm4py.
SLOW_SOUNDNESS = {"roadtraffic"}
# The reachability graph of these nets is infinite (the inhibitor and reset
# nets have unbounded counters) or too large to build.
UNBOUNDED = {
    "and-split-xor-join",
    "unbounded-loop",
    "samplenet",
    "insurance-claim",
    "order-fulfillment",
    "roadtraffic",
}


def _describe(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    from cases.model import describe_net

    return describe_net(net, im, fm)


def _build(spec: dict[str, Any]) -> tuple[Any, Any, Any]:
    """Builds a net from ``places``, ``transitions`` ([name, label, pre, post])
    and the two markings (place → tokens)."""
    net = PetriNet(spec.get("name", "net"))
    places = {}
    for p in spec["places"]:
        places[p] = PetriNet.Place(p)
        net.places.add(places[p])
    for name, label, pre, post in spec["transitions"]:
        t = PetriNet.Transition(name, label)
        net.transitions.add(t)
        for p in pre:
            petri_utils.add_arc_from_to(places[p], t, net)
        for p in post:
            petri_utils.add_arc_from_to(t, places[p], net)
    im = Marking({places[p]: n for p, n in spec["im"].items()})
    fm = Marking({places[p]: n for p, n in spec["fm"].items()})
    return net, im, fm


SPECS: dict[str, dict[str, Any]] = {
    # XOR split joined by an AND join: c is dead.
    "xor-split-and-join": {
        "places": ["i", "p1", "p2", "o"],
        "transitions": [
            ["a", "a", ["i"], ["p1"]],
            ["b", "b", ["i"], ["p2"]],
            ["c", "c", ["p1", "p2"], ["o"]],
        ],
        "im": {"i": 1},
        "fm": {"o": 1},
    },
    # AND split joined by an XOR join: two tokens can reach o.
    "and-split-xor-join": {
        "places": ["i", "p1", "p2", "o"],
        "transitions": [
            ["a", "a", ["i"], ["p1", "p2"]],
            ["b", "b", ["p1"], ["o"]],
            ["c", "c", ["p2"], ["o"]],
        ],
        "im": {"i": 1},
        "fm": {"o": 1},
    },
    # A loop that adds a token to p2 each round.
    "unbounded-loop": {
        "places": ["i", "p1", "p2", "o"],
        "transitions": [
            ["a", "a", ["i"], ["p1"]],
            ["b", "b", ["p1"], ["p1", "p2"]],
            ["c", "c", ["p1", "p2"], ["o"]],
        ],
        "im": {"i": 1},
        "fm": {"o": 1},
    },
    # Two source places: not a workflow net.
    "two-sources": {
        "places": ["i1", "i2", "o"],
        "transitions": [["a", "a", ["i1", "i2"], ["o"]]],
        "im": {"i1": 1, "i2": 1},
        "fm": {"o": 1},
    },
    # A sound net with a silent skip and an implicit place.
    "skip-implicit": {
        "places": ["i", "p1", "p2", "q", "o"],
        "transitions": [
            ["a", "a", ["i"], ["p1", "q"]],
            ["skip", None, ["p1"], ["p2"]],
            ["b", "b", ["p1"], ["p2"]],
            ["c", "c", ["p2", "q"], ["o"]],
        ],
        "im": {"i": 1},
        "fm": {"o": 1},
    },
    # A sound net that is not free choice: d needs both branches.
    "parallel-choice": {
        "places": ["i", "p1", "p2", "p3", "p4", "o"],
        "transitions": [
            ["a", "a", ["i"], ["p1", "p2"]],
            ["b", "b", ["p1"], ["p3"]],
            ["tau_1", None, ["p1"], ["p3"]],
            ["c", "c", ["p2"], ["p4"]],
            ["d", "d", ["p3", "p4"], ["o"]],
        ],
        "im": {"i": 1},
        "fm": {"o": 1},
    },
}


def analyse(name: str, make: Any) -> dict[str, Any]:
    """Runs every net function on a fresh copy of the net from ``make()``.

    pm4py's ``copy.deepcopy`` of a net drops the arc types
    (``PetriNet.__deepcopy__`` re-adds every arc as a normal arc), so nets
    with inhibitor or reset arcs must be rebuilt, not copied."""
    from pm4py.algo.analysis.woflan.algorithm import Outputs

    net, im, fm = make()
    out: dict[str, Any] = {"model": _describe(net, im, fm)}
    out["workflow_net"] = pm4py.check_is_workflow_net(make()[0])
    if name in SLOW_SOUNDNESS:
        out["soundness"] = None
        out["is_sound"] = None
    else:
        sound, diag = pm4py.check_soundness(*make())
        out["soundness"] = {
            "sound": sound,
            "messages": [str(m) for m in diag[Outputs.DIAGNOSTIC_MESSAGES]],
        }
        out["is_sound"] = pm4py.analysis.check_is_sound(*make())
    out["simplicity"] = {
        variant: (
            None
            if variant == "extended_cyclomatic" and name in UNBOUNDED
            else pm4py.simplicity_petri_net(*make(), variant=variant)
        )
        for variant in ["arc_degree", "extended_cardoso", "extended_cyclomatic"]
    }
    # Of the transitions that share a label, pm4py's decomposition keeps the
    # last one in set order, which follows memory addresses and changes from
    # run to run. Iterating the transitions in name order makes it the last
    # by name.
    n3, im3, fm3 = make()
    n3._PetriNet__transitions = sorted(n3.transitions, key=lambda t: t.name)
    components = [_describe(*c) for c in pm4py.maximal_decomposition(n3, im3, fm3)]
    out["decomposition"] = sorted(components, key=lambda c: json.dumps(c, sort_keys=True))
    out["implicit_places"] = _describe(*pm4py.reduce_petri_net_implicit_places(*make()))
    n2, im2, fm2 = make()
    out["invisibles"] = _describe(pm4py.reduce_petri_net_invisibles(n2), im2, fm2)
    out["enabled"] = sorted(t.name for t in pm4py.get_enabled_transitions(net, im))
    out["marking_equation"] = pm4py.solve_marking_equation(*make())
    return out


def from_pnml(fixtures: dict[str, Path], name: str) -> dict[str, Any]:
    return analyse(name, lambda: load_model(fixtures["model"]))


def from_spec(fixtures: dict[str, Path], name: str) -> dict[str, Any]:
    return analyse(name, lambda: _build(SPECS[name]))


def from_tree(fixtures: dict[str, Path], name: str) -> dict[str, Any]:
    net, im, fm = pm4py.convert_to_petri_net(load_model(fixtures["model"]))
    # Visible transitions get random UUID names; name them by label and
    # neighbouring places, which have counter-based names.
    visible = sorted(
        (t for t in net.transitions if t.label is not None),
        key=lambda t: (
            t.label,
            sorted(a.source.name for a in t.in_arcs),
            sorted(a.target.name for a in t.out_arcs),
        ),
    )
    for i, t in enumerate(visible):
        t.name = f"t_{i}"
    # These nets have normal arcs only, so a deep copy keeps them whole.
    return analyse(name, lambda: copy.deepcopy((net, im, fm)))


NET_FUNCTIONS = [
    "pm4py.check_is_workflow_net",
    "pm4py.check_soundness",
    "pm4py.analysis.check_is_sound",
    "pm4py.simplicity_petri_net",
    "pm4py.maximal_decomposition",
    "pm4py.reduce_petri_net_implicit_places",
    "pm4py.reduce_petri_net_invisibles",
    "pm4py.get_enabled_transitions",
    "pm4py.solve_marking_equation",
]

PNML = [
    "running-example",
    "receipt_one_variant",
    "ex1",
    "ex2",
    "murata1",
    "murata2",
    "murata3",
    "data_petri_net",
    "stochastic_running_example",
    "roadtraffic",
    "big_wf_net",
    "more_models/SampleNet",
    "inh_res_nets/insurance_claim",
    "inh_res_nets/order_fulfillment",
]


def _slug(name: str) -> str:
    return name.split("/")[-1].replace("_", "-").lower()


for _name in PNML:
    case(
        f"net-{_slug(_name)}",
        fixtures={"model": f"{_name}.pnml"},
        functions=NET_FUNCTIONS,
        params={"name": _slug(_name)},
    )(from_pnml)

for _name in SPECS:
    case(f"net-{_name}", functions=NET_FUNCTIONS, params={"name": _name})(from_spec)

for _name in ["tree_ex_with_loops", "tree_ex_wo_loops"]:
    case(
        f"net-tree-{_slug(_name)}",
        fixtures={"model": f"{_name}.ptml"},
        functions=NET_FUNCTIONS,
        params={"name": _slug(_name)},
    )(from_tree)


def _tuple(t: Any) -> str:
    return f"({t[0]}, {t[1]})"


def describe_sync(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    places = sorted(_tuple(p.name) for p in net.places)
    transitions = sorted(
        ({"name": _tuple(t.name), "label": _tuple(t.label)} for t in net.transitions),
        key=lambda t: t["name"],
    )
    arcs = sorted(
        ({"source": _tuple(a.source.name), "target": _tuple(a.target.name), "weight": a.weight} for a in net.arcs),
        key=lambda a: (a["source"], a["target"]),
    )
    return {
        "places": places,
        "transitions": transitions,
        "arcs": arcs,
        "initial_marking": {_tuple(p.name): n for p, n in im.items()},
        "final_marking": {_tuple(p.name): n for p, n in fm.items()},
    }


def sync(fixtures: dict[str, Path], variant: int) -> dict[str, Any]:
    log = load_log(fixtures["log"])
    net, im, fm = load_model(fixtures["model"])
    variants = sorted(pm4py.get_variants(log).items(), key=lambda kv: (-kv[1], kv[0]))
    activities = list(variants[variant][0])
    trace = pm4py.objects.log.obj.Trace(
        [pm4py.objects.log.obj.Event({"concept:name": a}) for a in activities]
    )
    sn, sim, sfm = pm4py.construct_synchronous_product_net(trace, net, im, fm)
    return {
        "model": _describe(net, im, fm),
        "trace": activities,
        "sync_net": describe_sync(sn, sim, sfm),
        "extended_marking_equation": pm4py.solve_extended_marking_equation(trace, sn, sim, sfm),
        # pm4py raises IndexError when the split point is past the trace.
        "extended_marking_equation_split": (
            pm4py.solve_extended_marking_equation(trace, sn, sim, sfm, split_points=[1])
            if len(activities) > 1
            else None
        ),
    }


SYNC_FUNCTIONS = [
    "pm4py.construct_synchronous_product_net",
    "pm4py.solve_extended_marking_equation",
]

for _model, _log, _variants in [
    ("running-example", "running-example.xes", range(6)),
    ("receipt_one_variant", "receipt.xes", range(3)),
]:
    for _v in _variants:
        case(
            f"sync-{_slug(_model)}-{_v}",
            fixtures={"model": f"{_model}.pnml", "log": _log},
            functions=SYNC_FUNCTIONS,
            params={"variant": _v},
        )(sync)


@case(
    "generate-marking",
    fixtures={"model": "running-example.pnml"},
    functions=["pm4py.generate_marking"],
)
def generate_marking(fixtures: dict[str, Path]) -> dict[str, Any]:
    net, im, fm = load_model(fixtures["model"])
    source = sorted(p.name for p in im)[0]
    sink = sorted(p.name for p in fm)[0]
    single = pm4py.generate_marking(net, source)
    counts = pm4py.generate_marking(net, {source: 2, sink: 1})
    return {
        "model": _describe(net, im, fm),
        "single": {"place": source, "marking": {p.name: n for p, n in single.items()}},
        "counts": {
            "places": {source: 2, sink: 1},
            "marking": {p.name: n for p, n in counts.items()},
        },
    }
