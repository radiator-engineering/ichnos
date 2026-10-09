"""Area ``model``: footprints of the Petri nets and process trees in ``fixtures/logs``.

ichnos has no PNML or PTML reader yet, so each case also emits the input
model in a form a Rust test can build directly:

- Petri net: ``{"kind": "petri_net", "places": [...], "transitions":
  [{"name", "label"}], "arcs": [{"source", "target", "weight", "type"}],
  "initial_marking": {place: n}, "final_marking": {place: n}}``. Lists are
  sorted by name, then by (source, target). ``label`` is ``null`` for a silent
  transition. ``type`` is pm4py's ``arctype`` property, ``"normal"`` when unset.
- Process tree: ``{"kind": "process_tree", "tree": str(tree)}``, pm4py's
  ``to_string`` form.

Expected value: ``{"model": ..., "footprints": pm4py.discover_footprints(...)}``.
Footprint relations are sorted lists of ``[a, b]`` pairs. A net's footprints
have no end activities, as in pm4py.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pm4py

from harness import case
from harness.fixtures import load_model

FUNCTIONS = ["pm4py.discover_footprints"]


def _node_name(node: Any) -> str:
    return node.name


def describe_net(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    places = sorted(p.name for p in net.places)
    transitions = sorted(
        ({"name": t.name, "label": t.label} for t in net.transitions),
        key=lambda t: t["name"],
    )
    if len(set(places)) != len(places) or len({t["name"] for t in transitions}) != len(transitions):
        raise ValueError(f"net {net.name!r} has duplicate place or transition names")
    arcs = sorted(
        (
            {
                "source": _node_name(a.source),
                "target": _node_name(a.target),
                "weight": a.weight,
                "type": a.properties.get("arctype", "normal"),
            }
            for a in net.arcs
        ),
        key=lambda a: (a["source"], a["target"]),
    )
    return {
        "kind": "petri_net",
        "places": places,
        "transitions": transitions,
        "arcs": arcs,
        "initial_marking": {p.name: n for p, n in im.items()},
        "final_marking": {p.name: n for p, n in fm.items()},
    }


def net_footprints(fixtures: dict[str, Path]) -> dict[str, Any]:
    net, im, fm = load_model(fixtures["model"])
    return {
        "model": describe_net(net, im, fm),
        "footprints": pm4py.discover_footprints(net, im, fm),
    }


def tree_footprints(fixtures: dict[str, Path]) -> dict[str, Any]:
    tree = load_model(fixtures["model"])
    return {
        "model": {"kind": "process_tree", "tree": str(tree)},
        "footprints": pm4py.discover_footprints(tree),
    }


NETS = [
    "big_wf_net",
    "data_petri_net",
    "ex1",
    "ex2",
    "murata1",
    "murata2",
    "murata3",
    "receipt_one_variant",
    "roadtraffic",
    "running-example",
    "stochastic_running_example",
]
TREES = ["running-example", "tree_ex_with_loops", "tree_ex_wo_loops"]

for name in NETS:
    case_id = "footprints-net-" + name.lower().replace("_", "-")
    case(case_id, fixtures={"model": f"{name}.pnml"}, functions=FUNCTIONS)(net_footprints)

for name in TREES:
    case_id = "footprints-tree-" + name.lower().replace("_", "-")
    case(case_id, fixtures={"model": f"{name}.ptml"}, functions=FUNCTIONS)(tree_footprints)
