"""Area ``conformance``: conformance checking, ported in ``ichnos-conformance``.

Each conformance lane keeps its cases in its own clearly named functions
below.

Alignments (lane ``alignments``), cases ``alignments-<log>-<model>``:

- ``<model>`` is a fixture net (``pnml-<name>``) or the net that
  ``pm4py.discover_petri_net_inductive`` finds on the same log (``im``).
- The log is converted with ``pm4py.convert_to_event_log`` first, so the
  traces keep the order of the formatted table.

Each case emits:

- ``model``: the net in a form a Rust test can build directly (see
  :func:`describe_canonical_net`). Node names are canonical, not pm4py's:
  the inductive miner names transitions with random UUIDs.
- ``traces``: one record per trace, in log order: ``case_id``, ``cost``,
  ``fitness`` and ``bwc`` from ``pm4py.conformance_diagnostics_alignments``
  (the default variant, state-equation A*). Several optimal alignments can
  exist and pm4py picks one by hash order, so the moves are not emitted.
- ``fitness``: ``pm4py.fitness_alignments``.
- ``precision``: ``pm4py.precision_alignments``.
- ``precision_complete_closure``: the same with pm4py's
  ``get_visible_transitions_eventually_enabled_by_marking`` replaced by
  :func:`complete_eventually_enabled`. pm4py's version can miss markings
  reached through silent transitions (see ichnos-model's
  ``visible_transitions_eventually_enabled``); ichnos computes this value.

Alignments against other models (lane ``alignments``), each against a model
built from the log's ``SEQUENCE_TOP_K`` most frequent variants, so that some
traces deviate:

- ``alignments-dfg-<log>``: the DFG pm4py discovers (``dfg``, ``start``,
  ``end``) and, per trace, ``cost``, ``fitness``, ``bwc``, ``visited`` (pm4py's ``visited_states``) and
  ``closed`` from ``conformance_diagnostics_alignments(log, dfg, sa, ea)``.
- ``alignments-tree-<log>``: the inductive-miner tree (``tree``, pm4py's
  string form) and, per trace, ``cost`` and ``fitness`` from
  ``conformance_diagnostics_alignments(log, tree)``.
  ``alignments-tree-running-example-ptml`` uses the fixture tree instead.
- ``alignments-edit-distance-<log>``: the model variants (``model``) and,
  per trace, ``bwc`` and ``cost_choices`` from
  ``conformance_diagnostics_alignments(log, model_log)``. When several model
  variants are closest, pm4py's pick depends on string hashing, so
  ``cost_choices`` lists the cost of each pick it could make. ``cost`` and
  ``fitness`` are emitted only when there is one choice.
"""

from __future__ import annotations

import difflib
import hashlib
import json
from pathlib import Path
from typing import Any

import pm4py
from pm4py.algo.evaluation.precision.variants import align_etconformance
from pm4py.objects.petri_net import semantics
from pm4py.util import string_distance

from harness import case
from harness.fixtures import load_log, load_model

ALIGNMENT_FUNCTIONS = [
    "pm4py.convert_to_event_log",
    "pm4py.conformance_diagnostics_alignments",
    "pm4py.fitness_alignments",
    "pm4py.precision_alignments",
]

ALIGNMENT_LOGS = {
    "running-example": "running-example.csv",
    "receipt": "receipt.csv",
    "roadtraffic100traces": "roadtraffic100traces.csv",
}

# Log id -> fixture nets aligned against that log.
ALIGNMENT_NETS = {
    "running-example": ["running-example.pnml", "stochastic_running_example.pnml"],
    "receipt": ["receipt_one_variant.pnml"],
    "roadtraffic100traces": ["roadtraffic.pnml", "data_petri_net.pnml"],
}


def _digest(value: Any) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode("utf-8")).hexdigest()


