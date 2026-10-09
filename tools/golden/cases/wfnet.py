"""Area ``wfnet``: pm4py's workflow net to process tree conversion.

Each case describes its input net as ``model.py`` does (``model``) and
gives ``tree``: ``str`` of ``pm4py.convert_to_process_tree`` on the net, or
``null`` when pm4py raises. ``error`` names the ``ValueError`` pm4py raised:
``not_workflow_net`` or ``not_block_structured``, else ``null``.
``powl`` describes ``pm4py.convert_to_powl`` on a copy of the net, as
``powl.py`` does, or is ``null`` when pm4py raises; ``powl_error`` then
names the failure (see :func:`powl_of`). pm4py's choice and partial-order
children follow set order, so ``powl`` sorts them (see
:func:`canonical_powl`).

- ``net-*``: PNML fixtures, and ``net-two-sources``, built inline, which is
  not a workflow net. ``cyber-incident-response`` has inhibitor and reset
  arcs.
- ``tree-*``: a process tree (``tree_in``) converted with
  ``pm4py.convert_to_petri_net``, then back. Visible transitions of that net
  are renamed ``t_<n>`` in order of label and neighbouring places, because
  pm4py names them with random UUIDs.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

import pm4py

from harness import case
from harness.fixtures import load_model


ERRORS = {
    "The Petri net provided is not a WF-net": "not_workflow_net",
    "Parsing of WF-net Failed": "not_block_structured",
}


POWL_ERRORS = {
    "Not a WF-net!": "not_workflow_net",
    "Unique local start property is violated!": "no_unique_local_start_or_end",
    "Unique local end property is violated!": "no_unique_local_start_or_end",
    "Conversion failed!": "cyclic_order",
}


def _key(d: dict[str, Any]) -> str:
    return json.dumps(d, sort_keys=True)


def canonical_powl(d: dict[str, Any]) -> dict[str, Any]:
    """Sorts choice children by their description, and partial-order
    children by their description and those of their predecessors and
    successors, renumbering the order. pm4py builds both in set order."""
    if d["kind"] not in ("xor", "loop", "po"):
        return d
    children = [canonical_powl(c) for c in d["children"]]
    if d["kind"] == "loop":
        return {**d, "children": children}
    if d["kind"] == "xor":
        return {**d, "children": sorted(children, key=_key)}
    keys = [_key(c) for c in children]
    order = d["order"]

    def rank(i: int) -> tuple[Any, ...]:
        before = sorted(keys[a] for a, b in order if b == i)
        after = sorted(keys[b] for a, b in order if a == i)
        return (keys[i], before, after)

    perm = sorted(range(len(children)), key=rank)
    pos = {old: new for new, old in enumerate(perm)}
    return {
        **d,
        "children": [children[i] for i in perm],
        "order": sorted([pos[a], pos[b]] for a, b in order),
    }


def powl_of(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    """``pm4py.convert_to_powl`` on a copy of the net, which pm4py changes.

    ``powl_error`` is ``special_arcs`` for any failure on a net with
    inhibitor or reset arcs: pm4py treats them as normal arcs, and on the
    seven ``inh_res_nets`` fixtures it finds no structure. Otherwise it is
    ``no_structure`` when pm4py finds no structure (its own message, or a
    ``KeyError`` or ``IndexError`` on the way), or a value of
    ``POWL_ERRORS``.

    pm4py's places and transitions hash by ``id()``, so its set order
    follows object addresses, not ``PYTHONHASHSEED``. Runs under several
    hash seeds therefore do not vary it. The goldens store canonical POWL,
    and the nets here gave one canonical result in 40 runs each with
    shifted object addresses.
    """
    from cases.powl import describe_powl

    # pm4py's deep copy of a net drops the arc properties.
    special = any("arctype" in a.properties for a in net.arcs)
    net, im, fm = copy.deepcopy((net, im, fm))
    try:
        powl = pm4py.convert_to_powl(net, im, fm)
        return {"powl": canonical_powl(describe_powl(powl)), "powl_error": None}
    except Exception as e:
        if special:
            error = "special_arcs"
        elif isinstance(e, (KeyError, IndexError)) or str(e).startswith(
            "Failed to detect a POWL structure"
        ):
            error = "no_structure"
        elif str(e) in POWL_ERRORS:
            error = POWL_ERRORS[str(e)]
        else:
            raise
        return {"powl": None, "powl_error": error}


def _tree(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    powl = powl_of(net, im, fm)
    try:
        out = {"tree": str(pm4py.convert_to_process_tree(net, im, fm)), "error": None}
    except ValueError as e:
        out = {"tree": None, "error": ERRORS[str(e)]}
    return {**out, **powl}


def from_net(fixtures: dict[str, Path]) -> dict[str, Any]:
    from cases.model import describe_net

    net, im, fm = load_model(fixtures["model"])
    return {"model": describe_net(net, im, fm), **_tree(net, im, fm)}


@case("net-two-sources", functions=["pm4py.convert_to_process_tree", "pm4py.convert_to_powl"])
def two_sources(fixtures: dict[str, Path]) -> dict[str, Any]:
    """Two source places, so not a workflow net."""
    from cases.model import describe_net
    from pm4py.objects.petri_net.obj import Marking, PetriNet
    from pm4py.objects.petri_net.utils.petri_utils import add_arc_from_to

    net = PetriNet("two-sources")
    i1, i2, o = (PetriNet.Place(n) for n in ["i1", "i2", "o"])
    a, b = PetriNet.Transition("a", "a"), PetriNet.Transition("b", "b")
    net.places.update([i1, i2, o])
    net.transitions.update([a, b])
    for x, y in [(i1, a), (a, o), (i2, b), (b, o)]:
        add_arc_from_to(x, y, net)
    im, fm = Marking({i1: 1, i2: 1}), Marking({o: 2})
    return {"model": describe_net(net, im, fm), **_tree(net, im, fm)}


def from_tree(fixtures: dict[str, Path], tree: str | None = None) -> dict[str, Any]:
    from cases.model import describe_net

    t = pm4py.parse_process_tree(tree) if tree is not None else load_model(fixtures["model"])
    net, im, fm = pm4py.convert_to_petri_net(t)
    # pm4py names visible transitions with random UUIDs; name them by label
    # and neighbouring places, which have counter-based names.
    visible = sorted(
        (t for t in net.transitions if t.label is not None),
        key=lambda t: (
            t.label,
            sorted(a.source.name for a in t.in_arcs),
            sorted(a.target.name for a in t.out_arcs),
        ),
    )
    for i, tr in enumerate(visible):
        tr.name = f"t_{i}"
    return {"tree_in": str(t), "model": describe_net(net, im, fm), **_tree(net, im, fm)}


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
    "data_petri_net",
    "stochastic_running_example",
    "synthetic_logs/a12/a12",
    "synthetic_logs/a22/a22",
    "inh_res_nets/cyber_incident_response",
]:
    case(
        f"net-{name.rsplit('/', 1)[-1].replace('_', '-')}",
        fixtures={"model": f"{name}.pnml"},
        functions=["pm4py.convert_to_process_tree", "pm4py.convert_to_powl"],
    )(from_net)

TREE_FUNCTIONS = [
    "pm4py.parse_process_tree",
    "pm4py.convert_to_petri_net",
    "pm4py.convert_to_process_tree",
    "pm4py.convert_to_powl",
]

for name in ["running-example", "tree_ex_with_loops", "tree_ex_wo_loops"]:
    case(
        f"tree-{name.replace('_', '-')}",
        fixtures={"model": f"{name}.ptml"},
        functions=TREE_FUNCTIONS,
    )(from_tree)

for name, text in [
    ("nested", "->( 'a', X( 'b', ->( 'c', 'd' ), tau ), +( 'e', *( 'f', tau ) ), 'g' )"),
    ("loops", "*( ->( 'a', X( 'b', tau ) ), X( 'c', ->( tau, 'd' ) ) )"),
    ("parallel-choices", "+( X( 'a', 'b', 'c' ), ->( 'd', +( 'e', 'f' ) ), *( 'g', 'h' ) )"),
    ("skips", "->( X( tau, 'a' ), *( tau, 'b' ), X( 'c', tau ) )"),
]:
    case(f"tree-{name}", functions=TREE_FUNCTIONS, params={"tree": text})(from_tree)
