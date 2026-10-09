"""Area ``viz``: pm4py's Graphviz visualisations.

Each case calls a ``pm4py.save_vis_*`` function with the visualizer's
``save`` replaced, so it records the ``gviz.source`` that pm4py would pass
to Graphviz. Node names in that source are Python object ids, string hashes
or random UUIDs, which change from run to run. The golden therefore keeps
the parsed graph, not the text:

``{"model": input, "dot": {"directed", "strict", "graph": {attr: value},
"nodes": [{"key", "attrs"}], "edges": [[source key, target key, attrs]]}}``

``graph`` merges the graph attributes of the top level. A node's ``attrs``
include the defaults that earlier ``node [...]`` statements set, and a node
inside a subgraph gets the attribute ``subgraph`` with that subgraph's
attributes as a JSON string. HTML labels keep their angle brackets; quoted
strings lose their quotes. Keys ``n0``, ``n1``, ... come from colour
refinement on the attributes, as in ``bpmn.py``. The Rust tests parse
ichnos's DOT the same way and compare the two graphs up to isomorphism.

With ``order``, the golden also keeps the order of pm4py's statements,
which guides the Graphviz layout: ``"order": {"nodes": [key], "edges":
[index]}``, the node keys in declaration order and, for each edge
statement in turn, its index in ``edges``. Only cases whose order does not
depend on Python's set order record it. BPMN cases record only the node
order: pm4py adds flows in the order of the diagram's edge shapes, which
ichnos does not read.

``model`` describes the input:

- ``petri-net-*``: a net as ``model.py`` describes it. The transitions of
  a discovered net are renamed ``t<n>``, since pm4py names them with random
  UUIDs. ``guards`` maps transition names to their data-net guards.
- ``dfg-*``: ``{"graph": [[a, b, count]], "start": {a: count}, "end": {a:
  count}}`` from ``pm4py.discover_dfg``, with ``activities_count`` when
  the case passes one.
- ``performance-dfg-*``: as ``dfg-*``, with ``{"mean", "median", "min",
  "max", "sum", "stdev"}`` in place of the count, from
  ``pm4py.discover_performance_dfg``, with ``serv_time`` when the case
  passes one.
- ``process-tree-*``: ``{"tree": str(tree)}`` from
  ``pm4py.discover_process_tree_inductive``.
- ``bpmn-*``: ``null``; the diagram is the ``model`` fixture.
- ``heuristics-net-*``: the net as ``model.py`` describes it, from
  ``pm4py.discover_heuristics_net``.
"""

from __future__ import annotations

import json
import re
import tempfile
from pathlib import Path
from typing import Any

import pm4py
from pm4py.util import constants

from harness import case
from harness.fixtures import load_log, load_model

PUNCT = "[]{}=;,"


def _tokens(src: str) -> list[tuple[str, str]]:
    """Tokens of the DOT subset the ``graphviz`` package writes."""
    out: list[tuple[str, str]] = []
    i = 0
    while i < len(src):
        c = src[i]
        if c.isspace():
            i += 1
        elif src.startswith(("->", "--"), i):
            out.append(("op", src[i : i + 2]))
            i += 2
        elif c in PUNCT:
            out.append(("op", c))
            i += 1
        elif c == '"':
            j = i + 1
            buf = []
            while src[j] != '"':
                if src[j] == "\\" and src[j + 1] == '"':
                    buf.append('"')
                    j += 2
                else:
                    buf.append(src[j])
                    j += 1
            out.append(("id", "".join(buf)))
            i = j + 1
        elif c == "<":
            depth, j = 0, i
            while True:
                depth += {"<": 1, ">": -1}.get(src[j], 0)
                j += 1
                if depth == 0:
                    break
            out.append(("id", src[i:j]))
            i = j
        else:
            j = i
            while j < len(src) and not src[j].isspace() and src[j] not in PUNCT + '"<':
                if src.startswith(("->", "--"), j):
                    break
                j += 1
            out.append(("id", src[i:j]))
            i = j
    return out


