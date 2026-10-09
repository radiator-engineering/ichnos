"""Log I/O: every readable XES, CSV and Parquet fixture, with typed columns."""

import re
import json
import hashlib
from collections import Counter
from datetime import datetime, timezone
import pm4py
import pandas as pd

from harness import case
from harness.fixtures import load_log


def summarize(fixtures, variant="iterparse"):
    path = fixtures["log"]
    if path.name.endswith((".xes", ".xes.gz")):
        log = pm4py.read_xes(str(path), variant=variant, return_legacy_log_object=True)
    else:
        log = pm4py.convert_to_event_log(load_log(path))
    def types(objects):
        result = {}
        for attrs in objects:
            for key, value in attrs.items():
                # Arrow nulls become absent core attributes; report the types of
                # present values rather than pandas' missing-value sentinels.
                if value is None or (not isinstance(value, (dict, list, tuple)) and pd.isna(value)):
                    continue
                if isinstance(value, dict) and "value" in value and "children" in value:
                    if value["value"] is None:
                        kind = "list" if isinstance(value["children"], list) else "container"
                    else:
                        kind = type(value["value"]).__name__
                else:
                    kind = type(value).__name__
                kind = {"str": "string", "bool": "boolean", "datetime": "date", "Timestamp": "date"}.get(kind, kind)
                result.setdefault(key, set()).add(kind)
        return {key: sorted(values) for key, values in result.items()}
    def digest(value):
        data = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(data.encode("utf-8")).hexdigest()

    def attributes(attrs):
        return {str(k): attribute(v) for k, v in attrs.items()
                if k is not None and v is not None and
                (isinstance(v, (dict, list, tuple)) or not pd.isna(v))}

    def attribute(value):
        if isinstance(value, dict) and "value" in value and "children" in value:
            children = value["children"]
            if value["value"] is None:
                if isinstance(children, list):
                    return {"list": [[str(k), attribute(v)] for k, v in children]}
                return {"container": attributes(dict(children))}
            return {"value": attribute(value["value"]), "meta": attributes(children)}
        if isinstance(value, datetime):
            return value.astimezone(timezone.utc).isoformat(timespec="microseconds")
        return value

    sequences = [[str(e["concept:name"]) if "concept:name" in e else None for e in t] for t in log]
    profiles = {}
    for t in log:
        profile = {"trace": types([t.attributes]), "events": types(t)}
        key = digest(profile)
        profiles.setdefault(key, {**profile, "count": 0})["count"] += 1
    variants = Counter(digest(seq) for seq in sequences)
    return {
        "n_cases": len(log),
        "n_events": sum(len(trace) for trace in log),
        "trace_types": types(trace.attributes for trace in log),
        "event_types": types(event for trace in log for event in trace),
        "sequence_sha256": digest(sequences),
        "variants": dict(sorted(variants.items())),
        "type_profiles": dict(sorted(profiles.items())),
        "samples": [{"attributes": attributes(t.attributes),
                     "events": [attributes(e) for e in t]} for t in list(log)[:3]],
    }



