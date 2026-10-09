"""Analysis entry points: comparisons, transport, enrichment, and clustering.

Clustering fixtures use explicit Lloyd centers, so assignments test the
algorithm independently of sklearn's random K-means++ initialization.
"""
from __future__ import annotations
import difflib
import hashlib
import json
import numpy as np
import pm4py
from harness import case
from harness.fixtures import load_log

@case("labels", functions=["pm4py.label_sets_similarity", "pm4py.map_labels_from_second_model", "pm4py.replace_activity_labels"])
def labels(_):
    from pm4py.util import labels_similarity
    pairs=[["tide","diet"],["diet","tide"],["αβγ","αδγ"],["",""],["","x"],["x"+"a"*210,"y"+"a"*210],["a"*210+"x","a"*210+"y"],["abxcd","abcd"]]
    a=["Approve request","Archive","Receive request"]
    b=["Approve requests","Archive","Receive requests"]
    from pm4py.objects.process_tree.utils import generic
    first=generic.parse("->( 'Approve request', 'Archive', 'Receive request' )")
    second=generic.parse("->( 'Approve requests', 'Archive', 'Receive requests' )")
    renamed=pm4py.map_labels_from_second_model(first,second)
    return {"model_similarity":pm4py.label_sets_similarity(first,second),"mapped_model_labels":pm4py.get_activity_labels(renamed),"pairs":pairs,"ratios":[difflib.SequenceMatcher(None,a,b).ratio() for a,b in pairs],"a":a,"b":b,"similarity":labels_similarity.label_sets_similarity(a,b),"mapping":labels_similarity.map_labels(a,b)}

TREES=["tau","'A'","->( 'A', 'B', 'C' )","+( 'A', 'B' )","X( 'A', 'B' )","*( 'A', 'B' )","->( X( 'A', tau ), +( 'B', 'C' ) )"]
@case("models",functions=["pm4py.get_activity_labels","pm4py.replace_activity_labels","pm4py.behavioral_similarity","pm4py.structural_similarity","pm4py.label_sets_similarity","pm4py.map_labels_from_second_model"])
def models(_):
    from pm4py.objects.process_tree.utils import generic
    trees=[generic.parse(s) for s in TREES]
    out=[]
    for text,tree in zip(TREES,trees):
        net,im,fm=pm4py.convert_to_petri_net(tree)
        powl=pm4py.convert_to_powl(tree)
        bpmn=pm4py.convert_to_bpmn(tree)
        out.append({"tree":text,"labels":pm4py.get_activity_labels(tree),"renamed_labels":pm4py.get_activity_labels(pm4py.replace_activity_labels({"A":"Alpha"},tree)),
                    "behavioral":[pm4py.behavioral_similarity(tree,t) for t in trees],"structural":[pm4py.structural_similarity(tree,t) for t in trees],
                    "net_behavioral":[pm4py.behavioral_similarity(net,im,fm,t) for t in trees],
                    "net_structural":[pm4py.structural_similarity(net,im,fm,t) for t in trees],
                    "powl_structural":[pm4py.structural_similarity(powl,t) for t in trees],
                    "powl_behavioral":[pm4py.behavioral_similarity(powl,t) for t in trees],
                    "bpmn_labels":pm4py.get_activity_labels(bpmn),"powl_labels":pm4py.get_activity_labels(powl),
                    "net_renamed":pm4py.get_activity_labels(*pm4py.replace_activity_labels({"A":"Alpha"},net,im,fm)),
                    "bpmn_renamed":pm4py.get_activity_labels(pm4py.replace_activity_labels({"A":"Alpha"},bpmn)),
                    "powl_renamed":pm4py.get_activity_labels(pm4py.replace_activity_labels({"A":"Alpha"},powl))})
    return out

@case("emd",functions=["pm4py.compute_emd"])
def emd(_):
    cases=[([[["A"],.5],[["B"],.5]],[[["A"],.25],[["C"],.75]]),([[["A","B"],1]],[[["B","A"],1]]),([[["A"],2]],[[["A","B"],2]]),([[["A"],0],[["B"],1]],[[["B"],1]])]
    return [{"a":a,"b":b,"distance":pm4py.compute_emd({tuple(t):m for t,m in a},{tuple(t):m for t,m in b})} for a,b in cases]

def describe_case_times(df):
    columns=["@@service_time","@@sojourn_time","@@waiting_time","@@arrival_rate","@@finish_rate"]
    return sorted([[str(cid)]+[float(group.iloc[0][c]) for c in columns] for cid,group in df.groupby("case:concept:name")])

def enrichment(paths):
    df=load_log(paths["log"])
    out=pm4py.insert_case_service_waiting_time(df.copy())
    out=pm4py.insert_case_arrival_finish_rate(out)
    return {"cases":describe_case_times(out),"labels":pm4py.get_activity_labels(df)}

def transport(paths):
    df=load_log(paths["log"])
    variants=pm4py.get_variants(df)
    # Small transport supports drawn from the complete log, not a sampled prefix.
    selected=sorted(variants.items())[:16]
    total=sum(v for _,v in selected)
    a=[[list(t),n/total] for t,n in selected]
    b=[[list(selected[0][0]),1.0]]
    return {"a":a,"b":b,"distance":pm4py.compute_emd({tuple(t):m for t,m in a},{tuple(t):m for t,m in b})}