def parse_dot(src: str) -> dict[str, Any]:
    """The graph a DOT text describes, with node defaults resolved."""
    toks = _tokens(src)
    pos = 0

    def peek(k: int = 0) -> tuple[str, str] | None:
        return toks[pos + k] if pos + k < len(toks) else None

    def take() -> tuple[str, str]:
        nonlocal pos
        pos += 1
        return toks[pos - 1]

    def attr_list() -> dict[str, str]:
        attrs: dict[str, str] = {}
        while peek() == ("op", "["):
            take()
            while peek() != ("op", "]"):
                if peek() in (("op", ","), ("op", ";")):
                    take()
                    continue
                k = take()[1]
                assert take() == ("op", "=")
                attrs[k] = take()[1]
            take()
        return attrs

    strict = False
    if peek() == ("id", "strict"):
        take()
        strict = True
    kind = take()[1]
    if peek() != ("op", "{"):
        take()
    assert take() == ("op", "{")
    graph: dict[str, str] = {}
    nodes: dict[str, dict[str, str]] = {}
    edges: list[tuple[str, str, dict[str, str]]] = []
    # One frame per open graph or subgraph: (graph attrs, node defaults).
    frames: list[tuple[dict[str, str], dict[str, str]]] = [(graph, {})]
    while frames:
        t = take()
        if t == ("op", "}"):
            frames.pop()
            continue
        if t == ("op", ";"):
            continue
        gattrs, defaults = frames[-1]
        if t == ("id", "subgraph"):
            if peek() != ("op", "{"):
                take()
            assert take() == ("op", "{")
            frames.append(({}, dict(defaults)))
            continue
        if t[1] in ("graph", "node", "edge") and peek() == ("op", "["):
            attrs = attr_list()
            if t[1] == "graph":
                gattrs.update(attrs)
            elif t[1] == "node":
                defaults.update(attrs)
            continue
        if peek() == ("op", "="):
            take()
            gattrs[t[1]] = take()[1]
            continue
        if peek() is not None and peek()[1] in ("->", "--"):
            take()
            target = take()[1]
            edges.append((t[1], target, attr_list()))
            continue
        attrs = dict(defaults)
        attrs.update(attr_list())
        # The graphviz package writes a subgraph's attributes before its
        # nodes, so they are complete here.
        if len(frames) > 1:
            attrs["subgraph"] = json.dumps(frames[-1][0], sort_keys=True)
        nodes[t[1]] = attrs
    return {"strict": strict, "kind": kind, "graph": graph, "nodes": nodes, "edges": edges}


def canonical_dot(src: str, order: bool | str = False) -> dict[str, Any]:
    """``order`` is ``True`` to record the node and edge order, ``"nodes"``
    for the node order alone."""
    p = parse_dot(src)
    names = list(p["nodes"])
    label = {n: json.dumps(p["nodes"][n], sort_keys=True) for n in names}
    edges = [(s, t, json.dumps(a, sort_keys=True)) for s, t, a in p["edges"]]
    missing = {x for s, t, _ in edges for x in (s, t)} - set(names)
    if missing:
        raise ValueError(f"edges name undeclared nodes {sorted(missing)}")
    color = dict(label)
    classes = len(set(color.values()))
    while True:
        sig = {
            n: repr(
                (
                    color[n],
                    sorted((c, color[t]) for s, t, c in edges if s == n),
                    sorted((c, color[s]) for s, t, c in edges if t == n),
                )
            )
            for n in names
        }
        ranks = {s: f"{i:05d}" for i, s in enumerate(sorted(set(sig.values())))}
        new = {n: ranks[sig[n]] for n in names}
        if len(set(new.values())) == classes:
            break
        color, classes = new, len(set(new.values()))
    ranked = sorted(names, key=lambda n: (color[n], label[n]))
    key = {n: f"n{i}" for i, n in enumerate(ranked)}
    declared = [[key[s], key[t], a] for s, t, a in p["edges"]]
    out = {
        "directed": p["kind"] == "digraph",
        "strict": p["strict"],
        "graph": p["graph"],
        "nodes": [{"key": key[n], "attrs": p["nodes"][n]} for n in ranked],
        "edges": sorted(declared, key=json.dumps),
    }
    if order:
        index = {json.dumps(e): i for i, e in reversed(list(enumerate(out["edges"])))}
        out["order"] = {"nodes": [key[n] for n in names]}
        if order is True:
            out["order"]["edges"] = [index[json.dumps(e)] for e in declared]
    return out


