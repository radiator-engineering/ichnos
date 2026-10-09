# Synthetic BPMN fixtures

Hand-written inputs for the BPMN reader and writer tests; they are not from
pm4py. `all_kinds.bpmn` holds every element kind pm4py's importer tells
apart: a collaboration with two pools and a message flow, every event type,
user, send and service tasks, the four gateway types and their directions,
a subprocess, text annotations with an association, a flow whose ends come
only from `incoming` and `outgoing` children, names with line breaks, and
shapes, labels and edges. The `bpmn-*` io goldens read it with pm4py
2.7.23.8 (commit 24a3bf6).