def clustering(paths):
    from pm4py.algo.transformation.trace_encodings.variants import trace_based
    from sklearn.cluster import KMeans
    log=pm4py.convert_to_event_log(load_log(paths["log"]))
    params={"str_ev_attr":["concept:name"],"str_tr_attr":[],"num_tr_attr":[],"num_ev_attr":[],"str_evsucc_attr":["concept:name"]}
    data,names=trace_based.apply(log,parameters=params)
    order=sorted(range(len(names)),key=lambda i:names[i]);names=[names[i] for i in order]
    data=[[row[i] for i in order] for row in data]
    centers=[data[0],max(enumerate(data),key=lambda p:(sum((a-b)**2 for a,b in zip(p[1],data[0])),-p[0]))[1]]
    class ExplicitLloyd:
        def fit_predict(self,rows):
            # Native cluster_log extracts the same columns in a different order.
            native_centers=np.array([[c[names.index(name)] for name in original_names] for c in centers])
            return KMeans(n_clusters=2,init=native_centers,n_init=1,algorithm="lloyd",max_iter=300,tol=1e-4).fit_predict(rows)
    _,original_names=trace_based.apply(log,parameters=params)
    from pm4py.objects.log.obj import EventLog, Trace, Event
    # The top-level wrapper chooses profiles automatically. Project to the
    # activity-only feature contract to separate that from attribute sampling.
    projected=EventLog([Trace([Event({"concept:name":e["concept:name"]}) for e in t],attributes={"concept:name":t.attributes["concept:name"]}) for t in log])
    clusters=list(pm4py.cluster_log(projected,sklearn_clusterer=ExplicitLloyd()))
    groups=[[str(t.attributes["concept:name"]) for t in cluster] for cluster in clusters]
    binary=bytes(int(x) for row in data for x in row)
    return {"names":names,"rows":len(data),"profile_sha256":hashlib.sha256(binary).hexdigest(),"centers":centers,"groups":groups,"sklearn_version":__import__("sklearn").__version__}

for name in ["running-example","receipt","roadtraffic100traces"]:
    fixture=name+".xes"
    case("times-"+name,fixture=fixture,functions=["pm4py.insert_case_service_waiting_time","pm4py.insert_case_arrival_finish_rate","pm4py.get_activity_labels"])(enrichment)
    case("emd-"+name,fixture=fixture,functions=["pm4py.compute_emd"])(transport)
    case("clusters-"+name,fixture=fixture,functions=["pm4py.cluster_log"])(clustering)

@case("times-intervals",functions=["pm4py.insert_case_service_waiting_time","pm4py.insert_case_arrival_finish_rate"])
def intervals(_):
    import pandas as pd
    spec=[[2,0,10],[2,5,15],[10,20,19],[1,0,20],[3,25,26]]
    df=pd.DataFrame(spec,columns=["case:concept:name","start","end"])
    df["concept:name"]="A"
    df["time:start"]=pd.to_datetime(df["start"],unit="s",utc=True)
    df["time:timestamp"]=pd.to_datetime(df["end"],unit="s",utc=True)
    out=pm4py.insert_case_service_waiting_time(df,start_timestamp_key="time:start")
    out=pm4py.insert_case_arrival_finish_rate(out,start_timestamp_key="time:start")
    return {"events":spec,"cases":describe_case_times(out),"durations":out["@@diff_start_end"].tolist()}

@case("profiles-numeric",functions=["pm4py.cluster_log"])
def numeric(_):
    from pm4py.objects.log.obj import EventLog,Trace,Event
    from pm4py.algo.transformation.trace_encodings.variants import trace_based
    from sklearn.cluster import KMeans
    from pm4py.algo.clustering.profiles import algorithm
    spec=[{"group":"low","score":1,"events":[["A",1],["B",2]]},{"group":"low","score":2,"events":[["B",2],["A",3]]},{"group":"high","score":20,"events":[["C",20],["C",21]]},{"group":"high","score":21,"events":[["C",22]]}]
    log=EventLog([Trace([Event({"concept:name":a,"cost":n}) for a,n in s["events"]],attributes={"concept:name":str(i),"group":s["group"],"score":s["score"]}) for i,s in enumerate(spec)])
    params={"str_tr_attr":["group"],"str_ev_attr":["concept:name"],"num_tr_attr":["score"],"num_ev_attr":["cost"],"str_evsucc_attr":["concept:name"]}
    data,names=trace_based.apply(log,parameters=params)
    order=sorted(range(len(names)),key=lambda i:names[i]);sorted_names=[names[i] for i in order];sorted_data=[[r[i] for i in order] for r in data]
    clusterer=KMeans(n_clusters=2,init=np.array([data[0],data[2]]),n_init=1,algorithm="lloyd")
    clusters=list(algorithm.apply(log,parameters={**params,"sklearn_clusterer":clusterer}))
    inferred,inferred_names=trace_based.apply(log,parameters={"enable_activity_def_representation":True,"enable_succ_def_representation":True})
    infer_order=sorted(range(len(inferred_names)),key=lambda i:inferred_names[i])
    return {"inferred_names":[inferred_names[i] for i in infer_order],"inferred_data":[[r[i] for i in infer_order] for r in inferred],"input":spec,"names":sorted_names,"data":sorted_data,"groups":[[str(t.attributes["concept:name"]) for t in c] for c in clusters]}
