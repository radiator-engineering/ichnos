"""Machine-learning utilities checked against the pinned pm4py oracle.

pm4py orders some feature columns by Python's set order, which changes with
the hash seed, so features are recorded as a ``{name: column}`` map. Values
are plain floats; a missing value is NaN.

The split case seeds Python's ``random`` and records the ``_randbelow`` draws
that ``random.shuffle`` makes, then reseeds and runs pm4py, so the Rust test
can replay the same draws.
"""

import random

import pandas as pd
import pm4py

from harness import case
from harness.fixtures import load_log


def _frame(fixtures):
    df = load_log(fixtures["log"])
    if "start_timestamp" in df.columns:
        df["start_timestamp"] = pd.to_datetime(df["start_timestamp"], utc=True, format="ISO8601")
    return df


def _log(fixtures):
    return pm4py.convert_to_event_log(_frame(fixtures))


def _columns(df, skip=()):
    return {str(c): [float(v) for v in df[c]] for c in df.columns if c not in skip}


def split(fixtures, seed, train_percentage):
    log = _log(fixtures)
    random.seed(seed)
    draws = [random._inst._randbelow(i + 1) for i in reversed(range(1, len(log)))]
    random.seed(seed)
    train, test = pm4py.split_train_test(log, train_percentage=train_percentage)
    ids = lambda part: [t.attributes["concept:name"] for t in part]
    return {"draws": draws, "train": ids(train), "test": ids(test)}


for _name in ["running-example", "receipt"]:
    case(f"split-{_name}", fixture=f"{_name}.csv", params={"seed": 7, "train_percentage": 0.8},
         functions=["pm4py.split_train_test"])(split)


def prefixes(fixtures, lengths):
    log = _log(fixtures)
    out = {}
    for n in lengths:
        prefixed = pm4py.get_prefixes_from_log(log, n)
        # Case, event count and the @@index of the last kept event.
        out[str(n)] = [[t.attributes["concept:name"], len(t), int(t[-1]["@@index"])] for t in prefixed]
    return out


for _name, _lengths in [("running-example", [1, 3]), ("receipt", [3])]:
    case(f"prefixes-{_name}", fixture=f"{_name}.csv", params={"lengths": _lengths},
         functions=["pm4py.get_prefixes_from_log"])(prefixes)


def features_df(fixtures, **kwargs):
    df = pm4py.extract_features_dataframe(_frame(fixtures), include_case_id=True, **kwargs)
    return {"case_ids": [str(c) for c in df["case:concept:name"]],
            "features": _columns(df, skip=("case:concept:name",))}


_FEATURE_VARIANTS = {
    "auto": {},
    "count": {"count_occurrences": True},
    "stats": {"enable_numeric_attribute_statistics": True},
    "aggs": {"numeric_attribute_aggregations": ["sum", "std", "max", "sum"]},
    "lists": {"str_ev_attr": ["concept:name", "org:resource"], "num_ev_attr": ["Costs"],
              "str_tr_attr": ["creator"]},
}
for _variant, _params in _FEATURE_VARIANTS.items():
    case(f"features-df-running-example-{_variant}", fixture="running-example.csv", params=_params,
         functions=["pm4py.extract_features_dataframe"])(features_df)
case("features-df-roadtraffic100traces-auto", fixture="roadtraffic100traces.csv",
     functions=["pm4py.extract_features_dataframe"])(features_df)


def features_log(fixtures, cases, start_timestamp_key=None, **kwargs):
    if start_timestamp_key is not None:
        kwargs["pm4py:param:start_timestamp_key"] = start_timestamp_key
    log = _log(fixtures)
    if cases is not None:
        log = pm4py.objects.log.obj.EventLog(log[:cases], attributes=log.attributes)
    df = pm4py.extract_features_dataframe(log, **kwargs)
    return {"case_ids": [t.attributes["concept:name"] for t in log], "features": _columns(df)}


case("features-log-running-example", fixture="running-example.csv",
     params={"cases": None, "str_ev_attr": ["concept:name", "org:resource"], "num_ev_attr": ["Costs"],
             "str_tr_attr": ["creator"], "str_evsucc_attr": ["concept:name"],
             "enable_all_extra_features": True},
     functions=["pm4py.extract_features_dataframe"])(features_log)
case("features-log-interval-event-log", fixture="interval_event_log.csv",
     params={"cases": 12, "str_ev_attr": ["concept:name"], "start_timestamp_key": "start_timestamp",
             "enable_all_extra_features": True},
     functions=["pm4py.extract_features_dataframe"])(features_log)


