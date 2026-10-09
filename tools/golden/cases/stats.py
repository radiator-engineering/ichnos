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


from pm4py.statistics.traces.generic.log import case_statistics as cs
from pm4py.statistics.traces.generic.log import case_arrival
from pm4py.statistics.traces.generic.pandas import case_statistics as cs_table
from pm4py.statistics.traces.generic.common import case_duration
from pm4py.statistics.traces.cycle_time.util import compute as cycle_compute
from pm4py.statistics.traces.cycle_time.log import get as cycle_get
from pm4py.statistics.concurrent_activities.log import get as concurrent
from pm4py.statistics.eventually_follows.log import get as eventually
from pm4py.statistics.eventually_follows.uvcl import get as eventually_sequences
from pm4py.algo.discovery.inductive.dtypes.im_ds import IMDataStructureUVCL
from pm4py.statistics.overlap.cases.log import get as overlap_cases
from pm4py.statistics.overlap.interval_events.log import get as overlap_events
from pm4py.statistics.overlap.utils import compute as overlap_compute
from pm4py.statistics.passed_time.log.variants import prepost, pre, post
from pm4py.statistics.service_time.log import get as service
from pm4py.statistics.util import times_bipartite_matching
from collections import Counter


def case_statistics(fixtures: dict[str, Path]):
    df = load_log(fixtures["log"])
    # pandas 3's Arrow string aggregation creates a list extension array that
    # its factorizer cannot encode. Use the oracle's ordinary object backend.
    df["concept:name"] = df["concept:name"].astype(object)
    log = pm4py.convert_to_event_log(df, stream_postprocessing=True)
    descriptions = cs.get_cases_description(log)
    _, durations = variant_get.get_variants_along_with_case_durations(log)
    activities = sorted(pm4py.get_event_attribute_values(log,"concept:name"))
    rows = cs_table.get_variants_df_with_case_duration(df)
    return {
        "descriptions": descriptions,
        "description_options": [list(cs.get_cases_description(log, {"sort_by_index": key, "sort_ascending": False, "max_ret_cases": 3}).items()) for key in range(4)],
        "durations": pm4py.get_all_case_durations(log),
        "business_durations": pm4py.get_all_case_durations(log,business_hours=True),
        "individual": {cid: pm4py.get_case_duration(log,cid) for cid in descriptions},
        "quartile": cs.get_first_quartile_case_duration(log),
        "median": cs.get_median_case_duration(log),
        "arrival": pm4py.get_case_arrival_average(log),
        "dispersion": case_arrival.get_case_dispersion_avg(log),
        "business_arrival": case_arrival.get_case_arrival_avg(log,{"business_hours":True}),
        "business_dispersion": case_arrival.get_case_dispersion_avg(log,{"business_hours":True}),
        "variants": cs.get_variant_statistics(log,{"var_durations":durations}),
        "variants_limited": cs.get_variant_statistics(log,{"max_variants_to_return":2}),
        "variant_rows": [{"case_id": str(cid), "variant": list(row["variant"]), "duration": float(row["caseDuration"])} for cid,row in rows.iterrows()],
        "variant_list": cs_table.get_variants_df_and_list(df)[1],
        "indexed_lengths": {str(k):len(v) for k,v in cs.index_log_caseid(log).items()},
        "events": {str(cid):[str(e["concept:name"]) for e in cs.get_events(log,cid)] for cid in cs.index_log_caseid(log)},
        "self_distances": pm4py.get_minimum_self_distances(log),
        "witnesses": {a:sorted(v) for a,v in pm4py.get_minimum_self_distance_witnesses(log).items()},
        "rework": pm4py.get_rework_cases_per_activity(log),
        "positions": {a:{str(k):v for k,v in pm4py.get_activity_position_summary(log,a).items()} for a in activities},
        "kde": cs.get_kde_caseduration(log,{"graph_points":20}),
        "kde_values": [case_duration.get_kde_caseduration(v,{"graph_points":20}) for v in [[],[5],[0,1,4,8]]],
    }


def _edges(counts):
    return [{"source":a,"target":b,"count":n} for (a,b),n in sorted(counts.items())]


