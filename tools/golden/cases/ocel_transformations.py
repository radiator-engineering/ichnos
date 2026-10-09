"""OCEL enrichment and transformations from pm4py 2.7.23.8.

Sampling records every valid subset on small populations, so Rust's independent
RNG can be checked against the complete oracle outcome space. UUID outputs are
renamed by first event occurrence. Unordered appended graph rows are sorted.
"""

import copy
import itertools
import json
import os
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

from harness import case
import pandas as pd
import pm4py
from cases.ocel import _tables
from cases.ocel_filters import synthetic as filter_log

FUNCTIONS = [
    "ocel_o2o_enrichment",
    "ocel_e2o_lifecycle_enrichment",
    "sample_ocel_objects",
    "sample_ocel_connected_components",
    "ocel_drop_duplicates",
    "ocel_merge_duplicates",
    "ocel_sort_by_additional_column",
    "ocel_add_index_based_timedelta",
    "cluster_equivalent_ocel",
    "ocel_drill_down",
    "ocel_roll_up",
    "ocel_unfold",
    "ocel_fold",
]


def wire(log, merge=False):
    tables = _tables(log)
    # Rust derives these columns by id; keep only the actual typed row fields.
    for relation in tables["relations"]:
        for key in ("activity", "timestamp", "type"):
            relation.pop(key)
    tables["o2o"].sort(
        key=lambda row: (row["source"], row["target"], row["qualifier"] or "")
    )
    if merge:
        names = {}
        for event in tables["events"]:
            names.setdefault(event["id"], "event-" + str(len(names)))
            event["id"] = names[event["id"]]
        for relation in tables["relations"]:
            relation["event"] = names[relation["event"]]
    return tables


def synthetic():
    log = filter_log()
    # The graph/cluster oracle requires every event and object to be related.
    log.events = log.events[log.events["ocel:eid"] != "e0"].copy()
    log.objects = log.objects[log.objects["ocel:oid"] != "orphan"].copy()
    log.o2o = log.o2o.iloc[:1].copy()
    log.e2e = log.e2e.iloc[1:].copy()
    log.object_changes = log.object_changes.iloc[:1].copy()
    log.events.loc[log.events["ocel:eid"] == "e3", "ocel:timestamp"] = (
        log.events.iloc[0]["ocel:timestamp"]
    )
    times = log.events.set_index("ocel:eid")["ocel:timestamp"].to_dict()
    log.relations["ocel:timestamp"] = log.relations["ocel:eid"].map(times)
    log.objects["category"] = ["north", "", None, "south", "single"]

    # A second interacting component distinguishes component draws/order.
    extra_event = {
        "ocel:eid": "e5",
        "ocel:activity": "single",
        "ocel:timestamp": log.events.iloc[-1]["ocel:timestamp"],
        "score": 2,
    }
    log.events = pd.concat(
        [log.events, pd.DataFrame([extra_event])], ignore_index=True
    )
    extra_objects = [
        {"ocel:oid": "x", "ocel:type": "order", "category": "west"},
        {"ocel:oid": "y", "ocel:type": "item", "category": "west"},
    ]
    log.objects = pd.concat(
        [log.objects, pd.DataFrame(extra_objects)], ignore_index=True
    )
    extra_relations = [
        dict(
            extra_event,
            **{"ocel:oid": object_id, "ocel:type": object_type, "ocel:qualifier": None},
        )
        for object_id, object_type in (("x", "order"), ("y", "item"))
    ]
    log.relations = pd.concat(
        [log.relations, pd.DataFrame(extra_relations)], ignore_index=True
    )
    return log


