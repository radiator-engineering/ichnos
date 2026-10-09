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
attributes as a JSON string. Below the first level of subgraphs it also gets
``subgraph_depth``, the number of enclosing subgraphs. An edge's ``lhead``
and ``ltail`` name a cluster; they become ``cluster@<depth>``. An ``image``
keeps only its file name. HTML labels keep their angle brackets; quoted
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
- ``transition-system-*``: ``{"states": [name], "transitions": [[from,
  to, name]]}`` from ``pm4py.discover_transition_system``, with states as
  indices into ``states``.
- ``prefix-tree-*``: ``{"nodes": [{"label", "parent", "final",
  "depth"}]}`` from ``pm4py.discover_prefix_tree``, the root first and then
  breadth first, children in label order; ``parent`` is an index.
- ``footprints-*``: ``{"sequence": [[a, b]], "parallel": [[a, b]]}`` from
  ``pm4py.discover_footprints``; comparisons hold two, as ``first`` and
  ``second``.
- ``alignments-*``: ``{"rows": [{"activities", "count", "moves"}]}``, one
  row per variant in order of its first trace. ``moves`` are the
  ``[log, model]`` label pairs of the variant's first trace, with ``>>``
  and ``null`` kept; ``alignments`` says how they are built.
- ``powl-*``: the model as ``powl.py`` describes it, before pm4py's
  ``simplify_using_frequent_transitions``. Children are sorted, so the
  description does not depend on set order.
- ``ocdfg-*``: ``{"activities": {a: counts}, "start": {ot: {a: counts}},
  "end": ..., "edges": {ot: [[a, b, counts, event-couple durations,
  object durations]]}, "colors": {ot: colour}}`` from
  ``pm4py.discover_ocdfg``, where ``counts`` is ``[events, unique objects,
  total objects]``.
- ``ocpn-*``: ``{"activities", "nets": {ot: {"net", "double_arcs",
  "diagnostics"}}, "colors"}`` from ``pm4py.discover_oc_petri_net``, each
  net as ``model.py`` describes it with transitions renamed ``t<n>``.
  ``diagnostics`` holds the replay counts ``[p, m, c, r]`` by place and the
  firing counts by transition, or ``null``.
- ``object-graph-*``: ``{"objects": [[id, type]], "graph": [[a, b]],
  "colors"}`` from ``pm4py.discover_objects_graph``.
- ``network-analysis-*``: ``{"edges": [[source, target, [[value,
  measure]]]]}`` from ``pm4py.discover_network_analysis``, in pm4py's
  order, which its pen widths depend on. The measure is a count, or the
  list of durations for ``performance``.
- ``dotted-chart-*``: ``{"attributes", "colors": {str(value): colour}}``;
  the points come from the ``log`` fixture.
- ``performance-spectrum-*``: ``{"activities", "points"}`` from pm4py's
  ``log`` variant of the discovery, on the ``log`` fixture.

The OCEL drawings colour object types by Python's string hash, and the
dotted chart picks colours at random. The cases replace both with fixed
colours and record them. The dotted chart and the performance spectrum
write their DOT file and run ``neato`` themselves; the cases record that
file, with the time zone set to UTC for the spectrum's dates.
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
    # Nesting depth of each subgraph, by name.
    depth: dict[str, int] = {}
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
                depth[take()[1]] = len(frames)
            assert take() == ("op", "{")
            frames.append(({}, dict(defaults)))
            continue
        if t[1] in ("graph", "node", "edge"):
            # ``attr("node")`` writes the keyword alone, which sets nothing.
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
            attrs = attr_list()
            for k in ("lhead", "ltail"):
                if k in attrs:
                    attrs[k] = f"cluster@{depth[attrs[k]]}"
            edges.append((t[1], target, attrs))
            continue
        attrs = dict(defaults)
        attrs.update(attr_list())
        if "image" in attrs:
            attrs["image"] = Path(attrs["image"]).name
        # The graphviz package writes a subgraph's attributes before its
        # nodes, so they are complete here.
        if len(frames) > 1:
            attrs["subgraph"] = json.dumps(frames[-1][0], sort_keys=True)
        if len(frames) > 2:
            attrs["subgraph_depth"] = str(len(frames) - 1)
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


