"""Area ``wfnet``: pm4py's workflow net to process tree conversion.

Each case describes its input net as ``model.py`` does (``model``) and
gives ``tree``: ``str`` of ``pm4py.convert_to_process_tree`` on the net, or
``null`` when pm4py raises. ``error`` names the ``ValueError`` pm4py raised:
``not_workflow_net`` or ``not_block_structured``, else ``null``.

- ``net-*``: PNML fixtures, and ``net-two-sources``, built inline, which is
  not a workflow net.
- ``tree-*``: a process tree (``tree_in``) converted with
  ``pm4py.convert_to_petri_net``, then back. Visible transitions of that net
  are renamed ``t_<n>`` in order of label and neighbouring places, because
  pm4py names them with random UUIDs.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pm4py

from harness import case
from harness.fixtures import load_model


ERRORS = {
    "The Petri net provided is not a WF-net": "not_workflow_net",
    "Parsing of WF-net Failed": "not_block_structured",
}


def _tree(net: Any, im: Any, fm: Any) -> dict[str, Any]:
    try:
        return {"tree": str(pm4py.convert_to_process_tree(net, im, fm)), "error": None}
    except ValueError as e:
        return {"tree": None, "error": ERRORS[str(e)]}


def from_net(fixtures: dict[str, Path]) -> dict[str, Any]:
    from cases.model import describe_net

    net, im, fm = load_model(fixtures["model"])
    return {"model": describe_net(net, im, fm), **_tree(net, im, fm)}


@case("net-two-sources", functions=["pm4py.convert_to_process_tree"])
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
]:
    case(
        f"net-{name.replace('_', '-')}",
        fixtures={"model": f"{name}.pnml"},
        functions=["pm4py.convert_to_process_tree"],
    )(from_net)

TREE_FUNCTIONS = [
    "pm4py.parse_process_tree",
    "pm4py.convert_to_petri_net",
    "pm4py.convert_to_process_tree",
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