def describe_canonical_net(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    """The net with canonical node names, in the JSON form of ``cases/model.py``.

    Nodes get colours by colour refinement (Weisfeiler-Leman) on the net
    graph. A place starts from its initial and final token counts, a
    transition from its label; each round hashes a node's colour with the
    sorted colours of its arcs and neighbours. When nodes share a colour, one
    of them is singled out and refinement runs again. Places are then named
    ``p<i>`` and transitions ``t<i>`` in colour order. The names depend only
    on the structure of the net, so two runs give the same file.
    """
    nodes = list(net.places) + list(net.transitions)
    is_place = {id(p) for p in net.places}
    colour: dict[int, str] = {}
    for n in nodes:
        if id(n) in is_place:
            colour[id(n)] = _digest(["place", im.get(n, 0), fm.get(n, 0)])
        else:
            colour[id(n)] = _digest(["transition", n.label])
    edges: dict[int, list[tuple[str, int, str, Any]]] = {id(n): [] for n in nodes}
    for a in net.arcs:
        kind = a.properties.get("arctype", "normal")
        edges[id(a.source)].append(("out", a.weight, kind, a.target))
        edges[id(a.target)].append(("in", a.weight, kind, a.source))

    def refine() -> None:
        while True:
            before = len(set(colour.values()))
            new = {
                id(n): _digest(
                    [
                        colour[id(n)],
                        sorted([d, w, k, colour[id(m)]] for d, w, k, m in edges[id(n)]),
                    ]
                )
                for n in nodes
            }
            colour.update(new)
            if len(set(colour.values())) == before:
                return

    refine()
    while True:
        classes: dict[str, list[Any]] = {}
        for n in nodes:
            classes.setdefault(colour[id(n)], []).append(n)
        tied = sorted(c for c, members in classes.items() if len(members) > 1)
        if not tied:
            break
        chosen = classes[tied[0]][0]
        colour[id(chosen)] = _digest(["chosen", colour[id(chosen)]])
        refine()

    places = sorted(net.places, key=lambda p: colour[id(p)])
    transitions = sorted(net.transitions, key=lambda t: colour[id(t)])
    name = {id(p): f"p{i}" for i, p in enumerate(places)}
    name.update({id(t): f"t{i}" for i, t in enumerate(transitions)})
    return {
        "kind": "petri_net",
        "places": sorted(name[id(p)] for p in places),
        "transitions": sorted(
            ({"name": name[id(t)], "label": t.label} for t in transitions),
            key=lambda t: t["name"],
        ),
        "arcs": sorted(
            (
                {
                    "source": name[id(a.source)],
                    "target": name[id(a.target)],
                    "weight": a.weight,
                    "type": a.properties.get("arctype", "normal"),
                }
                for a in net.arcs
            ),
            key=lambda a: (a["source"], a["target"]),
        ),
        "initial_marking": {name[id(p)]: n for p, n in im.items()},
        "final_marking": {name[id(p)]: n for p, n in fm.items()},
    }


def complete_eventually_enabled(net: Any, marking: Any) -> set[Any]:
    """Visible transitions enabled in ``marking`` or after silent moves, over every reachable marking."""
    visible = set()
    seen = {marking}
    queue = [marking]
    while queue:
        m = queue.pop()
        for t in semantics.enabled_transitions(net, m):
            if t.label is not None:
                visible.add(t)
            else:
                n = semantics.execute(t, net, m)
                if n not in seen:
                    seen.add(n)
                    queue.append(n)
    return visible


def precision_complete_closure(log: Any, net: Any, im: Any, fm: Any) -> float:
    """``pm4py.precision_alignments`` with :func:`complete_eventually_enabled`."""
    utils = align_etconformance.utils
    original = utils.get_visible_transitions_eventually_enabled_by_marking
    # align_etconformance imports the function by name too; patch both.
    imported = align_etconformance.get_visible_transitions_eventually_enabled_by_marking
    utils.get_visible_transitions_eventually_enabled_by_marking = complete_eventually_enabled
    align_etconformance.get_visible_transitions_eventually_enabled_by_marking = complete_eventually_enabled
    try:
        return pm4py.precision_alignments(log, net, im, fm)
    finally:
        utils.get_visible_transitions_eventually_enabled_by_marking = original
        align_etconformance.get_visible_transitions_eventually_enabled_by_marking = imported


def alignments(fixtures: dict[str, Path]) -> dict[str, Any]:
    """Alignments of the log in ``fixtures`` against its model, or against the IM net."""
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    if "model" in fixtures:
        net, im, fm = load_model(fixtures["model"])
    else:
        net, im, fm = pm4py.discover_petri_net_inductive(log)
    diagnostics = pm4py.conformance_diagnostics_alignments(log, net, im, fm)
    return {
        "model": describe_canonical_net(net, im, fm),
        "traces": [
            {
                "case_id": trace.attributes["concept:name"],
                "cost": d["cost"],
                "fitness": d["fitness"],
                "bwc": d["bwc"],
            }
            for trace, d in zip(log, diagnostics)
        ],
        "fitness": pm4py.fitness_alignments(log, net, im, fm),
        "precision": pm4py.precision_alignments(log, net, im, fm),
        "precision_complete_closure": precision_complete_closure(log, net, im, fm),
    }


for _log_id, _log in ALIGNMENT_LOGS.items():
    case(
        f"alignments-{_log_id}-im",
        fixture=_log,
        functions=["pm4py.discover_petri_net_inductive", *ALIGNMENT_FUNCTIONS],
    )(alignments)
    for _net in ALIGNMENT_NETS[_log_id]:
        _net_id = Path(_net).stem.replace("_", "-").lower()
        case(
            f"alignments-{_log_id}-pnml-{_net_id}",
            fixtures={"log": _log, "model": _net},
            functions=ALIGNMENT_FUNCTIONS,
        )(alignments)


SEQUENCE_TOP_K = 2

SEQUENCE_FUNCTIONS = [
    "pm4py.convert_to_event_log",
    "pm4py.filter_variants_top_k",
    "pm4py.conformance_diagnostics_alignments",
]


def _variant(trace: Any) -> tuple[str, ...]:
    return tuple(e["concept:name"] for e in trace)


def _top_k(log: Any) -> Any:
    return pm4py.filter_variants_top_k(log, SEQUENCE_TOP_K)


def alignments_dfg(fixtures: dict[str, Path]) -> dict[str, Any]:
    """Alignments against the DFG of the top variants."""
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    dfg, sa, ea = pm4py.discover_dfg(_top_k(log))
    diagnostics = pm4py.conformance_diagnostics_alignments(log, dfg, sa, ea)
    return {
        "dfg": sorted([a, b, n] for (a, b), n in dfg.items()),
        "start": dict(sorted(sa.items())),
        "end": dict(sorted(ea.items())),
        "traces": [
            {
                "case_id": trace.attributes["concept:name"],
                "cost": d["cost"],
                "fitness": d["fitness"],
                "bwc": d["bwc"],
                "visited": d["visited_states"],
                "closed": d["closed"],
            }
            for trace, d in zip(log, diagnostics)
        ],
    }


def alignments_tree(fixtures: dict[str, Path]) -> dict[str, Any]:
    """Alignments against the fixture tree, or the IM tree of the top variants."""
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    if "model" in fixtures:
        tree = load_model(fixtures["model"])
    else:
        tree = pm4py.discover_process_tree_inductive(_top_k(log))
    diagnostics = pm4py.conformance_diagnostics_alignments(log, tree)
    return {
        "tree": str(tree),
        "traces": [
            {
                "case_id": trace.attributes["concept:name"],
                "cost": d["cost"],
                "fitness": d["fitness"],
            }
            for trace, d in zip(log, diagnostics)
        ],
    }


def _matched(a: tuple[str, ...], b: tuple[str, ...]) -> int:
    return sum(m.size for m in difflib.SequenceMatcher(None, a, b).get_matching_blocks())


def edit_distance_cost_choices(trace: tuple[str, ...], model: list[tuple[str, ...]]) -> list[int]:
    """Costs pm4py's edit-distance alignment can give ``trace``, one per
    closest model variant it could pick.

    pm4py takes the trace itself if it is a model variant. Otherwise it
    takes the first variant at the smallest Levenshtein distance in its
    candidate order: smallest length difference, then shortest, then an
    order set by string hashing.
    """
    if trace in model:
        candidates = [trace]
    else:
        dist = {m: string_distance.levenshtein_distance(trace, m) for m in model}
        best = min(dist.values())
        closest = [m for m in model if dist[m] == best]
        rank = min((abs(len(m) - len(trace)), len(m)) for m in closest)
        candidates = [m for m in closest if (abs(len(m) - len(trace)), len(m)) == rank]
    return sorted({(len(trace) + len(m) - 2 * _matched(trace, m)) * 10000 for m in candidates})


def alignments_edit_distance(fixtures: dict[str, Path]) -> dict[str, Any]:
    """Edit-distance alignments against the top variants."""
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    model_log = _top_k(log)
    model = sorted({_variant(t) for t in model_log})
    diagnostics = pm4py.conformance_diagnostics_alignments(log, model_log)
    traces = []
    for trace, d in zip(log, diagnostics):
        choices = edit_distance_cost_choices(_variant(trace), model)
        assert d["cost"] in choices, (d["cost"], choices)
        record = {
            "case_id": trace.attributes["concept:name"],
            "bwc": d["bwc"],
            "cost_choices": choices,
        }
        # With several choices pm4py's cost changes between runs.
        if len(choices) == 1:
            record["cost"] = d["cost"]
            record["fitness"] = d["fitness"]
        traces.append(record)
    return {"model": [list(m) for m in model], "traces": traces}


for _log_id, _log in ALIGNMENT_LOGS.items():
    case(
        f"alignments-dfg-{_log_id}",
        fixture=_log,
        functions=[*SEQUENCE_FUNCTIONS, "pm4py.discover_dfg"],
    )(alignments_dfg)
    case(
        f"alignments-tree-{_log_id}",
        fixture=_log,
        functions=[*SEQUENCE_FUNCTIONS, "pm4py.discover_process_tree_inductive"],
    )(alignments_tree)
    case(
        f"alignments-edit-distance-{_log_id}",
        fixture=_log,
        functions=SEQUENCE_FUNCTIONS,
    )(alignments_edit_distance)

case(
    "alignments-tree-running-example-ptml",
    fixtures={"log": "running-example.csv", "model": "running-example.ptml"},
    functions=["pm4py.convert_to_event_log", "pm4py.conformance_diagnostics_alignments"],
)(alignments_tree)