# These fixtures were verified against the pinned oracle. They include every
# readable event log; OCEL tables and tables missing standard event keys fail
# pm4py.format_dataframe and are intentionally not registered here.
READABLE_FIXTURES = [
    'bpic2012.xes.gz',
    'fairness/hiring_log_high.xes.gz',
    'fairness/hospital_log_high.xes.gz',
    'fairness/lending_log_high.xes.gz',
    'fairness/renting_log_high.xes.gz',
    'helpdesk.xes.gz',
    'inh_res_nets/logs/cyber_incident_response.xes',
    'inh_res_nets/logs/emergency_evacuation.xes',
    'inh_res_nets/logs/hospital_discharge.xes',
    'inh_res_nets/logs/insurance_claim.xes',
    'inh_res_nets/logs/loan_underwriting.xes',
    'inh_res_nets/logs/manufacturing_batch.xes',
    'inh_res_nets/logs/order_fulfillment.xes',
    'interval_event_log.xes',
    'partial order example 3.xes',
    'partial order example.xes',
    'receipt.xes',
    'reviewing.xes',
    'roadtraffic100traces.xes',
    'roadtraffic50traces.xes',
    'roadtraffic_10000.xes.gz',
    'running-example.xes',
    'synthetic_logs/a12/a12f0n00.xes.gz',
    'synthetic_logs/a12/a12f0n05.xes.gz',
    'synthetic_logs/a12/a12f0n10.xes.gz',
    'synthetic_logs/a12/a12f0n20.xes.gz',
    'synthetic_logs/a12/a12f0n50.xes.gz',
    'synthetic_logs/a22/a22f0n00.xes.gz',
    'synthetic_logs/a22/a22f0n05.xes.gz',
    'synthetic_logs/a22/a22f0n10.xes.gz',
    'synthetic_logs/a22/a22f0n20.xes.gz',
    'synthetic_logs/a22/a22f0n50.xes.gz',
    'synthetic_logs/a32/a32f0n00.xes.gz',
    'synthetic_logs/a32/a32f0n05.xes.gz',
    'synthetic_logs/a32/a32f0n10.xes.gz',
    'synthetic_logs/a32/a32f0n20.xes.gz',
    'synthetic_logs/a32/a32f0n50.xes.gz',
    'synthetic_logs/a42/a42f0n00.xes.gz',
    'synthetic_logs/a42/a42f0n05.xes.gz',
    'synthetic_logs/a42/a42f0n10.xes.gz',
    'synthetic_logs/a42/a42f0n20.xes.gz',
    'synthetic_logs/a42/a42f0n50.xes.gz',
]

for rel in READABLE_FIXTURES:
    case_id = re.sub(r"[^a-z0-9_-]", "-", rel.lower())
    case(case_id, fixture=rel, functions=["pm4py.read_xes"] if ".xes" in rel else ["pm4py.format_dataframe", "pm4py.convert_to_event_log"])(summarize)

# The 2.0 backend preserves containers which the legacy iterparse backend
# ignores. This fixture intentionally lacks activity names.
case("xes_20-xes", fixture="xes_20.xes", functions=["pm4py.read_xes"], params={"variant": "iterparse_20"})(summarize)

# Table cases are introduced by the stacked CSV/Parquet package.
TABLE_FIXTURES = [
    'correlation_mining.csv',
    'interleavings/receipt_even.csv',
    'interleavings/receipt_odd.csv',
    'interval_event_log.csv',
    'receipt.csv',
    'receipt.parquet',
    'reviewing.csv',
    'roadtraffic.parquet',
    'roadtraffic100traces.csv',
    'running-example.csv',
    'running-example.parquet',
]
for rel in TABLE_FIXTURES:
    case_id = re.sub(r"[^a-z0-9_-]", "-", rel.lower())
    case(case_id, fixture=rel, functions=["pm4py.format_dataframe", "pm4py.convert_to_event_log"])(summarize)