def transition_system(fixtures: dict[str, Path]) -> dict[str, Any]:
    from pm4py.visualization.transition_system import visualizer

    ts = pm4py.discover_transition_system(load_log(fixtures["log"]))
    states = sorted(ts.states, key=lambda s: str(s.name))
    index = {id(s): i for i, s in enumerate(states)}
    model = {
        "states": [str(s.name) for s in states],
        "transitions": sorted(
            [index[id(t.from_state)], index[id(t.to_state)], str(t.name)]
            for t in ts.transitions
        ),
    }
    source = _capture(visualizer, lambda path: pm4py.save_vis_transition_system(ts, path))
    return {"model": model, "dot": canonical_dot(source)}


def prefix_tree(fixtures: dict[str, Path]) -> dict[str, Any]:
    from pm4py.visualization.trie import visualizer

    trie = pm4py.discover_prefix_tree(load_log(fixtures["log"]))
    nodes: list[dict[str, Any]] = []
    queue = [(trie, None)]
    while queue:
        node, parent = queue.pop(0)
        here = len(nodes)
        nodes.append(
            {"label": node.label, "parent": parent, "final": node.final, "depth": node.depth}
        )
        queue.extend((c, here) for c in sorted(node.children, key=lambda c: c.label))
    source = _capture(visualizer, lambda path: pm4py.save_vis_prefix_tree(trie, path))
    return {"model": {"nodes": nodes}, "dot": canonical_dot(source)}


def _footprints(fp: dict[str, Any]) -> dict[str, Any]:
    return {k: sorted(list(x) for x in fp[k]) for k in ("sequence", "parallel")}


def footprints(fixtures: dict[str, Path], compare: str | None = None) -> dict[str, Any]:
    """Footprints of the log, or with ``compare`` set, the log's against
    those of its inductive net (``net``) or of the log without the activity
    ``compare`` names."""
    from pm4py.visualization.footprints import visualizer

    log = load_log(fixtures["log"])
    fp = pm4py.discover_footprints(log)
    if compare is None:
        source = _capture(visualizer, lambda path: pm4py.save_vis_footprints(fp, path))
        return {"model": _footprints(fp), "dot": canonical_dot(source)}
    if compare == "net":
        other = pm4py.discover_footprints(*pm4py.discover_petri_net_inductive(log))
    else:
        other = pm4py.discover_footprints(
            pm4py.filter_event_attribute_values(
                log, "concept:name", [compare], level="event", retain=False
            )
        )
    source = _capture(visualizer, lambda path: pm4py.save_vis_footprints((fp, other), path))
    model = {"first": _footprints(fp), "second": _footprints(other)}
    return {"model": model, "dot": canonical_dot(source)}


def alignments(fixtures: dict[str, Path]) -> dict[str, Any]:
    """The alignment table of the log, with alignments built from each
    trace rather than searched, since pm4py's search breaks ties
    differently from run to run. Activities in even positions of the sorted
    activity list are synchronous moves, the others log moves, and every
    alignment ends with a silent model move and a model move on ``end >
    model``, a label pm4py escapes."""
    from pm4py.statistics.variants.log import get as variants_get
    from pm4py.visualization.align_table import visualizer

    log = pm4py.convert_to_event_log(load_log(fixtures["log"]))
    order = sorted({e["concept:name"] for t in log for e in t})
    sync = set(order[::2])
    aligned = [
        {
            "alignment": [
                (a, a) if a in sync else (a, ">>")
                for a in (e["concept:name"] for e in t)
            ]
            + [(">>", None), (">>", "end > model")]
        }
        for t in log
    ]
    variants = variants_get.get_variants_from_log_trace_idx(log)
    rows = [
        {
            "activities": list(v),
            "count": len(idx),
            "moves": [list(m) for m in aligned[idx[0]]["alignment"]],
        }
        for v, idx in sorted(variants.items(), key=lambda x: x[1][0])
    ]
    source = _capture(visualizer, lambda path: pm4py.save_vis_alignments(log, aligned, path))
    return {"model": {"rows": rows}, "dot": canonical_dot(source)}