def time_statistics(fixtures: dict[str, Path], interval=False):
    df = load_log(fixtures["log"])
    log = pm4py.convert_to_event_log(df,stream_postprocessing=True)
    start_key = "start_timestamp" if interval else "time:timestamp"
    params = {"pm4py:param:start_timestamp_key":start_key}
    activities=sorted(pm4py.get_event_attribute_values(log,"concept:name"))
    projection=pm4py.project_on_event_attribute(log,"concept:name")
    return {
        "cycle": cycle_get.apply(log,params),
        "cycle_values": [cycle_compute.cycle_time(v,n) for v,n in [([(0,5),(3,8)],1), ([(0,3),(5,9),(11,12)],2)]],
        "service": {agg:service.apply(log,{**params,"aggregationMeasure":agg}) for agg in ["mean","median","min","max","sum"]},
        "business_service": service.apply(log,{**params,"business_hours":True}),
        "concurrent": _edges(concurrent.apply(log,params)),
        "concurrent_strict": _edges(concurrent.apply(log,{**params,"strict":True})),
        "eventually": _edges(eventually.apply(log,params)),
        "eventually_first": _edges(eventually.apply(log,{**params,"keep_first_following":True})),
        "eventually_sequences": _edges(eventually_sequences.apply(IMDataStructureUVCL(Counter(tuple(t) for t in projection)))),
        "overlap_cases": overlap_cases.apply(log,params),
        "overlap_events": overlap_events.apply(log,params),
        "overlap_values": overlap_compute.apply([(0,1),(0,1),(1,2),(3,4)]),
        "passed": {a:prepost.apply(log,a,params) for a in activities},
        "passed_median": {a:prepost.apply(log,a,{**params,"aggregationMeasure":"median"}) for a in activities},
        "passed_stdev": {a:prepost.apply(log,a,{**params,"aggregationMeasure":"stdev"}) for a in activities},
        "business_passed": {a:prepost.apply(log,a,{**params,"business_hours":True}) for a in activities},
        "pre": {a:pre.apply(log,a,params) for a in activities},
        "post": {a:post.apply(log,a,params) for a in activities},
    }


def stats_util(fixtures):
    inputs = [([0,5,12],[3,9,16]),([0,10],[2,5,15]),([0,5,10],[3,12]),([10,20],[0,15]),([0,0],[1,2])]
    return [{"left":a,"right":b,"pairs":times_bipartite_matching.exact_match_minimum_average(a,b)} for a,b in inputs]


for name in ["running-example", "receipt", "roadtraffic100traces", "interval_event_log"]:
    case(f"cases-{name.replace('_','-')}",fixture=f"{name}.csv",functions=[
        "pm4py.get_all_case_durations","pm4py.get_case_duration","pm4py.get_case_arrival_average",
        "pm4py.get_rework_cases_per_activity","pm4py.get_minimum_self_distances",
        "pm4py.get_minimum_self_distance_witnesses","pm4py.get_activity_position_summary",
        *[f"pm4py.statistics.traces.generic.log.case_statistics.{fn}" for fn in ["get_cases_description","get_variant_statistics","index_log_caseid","get_events","get_first_quartile_case_duration","get_median_case_duration","get_kde_caseduration"]],
        "pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg",
        "pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_with_case_duration",
        "pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_and_list",
        "pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration",
    ])(case_statistics)
    case(f"time-{name.replace('_','-')}",fixture=f"{name}.csv",params={"interval":name=="interval_event_log"},functions=[
        "pm4py.statistics.traces.cycle_time.log.get.apply","pm4py.statistics.traces.cycle_time.util.compute.cycle_time",
        "pm4py.statistics.concurrent_activities.log.get.apply","pm4py.statistics.eventually_follows.log.get.apply",
        "pm4py.statistics.eventually_follows.uvcl.get.apply","pm4py.statistics.overlap.cases.log.get.apply",
        "pm4py.statistics.overlap.interval_events.log.get.apply","pm4py.statistics.overlap.utils.compute.apply",
        "pm4py.statistics.service_time.log.get.apply","pm4py.statistics.passed_time.log.variants.prepost.apply",
        "pm4py.statistics.passed_time.log.variants.pre.apply","pm4py.statistics.passed_time.log.variants.post.apply",
    ])(time_statistics)

case("stats-util-matching",functions=["pm4py.statistics.util.times_bipartite_matching.exact_match_minimum_average"])(stats_util)


def business_time_options(fixtures):
    from datetime import datetime, date
    from pm4py.util.business_hours import BusinessHours
    class Calendar:
        def is_working_day(self, day):
            return day != date(2024,1,8)
    pairs = [("2024-01-05T16:00:00+00:00","2024-01-08T08:00:00+00:00"),
             ("2024-01-01T00:00:00+01:00","2024-01-15T00:00:00-05:00"),
             ("2024-01-08T06:00:00+00:00","2024-01-08T19:00:00+00:00"),
             ("2024-01-08T19:00:00+00:00","2024-01-08T06:00:00+00:00")]
    slots = [(7*3600,12*3600),(12*3600+1,17*3600),(10*3600,13*3600)]
    return [{"start":a,"end":b,
             "default":BusinessHours(datetime.fromisoformat(a),datetime.fromisoformat(b)).get_seconds(),
             "slots":BusinessHours(datetime.fromisoformat(a),datetime.fromisoformat(b),business_hour_slots=slots).get_seconds(),
             "holiday":BusinessHours(datetime.fromisoformat(a),datetime.fromisoformat(b),workcalendar=Calendar()).get_seconds()} for a,b in pairs]


case("time-business-options",functions=["pm4py.util.business_hours.BusinessHours.get_seconds"])(business_time_options)
