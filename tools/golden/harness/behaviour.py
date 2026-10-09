"""Model behaviour, the comparable form of a discovered or loaded model.

Rust must not match pm4py's node ids (place names, transition ids, tree node
order). A case that produces a model emits :func:`model_behaviour` instead. See
"Models: emit behaviour, not structure" in ``tools/golden/README.md``.
"""

from __future__ import annotations

from typing import Any

import pandas as pd
import pm4py
from pm4py.objects.process_tree.obj import ProcessTree

FUNCTIONS = [
    "pm4py.discover_footprints",
    "pm4py.fitness_token_based_replay",
    "pm4py.precision_token_based_replay",
]
ALIGNMENT_FUNCTIONS = ["pm4py.fitness_alignments", "pm4py.precision_alignments"]


def model_behaviour(log: pd.DataFrame, model: Any, *, alignments: bool = False) -> dict[str, Any]:
    """Behaviour of ``model`` on ``log``.

    ``model`` is a ``(net, im, fm)`` tuple, a ``ProcessTree``, or a BPMN graph.
    Process trees and BPMN graphs are converted with
    ``pm4py.convert_to_petri_net`` for replay; footprints come from the model
    as given, since pm4py computes richer footprints for trees.

    Set ``alignments=True`` to add alignment-based fitness and precision. They
    are exact but slow, so use them on small logs only.
    """
    if isinstance(model, tuple):
        kind = "petri_net"
        net, im, fm = model
        footprints = pm4py.discover_footprints(net, im, fm)
    elif isinstance(model, ProcessTree):
        kind = "process_tree"
        footprints = pm4py.discover_footprints(model)
        net, im, fm = pm4py.convert_to_petri_net(model)
    else:
        kind = "bpmn"
        net, im, fm = pm4py.convert_to_petri_net(model)
        footprints = pm4py.discover_footprints(net, im, fm)

    out: dict[str, Any] = {
        "model_kind": kind,
        "footprints": footprints,
        "fitness_tbr": pm4py.fitness_token_based_replay(log, net, im, fm),
        "precision_tbr": pm4py.precision_token_based_replay(log, net, im, fm),
    }
    if alignments:
        out["fitness_alignments"] = pm4py.fitness_alignments(log, net, im, fm)
        out["precision_alignments"] = pm4py.precision_alignments(log, net, im, fm)
    return out
