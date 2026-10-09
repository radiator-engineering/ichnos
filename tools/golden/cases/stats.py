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


def _filters_rows(log, compact=False):
    rows = [{"id": str(t.attributes.get("concept:name", "")),
             "subcase": t.attributes.get("case:concept:name"),
             "indices": ",".join(str(int(e["@@index"])) for e in t)} for t in log]
    if not compact:
        return rows
    import hashlib
    import json
    rows.sort(key=lambda row: json.dumps(row, sort_keys=True, ensure_ascii=False, separators=(",", ":")))
    encoded = json.dumps(rows, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()
    return {"case_count": len(rows), "rows_sha256": hashlib.sha256(encoded).hexdigest(), "sample": rows[:3]}


def filters_log(fixtures):
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]), stream_postprocessing=True)
    return _filters_compute(log, compact=fixtures["log"].name != "running-example.csv")


def _filters_compute(log, compact=False):
    names = sorted({e["concept:name"] for t in log for e in t})
    a, b = names[:2]
    first = [e["concept:name"] for e in log[0]]
    path = first[:2]
    times = sorted(e["time:timestamp"] for t in log for e in t)
    lo, hi = times[len(times)//3], times[len(times)*2//3]
    durations = sorted((t[-1]["time:timestamp"]-t[0]["time:timestamp"]).total_seconds() for t in log if t)
    maximum = durations[len(durations)//2]
    results = []

    def add(function, params):
        call_params = dict(params)
        if function == "filter_variants":
            call_params["variants"] = [tuple(v) for v in params["variants"]]
        if function == "filter_directly_follows_relation":
            call_params["relations"] = [tuple(v) for v in params["relations"]]
        if function == "filter_time_range":
            from datetime import datetime
            for key in ["dt1", "dt2"]:
                call_params[key] = datetime.fromisoformat(params[key])
        filtered = getattr(pm4py, function)(log, **call_params)
        results.append({"function": function, "params": params, "result": _filters_rows(filtered, compact)})

    for retain in [True, False]:
        for function in ["filter_start_activities", "filter_end_activities"]:
            add(function, {"activities": [a], "retain": retain})
        for level in ["event", "case"]:
            add("filter_event_attribute_values", {"attribute_key": "concept:name", "values": [a,b], "level":level, "retain":retain})
        add("filter_trace_attribute_values", {"attribute_key":"concept:name", "values":[t.attributes["concept:name"] for t in log[:2]], "retain":retain})
        add("filter_variants", {"variants":[first], "retain":retain})
        add("filter_directly_follows_relation", {"relations":[path], "retain":retain})
        add("filter_eventually_follows_relation", {"relations":[path,[a,b,a]], "retain":retain})
        add("filter_paths_performance", {"path":path, "min_performance":0, "max_performance":maximum, "keep":retain})
        add("filter_four_eyes_principle", {"activity1":a,"activity2":b,"keep_violations":not retain})
        add("filter_activity_done_different_resources", {"activity":a,"keep_violations":retain})
        for pattern in [["...",a,"...",b,"..."], first, [a,"..."], ["...",b]]:
            add("filter_trace_segments", {"admitted_traces":[pattern], "positive":retain})
    for level in ["cases","events"]:
        add("filter_log_relative_occurrence_event_attribute", {"min_relative_stake":0.3,"level":level})
    add("filter_case_size", {"min_size":3,"max_size":len(first)})
    add("filter_case_performance", {"min_performance":0,"max_performance":maximum})
    for minimum in [1,2,3]:
        add("filter_activities_rework", {"activity":a,"min_occurrences":minimum})
    for k in [0,1,3]:
        add("filter_variants_top_k", {"k":k})
    for coverage in [0.1,0.5]:
        add("filter_variants_by_coverage_percentage", {"min_coverage_percentage":coverage})
    for activity in [a,first[0],first[-1]]:
        for strict in [True,False]:
            for occurrence in ["first","last"]:
                for function in ["filter_prefixes","filter_suffixes"]:
                    add(function,{"activity":activity,"strict":strict,"first_or_last":occurrence})
    for starts,ends in [(path[:1],path[1:]),([a],[a]),([a,b],[a,b])]:
        add("filter_between", {"act1":starts,"act2":ends})
    for mode in ["events","traces_contained","traces_intersecting","traces_starting_in","traces_starting_in_exclude","traces_completing_in","traces_completing_in_exclude"]:
        add("filter_time_range", {"dt1":lo.isoformat(),"dt2":hi.isoformat(),"mode":mode})
    return results


def filters_edges(fixtures):
    from pm4py.objects.log.obj import Event, EventLog, Trace
    from datetime import datetime, timedelta, timezone
    log = EventLog()
    index = 0
    for i, (activities, resources) in enumerate([
        (["A","B","A","B"], ["r1","r2","r2","r1"]),
        (["A","A","B"], ["r1","r1","r1"]),
        (["B"], [None]),
        (["A","B"], ["r1","r2"]),
        ([], []),
    ]):
        trace = Trace(attributes={"concept:name":f"s{i}", "marker":"kept"})
        for activity, resource in zip(activities, resources):
            event = Event({"concept:name":activity, "@@index":index,
                "time:timestamp":datetime(2024,1,1,tzinfo=timezone.utc)+timedelta(seconds=index*5)})
            if resource is not None:
                event["org:resource"] = resource
            trace.append(event)
            index += 1
        log.append(trace)
    return _filters_compute(log)


def filters_dfg(fixtures):
    log = pm4py.convert_to_event_log(load_log(fixtures["log"]), stream_postprocessing=True)
    dfg, starts, ends = pm4py.discover_dfg(log)
    def describe(graph, sa, ea):
        return {"edges":[[a,b,n] for (a,b),n in sorted(graph.items())], "start":sa,"end":ea}
    rows = []
    for percentage in [0,0.2,0.5,1]:
        for function in ["filter_dfg_activities_percentage","filter_dfg_paths_percentage"]:
            rows.append({"function":function,"percentage":percentage,"result":describe(*getattr(pm4py,function)(dfg,starts,ends,percentage))})
    return {"input":describe(dfg,starts,ends),"filters":rows}


_FILTER_FUNCTIONS = ["pm4py."+name for name in (
    "filter_log_relative_occurrence_event_attribute filter_start_activities filter_end_activities "
    "filter_event_attribute_values filter_trace_attribute_values filter_variants filter_directly_follows_relation "
    "filter_eventually_follows_relation filter_time_range filter_between filter_case_size filter_case_performance "
    "filter_activities_rework filter_paths_performance filter_variants_top_k filter_variants_by_coverage_percentage "
    "filter_prefixes filter_suffixes filter_four_eyes_principle filter_activity_done_different_resources filter_trace_segments").split()]
case("filters-log-edges",fixture="running-example.csv",functions=_FILTER_FUNCTIONS)(filters_edges)
for _name in ["running-example","receipt","roadtraffic100traces"]:
    case(f"filters-log-{_name}",fixture=f"{_name}.csv",functions=_FILTER_FUNCTIONS)(filters_log)
    case(f"filters-dfg-{_name}",fixture=f"{_name}.csv",functions=["pm4py.filter_dfg_activities_percentage","pm4py.filter_dfg_paths_percentage"])(filters_dfg)


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


def case_review(fixtures, interval=False):
    import json
    import polars as pl
    from pm4py.statistics.traces.generic.polars import case_statistics as cs_polars
    from pm4py.statistics.concurrent_activities.polars import get as concurrent_polars
    from pm4py.statistics.eventually_follows.polars import get as eventually_polars
    from pm4py.statistics.passed_time.log import algorithm as passed_algorithm
    from pm4py.util import constants
    df = load_log(fixtures["log"])
    log = pm4py.convert_to_event_log(df, stream_postprocessing=True)
    start_key = "start_timestamp" if interval else "time:timestamp"
    columns = list(dict.fromkeys(["case:concept:name", "concept:name", "time:timestamp", start_key, "@@index"]))
    lf = pl.from_pandas(df[columns]).lazy()
    if not interval:
        lf=lf.with_columns(pl.col("time:timestamp").alias("start_timestamp"))
    relation_start="start_timestamp"
    ordered = lf.sort(["case:concept:name",relation_start,"time:timestamp"],maintain_order=True).collect().to_dicts()
    positions = {int(e["@@index"]):(ti,ei) for ti,t in enumerate(log) for ei,e in enumerate(t)}
    def relations(frame, duration, business=False):
        result=[]
        for row in frame.collect().to_dicts():
            source = positions[int(ordered[row["__index__"]]["@@index"])]
            target = positions[int(ordered[row["__index___2"]]["@@index"])]
            if duration == "__diff_maxs_minc":
                precise=(min(row["time:timestamp"],row["time:timestamp_2"])-max(row[relation_start],row[relation_start+"_2"])).total_seconds()
            else:
                precise=float(row[duration]) if business else (row[relation_start+"_2"]-row["time:timestamp"]).total_seconds()
            result.append({"trace":source[0],"source":source[1],"target":target[1],"duration":float(row[duration]),"precise_duration":precise})
        # Compact representation retains every event pair and both duration values.
        return [",".join(str(r[k]) for k in ["trace","source","target","duration","precise_duration"]) for r in sorted(result,key=lambda r:(r["trace"],r["source"],r["target"]))]
    def variant_rows(frame):
        return sorted([{"case_id":str(row["case:concept:name"]),"variant":list(row["variant"])} for row in frame.to_dicts()],key=lambda r:r["case_id"])
    pandas_rows = cs_table.get_variants_df(df)
    polars_rows, polars_list = cs_polars.get_variants_df_and_list(lf)
    params={"pm4py:param:start_timestamp_key":start_key}
    return {
        "pandas_variants":sorted([{"case_id":str(cid),"variant":list(row["variant"])} for cid,row in pandas_rows.iterrows()],key=lambda r:r["case_id"]),
        "polars_variants":variant_rows(cs_polars.get_variants_df(lf)),
        "polars_variants_and_list_rows":variant_rows(polars_rows),
        "polars_variants_list":polars_list,
        "kde_log_json":json.loads(cs.get_kde_caseduration_json(log,{"graph_points":20})),
        "kde_values_json":json.loads(case_duration.get_kde_caseduration_json([0,1,4,8],{"graph_points":20})),
        "kde_small":[case_duration.get_kde_caseduration([0,1,4,8],{"graph_points":gp}) for gp in [2,3]],
        "passed_algorithm":{a:passed_algorithm.apply(log,a,parameters=params) for a in sorted({e["concept:name"] for t in log for e in t})},
        "concurrent_rows":relations(concurrent_polars.get_concurrent_events_dataframe(lf,start_timestamp_key=relation_start),"__diff_maxs_minc"),
        "partial_rows":relations(eventually_polars.get_partial_order_dataframe(lf,start_timestamp_key=relation_start,keep_first_following=False),constants.DEFAULT_FLOW_TIME),
        "partial_business_rows":relations(eventually_polars.get_partial_order_dataframe(lf,start_timestamp_key=relation_start,keep_first_following=False,business_hours=True,business_hours_slot=constants.DEFAULT_BUSINESS_HOUR_SLOTS),constants.DEFAULT_FLOW_TIME,business=True),
    }


def case_typed_ids(fixtures):
    from pm4py.objects.log.obj import Event, EventLog, Trace
    from datetime import datetime, timedelta, timezone
    def make(ids):
        log=EventLog()
        for i,cid in enumerate(ids):
            start=datetime(2024,1,1,tzinfo=timezone.utc)+timedelta(seconds=i*10)
            log.append(Trace([Event({"concept:name":"A","time:timestamp":start}),Event({"concept:name":"B","time:timestamp":start+timedelta(seconds=i+1)})],attributes={"concept:name":cid}))
        return log
    def describe(ids,sort):
        log=make(ids)
        rows=cs.get_cases_description(log,{"enable_sort":sort})
        index=cs.index_log_caseid(log)
        ordered=sorted(log,key=lambda t:t.attributes["concept:name"]) if sort else log
        typed={}
        for trace in ordered:
            typed[trace.attributes["concept:name"]]=next(iter(cs.get_cases_description(EventLog([trace]),{"enable_sort":False}).values()))
        return {"typed_descriptions":[{"id":cid,"type":type(cid).__name__,"duration":row["caseDuration"]} for cid,row in typed.items()],"ids":ids,"sort":sort,"descriptions":[{"id":cid,"type":type(cid).__name__,"duration":row["caseDuration"]} for cid,row in rows.items()],"indexed":[{"id":cid,"type":type(cid).__name__,"duration":(trace[-1]["time:timestamp"]-trace[0]["time:timestamp"]).total_seconds()} for cid,trace in index.items()]}
    return [describe([10,2,1.0,True],True),describe([1,"1"],False),describe([9007199254740993,9007199254740992],True)]


case("case-typed-ids",functions=["pm4py.statistics.traces.generic.log.case_statistics.get_cases_description","pm4py.statistics.traces.generic.log.case_statistics.index_log_caseid"])(case_typed_ids)
for _name in ["running-example","receipt","roadtraffic100traces","interval_event_log"]:
    case(f"case-review-{_name.replace('_','-')}",fixture=f"{_name}.csv",params={"interval":_name=="interval_event_log"},functions=[
        "pm4py.statistics.passed_time.log.algorithm.apply",
        "pm4py.statistics.concurrent_activities.polars.get.get_concurrent_events_dataframe",
        "pm4py.statistics.eventually_follows.polars.get.get_partial_order_dataframe",
        "pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration_json",
        "pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json",
        "pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df",
        "pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df_and_list",
        "pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df",
    ])(case_review)


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
