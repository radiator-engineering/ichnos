"""Area ``powl``: pm4py's POWL models and their conversions.

A POWL model is described as a tree of nodes:

- ``{"kind": "silent"}``
- ``{"kind": "activity", "label": label}``
- ``{"kind": "frequent", "activity": label, "skippable": bool,
  "selfloop": bool, "label": label}``
- ``{"kind": "xor", "children": [...]}`` and ``{"kind": "loop", "children":
  [do, redo]}``
- ``{"kind": "po", "children": [...], "order": [[i, j], ...]}``: ``[i, j]``
  says child ``i`` comes before child ``j``. Pairs are in index order.

A Petri net a POWL converts to keeps pm4py's place names (``source``,
``sink``, ``p_<n>``) and silent transition names (``tauSplit_<n>``, ...).
Visible transitions get random UUIDs in pm4py, so their names are ``null``.

Cases:

- ``model-*``: from the POWL string ``text``, parsed with
  ``pm4py.parse_powl_model_string``: ``repr`` (``str`` of the model, or
  ``null`` when it names a node by its hash), ``powl`` (the parsed model),
  ``simplified`` (``simplify()``), ``frequent``
  (``simplify_using_frequent_transitions()``), ``petri_net``
  (``pm4py.convert_to_petri_net``), ``tree`` (``str`` of
  ``pm4py.convert_to_process_tree``) and ``precise`` (``false`` when pm4py
  warns that the tree is not a precise conversion). ``tree`` is ``null`` when
  pm4py rejects the model.
- ``tree-*``: the same results for ``pm4py.convert_to_powl`` of a process
  tree, given as ``tree_in``. The POWL of a sequence orders only neighbours,
  as pm4py builds it.
"""

from __future__ import annotations

import re
import warnings
from pathlib import Path
from typing import Any

import pm4py
from pm4py.objects.powl.obj import (
    FrequentTransition,
    OperatorPOWL,
    SilentTransition,
    StrictPartialOrder,
    Transition,
)
from pm4py.objects.process_tree.obj import Operator

from harness import case
from harness.fixtures import load_model

UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")


def describe_powl(p: Any) -> dict[str, Any]:
    if isinstance(p, SilentTransition):
        return {"kind": "silent"}
    if isinstance(p, FrequentTransition):
        return {
            "kind": "frequent",
            "activity": p.activity,
            "skippable": p.skippable,
            "selfloop": p.selfloop,
            "label": p.label,
        }
    if isinstance(p, Transition):
        return {"kind": "activity", "label": p.label}
    if isinstance(p, OperatorPOWL):
        kind = {Operator.XOR: "xor", Operator.LOOP: "loop"}[p.operator]
        return {"kind": kind, "children": [describe_powl(c) for c in p.children]}
    if isinstance(p, StrictPartialOrder):
        nodes = p.order.nodes
        return {
            "kind": "po",
            "children": [describe_powl(c) for c in nodes],
            "order": [
                [i, j]
                for i in range(len(nodes))
                for j in range(len(nodes))
                if p.order.is_edge_id(i, j)
            ],
        }
    raise TypeError(type(p).__name__)


