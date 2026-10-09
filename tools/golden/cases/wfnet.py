"""Area ``wfnet``: pm4py's workflow net to process tree conversion.

Each case describes its input net as ``model.py`` does (``model``) and
gives ``tree``: ``str`` of ``pm4py.convert_to_process_tree`` on the net, or
``null`` when pm4py raises (the net is not a workflow net, or not block
structured).

- ``net-*``: PNML fixtures.
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


def _tree(net: Any, im: Any, fm: Any) -> str | None:
    try:
        return str(pm4py.convert_to_process_tree(net, im, fm))
    except ValueError:
        return None


def from_net(fixtures: dict[str, Path]) -> dict[str, Any]:
    from cases.model import describe_net

    net, im, fm = load_model(fixtures["model"])
    return {"model": describe_net(net, im, fm), "tree": _tree(net, im, fm)}


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
    return {"tree_in": str(t), "model": describe_net(net, im, fm), "tree": _tree(net, im, fm)}


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