# Imported PNML ids come from the file, so structure can be compared exactly.
def summarize_model(fixtures, max_markings=10000):
    path = fixtures["model"]
    if path.suffix == ".pnml":
        from pm4py.objects.petri_net.obj import InhibitorNet, ResetNet
        from pm4py.objects.petri_net.importer import importer
        from pm4py.algo.discovery.footprints.petri.variants import reach_graph
        from pm4py.objects.petri_net.semantics import ClassicSemantics
        class StateSpaceLimit(Exception):
            pass
        class LimitedSemantics(ClassicSemantics):
            def __init__(self):
                self.markings = set()
            def weak_execute(self, transition, net, marking, **kwargs):
                result = super().weak_execute(transition, net, marking, **kwargs)
                self.markings.add(result)
                if len(self.markings) > max_markings:
                    raise StateSpaceLimit()
                return result
        net, im, fm = pm4py.read_pnml(str(path))
        _, _, _, stochastic = importer.apply(str(path), parameters={
            "auto_guess_final_marking": False, "return_stochastic_map": True})
        if "inh_res_nets" in path.parts:
            footprints, status = None, "unbounded"
        else:
            semantics = LimitedSemantics()
            semantics.markings.add(im)
            try:
                footprints = reach_graph.apply(net, im, parameters={"petri_semantics": semantics})
                status = "complete"
            except StateSpaceLimit:
                footprints, status = None, "state_space_limit"
        return {
            "model_kind": "petri_net",
            "arc_structure": sorted([
                [a.source.name, a.target.name, a.weight,
                 "inhibitor" if isinstance(a, InhibitorNet.InhibitorArc) else
                 "reset" if isinstance(a, ResetNet.ResetArc) else "normal"]
                for a in net.arcs]),
            "transition_labels": sorted([[t.name, t.label] for t in net.transitions]),
            "initial_marking": {p.name: n for p, n in im.items()},
            "final_marking": {p.name: n for p, n in fm.items()},
            "places": len(net.places), "transitions": len(net.transitions),
            "arcs": len(net.arcs),
            "silent_transitions": sum(t.label is None for t in net.transitions),
            "inhibitor_arcs": sum(isinstance(a, InhibitorNet.InhibitorArc) for a in net.arcs),
            "reset_arcs": sum(isinstance(a, ResetNet.ResetArc) for a in net.arcs),
            "initial_tokens": sum(im.values()), "final_tokens": sum(fm.values()),
            "stochastic": sorted([{
                "label": t.label,
                "distribution_type": rv.get_distribution_type(),
                "priority": rv.get_priority(), "weight": rv.get_weight(),
            } for t, rv in stochastic.items()], key=lambda x: json.dumps(x, sort_keys=True)),
            "footprints": footprints, "footprints_status": status,
        }
    if path.suffix == ".ptml":
        tree = pm4py.read_ptml(str(path))
        def counts(node):
            descendants = [counts(c) for c in node.children]
            return (1 + sum(n for n, _, _ in descendants),
                    int(node.operator is None and node.label is not None) + sum(n for _, n, _ in descendants),
                    int(node.operator is None and node.label is None) + sum(n for _, _, n in descendants))
        nodes, activities, silent = counts(tree)
        return {"model_kind": "process_tree", "nodes": nodes,
                "activity_nodes": activities, "silent_nodes": silent,
                "footprints": pm4py.discover_footprints(tree)}
    graph, starts, ends = pm4py.read_dfg(str(path))
    return {"model_kind": "dfg", "edges": len(graph),
            "edge_frequency": sum(graph.values()),
            "start_activities": starts, "end_activities": ends,
            "frequencies": [[a, b, count] for (a, b), count in sorted(graph.items())],
            "footprints": pm4py.discover_footprints(graph)}

# SampleNet.pnml is unbounded; reachability-based footprints do not terminate.
# It is covered by a Rust import/round-trip test instead of a generator case.
MODEL_FIXTURES = [
    'big_wf_net.pnml',
    'data_petri_net.pnml',
    'ex1.pnml',
    'ex2.pnml',
    'inh_res_nets/cyber_incident_response.pnml',
    'inh_res_nets/emergency_evacuation.pnml',
    'inh_res_nets/hospital_discharge.pnml',
    'inh_res_nets/insurance_claim.pnml',
    'inh_res_nets/loan_underwriting.pnml',
    'inh_res_nets/manufacturing_batch.pnml',
    'inh_res_nets/order_fulfillment.pnml',
    'murata1.pnml',
    'murata2.pnml',
    'murata3.pnml',
    'receipt_one_variant.pnml',
    'roadtraffic.pnml',
    'running-example.dfg',
    'running-example.pnml',
    'running-example.ptml',
    'stochastic_running_example.pnml',
    'synthetic_logs/a12/a12.pnml',
    'synthetic_logs/a12/a12.ptml',
    'synthetic_logs/a22/a22.pnml',
    'synthetic_logs/a22/a22.ptml',
    'synthetic_logs/a32/a32.pnml',
    'synthetic_logs/a32/a32.ptml',
    'synthetic_logs/a42/a42.pnml',
    'synthetic_logs/a42/a42.ptml',
    'tree_ex_with_loops.ptml',
    'tree_ex_wo_loops.ptml',
]
for rel in MODEL_FIXTURES:
    case_id = "model-" + re.sub(r"[^a-z0-9_-]", "-", rel.lower())
    function = "pm4py.read_" + rel.rsplit(".", 1)[1]
    functions = [function, "pm4py.discover_footprints"]
    params = {}
    if rel.endswith(".pnml"):
        functions = [function, "pm4py.objects.petri_net.importer.importer.apply",
                     "pm4py.algo.discovery.footprints.petri.variants.reach_graph.apply"]
        params = {"max_markings": 10000}
    case(case_id, fixtures={"model": rel}, params=params, functions=functions)(summarize_model)

