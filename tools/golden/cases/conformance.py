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
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import pm4py
from pm4py.algo.evaluation.precision.variants import align_etconformance
from pm4py.objects.petri_net import semantics

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