def outcome(fixtures):
    df = _frame(fixtures)
    features = pm4py.extract_features_dataframe(df, include_case_id=True)
    out = pm4py.extract_outcome_enriched_dataframe(df)
    times = ["@@arrival_rate", "@@finish_rate", "@@diff_start_end", "@@service_time", "@@sojourn_time",
             "@@waiting_time"]
    first = out.groupby("case:concept:name", sort=True).first()
    cols = {}
    for c in features.columns:
        if c == "case:concept:name":
            continue
        merged = c if c in out.columns else f"{c}_y"
        cols[str(c)] = [float(v) for v in first[merged]]
    return {"case_ids": [str(c) for c in out["case:concept:name"]],
            "times": {t: [float(v) for v in out[t]] for t in times},
            "feature_case_ids": [str(c) for c in first.index],
            "features": cols}


for _name in ["running-example", "roadtraffic100traces"]:
    case(f"outcome-{_name}", fixture=f"{_name}.csv",
         functions=["pm4py.extract_outcome_enriched_dataframe"])(outcome)


def temporal(fixtures, grouper_freq, start_timestamp_key="time:timestamp"):
    df = pm4py.extract_temporal_features_dataframe(_frame(fixtures), grouper_freq=grouper_freq,
                                                   start_timestamp_key=start_timestamp_key)
    rows = []
    for rec in df.to_dict(orient="records"):
        row = {k: float(v) for k, v in rec.items() if k != "timestamp"}
        row["timestamp"] = rec["timestamp"].isoformat()
        rows.append(row)
    return rows


for _freq in ["W", "D", "2D", "3h", "MS", "ME", "YS", "YE"]:
    case(f"temporal-running-example-{_freq.lower()}", fixture="running-example.csv",
         params={"grouper_freq": _freq}, functions=["pm4py.extract_temporal_features_dataframe"])(temporal)
case("temporal-receipt-w", fixture="receipt.csv", params={"grouper_freq": "W"},
     functions=["pm4py.extract_temporal_features_dataframe"])(temporal)


def target(fixtures, variants):
    log = _log(fixtures)
    out = {}
    for v in variants:
        t, classes = pm4py.extract_target_vector(log, v)
        out[v] = {"target": [[[float(x) for x in row] for row in tr] if v == "next_activity"
                             else [float(x) for x in tr] for tr in t],
                  "classes": list(classes)}
    return out


case("target-running-example", fixture="running-example.csv",
     params={"variants": ["next_activity", "next_time", "remaining_time"]},
     functions=["pm4py.extract_target_vector"])(target)
case("target-roadtraffic100traces", fixture="roadtraffic100traces.csv",
     params={"variants": ["next_activity", "next_time", "remaining_time"]},
     functions=["pm4py.extract_target_vector"])(target)


def ocel_features(fixtures, version, obj_type, **kwargs):
    path = str(fixtures["log"])
    ocel = pm4py.read_ocel2(path) if version == 2 else pm4py.read_ocel(path)
    df = pm4py.extract_ocel_features(ocel, obj_type, include_obj_id=True, **kwargs)
    return {"object_ids": [str(o) for o in df[ocel.object_id_column]],
            "features": _columns(df, skip=(ocel.object_id_column,))}


for _type in ["order", "element", "delivery"]:
    case(f"ocel-example-log-{_type}", fixture="ocel/example_log.jsonocel",
         params={"version": 1, "obj_type": _type, "enable_object_work_in_progress": True,
                 "object_str_attributes": ["oattr1"], "object_num_attributes": ["oattr2"]},
         functions=["pm4py.extract_ocel_features"])(ocel_features)
case("ocel-example-log-order-defaults", fixture="ocel/example_log.jsonocel",
     params={"version": 1, "obj_type": "order", "enable_object_lifecycle_paths": False},
     functions=["pm4py.extract_ocel_features"])(ocel_features)
case("ocel-example-log-missing-type", fixture="ocel/example_log.jsonocel",
     params={"version": 1, "obj_type": "nothing"}, functions=["pm4py.extract_ocel_features"])(ocel_features)
for _type in ["Invoice", "Purchase Order"]:
    case(f"ocel-ocel20-example-{_type.lower().replace(' ', '-')}", fixture="ocel/ocel20_example.jsonocel",
         params={"version": 2, "obj_type": _type, "enable_object_work_in_progress": True,
                 "object_str_attributes": ["is_blocked"], "object_num_attributes": ["po_quantity"]},
         functions=["pm4py.extract_ocel_features"])(ocel_features)