def scenarios(log):
    types = sorted(log.objects["ocel:type"].unique())
    object_type = types[0]
    activity = str(log.events.iloc[0]["ocel:activity"])
    out = []

    def add(function, args=None, kwargs=None, prep=None):
        out.append(
            dict(function=function, args=args or [], kwargs=kwargs or {}, prep=prep)
        )

    graph_selections = (
        None,
        [],
        ["object_interaction_graph"],
        ["object_descendants_graph"],
        ["object_inheritance_graph"],
        ["object_cobirth_graph"],
        ["object_codeath_graph"],
    )
    for graphs in graph_selections:
        add(FUNCTIONS[0], kwargs={"included_graphs": graphs})
    add(FUNCTIONS[1])
    add(FUNCTIONS[4])
    add(FUNCTIONS[5], kwargs={"have_common_object": False})
    add(FUNCTIONS[5], kwargs={"have_common_object": True})
    add(FUNCTIONS[6], ["ocel:activity"])
    if "score" in log.events:
        add(FUNCTIONS[6], ["score"])
        add(FUNCTIONS[6], ["score"], {"primary_column": "ocel:activity"})
    add(FUNCTIONS[7])
    for limit in (0, 1, 100000):
        add(FUNCTIONS[8], [object_type], {"max_objs": limit})
    add(
        FUNCTIONS[8],
        [object_type],
        {"exclude_object_types_from_renaming": set(types)},
    )
    add(FUNCTIONS[8], ["missing"])
    attribute = "category" if "category" in log.objects else "ocel:oid"
    add(FUNCTIONS[9], [object_type, attribute])
    add(FUNCTIONS[10], [object_type], prep="drill")
    add(FUNCTIONS[10], [object_type, attribute])
    add(FUNCTIONS[10], ["missing"])
    for qualifiers in (None, [], ["primary"], [""], [None]):
        add(FUNCTIONS[11], [activity, object_type], {"qualifiers": qualifiers})
    add(FUNCTIONS[12], [activity, object_type], prep="unfold")
    add(FUNCTIONS[12], [activity, object_type])
    add(FUNCTIONS[11], ["missing", object_type])
    return out


def json_kwargs(kwargs):
    return {
        key: sorted(value) if isinstance(value, set) else value
        for key, value in kwargs.items()
    }


def compute(log, only=None):
    results = []
    for scenario in scenarios(log):
        function = scenario["function"]
        if only and function != only:
            continue
        source = copy.deepcopy(log)
        if scenario["prep"] == "drill":
            attribute = "category" if "category" in source.objects else "ocel:oid"
            source = pm4py.ocel_drill_down(source, scenario["args"][0], attribute)
        if scenario["prep"] == "unfold":
            source = pm4py.ocel_unfold(source, *scenario["args"])
        output = getattr(pm4py, function)(
            source, *scenario["args"], **scenario["kwargs"]
        )
        if function == FUNCTIONS[8]:
            clusters = [
                {
                    "description": [list(map(list, key[0])), list(map(list, key[1]))],
                    "central_objects": [log.parameters["@@central_object"] for log in logs],
                    "logs": [wire(log) for log in logs],
                }
                for key, logs in output.items()
            ]
            output = sorted(clusters, key=lambda row: json.dumps(row["description"]))
        else:
            output = wire(output, merge=function == FUNCTIONS[5])
        results.append(
            dict(scenario, kwargs=json_kwargs(scenario["kwargs"]), output=output)
        )
    return {"input": wire(log), "results": results}


def sampling(log, components=False):
    if components:
        from pm4py.algo.transformation.ocel.split_ocel import algorithm

        population = algorithm.apply(
            log, variant=algorithm.Variants.CONNECTED_COMPONENTS
        )
        options = [dict(connected_components=count) for count in (0, 1, 2, 100)]
        limit_fields = (
            "max_num_events_per_cc",
            "max_num_objects_per_cc",
            "max_num_e2o_relations_per_cc",
        )
        options += [
            dict(connected_components=2, **{key: limit})
            for key in limit_fields
            for limit in (0, 2, 3, 4, 8)
        ]
    else:
        population = list(log.objects["ocel:oid"].unique())
        options = [
            dict(num_objects=count) for count in (0, 1, 2, len(population), 100)
        ]
    results = []
    for option in options:
        choices = []
        if components:
            eligible = [
                component
                for component in population
                if len(component.events) <= option.get("max_num_events_per_cc", sys.maxsize)
                and len(component.objects) <= option.get("max_num_objects_per_cc", sys.maxsize)
                and len(component.relations) <= option.get("max_num_e2o_relations_per_cc", sys.maxsize)
            ]
            count = min(option["connected_components"], len(eligible))
            for subset in itertools.permutations(range(len(eligible)), count):
                def sample(population, count, subset=subset):
                    return [population[index] for index in subset]

                with patch("random.sample", side_effect=sample):
                    output = pm4py.sample_ocel_connected_components(
                        copy.deepcopy(log), **option
                    )
                    choices.append(wire(output))
        else:
            count = min(option["num_objects"], len(population))
            for subset in itertools.combinations(population, count):
                def shuffle(population, subset=subset):
                    population[:] = list(subset) + [
                        value for value in population if value not in subset
                    ]

                with patch("random.shuffle", side_effect=shuffle):
                    output = pm4py.sample_ocel_objects(copy.deepcopy(log), **option)
                    choices.append(wire(output))
        # Component enumeration order varies with NetworkX's set iteration.
        # Sort outcomes, while preserving row order inside each sampled log.
        choices.sort(key=lambda output: json.dumps(output, sort_keys=True))
        results.append({"kwargs": option, "outputs": choices})
    return {"input": wire(log), "results": results}