def _capture(module: Any, call: Any) -> str:
    """Runs ``call`` with ``module.save`` recording the source it is given."""
    sources: list[str] = []
    original = module.save
    module.save = lambda gviz, *args, **kwargs: sources.append(gviz.source)
    try:
        with tempfile.TemporaryDirectory() as tmp:
            call(str(Path(tmp) / "out.svg"))
    finally:
        module.save = original
    if len(sources) != 1:
        raise RuntimeError(f"expected one saved graph, got {len(sources)}")
    return sources[0]


def _styled(params: dict[str, Any]) -> dict[str, Any]:
    return {k: params[k] for k in ("bgcolor", "rankdir", "graph_title") if k in params}


UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")


def _name_transitions(net: Any) -> None:
    """Renames transitions with random UUID names to ``t<n>``.

    They are numbered in the order of their label (silent first), then
    their sorted input and output place names. Two that tie are
    interchangeable.
    """
    named = [t for t in net.transitions if UUID.match(t.name)]
    named.sort(
        key=lambda t: (
            t.label is not None,
            t.label or "",
            sorted(a.source.name for a in t.in_arcs),
            sorted(a.target.name for a in t.out_arcs),
        )
    )
    for i, t in enumerate(named):
        t.name = f"t{i}"


def _decorations(net: Any, spec: dict[str, Any] | None) -> dict[Any, Any] | None:
    """pm4py's decorations from ``{"places": {name: deco}, "transitions":
    {name: deco}, "arcs": [[source, target, deco]]}``."""
    if spec is None:
        return None
    places = {p.name: p for p in net.places}
    transitions = {t.name: t for t in net.transitions}
    arcs = {(a.source.name, a.target.name): a for a in net.arcs}
    out: dict[Any, Any] = {}
    out.update({places[n]: d for n, d in spec.get("places", {}).items()})
    out.update({transitions[n]: d for n, d in spec.get("transitions", {}).items()})
    out.update({arcs[(s, t)]: d for s, t, d in spec.get("arcs", [])})
    return out


def _sequence_net() -> tuple[Any, Any, Any]:
    """A sequence a, b, c, d, e whose names sort against the firing order,
    plus a place no marking reaches, so the drawing order differs from the
    name order."""
    from pm4py.objects.petri_net.obj import Marking, PetriNet
    from pm4py.objects.petri_net.utils import petri_utils

    net = PetriNet("sequence")
    places = [PetriNet.Place(f"p{9 - i}") for i in range(6)]
    transitions = [PetriNet.Transition(f"t{9 - i}", a) for i, a in enumerate("abcde")]
    for x in places + [PetriNet.Place("p0")]:
        net.places.add(x)
    for t in transitions:
        net.transitions.add(t)
    for i, t in enumerate(transitions):
        petri_utils.add_arc_from_to(places[i], t, net)
        petri_utils.add_arc_from_to(t, places[i + 1], net)
    unreached = next(p for p in net.places if p.name == "p0")
    petri_utils.add_arc_from_to(unreached, transitions[2], net)
    return net, Marking({places[0]: 1}), Marking({places[-1]: 1})