def _sorted_powl(p: Any) -> Any:
    """A copy of a pm4py POWL model with the children of choices and partial
    orders sorted by their description, keeping the order relation.

    pm4py's discovery orders children by Python's set order. The drawing
    joins an edge into a cluster to the cluster's first child, so the
    golden draws this copy to stay the same under every hash seed."""
    from cases.powl import describe_powl
    from pm4py.objects.powl.obj import OperatorPOWL, StrictPartialOrder
    from pm4py.objects.process_tree.obj import Operator

    def key(c: Any) -> str:
        return json.dumps(describe_powl(c), sort_keys=True)

    if isinstance(p, OperatorPOWL):
        children = [_sorted_powl(c) for c in p.children]
        if p.operator == Operator.XOR:
            children.sort(key=key)
        return OperatorPOWL(p.operator, children)
    if isinstance(p, StrictPartialOrder):
        nodes = p.order.nodes
        children = [_sorted_powl(c) for c in nodes]
        perm = sorted(range(len(children)), key=lambda i: key(children[i]))
        out = StrictPartialOrder([children[i] for i in perm])
        for i, a in enumerate(nodes):
            for j, b in enumerate(nodes):
                if p.order.is_edge(a, b):
                    out.order.add_edge(children[i], children[j])
        return out
    return p


class _Drawn(Exception):
    pass


def powl(fixtures: dict[str, Path], text: str | None = None, **params: Any) -> dict[str, Any]:
    """pm4py's POWL drawing, captured before pm4py renders it to SVG."""
    from cases.powl import describe_powl
    from pm4py.visualization.powl.variants import basic

    if text is None:
        model = pm4py.discover_powl(load_log(fixtures["log"]))
    else:
        model = pm4py.parse_powl_model_string(text)
    model = _sorted_powl(model)
    sources: list[str] = []
    original = basic.apply

    def apply(*args: Any, **kwargs: Any) -> Any:
        sources.append(original(*args, **kwargs).source)
        raise _Drawn

    basic.apply = apply
    try:
        with tempfile.TemporaryDirectory() as tmp:
            pm4py.save_vis_powl(model, str(Path(tmp) / "out.svg"), **params)
    except _Drawn:
        pass
    finally:
        basic.apply = original
    if len(sources) != 1:
        raise RuntimeError(f"expected one drawn graph, got {len(sources)}")
    return {"model": describe_powl(model), "dot": canonical_dot(sources[0])}


def _ocel(fixtures: dict[str, Path]) -> Any:
    path = fixtures["log"]
    if "ocel20" in path.name:
        return pm4py.read_ocel2(str(path))
    return pm4py.read_ocel(str(path))


def _md5_color(value: str) -> str:
    """The first three bytes of the MD5 digest, as ichnos's default."""
    import hashlib

    return "#" + hashlib.md5(value.encode()).hexdigest()[:6].upper()


class _StableColors:
    """Replaces ``ot_to_color`` in a visualizer module while it is open.

    pm4py derives object-type colours from Python's string hash, which
    changes from one process to the next, so the goldens would change on
    every run."""

    def __init__(self, module: Any) -> None:
        self.module = module

    def __enter__(self) -> None:
        self.original = self.module.ot_to_color
        self.module.ot_to_color = _md5_color

    def __exit__(self, *exc: Any) -> None:
        self.module.ot_to_color = self.original


def _colors(ocel: Any) -> dict[str, str]:
    return {ot: _md5_color(ot) for ot in sorted(ocel.objects["ocel:type"].unique())}


def _counts(d: dict[str, Any], *keys: Any) -> list[int]:
    out = []
    for metric in ("events", "unique_objects", "total_objects"):
        v = d[metric]
        for k in keys:
            v = v.get(k, ()) if isinstance(v, dict) else ()
        out.append(len(v))
    return out