# Exact bytes produced by ichnos, read by the reference implementation.
for rel in ["special.pnml", "loop.ptml", "boundary.dfg"]:
    case("writer-" + rel.replace(".", "-"), fixtures={"model": "writer-output/" + rel},
         functions=["pm4py.read_" + rel.rsplit(".", 1)[1], "pm4py.discover_footprints"])(summarize_model)


# BPMN diagrams. ``bpmn-read-*`` is pm4py.read_bpmn of a fixture;
# ``bpmn-write-*`` reads the fixture, writes it with the etree exporter (what
# pm4py.write_bpmn does with auto_layout=False) and reads the XML back.
# ``bpmn-writer-*`` reads a file that ichnos's writer produced. pm4py gives
# nodes outside any process a random DEFAULT_PROCESS, which is removed from
# every process name here. The process id of a written diagram is the last
# process element, whose order in pm4py's output depends on hashing, so
# write cases leave it out. Failures are recorded as {"error": class name,
# "stage": "read" or "write"}.
def describe_bpmn_io(b, with_process_id=True):
    from pm4py.objects.bpmn.obj import BPMN, DEFAULT_PROCESS
    from cases.bpmn import describe_bpmn

    def process(p):
        return str(p).replace(DEFAULT_PROCESS, "")

    layout = b.get_layout()
    nodes = {str(n.get_id()): n for n in b.get_nodes()}
    described = describe_bpmn(b)
    for d in described["nodes"]:
        n = nodes[d["id"]]
        start = isinstance(n, BPMN.StartEvent)
        lay = layout.get(n)
        d.update({
            "process": process(d["process"]),
            "text": n.text if isinstance(n, BPMN.TextAnnotation) else None,
            "process_ref": n.process_ref if isinstance(n, BPMN.Participant) else None,
            "interrupting": n.get_isInterrupting() if start else None,
            "parallel_multiple": n.get_parallelMultiple() if start else None,
            "bounds": [float(lay.get_x()), float(lay.get_y()),
                       float(lay.get_width()), float(lay.get_height())],
        })
    for f in described["flows"]:
        f["process"] = process(f["process"])
    described["name"] = b.get_name()
    if not with_process_id:
        del described["process_id"]
    return described


def bpmn_read(fixtures):
    try:
        b = pm4py.read_bpmn(str(fixtures["model"]))
    except Exception as e:
        return {"error": type(e).__name__, "stage": "read"}
    return describe_bpmn_io(b)


def bpmn_write(fixtures):
    from pm4py.objects.bpmn.exporter.variants import etree
    from pm4py.objects.bpmn.importer.variants import lxml

    try:
        b = pm4py.read_bpmn(str(fixtures["model"]))
    except Exception as e:
        return {"error": type(e).__name__, "stage": "read"}
    try:
        xml = etree.get_xml_string(b)
    except Exception as e:
        return {"error": type(e).__name__, "stage": "write"}
    return describe_bpmn_io(lxml.import_from_string(xml), with_process_id=False)


BPMN_FIXTURES = [
    'a32f0n00.bpmn',
    'more_models/SimpleParallel.bpmn',
    'more_models/Subprocess1.bpmn',
    'more_models/Subprocess3.bpmn',
    'more_models/ch7_CreditAppSimulation.bpmn',
    'more_models/ch7_InsuranceClaimsSimulationNormalSeason.bpmn',
    'more_models/simple_model.bpmn',
    'receipt.bpmn',
    'running-example.bpmn',
    'synthetic-bpmn/all_kinds.bpmn',
]
BPMN_WRITE_FUNCTIONS = [
    "pm4py.read_bpmn",
    "pm4py.objects.bpmn.exporter.variants.etree.get_xml_string",
    "pm4py.objects.bpmn.importer.variants.lxml.import_from_string",
]
for rel in BPMN_FIXTURES:
    stem = re.sub(r"[^a-z0-9_-]", "-", rel.rsplit("/", 1)[-1].rsplit(".", 1)[0].lower())
    case(f"bpmn-read-{stem}", fixtures={"model": rel}, functions=["pm4py.read_bpmn"])(bpmn_read)
    case(f"bpmn-write-{stem}", fixtures={"model": rel}, functions=BPMN_WRITE_FUNCTIONS)(bpmn_write)
case("bpmn-writer-special", fixtures={"model": "writer-output/special.bpmn"},
     functions=["pm4py.read_bpmn"])(bpmn_read)