def petri_net(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from cases.model import describe_net
    from pm4py.objects.petri_net import properties
    from pm4py.visualization.petri_net import visualizer

    if params.get("sequence"):
        net, im, fm = _sequence_net()
    elif "model" in fixtures:
        net, im, fm = load_model(fixtures["model"])
    else:
        net, im, fm = pm4py.discover_petri_net_inductive(load_log(fixtures["log"]))
        _name_transitions(net)
    decorations = _decorations(net, params.get("decorations"))
    debug = params.get("debug", False)
    if "font_size" in params:
        # ``save_vis_petri_net`` does not pass the font size on.
        parameters = {
            "format": "svg",
            "font_size": params["font_size"],
            "decorations": decorations,
            "debug": debug,
        }
        source = visualizer.apply(net, im, fm, parameters=parameters).source
    else:
        source = _capture(
            visualizer,
            lambda path: pm4py.save_vis_petri_net(
                net, im, fm, path, decorations=decorations, debug=debug, **_styled(params)
            ),
        )
    model = describe_net(net, im, fm)
    guards = {
        t.name: t.properties[properties.TRANS_GUARD]
        for t in net.transitions
        if properties.TRANS_GUARD in t.properties
    }
    if guards:
        model["guards"] = dict(sorted(guards.items()))
    return {"model": model, "dot": canonical_dot(source, order=bool(params.get("sequence")))}


def dfg(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from pm4py.visualization.dfg import visualizer

    graph, start, end = pm4py.discover_dfg(load_log(fixtures["log"]))
    # pm4py writes start and end edges in dict order; ichnos keeps them
    # sorted by activity.
    start, end = dict(sorted(start.items())), dict(sorted(end.items()))
    model = {
        "graph": sorted([a, b, n] for (a, b), n in graph.items()),
        "start": dict(start),
        "end": dict(end),
    }
    if params.get("activities_count"):
        # ``save_vis_dfg`` does not take activity counts or a font size.
        from pm4py.visualization.dfg.variants.frequency import Parameters

        counts = {a: 3 * len(a) for a in sorted({x for e in graph for x in e})}
        model["activities_count"] = counts
        parameters = {
            Parameters.FORMAT: "svg",
            Parameters.START_ACTIVITIES: start,
            Parameters.END_ACTIVITIES: end,
            Parameters.FONT_SIZE: params["font_size"],
        }
        source = visualizer.apply(dict(graph), activities_count=counts, parameters=parameters).source
    else:
        source = _capture(
            visualizer,
            lambda path: pm4py.save_vis_dfg(dict(graph), start, end, path, **params),
        )
    return {"model": model, "dot": canonical_dot(source, order=True)}


MEASURES = ["mean", "median", "min", "max", "sum", "stdev"]


def performance_dfg(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from pm4py.visualization.dfg import visualizer

    graph, start, end = pm4py.discover_performance_dfg(load_log(fixtures["log"]))
    start, end = dict(sorted(start.items())), dict(sorted(end.items()))
    model = {
        "graph": sorted([a, b, {m: v[m] for m in MEASURES}] for (a, b), v in graph.items()),
        "start": dict(start),
        "end": dict(end),
    }
    kwargs = dict(params)
    if kwargs.pop("business_hours", False):
        kwargs["business_hour_slots"] = constants.DEFAULT_BUSINESS_HOUR_SLOTS
    if kwargs.pop("serv_time", False):
        kwargs["serv_time"] = {
            a: 3600.0 * len(a) for a in sorted({x for e in graph for x in e})
        }
        model["serv_time"] = kwargs["serv_time"]
    source = _capture(
        visualizer,
        lambda path: pm4py.save_vis_performance_dfg(graph, start, end, path, **kwargs),
    )
    return {"model": model, "dot": canonical_dot(source, order=True)}


def process_tree(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from pm4py.visualization.process_tree import visualizer

    tree = pm4py.discover_process_tree_inductive(load_log(fixtures["log"]))
    if params:
        # ``save_vis_process_tree`` passes neither option on.
        parameters = {"format": "svg", **params}
        source = visualizer.apply(tree, parameters=parameters).source
    else:
        source = _capture(visualizer, lambda path: pm4py.save_vis_process_tree(tree, path))
    return {"model": {"tree": str(tree)}, "dot": canonical_dot(source)}


def bpmn(fixtures: dict[str, Path], order: bool = True, **params: Any) -> dict[str, Any]:
    """``order=False`` for diagrams whose node order pm4py takes from a
    set."""
    from pm4py.visualization.bpmn import visualizer

    diagram = load_model(fixtures["model"])
    if params:
        # ``save_vis_bpmn`` passes none of these options on.
        parameters = {"format": "svg", **params}
        source = visualizer.apply(diagram, parameters=parameters).source
    else:
        source = _capture(visualizer, lambda path: pm4py.save_vis_bpmn(diagram, path))
    return {"model": None, "dot": canonical_dot(source, order="nodes" if order else False)}


def heuristics_net(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from cases.model import describe_heuristics_net
    from pm4py.algo.discovery.heuristics import algorithm as heuristics_miner
    from pm4py.visualization.heuristics_net import visualizer

    log = load_log(fixtures["log"])
    if params:
        # ``discover_heuristics_net`` does not take ``min_dfg_occurrences``.
        net = heuristics_miner.apply_heu(log, parameters=params)
    else:
        net = pm4py.discover_heuristics_net(log)
    source = _capture(visualizer, lambda path: pm4py.save_vis_heuristics_net(net, path))
    return {"model": describe_heuristics_net(net), "dot": canonical_dot(source)}


LOGS = {"running-example": "running-example.xes", "receipt": "receipt.xes"}

for name, log in LOGS.items():
    case(
        f"petri-net-{name}",
        fixture=log,
        functions=["pm4py.discover_petri_net_inductive", "pm4py.save_vis_petri_net"],
    )(petri_net)
    case(
        f"dfg-{name}",
        fixture=log,
        functions=["pm4py.discover_dfg", "pm4py.save_vis_dfg"],
    )(dfg)
    case(
        f"performance-dfg-{name}",
        fixture=log,
        functions=["pm4py.discover_performance_dfg", "pm4py.save_vis_performance_dfg"],
    )(performance_dfg)
    case(
        f"process-tree-{name}",
        fixture=log,
        functions=["pm4py.discover_process_tree_inductive", "pm4py.save_vis_process_tree"],
    )(process_tree)
    case(
        f"bpmn-{name}",
        fixtures={"model": f"{name}.bpmn"},
        functions=["pm4py.read_bpmn", "pm4py.save_vis_bpmn"],
    )(bpmn)
    case(
        f"heuristics-net-{name}",
        fixture=log,
        functions=["pm4py.discover_heuristics_net", "pm4py.save_vis_heuristics_net"],
    )(heuristics_net)

case(
    "petri-net-running-example-styled",
    fixture="running-example.xes",
    functions=["pm4py.discover_petri_net_inductive", "pm4py.save_vis_petri_net"],
    params={"bgcolor": "#ffeedd", "rankdir": "TB", "graph_title": "Running example"},
)(petri_net)
case(
    "petri-net-inh_res_nets-order_fulfillment",
    fixtures={"model": "inh_res_nets/order_fulfillment.pnml"},
    functions=["pm4py.read_pnml", "pm4py.save_vis_petri_net"],
)(petri_net)
# Below 6 edges pm4py drops an end activity's node and then fails on its
# arc, so the case keeps 8 of the 16 edges.
case(
    "dfg-running-example-max-edges-8",
    fixture="running-example.xes",
    functions=["pm4py.discover_dfg", "pm4py.save_vis_dfg"],
    params={"max_num_edges": 8},
)(dfg)
case(
    "performance-dfg-receipt-median",
    fixture="receipt.xes",
    functions=["pm4py.discover_performance_dfg", "pm4py.save_vis_performance_dfg"],
    params={"aggregation_measure": "median"},
)(performance_dfg)
case(
    "performance-dfg-running-example-business-hours",
    fixture="running-example.xes",
    functions=["pm4py.discover_performance_dfg", "pm4py.save_vis_performance_dfg"],
    params={"business_hours": True},
)(performance_dfg)
case(
    "petri-net-sequence",
    functions=["pm4py.save_vis_petri_net"],
    params={"sequence": True},
)(petri_net)
case(
    "petri-net-data_petri_net",
    fixtures={"model": "data_petri_net.pnml"},
    functions=["pm4py.read_pnml", "pm4py.save_vis_petri_net"],
)(petri_net)
case(
    "petri-net-inh_res_nets-order_fulfillment-debug",
    fixtures={"model": "inh_res_nets/order_fulfillment.pnml"},
    functions=["pm4py.read_pnml", "pm4py.visualization.petri_net.visualizer.apply"],
    params={"font_size": 16, "debug": True},
)(petri_net)
case(
    "petri-net-inh_res_nets-order_fulfillment-decorated",
    fixtures={"model": "inh_res_nets/order_fulfillment.pnml"},
    functions=["pm4py.read_pnml", "pm4py.save_vis_petri_net"],
    params={
        "decorations": {
            "places": {
                "gate": {"label": "gate", "color": "#ffcc00"},
                "joined": {"color": "#00ccff"},
            },
            "transitions": {
                "check": {"label": "Check payment", "color": "#ccffcc"},
                "join": {"color": "#ffcccc"},
            },
            "arcs": [
                ["a1", "branch_a_middle", {"label": "x2", "color": "red", "penwidth": "3"}],
                ["authorized", "authorize", {"color": "blue"}],
            ],
        }
    },
)(petri_net)
case(
    "dfg-running-example-activities-count",
    fixture="running-example.xes",
    functions=["pm4py.discover_dfg", "pm4py.visualization.dfg.visualizer.apply"],
    params={"activities_count": True, "font_size": 14},
)(dfg)
case(
    "performance-dfg-running-example-serv-time",
    fixture="running-example.xes",
    functions=["pm4py.discover_performance_dfg", "pm4py.save_vis_performance_dfg"],
    params={"serv_time": True},
)(performance_dfg)
case(
    "process-tree-running-example-unsorted",
    fixture="running-example.xes",
    functions=[
        "pm4py.discover_process_tree_inductive",
        "pm4py.visualization.process_tree.visualizer.apply",
    ],
    params={"enable_deepcopy": False, "font_size": 20},
)(process_tree)
case(
    "bpmn-all_kinds",
    fixtures={"model": "synthetic-bpmn/all_kinds.bpmn"},
    functions=["pm4py.read_bpmn", "pm4py.save_vis_bpmn"],
    params={"order": False},
)(bpmn)
case(
    "bpmn-all_kinds-options",
    fixtures={"model": "synthetic-bpmn/all_kinds.bpmn"},
    functions=["pm4py.read_bpmn", "pm4py.visualization.bpmn.visualizer.apply"],
    params={
        "include_name_in_events": False,
        "endpoints_shape": "box",
        "swimlanes_margin": 10,
        "font_size": 14,
        "order": False,
    },
)(bpmn)
case(
    "bpmn-all_kinds-no-swimlanes",
    fixtures={"model": "synthetic-bpmn/all_kinds.bpmn"},
    functions=["pm4py.read_bpmn", "pm4py.visualization.bpmn.visualizer.apply"],
    params={"enable_swimlanes": False, "order": False},
)(bpmn)
case(
    "heuristics-net-running-example-min-dfg-3",
    fixture="running-example.xes",
    functions=[
        "pm4py.algo.discovery.heuristics.algorithm.apply_heu",
        "pm4py.save_vis_heuristics_net",
    ],
    params={"min_dfg_occurrences": 3},
)(heuristics_net)
