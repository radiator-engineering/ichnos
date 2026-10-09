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

Cases:

- ``footprints-*``: ``{"model": ..., "footprints": pm4py.discover_footprints(...)}``.
  Footprint relations are sorted lists of ``[a, b]`` pairs. A net's footprints
  have no end activities, as in pm4py.
- ``reachability-graph-net-*``: ``{"model": ..., "states": [...], "edges":
  [[from, name, to], ...]}`` from ``pm4py.convert_to_reachability_graph``.
  pm4py states are equal when their names are, so both lists are sorted sets
  of names. Edge names are pm4py's transition ``repr``.
- ``heuristics-net-*``: ``pm4py.discover_heuristics_net`` on a log with default
  parameters, then ``pm4py.convert_to_petri_net``. ``heuristics_net``
  describes the discovered net (see :func:`describe_heuristics_net`);
  ``petri_net`` describes the converted net (see :func:`describe_converted`).
  Converted heuristics nets are often unbounded, so no footprints: pm4py's
  reachability graph does not end on them.
- ``networkx-net-*``: ``{"model": ..., "nodes": [...], "edges": [...]}`` from
  ``pm4py.convert_petri_net_to_networkx``. A node is its name plus its
  ``attr`` dictionary, sorted by type and name; an edge is ``{"source",
  "target", "weight", "type"}``, with ``type`` the ``arctype`` property
  (``"normal"`` when unset), sorted by source and target.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pm4py

from harness import case
from harness.fixtures import load_log, load_model

FUNCTIONS = ["pm4py.discover_footprints"]
RG_FUNCTIONS = ["pm4py.convert_to_reachability_graph"]


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


def net_reachability_graph(fixtures: dict[str, Path]) -> dict[str, Any]:
    net, im, fm = load_model(fixtures["model"])
    ts = pm4py.convert_to_reachability_graph(net, im, fm)
    return {
        "model": describe_net(net, im, fm),
        "states": sorted({s.name for s in ts.states}),
        "edges": sorted({(t.from_state.name, t.name, t.to_state.name) for t in ts.transitions}),
    }


def net_networkx(fixtures: dict[str, Path]) -> dict[str, Any]:
    net, im, fm = load_model(fixtures["model"])
    g = pm4py.convert_petri_net_to_networkx(net, im, fm)
    nodes = sorted(
        ({"name": n, **d["attr"]} for n, d in g.nodes(data=True)),
        key=lambda d: (d["type"], d["name"]),
    )
    edges = sorted(
        (
            {
                "source": s,
                "target": t,
                "weight": d["attr"]["weight"],
                "type": d["attr"]["properties"].get("arctype", "normal"),
            }
            for s, t, d in g.edges(data=True)
        ),
        key=lambda e: (e["source"], e["target"]),
    )
    return {"model": describe_net(net, im, fm), "nodes": nodes, "edges": edges}


def _matrix(m: dict[Any, dict[Any, Any]]) -> list[list[Any]]:
    return sorted([a, b, v] for a, row in m.items() for b, v in row.items())


def _edges(conns: dict[Any, list[Any]]) -> list[dict[str, Any]]:
    return [
        {"target": other.node_name, "dependency": e.dependency_value, "frequency": e.dfg_value}
        for other, edges in conns.items()
        for e in edges
    ]


def describe_heuristics_net(h: Any) -> dict[str, Any]:
    """The discovered heuristics net. Nodes keep pm4py's dict order."""
    return {
        "activities": sorted(h.activities),
        "activities_occurrences": dict(h.activities_occurrences),
        "start_activities": [dict(x) for x in h.start_activities],
        "end_activities": [dict(x) for x in h.end_activities],
        "dfg_matrix": _matrix(h.dfg_matrix),
        "dependency_matrix": _matrix(h.dependency_matrix),
        "freq_triples_matrix": _matrix(h.freq_triples_matrix),
        "nodes": [
            {
                "name": n.node_name,
                "occurrences": n.node_occ,
                "outputs": _edges(n.output_connections),
                "inputs": _edges(n.input_connections),
                "and_measures_in": _matrix(n.and_measures_in),
                "and_measures_out": _matrix(n.and_measures_out),
                "loop_length_two": dict(n.loop_length_two),
            }
            for n in h.nodes.values()
        ],
    }


def describe_converted(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    """A converted net without pm4py's counter-based silent names (``hid_<n>``).

    Place names and visible transition names come from activity names, so
    they are part of the result. Each transition is its label (``null`` when
    silent) with the sorted names of its input and output places.
    """
    return {
        "places": sorted(p.name for p in net.places),
        "transitions": sorted(
            (
                {
                    "label": t.label,
                    "preset": sorted(a.source.name for a in t.in_arcs),
                    "postset": sorted(a.target.name for a in t.out_arcs),
                }
                for t in net.transitions
            ),
            key=lambda t: (t["label"] or "", t["preset"], t["postset"]),
        ),
        "arcs": len(net.arcs),
        "initial_marking": {p.name: n for p, n in im.items()},
        "final_marking": {p.name: n for p, n in fm.items()},
    }


def heuristics_net(fixtures: dict[str, Path]) -> dict[str, Any]:
    log = load_log(fixtures["log"])
    h = pm4py.discover_heuristics_net(log)
    net, im, fm = pm4py.convert_to_petri_net(h)
    return {
        "heuristics_net": describe_heuristics_net(h),
        "petri_net": describe_converted(net, im, fm),
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

# roadtraffic is left out: its reachability graph golden is over 3 MB.
for name in [n for n in NETS if n != "roadtraffic"]:
    case_id = "reachability-graph-net-" + name.lower().replace("_", "-")
    case(case_id, fixtures={"model": f"{name}.pnml"}, functions=RG_FUNCTIONS)(net_reachability_graph)

for name in [*NETS, "inh_res_nets/cyber_incident_response"]:
    case_id = "networkx-net-" + name.rsplit("/", 1)[-1].lower().replace("_", "-")
    case(
        case_id,
        fixtures={"model": f"{name}.pnml"},
        functions=["pm4py.convert_petri_net_to_networkx"],
    )(net_networkx)

for case_id, fixture in [
    ("heuristics-net-running-example", "running-example.xes"),
    ("heuristics-net-receipt", "receipt.xes"),
    ("heuristics-net-roadtraffic100traces", "roadtraffic100traces.xes"),
    ("heuristics-net-helpdesk", "helpdesk.xes.gz"),
]:
    case(
        case_id,
        fixture=fixture,
        functions=[
            "pm4py.discover_heuristics_net",
            "pm4py.convert_to_petri_net",
        ],
    )(heuristics_net)
