"""OCEL enrichment and transformations from pm4py 2.7.23.8.

Sampling records every valid subset on small populations, so Rust's independent
RNG can be checked against the complete oracle outcome space. UUID outputs are
renamed by first event occurrence. Unordered appended graph rows are sorted.
"""
import copy
import itertools
import json
import os
import random
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch
from harness import case
import pandas as pd
import pm4py
from cases.ocel import _tables
from cases.ocel_filters import synthetic as filter_log

FUNCTIONS = ["ocel_o2o_enrichment", "ocel_e2o_lifecycle_enrichment", "sample_ocel_objects",
             "sample_ocel_connected_components", "ocel_drop_duplicates", "ocel_merge_duplicates",
             "ocel_sort_by_additional_column", "ocel_add_index_based_timedelta", "cluster_equivalent_ocel",
             "ocel_drill_down", "ocel_roll_up", "ocel_unfold", "ocel_fold"]

def wire(log, merge=False):
    tables = _tables(log)
    # Rust derives these columns by id; keep only the actual typed row fields.
    for r in tables["relations"]:
        for k in ("activity", "timestamp", "type"): r.pop(k)
    tables["o2o"].sort(key=lambda r:(r["source"],r["target"],r["qualifier"] or ""))
    if merge:
        names = {}
        for e in tables["events"]:
            names.setdefault(e["id"], "event-" + str(len(names)))
            e["id"] = names[e["id"]]
        for r in tables["relations"]: r["event"] = names[r["event"]]
    return tables

def synthetic():
    log = filter_log()
    # The graph/cluster oracle requires every event and object to be related.
    log.events = log.events[log.events["ocel:eid"] != "e0"].copy()
    log.objects = log.objects[log.objects["ocel:oid"] != "orphan"].copy()
    log.o2o = log.o2o.iloc[:1].copy()
    log.e2e = log.e2e.iloc[1:].copy()
    log.object_changes = log.object_changes.iloc[:1].copy()
    log.events.loc[log.events["ocel:eid"] == "e3", "ocel:timestamp"] = log.events.iloc[0]["ocel:timestamp"]
    times = log.events.set_index("ocel:eid")["ocel:timestamp"].to_dict()
    log.relations["ocel:timestamp"] = log.relations["ocel:eid"].map(times)
    log.objects["category"] = ["north", "", None, "south", "single"]
    # A second interacting component distinguishes component draws/order.
    extra_event = {"ocel:eid": "e5", "ocel:activity": "single", "ocel:timestamp": log.events.iloc[-1]["ocel:timestamp"], "score": 2}
    log.events = pd.concat([log.events, pd.DataFrame([extra_event])], ignore_index=True)
    log.objects = pd.concat([log.objects, pd.DataFrame([
        {"ocel:oid":"x", "ocel:type":"order", "category":"west"},
        {"ocel:oid":"y", "ocel:type":"item", "category":"west"}])], ignore_index=True)
    log.relations = pd.concat([log.relations, pd.DataFrame([
        dict(extra_event, **{"ocel:oid":oid,"ocel:type":typ,"ocel:qualifier":None})
        for oid,typ in (("x","order"),("y","item"))])], ignore_index=True)
    return log

def scenarios(log):
    types=sorted(log.objects["ocel:type"].unique());typ=types[0]
    act=str(log.events.iloc[0]["ocel:activity"])
    out=[]
    def add(fn,args=None,kwargs=None,prep=None):out.append(dict(function=fn,args=args or [],kwargs=kwargs or {},prep=prep))
    for graphs in (None,[],["object_interaction_graph"],["object_descendants_graph"],["object_inheritance_graph"],["object_cobirth_graph"],["object_codeath_graph"]): add(FUNCTIONS[0],kwargs={"included_graphs":graphs})
    add(FUNCTIONS[1]);add(FUNCTIONS[4]);add(FUNCTIONS[5],kwargs={"have_common_object":False});add(FUNCTIONS[5],kwargs={"have_common_object":True})
    add(FUNCTIONS[6],["ocel:activity"])
    if "score" in log.events:add(FUNCTIONS[6],["score"]);add(FUNCTIONS[6],["score"],{"primary_column":"ocel:activity"})
    add(FUNCTIONS[7])
    for limit in (0,1,100000):add(FUNCTIONS[8],[typ],{"max_objs":limit})
    add(FUNCTIONS[8],[typ],{"exclude_object_types_from_renaming":set(types)})
    add(FUNCTIONS[8],["missing"])
    attr="category" if "category" in log.objects else "ocel:oid"
    add(FUNCTIONS[9],[typ,attr]);add(FUNCTIONS[10],[typ],prep="drill")
    add(FUNCTIONS[10],[typ,attr]);add(FUNCTIONS[10],["missing"])
    for qs in (None,[],["primary"],[""],[None]):add(FUNCTIONS[11],[act,typ],{"qualifiers":qs})
    add(FUNCTIONS[12],[act,typ],prep="unfold");add(FUNCTIONS[12],[act,typ]);add(FUNCTIONS[11],["missing",typ])
    return out

def json_kwargs(kwargs):
    return {k:sorted(v) if isinstance(v,set) else v for k,v in kwargs.items()}

