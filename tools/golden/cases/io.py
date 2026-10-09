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
