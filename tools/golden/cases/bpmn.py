"""Area ``bpmn``: pm4py's BPMN conversions.

ichnos has no BPMN, PNML or PTML reader yet, so each case also describes its
input model (see ``model.py`` for nets and trees).

A BPMN diagram read from a file is described as ``{"process_id": id,
"nodes": [{"id", "name", "class", "direction", "process", "activity",
"depth"}], "flows": [{"id", "name", "class", "source", "target",
"process"}]}``. ``class`` is the pm4py class name; ``direction`` is the
gateway direction or ``null``; ``activity`` is a boundary event's activity
id; ``depth`` is a subprocess depth.

pm4py names the nodes it creates with random UUIDs. A diagram a conversion
produces is therefore described with canonical names: colour refinement on
(class, name, direction) and the flows gives each node a colour, and nodes
are numbered ``n0``, ``n1``, ... in colour order. The description is
``{"nodes": [{"key", "class", "name", "direction"}], "flows": [[source key,
target key, class]]}``. A Rust test compares it up to isomorphism.

A Petri net a BPMN converts to keeps pm4py's place names (flow ids,
``ent_<id>``, ``exi_<id>``, ``source``, ``sink``). Transition names that
are random UUIDs become ``null``.

Cases:

- ``tree-to-bpmn-*``: ``{"model": tree, "bpmn": canonical diagram}`` from
  ``pm4py.convert_to_bpmn`` on a process tree.
- ``petri-to-bpmn-*``: ``{"model": net, "bpmn": canonical diagram}`` from
  ``pm4py.convert_to_bpmn`` on an accepting Petri net.
- ``bpmn-to-petri-*``: ``{"model": diagram, "petri_net": net,
  "petri_net_unreduced": net}`` from ``pm4py.convert_to_petri_net`` and from
  the converter with ``ENABLE_REDUCTION`` off, plus ``tree``/``error`` and
  ``powl``/``powl_error`` for ``pm4py.convert_to_process_tree`` and
  ``pm4py.convert_to_powl`` of the diagram, as in ``wfnet.py``.
- ``bpmn-semantics-*``: ``{"model": diagram, "markings": [{node id:
  tokens}], "edges": [[from, node id, to]]}``, every marking reachable from
  ``get_initial_marking`` by ``weak_execute`` on ``enabled_nodes``, sorted;
  edges index into ``markings``.
- ``bpmn-collapse-*``: ``{"model": diagram, "bpmn": canonical diagram}``
  from ``reduction.apply`` with ``COLLAPSE_GATEWAYS`` on. The ``nested-*``
  diagrams nest two splits and two joins of one gateway type.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

import pm4py
from pm4py.objects.bpmn.obj import BPMN
from pm4py.objects.conversion.bpmn.variants import to_petri_net as bpmn_variant

from harness import case
from harness.fixtures import load_model

UUID = re.compile(r"^(id)?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")


def _direction(n: Any) -> str | None:
    return n.get_gateway_direction().value if isinstance(n, BPMN.Gateway) else None


def describe_bpmn(b: Any) -> dict[str, Any]:
    nodes = sorted(
        (
            {
                "id": str(n.get_id()),
                "name": n.get_name(),
                "class": type(n).__name__,
                "direction": _direction(n),
                "process": str(n.get_process()),
                "activity": n.get_activity() if isinstance(n, BPMN.BoundaryEvent) else None,
                "depth": n.get_depth() if isinstance(n, BPMN.SubProcess) else None,
            }
            for n in b.get_nodes()
        ),
        key=lambda n: n["id"],
    )
    flows = sorted(
        (
            {
                "id": str(f.get_id()),
                "name": f.get_name(),
                "class": type(f).__name__,
                "source": str(f.get_source().get_id()),
                "target": str(f.get_target().get_id()),
                "process": str(f.get_process()),
            }
            for f in b.get_flows()
        ),
        key=lambda f: (f["source"], f["target"], f["id"]),
    )
    return {"process_id": str(b.get_process_id()), "nodes": nodes, "flows": flows}


def canonical_bpmn(b: Any) -> dict[str, Any]:
    nodes = list(b.get_nodes())
    label = {n: repr((type(n).__name__, n.get_name(), _direction(n))) for n in nodes}
    flows = [(f.get_source(), f.get_target(), type(f).__name__) for f in b.get_flows()]
    color: dict[Any, str] = dict(label)
    classes = len(set(color.values()))
    while True:
        sig = {
            n: repr(
                (
                    color[n],
                    sorted((c, color[t]) for s, t, c in flows if s is n),
                    sorted((c, color[s]) for s, t, c in flows if t is n),
                )
            )
            for n in nodes
        }
        ranks = {s: f"{i:05d}" for i, s in enumerate(sorted(set(sig.values())))}
        new = {n: ranks[sig[n]] for n in nodes}
        if len(set(new.values())) == classes:
            break
        color, classes = new, len(set(new.values()))
    order = sorted(nodes, key=lambda n: (color[n], label[n]))
    key = {n: f"n{i}" for i, n in enumerate(order)}
    return {
        "nodes": [
            {
                "key": key[n],
                "class": type(n).__name__,
                "name": n.get_name(),
                "direction": _direction(n),
            }
            for n in order
        ],
        "flows": sorted([key[s], key[t], c] for s, t, c in flows),
    }


def describe_converted_net(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    def tname(t: Any) -> str | None:
        return None if UUID.match(t.name) else t.name

    transitions = sorted(
        (
            {
                "name": tname(t),
                "label": t.label,
                "preset": sorted(a.source.name for a in t.in_arcs),
                "postset": sorted(a.target.name for a in t.out_arcs),
            }
            for t in net.transitions
        ),
        key=lambda t: (t["name"] or "", t["label"] or "", t["preset"], t["postset"]),
    )
    return {
        "places": sorted(p.name for p in net.places),
        "transitions": transitions,
        "initial_marking": {p.name: n for p, n in im.items()},
        "final_marking": {p.name: n for p, n in fm.items()},
    }


def tree_to_bpmn(fixtures: dict[str, Path], tree: str | None = None) -> dict[str, Any]:
    t = pm4py.parse_process_tree(tree) if tree is not None else load_model(fixtures["model"])
    return {
        "model": {"kind": "process_tree", "tree": str(t)},
        "bpmn": canonical_bpmn(pm4py.convert_to_bpmn(t)),
    }


def petri_to_bpmn(fixtures: dict[str, Path]) -> dict[str, Any]:
    from cases.model import describe_net

    net, im, fm = load_model(fixtures["model"])
    return {
        "model": describe_net(net, im, fm),
        "bpmn": canonical_bpmn(pm4py.convert_to_bpmn(net, im, fm)),
    }


def bpmn_to_petri(fixtures: dict[str, Path]) -> dict[str, Any]:
    b = load_model(fixtures["model"])
    reduced = pm4py.convert_to_petri_net(b)
    raw = bpmn_variant.apply(
        b, parameters={bpmn_variant.Parameters.ENABLE_REDUCTION: False}
    )
    from cases.wfnet import ERRORS, powl_of

    try:
        tree = {"tree": str(pm4py.convert_to_process_tree(b)), "error": None}
    except ValueError as e:
        tree = {"tree": None, "error": ERRORS[str(e)]}
    return {
        "model": describe_bpmn(b),
        "petri_net": describe_converted_net(*reduced),
        "petri_net_unreduced": describe_converted_net(*raw),
        **tree,
        **powl_of(*pm4py.convert_to_petri_net(b)),
    }


TREE_FUNCTIONS = ["pm4py.parse_process_tree", "pm4py.convert_to_bpmn"]

for name in ["running-example", "tree_ex_with_loops", "tree_ex_wo_loops"]:
    case(
        f"tree-to-bpmn-{name}",
        fixtures={"model": f"{name}.ptml"},
        functions=TREE_FUNCTIONS,
    )(tree_to_bpmn)

for name, tree in [
    ("nested", "->( 'a', X( 'b', ->( 'c', 'd' ), tau ), +( 'e', *( 'f', tau ) ), 'g' )"),
    ("loops", "*( ->( 'a', X( 'b', tau ) ), X( 'c', ->( tau, 'd' ) ) )"),
    ("taus", "->( tau, X( tau, 'a' ), +( tau, 'b' ), tau )"),
    ("inclusive", "->( 'a', O( 'b', 'c', ->( 'd', 'e' ) ), 'f' )"),
]:
    case(
        f"tree-to-bpmn-{name}",
        functions=TREE_FUNCTIONS,
        params={"tree": tree},
    )(tree_to_bpmn)

for name in [
    "running-example",
    "receipt_one_variant",
    "ex1",
    "ex2",
    "murata1",
    "murata2",
    "murata3",
    "roadtraffic",
    "big_wf_net",
]:
    case(
        f"petri-to-bpmn-{name}",
        fixtures={"model": f"{name}.pnml"},
        functions=["pm4py.convert_to_bpmn"],
    )(petri_to_bpmn)

for name, path in [
    ("running-example", "running-example.bpmn"),
    ("receipt", "receipt.bpmn"),
    ("a32f0n00", "a32f0n00.bpmn"),
    ("simple-model", "more_models/simple_model.bpmn"),
    ("simple-parallel", "more_models/SimpleParallel.bpmn"),
    ("subprocess1", "more_models/Subprocess1.bpmn"),
]:
    case(
        f"bpmn-to-petri-{name}",
        fixtures={"model": path},
        functions=[
            "pm4py.convert_to_petri_net",
            "pm4py.objects.conversion.bpmn.variants.to_petri_net.apply",
            "pm4py.convert_to_process_tree",
            "pm4py.convert_to_powl",
        ],
    )(bpmn_to_petri)


def _reachable_markings(b: Any, limit: int = 2000) -> dict[str, Any]:
    """Every marking reachable from ``get_initial_marking`` by
    ``weak_execute`` on ``enabled_nodes``, breadth first. pm4py's
    ``execute`` calls ``weak_execute`` with too few arguments, so it is not
    used."""
    from pm4py.objects.bpmn import semantics
    from pm4py.objects.bpmn.util import bpmn_utils

    def key(m: Any) -> tuple[tuple[str, int], ...]:
        return tuple(sorted((str(n.get_id()), c) for n, c in m.items() if c > 0))

    start = bpmn_utils.get_initial_marking(b)
    seen = {key(start): start}
    queue = [start]
    edges = set()
    while queue:
        m = queue.pop(0)
        for node in semantics.enabled_nodes(b, m):
            for m2 in semantics.weak_execute(node, m, b):
                k2 = key(m2)
                edges.add((key(m), str(node.get_id()), k2))
                if k2 not in seen:
                    if len(seen) >= limit:
                        raise ValueError(f"more than {limit} markings")
                    seen[k2] = m2
                    queue.append(m2)
    order = sorted(seen)
    index = {k: i for i, k in enumerate(order)}
    return {
        "markings": [dict(k) for k in order],
        "edges": sorted([index[a], n, index[c]] for a, n, c in edges),
    }


def bpmn_semantics(fixtures: dict[str, Path]) -> dict[str, Any]:
    b = load_model(fixtures["model"])
    return {"model": describe_bpmn(b), **_reachable_markings(b)}


for name, path in [
    ("running-example", "running-example.bpmn"),
    ("simple-parallel", "more_models/SimpleParallel.bpmn"),
    ("subprocess1", "more_models/Subprocess1.bpmn"),
]:
    case(
        f"bpmn-semantics-{name}",
        fixtures={"model": path},
        functions=[
            "pm4py.objects.bpmn.semantics.enabled_nodes",
            "pm4py.objects.bpmn.semantics.weak_execute",
            "pm4py.objects.bpmn.util.bpmn_utils.get_initial_marking",
        ],
    )(bpmn_semantics)


def _nested_gateways(kind: str) -> Any:
    """start → split → (a | split → (b | c) → join) → join → end, with
    gateways of ``kind`` and fixed ids."""
    pid = "nested"
    gateway = {"xor": BPMN.ExclusiveGateway, "and": BPMN.ParallelGateway}[kind]
    div, conv = BPMN.Gateway.Direction.DIVERGING, BPMN.Gateway.Direction.CONVERGING
    nodes = {
        "start": BPMN.StartEvent(id="start", name="start", process=pid),
        "s1": gateway(id="s1", name="s1", gateway_direction=div, process=pid),
        "s2": gateway(id="s2", name="s2", gateway_direction=div, process=pid),
        "j2": gateway(id="j2", name="j2", gateway_direction=conv, process=pid),
        "j1": gateway(id="j1", name="j1", gateway_direction=conv, process=pid),
        "end": BPMN.EndEvent(id="end", name="end", process=pid),
    }
    for t in ["a", "b", "c"]:
        nodes[t] = BPMN.Task(id=t, name=t, process=pid)
    b = BPMN(process_id=pid)
    for n in nodes.values():
        b.add_node(n)
    for i, (s, t) in enumerate(
        [("start", "s1"), ("s1", "a"), ("s1", "s2"), ("s2", "b"), ("s2", "c"),
         ("b", "j2"), ("c", "j2"), ("a", "j1"), ("j2", "j1"), ("j1", "end")]
    ):
        b.add_flow(BPMN.SequenceFlow(nodes[s], nodes[t], id=f"f{i}", process=pid))
    return b


def bpmn_collapse(fixtures: dict[str, Path], nested: str | None = None) -> dict[str, Any]:
    from pm4py.objects.bpmn.util import reduction

    b = _nested_gateways(nested) if nested is not None else load_model(fixtures["model"])
    model = describe_bpmn(b)
    reduced = reduction.apply(b, parameters={reduction.Parameters.COLLAPSE_GATEWAYS: True})
    return {"model": model, "bpmn": canonical_bpmn(reduced)}


COLLAPSE_FUNCTIONS = ["pm4py.objects.bpmn.util.reduction.apply"]

for kind in ["xor", "and"]:
    case(
        f"bpmn-collapse-nested-{kind}",
        functions=COLLAPSE_FUNCTIONS,
        params={"nested": kind},
    )(bpmn_collapse)

for name, path in [
    ("running-example", "running-example.bpmn"),
    ("a32f0n00", "a32f0n00.bpmn"),
]:
    case(
        f"bpmn-collapse-{name}",
        fixtures={"model": path},
        functions=COLLAPSE_FUNCTIONS,
    )(bpmn_collapse)
