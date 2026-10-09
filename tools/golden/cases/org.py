"""Organizational mining checked against the pinned pm4py oracle.

The social-network cases pass pm4py an ``EventLog``, so pm4py takes its
``*_log`` variants. Network analysis needs a data frame.

Network analysis sorts the events by timestamp with an unstable sort, so its
cases use logs whose links do not depend on how ties are ordered.

Social networks are a list of ``{"from", "to", "value"}`` records plus
``directed``. Roles keep pm4py's order. Network analyses are a list of
``{"source", "target", "edge", "value"}`` records; with ``performance`` the
value is the list of durations in seconds, sorted.
"""

import pm4py

from harness import case
from harness.fixtures import load_log


def _log(fixtures):
    return pm4py.convert_to_event_log(load_log(fixtures["log"]))


def _sna(result):
    rows = [{"from": str(a), "to": str(b), "value": v} for (a, b), v in result.connections.items()]
    return {"directed": result.is_directed, "connections": sorted(rows, key=lambda r: (r["from"], r["to"]))}


def handover(fixtures, beta=0):
    return _sna(pm4py.discover_handover_of_work_network(_log(fixtures), beta=beta))


def working_together(fixtures):
    return _sna(pm4py.discover_working_together_network(_log(fixtures)))


def similarity(fixtures):
    return _sna(pm4py.discover_activity_based_resource_similarity(_log(fixtures)))


def subcontracting(fixtures, n=2):
    return _sna(pm4py.discover_subcontracting_network(_log(fixtures), n=n))


def roles(fixtures):
    return [
        {"activities": list(r.activities), "originator_importance": {str(k): v for k, v in r.originator_importance.items()}}
        for r in pm4py.discover_organizational_roles(_log(fixtures))
    ]


NETWORK = dict(
    out_column="case:concept:name",
    in_column="case:concept:name",
    node_column_source="org:resource",
    node_column_target="org:resource",
    edge_column="concept:name",
    edge_reference="_out",
    performance=False,
)


def network_analysis(fixtures, **params):
    result = pm4py.discover_network_analysis(load_log(fixtures["log"]), **params)
    rows = []
    for (a, b), values in result.items():
        for edge, value in values.items():
            if params["performance"]:
                value = sorted(float(x) for x in value)
            rows.append({"source": str(a), "target": str(b), "edge": str(edge), "value": value})
    return sorted(rows, key=lambda r: (r["source"], r["target"], r["edge"]))


_SNA = {
    "handover": (handover, "pm4py.discover_handover_of_work_network"),
    "working-together": (working_together, "pm4py.discover_working_together_network"),
    "similarity": (similarity, "pm4py.discover_activity_based_resource_similarity"),
    "subcontracting": (subcontracting, "pm4py.discover_subcontracting_network"),
    "roles": (roles, "pm4py.discover_organizational_roles"),
}

for _name in ["running-example", "receipt", "reviewing"]:
    for _kind, (_fn, _function) in _SNA.items():
        # The receipt similarity network has 48 x 47 ordered pairs (250 KB).
        if (_kind, _name) != ("similarity", "receipt"):
            case(f"{_kind}-{_name}", fixture=f"{_name}.csv", functions=[_function])(_fn)

case("handover-running-example-beta", fixture="running-example.csv",
     functions=["pm4py.discover_handover_of_work_network"], params={"beta": 0.5})(handover)
case("handover-receipt-beta", fixture="receipt.csv",
     functions=["pm4py.discover_handover_of_work_network"], params={"beta": 1})(handover)
case("subcontracting-running-example-n3", fixture="running-example.csv",
     functions=["pm4py.discover_subcontracting_network"], params={"n": 3})(subcontracting)
case("subcontracting-receipt-n3", fixture="receipt.csv",
     functions=["pm4py.discover_subcontracting_network"], params={"n": 3})(subcontracting)

_NA = ["pm4py.discover_network_analysis"]
case("network-analysis-running-example", fixture="running-example.csv", functions=_NA,
     params=NETWORK)(network_analysis)
case("network-analysis-running-example-performance", fixture="running-example.csv", functions=_NA,
     params={**NETWORK, "performance": True})(network_analysis)
case("network-analysis-running-example-target-edge", fixture="running-example.csv", functions=_NA,
     params={**NETWORK, "edge_reference": "_in"})(network_analysis)
case("network-analysis-running-example-by-resource", fixture="running-example.csv", functions=_NA,
     params={**NETWORK, "out_column": "org:resource", "in_column": "org:resource",
             "node_column_source": "concept:name", "node_column_target": "concept:name",
             "edge_column": "case:concept:name"})(network_analysis)
case("network-analysis-receipt", fixture="receipt.csv", functions=_NA,
     params=NETWORK)(network_analysis)