def describe_net(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    transitions = sorted(
        (
            {
                "name": None if UUID.match(t.name) else t.name,
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


def results(p: Any) -> dict[str, Any]:
    text = str(p)
    out: dict[str, Any] = {
        "repr": None if re.search(r"id_-?\d+", text) else text,
        "powl": describe_powl(p),
        "simplified": describe_powl(p.simplify()),
        "frequent": describe_powl(p.simplify_using_frequent_transitions()),
        "petri_net": describe_net(*pm4py.convert_to_petri_net(p)),
    }
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            out["tree"] = str(pm4py.convert_to_process_tree(p))
        except Exception:
            out["tree"] = None
    out["precise"] = not any("cannot be converted precisely" in str(w.message) for w in caught)
    return out


def model(fixtures: dict[str, Path], text: str) -> dict[str, Any]:
    return {"text": text, **results(pm4py.parse_powl_model_string(text))}


def tree(fixtures: dict[str, Path], tree: str | None = None) -> dict[str, Any]:
    t = pm4py.parse_process_tree(tree) if tree is not None else load_model(fixtures["model"])
    return {"tree_in": str(t), **results(pm4py.convert_to_powl(t))}


RE_LOOP = "*( PO=(nodes={ PO=(nodes={ check ticket, X( 'examine thoroughly', 'examine casually' ) }, order={ }), decide }, order={ PO=(nodes={ check ticket, X( 'examine thoroughly', 'examine casually' ) }, order={ })-->decide }), 'reinitiate request' )"
RE_END = "X( 'reject request', 'pay compensation' )"
NESTED = "PO=(nodes={ b, c }, order={ b-->c })"

MODELS = [
    ("single", "'a'"),
    ("silent", "tau"),
    ("sequence", "PO=(nodes={ a, b, c }, order={ a-->b, b-->c, a-->c })"),
    ("concurrent", "PO=(nodes={ a, b }, order={ })"),
    (
        "model-description",
        "PO=(nodes={ NODE1, NODE2, NODE3, X ( NODE4, NODE5 ) }, order={ NODE1-->NODE2, "
        "NODE1-->X ( NODE4, NODE5 ), NODE2-->X ( NODE4, NODE5 ) })",
    ),
    ("diamond", "PO=(nodes={ a, b, c, d }, order={ a-->b, a-->c, a-->d, b-->d, c-->d })"),
    ("n-shape", "PO=(nodes={ a, b, c, d }, order={ a-->c, b-->c, b-->d })"),
    ("levels", "PO=(nodes={ a, b, c, d }, order={ a-->b, b-->d, c-->d, a-->d })"),
    ("cyclic", "PO=(nodes={ a, b }, order={ a-->b, b-->a })"),
    ("quoted", "PO=(nodes={ 'a b', 'c' }, order={ 'a b'-->'c' })"),
    (
        "loops-and-choices",
        "X ( *( a, tau ), PO=(nodes={ b, X ( c, tau ) }, order={ b-->X ( c, tau ) }) )",
    ),
    (
        "frequent",
        "PO=(nodes={ X ( a, tau ), * ( b, tau ), * ( tau, c ), X ( tau, d ), X ( tau, tau ) }, "
        "order={ X ( a, tau )-->* ( b, tau ) })",
    ),
    (
        "simplify-nested",
        f"PO=(nodes={{ a, {NESTED}, d }}, order={{ a-->{NESTED}, {NESTED}-->d, a-->d }})",
    ),
    ("simplify-unconnected", "PO=(nodes={ a, PO=(nodes={ b, c }, order={ }) }, order={ })"),
    (
        "simplify-xor",
        "X ( a, X ( b, c ), X ( tau, * ( tau, d ) ), X ( tau, * ( e, tau ) ) )",
    ),
    ("simplify-merge", "X ( tau, * ( e, tau ) )"),
    (
        "running-example",
        f"PO=(nodes={{ register request, {RE_LOOP}, {RE_END} }}, order={{ "
        f"register request-->{RE_LOOP}, register request-->{RE_END}, {RE_LOOP}-->{RE_END} }})",
    ),
]

for name, text in MODELS:
    case(
        f"model-{name}",
        functions=[
            "pm4py.parse_powl_model_string",
            "pm4py.convert_to_petri_net",
            "pm4py.convert_to_process_tree",
        ],
        params={"text": text},
    )(model)

TREE_FUNCTIONS = [
    "pm4py.parse_process_tree",
    "pm4py.convert_to_powl",
    "pm4py.convert_to_petri_net",
    "pm4py.convert_to_process_tree",
]

for name in ["running-example", "tree_ex_with_loops", "tree_ex_wo_loops"]:
    case(
        f"tree-{name.replace('_', '-')}",
        fixtures={"model": f"{name}.ptml"},
        functions=TREE_FUNCTIONS,
    )(tree)

for name, text in [
    ("nested", "->( 'a', X( 'b', ->( 'c', 'd' ), tau ), +( 'e', *( 'f', tau ) ), 'g' )"),
    ("loops", "*( ->( 'a', X( 'b', tau ) ), X( 'c', ->( tau, 'd' ) ) )"),
    ("parallel-sequences", "+( ->( 'a', 'b', 'c' ), ->( 'd', 'e' ), 'f' )"),
]:
    case(f"tree-{name}", functions=TREE_FUNCTIONS, params={"tree": text})(tree)