def ocdfg(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from pm4py.visualization.ocel.ocdfg import visualizer
    from pm4py.visualization.ocel.ocdfg.variants import classic

    ocel = _ocel(fixtures)
    d = pm4py.discover_ocdfg(ocel)
    edges: dict[str, list[Any]] = {}
    for ot in sorted(d["edges"]["event_couples"]):
        pairs = set()
        for metric in ("event_couples", "unique_objects", "total_objects"):
            pairs |= set(d["edges"][metric].get(ot, {}))
        edges[ot] = [
            [
                a,
                b,
                [len(d["edges"][m].get(ot, {}).get((a, b), ())) for m in
                 ("event_couples", "unique_objects", "total_objects")],
                list(d["edges_performance"]["event_couples"].get(ot, {}).get((a, b), [])),
                list(d["edges_performance"]["total_objects"].get(ot, {}).get((a, b), [])),
            ]
            for a, b in sorted(pairs)
        ]
    sides = {}
    for side in ("start_activities", "end_activities"):
        sides[side] = {
            ot: {act: _counts(d[side], ot, act) for act in sorted(acts)}
            for ot, acts in sorted(d[side]["events"].items())
        }
    model = {
        "activities": {
            act: _counts(d["activities_indep"], act)
            for act in sorted(d["activities_indep"]["events"])
        },
        "start": sides["start_activities"],
        "end": sides["end_activities"],
        "edges": edges,
        "colors": _colors(ocel),
    }
    kwargs = dict(params)
    if kwargs.pop("business_hours", False):
        kwargs["business_hour_slots"] = constants.DEFAULT_BUSINESS_HOUR_SLOTS
    with _StableColors(classic):
        source = _capture(visualizer, lambda path: pm4py.save_vis_ocdfg(d, path, **kwargs))
    return {"model": model, "dot": canonical_dot(source)}


def ocpn(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from cases.model import describe_net
    from pm4py.visualization.ocel.ocpn import visualizer
    from pm4py.visualization.ocel.ocpn.variants import wo_decoration

    ocel = _ocel(fixtures)
    tbr = params.get("diagnostics_with_tbr", False)
    p = pm4py.discover_oc_petri_net(ocel, diagnostics_with_tbr=tbr)
    nets = {}
    for ot, (net, im, fm) in sorted(p["petri_nets"].items()):
        _name_transitions(net)
        diagnostics = None
        if ot in p["tbr_results"]:
            places, transitions = p["tbr_results"][ot]
            diagnostics = {
                "places": {
                    x.name: [v["p"], v["m"], v["c"], v["r"]]
                    for x, v in sorted(places.items(), key=lambda kv: kv[0].name)
                },
                "transitions": {
                    x.name: n for x, n in sorted(transitions.items(), key=lambda kv: kv[0].name)
                },
            }
        nets[ot] = {
            "net": describe_net(net, im, fm),
            "double_arcs": dict(sorted(p["double_arcs_on_activity"][ot].items())),
            "diagnostics": diagnostics,
        }
    model = {
        "activities": sorted(p["activities"]),
        "nets": nets,
        "colors": _colors(ocel),
    }
    styled = _styled(params)
    with _StableColors(wo_decoration):
        source = _capture(visualizer, lambda path: pm4py.save_vis_ocpn(p, path, **styled))
    return {"model": model, "dot": canonical_dot(source)}


def object_graph(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from pm4py.visualization.ocel.object_graph import visualizer
    from pm4py.visualization.ocel.object_graph.variants import graphviz

    ocel = _ocel(fixtures)
    graph = pm4py.discover_objects_graph(ocel, graph_type=params.get("graph_type", "object_interaction"))
    nodes = {x for e in graph for x in e}
    types: dict[str, str] = {}
    for oid, ot in zip(ocel.objects["ocel:oid"], ocel.objects["ocel:type"]):
        if oid in nodes:
            types.setdefault(oid, ot)
    model = {
        "objects": sorted([oid, ot] for oid, ot in types.items()),
        "graph": sorted([a, b] for a, b in graph),
        "colors": _colors(ocel),
    }
    with _StableColors(graphviz):
        if "directed" in params:
            # ``save_vis_object_graph`` does not pass ``directed`` on.
            parameters = {"format": "svg", "bgcolor": "white", "directed": params["directed"]}
            source = visualizer.apply(ocel, graph, parameters=parameters).source
        else:
            styled = _styled(params)
            source = _capture(
                visualizer, lambda path: pm4py.save_vis_object_graph(ocel, graph, path, **styled)
            )
    return {"model": model, "dot": canonical_dot(source)}


NETWORK = {
    "out_column": "case:concept:name",
    "in_column": "case:concept:name",
    "node_column_source": "org:resource",
    "node_column_target": "org:resource",
    "edge_column": "concept:name",
}


def network_analysis(fixtures: dict[str, Path], **params: Any) -> dict[str, Any]:
    from pm4py.visualization.network_analysis import visualizer

    performance = params.get("performance", False)
    edges = pm4py.discover_network_analysis(
        load_log(fixtures["log"]), performance=performance, **NETWORK
    )
    # pm4py's pen widths depend on the order of the edges and values, so
    # the model keeps it.
    model = {
        "edges": [
            [a, b, [[k, list(v) if performance else v] for k, v in values.items()]]
            for (a, b), values in edges.items()
        ]
    }
    kwargs = {k: v for k, v in params.items() if k != "performance"}
    if kwargs.pop("business_hours", False):
        kwargs["business_hour_slots"] = constants.DEFAULT_BUSINESS_HOUR_SLOTS
    variant = "performance" if performance else "frequency"
    source = _capture(
        visualizer,
        lambda path: pm4py.save_vis_network_analysis(edges, path, variant=variant, **kwargs),
    )
    return {"model": model, "dot": canonical_dot(source)}


def _capture_neato(module: Any, call: Any) -> str:
    """Runs ``call`` with ``module.os.system`` recording the DOT file pm4py
    passes to ``neato`` and writing an empty image in its place. Dates come
    out in UTC."""
    import os
    import time

    sources: list[str] = []

    class _Os:
        @staticmethod
        def system(command: str) -> int:
            parts = command.split()
            sources.append(Path(parts[3]).read_text())
            Path(parts[5]).write_bytes(b"")
            return 0

    original, tz = module.os, os.environ.get("TZ")
    module.os = _Os()
    os.environ["TZ"] = "UTC"
    time.tzset()
    try:
        with tempfile.TemporaryDirectory() as tmp:
            call(str(Path(tmp) / "out.svg"))
    finally:
        module.os = original
        if tz is None:
            del os.environ["TZ"]
        else:
            os.environ["TZ"] = tz
        time.tzset()
    if len(sources) != 1:
        raise RuntimeError(f"expected one drawn chart, got {len(sources)}")
    return sources[0]


def dotted_chart(fixtures: dict[str, Path], attributes: list[str], **params: Any) -> dict[str, Any]:
    import random

    from pm4py.visualization.dotted_chart.variants import classic

    # pm4py picks the colours at random: seed them and record them.
    build, randint = vars(classic)["__build_color_dict"], classic.randint
    colors: dict[str, str] = {}

    def recorded(values: Any) -> Any:
        out = build(values)
        colors.update({str(k): v for k, v in out.items()})
        return out

    vars(classic)["__build_color_dict"] = recorded
    classic.randint = random.Random(0).randint
    try:
        source = _capture_neato(
            classic,
            lambda path: pm4py.save_vis_dotted_chart(
                load_log(fixtures["log"]), path, attributes=attributes, **params
            ),
        )
    finally:
        vars(classic)["__build_color_dict"] = build
        classic.randint = randint
    return {"model": {"attributes": attributes, "colors": colors}, "dot": canonical_dot(source)}


def performance_spectrum(
    fixtures: dict[str, Path], activities: list[str], sample_size: int | None = None, **params: Any
) -> dict[str, Any]:
    from pm4py.algo.discovery.performance_spectrum import algorithm
    from pm4py.visualization.performance_spectrum.variants import neato

    # An event log, not a data frame: pm4py then takes the ``log`` variant,
    # which samples without randomness.
    log = pm4py.read_xes(str(fixtures["log"]), return_legacy_log_object=True)
    if sample_size is None:
        spectrum = algorithm.apply(log, activities)
        source = _capture_neato(
            neato,
            lambda path: pm4py.save_vis_performance_spectrum(log, activities, path, **params),
        )
    else:
        # ``save_vis_performance_spectrum`` does not take a sample size.
        spectrum = algorithm.apply(log, activities, parameters={"sample_size": sample_size})
        source = _capture_neato(
            neato, lambda path: neato.apply(spectrum, parameters={"format": "svg"})
        )
    model = {"activities": activities, "points": spectrum["points"]}
    return {"model": model, "dot": canonical_dot(source)}


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

for name, log in LOGS.items():
    case(
        f"transition-system-{name}",
        fixture=log,
        functions=["pm4py.discover_transition_system", "pm4py.save_vis_transition_system"],
    )(transition_system)
    case(
        f"prefix-tree-{name}",
        fixture=log,
        functions=["pm4py.discover_prefix_tree", "pm4py.save_vis_prefix_tree"],
    )(prefix_tree)
    case(
        f"footprints-{name}",
        fixture=log,
        functions=["pm4py.discover_footprints", "pm4py.save_vis_footprints"],
    )(footprints)
    case(
        f"footprints-{name}-vs-net",
        fixture=log,
        functions=[
            "pm4py.discover_footprints",
            "pm4py.discover_petri_net_inductive",
            "pm4py.save_vis_footprints",
        ],
        params={"compare": "net"},
    )(footprints)
    case(
        f"alignments-{name}",
        fixture=log,
        functions=["pm4py.save_vis_alignments"],
    )(alignments)
    case(
        f"powl-{name}",
        fixture=log,
        functions=["pm4py.discover_powl", "pm4py.save_vis_powl"],
    )(powl)

case(
    "footprints-running-example-vs-filtered",
    fixture="running-example.xes",
    functions=[
        "pm4py.discover_footprints",
        "pm4py.filter_event_attribute_values",
        "pm4py.save_vis_footprints",
    ],
    params={"compare": "examine casually"},
)(footprints)
case(
    "powl-frequent-styled",
    functions=["pm4py.parse_powl_model_string", "pm4py.save_vis_powl"],
    params={
        "text": "PO=(nodes={ X ( a, tau ), * ( b, tau ), * ( tau, c ), X ( tau, d ), "
        "* ( e, f ) }, order={ X ( a, tau )-->* ( b, tau ), * ( b, tau )-->* ( e, f ) })",
        "bgcolor": "#ddeeff",
        "rankdir": "LR",
    },
)(powl)

OCELS = {"example_log": "ocel/example_log.jsonocel", "ocel20_example": "ocel/ocel20_example.jsonocel"}

for name, log in OCELS.items():
    case(
        f"ocdfg-{name}",
        fixture=log,
        functions=["pm4py.discover_ocdfg", "pm4py.save_vis_ocdfg"],
    )(ocdfg)
    case(
        f"ocpn-{name}",
        fixture=log,
        functions=["pm4py.discover_oc_petri_net", "pm4py.save_vis_ocpn"],
    )(ocpn)
    case(
        f"object-graph-{name}",
        fixture=log,
        functions=["pm4py.discover_objects_graph", "pm4py.save_vis_object_graph"],
    )(object_graph)

case(
    "ocdfg-example_log-performance",
    fixture="ocel/example_log.jsonocel",
    functions=["pm4py.discover_ocdfg", "pm4py.save_vis_ocdfg"],
    params={"annotation": "performance"},
)(ocdfg)
case(
    "ocdfg-example_log-performance-business-hours",
    fixture="ocel/example_log.jsonocel",
    functions=["pm4py.discover_ocdfg", "pm4py.save_vis_ocdfg"],
    params={"annotation": "performance", "edge_metric": "total_objects", "business_hours": True},
)(ocdfg)
case(
    "ocdfg-example_log-metrics",
    fixture="ocel/example_log.jsonocel",
    functions=["pm4py.discover_ocdfg", "pm4py.save_vis_ocdfg"],
    params={
        "act_metric": "unique_objects",
        "edge_metric": "total_objects",
        "act_threshold": 3,
        "edge_threshold": 2,
        "bgcolor": "#ffeedd",
        "rankdir": "TB",
        "graph_title": "Orders",
    },
)(ocdfg)
case(
    "ocpn-example_log-tbr",
    fixture="ocel/example_log.jsonocel",
    functions=["pm4py.discover_oc_petri_net", "pm4py.save_vis_ocpn"],
    params={"diagnostics_with_tbr": True, "rankdir": "TB", "graph_title": "Orders"},
)(ocpn)
case(
    "object-graph-example_log-descendants",
    fixture="ocel/example_log.jsonocel",
    functions=["pm4py.discover_objects_graph", "pm4py.save_vis_object_graph"],
    params={"graph_type": "object_descendants", "rankdir": "TB", "graph_title": "Descendants"},
)(object_graph)
case(
    "object-graph-example_log-undirected",
    fixture="ocel/example_log.jsonocel",
    functions=[
        "pm4py.discover_objects_graph",
        "pm4py.visualization.ocel.object_graph.visualizer.apply",
    ],
    params={"directed": False},
)(object_graph)

for name, log in LOGS.items():
    case(
        f"network-analysis-{name}",
        fixture=log,
        functions=["pm4py.discover_network_analysis", "pm4py.save_vis_network_analysis"],
    )(network_analysis)

case(
    "network-analysis-running-example-performance",
    fixture="running-example.xes",
    functions=["pm4py.discover_network_analysis", "pm4py.save_vis_network_analysis"],
    params={"performance": True},
)(network_analysis)
case(
    "network-analysis-running-example-thresholds",
    fixture="running-example.xes",
    functions=["pm4py.discover_network_analysis", "pm4py.save_vis_network_analysis"],
    params={"activity_threshold": 3, "edge_threshold": 2, "graph_title": "Hand-overs"},
)(network_analysis)
case(
    "network-analysis-running-example-performance-business-hours",
    fixture="running-example.xes",
    functions=["pm4py.discover_network_analysis", "pm4py.save_vis_network_analysis"],
    params={"performance": True, "business_hours": True},
)(network_analysis)

case(
    "dotted-chart-running-example",
    fixture="running-example.xes",
    functions=["pm4py.save_vis_dotted_chart"],
    params={"attributes": ["time:timestamp", "case:concept:name", "concept:name"]},
)(dotted_chart)
case(
    "dotted-chart-running-example-resources",
    fixture="running-example.xes",
    functions=["pm4py.save_vis_dotted_chart"],
    params={"attributes": ["concept:name", "org:resource"], "graph_title": "Resources"},
)(dotted_chart)
case(
    "dotted-chart-running-example-no-legend",
    fixture="running-example.xes",
    functions=["pm4py.save_vis_dotted_chart"],
    params={
        "attributes": ["org:resource", "time:timestamp", "concept:name"],
        "show_legend": False,
    },
)(dotted_chart)

case(
    "performance-spectrum-running-example",
    fixture="running-example.xes",
    functions=["pm4py.save_vis_performance_spectrum"],
    params={"activities": ["register request", "check ticket", "decide"]},
)(performance_spectrum)
case(
    "performance-spectrum-running-example-styled",
    fixture="running-example.xes",
    functions=["pm4py.save_vis_performance_spectrum"],
    params={
        "activities": ["examine casually", "check ticket", "decide", "pay compensation"],
        "graph_title": "Compensation",
    },
)(performance_spectrum)
case(
    "performance-spectrum-receipt-sampled",
    fixture="receipt.xes",
    functions=[
        "pm4py.algo.discovery.performance_spectrum.algorithm.apply",
        "pm4py.visualization.performance_spectrum.variants.neato.apply",
    ],
    params={
        "activities": ["Confirmation of receipt", "T02 Check confirmation of receipt"],
        "sample_size": 25,
    },
)(performance_spectrum)