def compute(log,only=None):
    results=[]
    for scenario in scenarios(log):
        fn=scenario["function"]
        if only and fn!=only:continue
        source=copy.deepcopy(log)
        if scenario["prep"]=="drill": source=pm4py.ocel_drill_down(source,scenario["args"][0],"category" if "category" in source.objects else "ocel:oid")
        if scenario["prep"]=="unfold":source=pm4py.ocel_unfold(source,*scenario["args"])
        output=getattr(pm4py,fn)(source,*scenario["args"],**scenario["kwargs"])
        if fn==FUNCTIONS[8]:
            output=sorted([{"description":[list(map(list,key[0])),list(map(list,key[1]))],
                            "central_objects":[oc.parameters["@@central_object"] for oc in logs],
                            "logs":[wire(oc) for oc in logs]} for key,logs in output.items()],key=lambda x:json.dumps(x["description"]))
        else:output=wire(output,merge=fn==FUNCTIONS[5])
        results.append(dict(scenario,kwargs=json_kwargs(scenario["kwargs"]),output=output))
    return {"input":wire(log),"results":results}

def sampling(log,components=False):
    if components:
        from pm4py.algo.transformation.ocel.split_ocel import algorithm
        population=algorithm.apply(log,variant=algorithm.Variants.CONNECTED_COMPONENTS)
        options=[dict(connected_components=n) for n in (0,1,2,100)]
        options += [dict(connected_components=2,**{key:n}) for key in ("max_num_events_per_cc","max_num_objects_per_cc","max_num_e2o_relations_per_cc") for n in (0,2,3,4,8)]
    else:
        population=list(log.objects["ocel:oid"].unique())
        options=[dict(num_objects=n) for n in (0,1,2,len(population),100)]
    results=[]
    for opt in options:
        choices=[]
        if components:
            eligible=[c for c in population if len(c.events)<=opt.get("max_num_events_per_cc",sys.maxsize) and len(c.objects)<=opt.get("max_num_objects_per_cc",sys.maxsize) and len(c.relations)<=opt.get("max_num_e2o_relations_per_cc",sys.maxsize)]
            count=min(opt["connected_components"],len(eligible))
            for subset in itertools.permutations(range(len(eligible)),count):
                with patch("random.sample",side_effect=lambda p,k,s=subset:[p[i] for i in s]):choices.append(wire(pm4py.sample_ocel_connected_components(copy.deepcopy(log),**opt)))
        else:
            count=min(opt["num_objects"],len(population))
            for subset in itertools.combinations(population,count):
                def shuffle(p,s=subset):p[:]=list(s)+[x for x in p if x not in s]
                with patch("random.shuffle",side_effect=shuffle):choices.append(wire(pm4py.sample_ocel_objects(copy.deepcopy(log),**opt)))
        results.append({"kwargs":opt,"outputs":choices})
    return {"input":wire(log),"results":results}

for fn in FUNCTIONS:
    def run(fixtures,fn=fn):
        log=synthetic()
        if fn in FUNCTIONS[2:4]: return sampling(log,fn==FUNCTIONS[3])
        result=compute(log,fn)
        variants=[]
        if fn==FUNCTIONS[7]:
            duplicate=copy.deepcopy(log)
            duplicate.events=pd.concat([duplicate.events,duplicate.events.iloc[:1]],ignore_index=True)
            variants.append(compute(duplicate,fn))
        if fn==FUNCTIONS[9]:
            scalar=copy.deepcopy(log)
            scalar.objects["category"]=pd.Series([1,True,None,2.5,pd.Timestamp("2020-02-03T04:05:06Z"),False,""],dtype=object)
            variants.append(compute(scalar,fn))
        if fn in (FUNCTIONS[0],FUNCTIONS[1],FUNCTIONS[4],FUNCTIONS[5],FUNCTIONS[7],FUNCTIONS[8]):
            empty=copy.deepcopy(log)
            empty.events=empty.events.iloc[:0];empty.objects=empty.objects.iloc[:0];empty.relations=empty.relations.iloc[:0]
            empty.o2o=empty.o2o.iloc[:0];empty.e2e=empty.e2e.iloc[:0];empty.object_changes=empty.object_changes.iloc[:0]
            # scenarios requires a populated table; directly evaluate selected entry points.
            if fn not in (FUNCTIONS[8],):
                kwargs={"have_common_object":False} if fn==FUNCTIONS[5] else {}
                try: output=getattr(pm4py,fn)(copy.deepcopy(empty),**kwargs)
                except (KeyError,ValueError,IndexError): pass
                else: variants.append({"input":wire(empty),"results":[dict(function=fn,args=[],kwargs=kwargs,prep=None,output=wire(output,merge=fn==FUNCTIONS[5]))]})
        result["variants"]=variants
        return result
    case(fn.replace("_","-"),functions=["pm4py."+fn])(run)

for name,reader in (("example_log.jsonocel","read_ocel_json"),("newocel.jsonocel","read_ocel_json"),("ocel20_example.jsonocel","read_ocel2_json")):
    def run(fixtures,name=name,reader=reader):
        env=dict(os.environ,PYTHONHASHSEED="0",PYTHONPATH=str(Path(__file__).resolve().parent.parent))
        result=subprocess.run([sys.executable,__file__,reader,str(fixtures["log"])],env=env,check=True,capture_output=True,text=True)
        return json.loads(result.stdout)
    case("real-"+name.replace(".","-").replace("_","-"),fixture="ocel/"+name,functions=["pm4py."+f for f in FUNCTIONS if f not in FUNCTIONS[2:4]])(run)

if __name__=="__main__":
    log=getattr(pm4py,sys.argv[1])(sys.argv[2])
    # Limit this real-log comparison to a coherent six-object neighborhood.
    ids=list(log.objects["ocel:oid"].unique())[:6]
    log=pm4py.filter_ocel_objects(log,ids)
    print(json.dumps(compute(log)))
