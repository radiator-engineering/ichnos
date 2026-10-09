"""Attribute and variant statistics checked against the pinned pm4py oracle."""
from pathlib import Path
import pm4py
from harness import case
from harness.fixtures import load_log
from pm4py.statistics.attributes.log import get, select
from pm4py.statistics.attributes.common import get as common
from pm4py.statistics.variants.log import get as variant_get
from pm4py.statistics.rework.log import get as rework
from pm4py.statistics.rework.cases.log import get as case_rework
from pm4py.statistics.chaotic_activities.variants import niek_sidorova
from pm4py.statistics.start_activities.common import get as start_common
from pm4py.statistics.end_activities.common import get as end_common


def attributes(fixtures: dict[str, Path]):
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]), stream_postprocessing=True)
    names = sorted(get.get_all_event_attributes_from_log(log))
    trace_names = sorted(get.get_all_trace_attributes_from_log(log))
    counts = get.get_attribute_values(log, "concept:name")
    ordered = common.get_sorted_attributes_list(counts)
    return {
        "event_attributes": names,
        "trace_attributes": trace_names,
        "activities": counts,
        "once": get.get_attribute_values(log, "concept:name", {"keep_once_per_case": True}),
        "trace_ids": {str(k): v for k, v in get.get_trace_attribute_values(log, "concept:name").items()},
        "start": pm4py.get_start_activities(log),
        "end": pm4py.get_end_activities(log),
        "sorted": ordered,
        "threshold": common.get_attributes_threshold(ordered, 0.6),
        "start_threshold": start_common.get_start_activities_threshold(start_common.get_sorted_start_activities_list(pm4py.get_start_activities(log)),0.6),
        "end_threshold": end_common.get_end_activities_threshold(end_common.get_sorted_end_activities_list(pm4py.get_end_activities(log)),0.6),
        "event_presence": select.check_event_attributes_presence(log, names.copy()),
        "trace_presence": select.check_trace_attributes_presence(log, trace_names.copy()),
        "selection": [sorted(group) for group in select.select_attributes_from_log_for_tree(log, max_cases_for_attr_selection=len(log))],
        "distributions": {d: list(zip(*get.get_events_distribution(log, d))) for d in ["days_month", "months", "years", "hours", "days_week", "weeks"]},
        "numeric_kde": [common.get_kde_numeric_attribute(v, {"graph_points": 20}) for v in [[], [0, 0], [1, 2, 4, 8], [-8, -4, -2, -1], [-2, 0, 3]]],
        "silverman": common.get_kde_numeric_attribute([1,2,4,8], {"graph_points":20,"bw_method":"silverman"}),
        "factor": common.get_kde_numeric_attribute([1,2,4,8], {"graph_points":20,"bw_method":0.5}),
        "date_kde": [list(x) for x in get.get_kde_date_attribute(log, parameters={"graph_points": 20})][1],
        "date_x": [x.timestamp() for x in get.get_kde_date_attribute(log, parameters={"graph_points":20})[0]],
        "event_numeric_kde": get.get_kde_numeric_attribute(log,"@@index",parameters={"graph_points":20}),
    }


for name in ["running-example", "receipt", "roadtraffic100traces"]:
    case(f"attributes-{name}", fixture=f"{name}.csv", functions=[
        "pm4py.get_start_activities", "pm4py.get_end_activities",
        "pm4py.statistics.attributes.log.get", "pm4py.statistics.attributes.log.select",
        "pm4py.statistics.attributes.common.get",
    ])(attributes)


def variants(fixtures: dict[str, Path]):
    dataframe = load_log(fixtures["log"])
    log = pm4py.convert_to_event_log(dataframe, stream_postprocessing=True)
    groups, durations = variant_get.get_variants_along_with_case_durations(log)
    paths = pm4py.get_variants_paths_duration(dataframe)
    return {
        "variants": [{"variant": list(v), "count": len(groups[v]), "indices": ids, "durations": list(durations[v])}
                     for v, ids in variant_get.get_variants_from_log_trace_idx(log).items()],
        "sorted": variant_get.get_variants_sorted_by_count(groups),
        "language": [{"variant": list(v), "probability": p} for v, p in variant_get.get_language(log).items()],
        "split_counts": [{"variant": list(v), "count": int(df["case:concept:name"].nunique())} for v, df in pm4py.split_by_process_variant(dataframe)],
        "rework": rework.apply(log),
        "cases": {str(k): v for k,v in case_rework.apply(log).items()},
        # pm4py sums across sets; round far below the metric tolerance to
        # make fixtures independent of Python's hash seed.
        "chaotic": [{k: round(v,12) if isinstance(v,float) else v for k,v in row.items()} for row in niek_sidorova.apply(log)],
        "entropy": round(niek_sidorova.total_entropy(pm4py.project_on_event_attribute(log, "concept:name")),12),
        "segments": [{"variant": list(v), "count": n} for v,n in pm4py.get_frequent_trace_segments(log, max(2, len(log)//2)).items()],
        "minimum": max(2, len(log)//2),
        "paths": [{"variant": list(row["@@variant_column"]), "count": int(row["@@variant_count"]),
                   "position": int(row["@@index_in_trace"]), "occurrence": int(row["@@cumulative_occ_path_column"]),
                   "duration": float(row["@@flow_time"])} for _, row in paths.iterrows()],
        "path_aggregations": {agg: [float(v) for v in pm4py.get_variants_paths_duration(dataframe,times_agg=agg)["@@flow_time"]] for agg in ["median","min","max","sum"]},
    }


for name in ["running-example", "receipt", "roadtraffic100traces"]:
    case(f"variants-{name}", fixture=f"{name}.csv", functions=[
        "pm4py.get_variants", "pm4py.split_by_process_variant", "pm4py.get_variants_paths_duration",
        "pm4py.get_frequent_trace_segments", "pm4py.statistics.variants.log.get",
        "pm4py.statistics.rework.log.get.apply", "pm4py.statistics.rework.cases.log.get.apply",
        "pm4py.statistics.chaotic_activities.variants.niek_sidorova.apply",
    ])(variants)