def synthetic_case(fixtures, function):
    log = synthetic()
    if function in FUNCTIONS[2:4]:
        return sampling(log, function == FUNCTIONS[3])
    result = compute(log, function)
    variants = []
    if function == FUNCTIONS[7]:
        duplicate = copy.deepcopy(log)
        duplicate.events = pd.concat(
            [duplicate.events, duplicate.events.iloc[:1]], ignore_index=True
        )
        variants.append(compute(duplicate, function))
    if function == FUNCTIONS[9]:
        scalar = copy.deepcopy(log)
        scalar.objects["category"] = pd.Series(
            [1, True, None, 2.5, pd.Timestamp("2020-02-03T04:05:06Z"), False, ""],
            dtype=object,
        )
        variants.append(compute(scalar, function))
    if function in (FUNCTIONS[0], FUNCTIONS[1], FUNCTIONS[4], FUNCTIONS[5], FUNCTIONS[7]):
        empty = copy.deepcopy(log)
        empty.events = empty.events.iloc[:0]
        empty.objects = empty.objects.iloc[:0]
        empty.relations = empty.relations.iloc[:0]
        empty.o2o = empty.o2o.iloc[:0]
        empty.e2e = empty.e2e.iloc[:0]
        empty.object_changes = empty.object_changes.iloc[:0]
        # Scenarios require populated tables; call the entry point directly.
        kwargs = {"have_common_object": False} if function == FUNCTIONS[5] else {}
        try:
            output = getattr(pm4py, function)(copy.deepcopy(empty), **kwargs)
        except (KeyError, ValueError, IndexError):
            pass
        else:
            scenario = dict(
                function=function,
                args=[],
                kwargs=kwargs,
                prep=None,
                output=wire(output, merge=function == FUNCTIONS[5]),
            )
            variants.append({"input": wire(empty), "results": [scenario]})
    result["variants"] = variants
    return result


for function in FUNCTIONS:
    def run(fixtures, function=function):
        return synthetic_case(fixtures, function)

    case(
        function.replace("_", "-"),
        functions=["pm4py." + function],
    )(run)


for name, reader in (
    ("example_log.jsonocel", "read_ocel_json"),
    ("newocel.jsonocel", "read_ocel_json"),
    ("ocel20_example.jsonocel", "read_ocel2_json"),
):
    def run(fixtures, name=name, reader=reader):
        # Pin pm4py's OCEL 2.0 JSON reader's set-dependent relation order.
        env = dict(
            os.environ,
            PYTHONHASHSEED="0",
            PYTHONPATH=str(Path(__file__).resolve().parent.parent),
        )
        result = subprocess.run(
            [sys.executable, __file__, reader, str(fixtures["log"])],
            env=env,
            check=True,
            capture_output=True,
            text=True,
        )
        return json.loads(result.stdout)

    case(
        "real-" + name.replace(".", "-").replace("_", "-"),
        fixture="ocel/" + name,
        functions=["pm4py." + function for function in FUNCTIONS if function not in FUNCTIONS[2:4]],
    )(run)


if __name__ == "__main__":
    log = getattr(pm4py, sys.argv[1])(sys.argv[2])
    # Limit this real-log comparison to a coherent six-object neighborhood.
    ids = list(log.objects["ocel:oid"].unique())[:6]
    log = pm4py.filter_ocel_objects(log, ids)
    print(json.dumps(compute(log)))
