# pm4py API coverage

Reference: a checkout of pm4py **2.7.23.8** (commit **24a3bf6**), cross-checked against its installed top-level exports. Use `PM4PY_SRC` for the source checkout and `PM4PY_PYTHON` for its Python interpreter when reproducing the inventory.

## Summary

todo: 100; ported: 344; dropped: 182; total: 626.

Recompute with `tools/parity_count.py`. Completion requires each row to be `ported` with a passing golden test or `dropped` with a reason.

All Rust paths below are **planned**. Lanes replace them with actual public paths when porting. Python module namespaces and runtime implementation helpers are dropped explicitly. Defined pandas/polars functions with log/common counterparts are dropped into the exact named canonical row; backend-only operations without those counterparts remain todo and share one Rust implementation. Pure imported aliases are counted once under their defining module (top-level spelling preferred). Statistics includes public common helpers conservatively; streaming includes direct functions, classes and public methods, with parameter/variant enums represented by the owning entry point. Private names and underscore metadata are excluded. Source paths are relative to `pm4py/`; arrows identify implementation dependencies. Variant lists describe available backends, not separate Rust implementations; HTML/matplotlib alternatives are excluded where dot export is retained.

## read

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.read_xes` | `read.py` → `objects/conversion/log/converter`, `objects/log/importer/xes/importer`, `objects/log/obj` | `ichnos_io::read_xes` | `ichnos-io` | ported | Streaming quick-xml; plain/gzip. All readable XES fixtures have io goldens and Rust round trips. See the ichnos-io Behaviour changes list for return types, null handling, validation and XML version differences. |
| `pm4py.read_pnml` | `read.py` → `objects/petri_net/importer/importer`, `objects/petri_net/obj` | `ichnos_io::read_pnml` | `ichnos-io` | ported | PnmlDocument holds an AcceptingPetriNet plus alternate finals and stochastic/data metadata. 22 fixture goldens and round trips; SampleNet is round-trip-only because it is unbounded. See ichnos-io Behaviour changes. |
| `pm4py.read_ptml` | `read.py` → `objects/process_tree/importer/importer`, `objects/process_tree/obj` | `ichnos_io::read_ptml` | `ichnos-io` | ported | Builds ProcessTree; seven fixture goldens and round trips. Referenced/shared subtrees, two/three-child loops and edge declaration order are supported. See ichnos-io Behaviour changes. |
| `pm4py.read_dfg` | `read.py` → `objects/dfg/importer/importer` | `ichnos_io::read_dfg` | `ichnos-io` | ported | Builds Dfg from the line format, including boundary/edge counts. Fixture counts, frequencies, footprints and round trip are golden-tested. Last duplicate frequency wins. |
| `pm4py.read_bpmn` | `read.py` → `objects/bpmn/importer/importer`, `objects/bpmn/obj` | `ichnos_io::read_bpmn` | `ichnos-io` | ported | Returns a `BpmnDocument`: the `Bpmn` plus shape bounds and edge waypoints. Goldens `io/bpmn-read-*` on nine pm4py fixtures and a synthetic diagram with every element kind (`fixtures/logs/synthetic-bpmn/all_kinds.bpmn`); `io/bpmn-writer-special` reads ichnos output with pm4py. ch7_CreditAppSimulation fails in both. See ichnos-io Behaviour changes. |
| `pm4py.read_ocel` | `read.py` → `objects/ocel/importer/csv/importer`, `objects/ocel/importer/jsonocel/importer`, `objects/ocel/importer/sqlite/importer`, `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_rustxes, ocel20_standard, pandas, pandas_importer. |
| `pm4py.read_ocel_csv` | `read.py` → `objects/ocel/importer/csv/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.read_ocel_json` | `read.py` → `objects/ocel/importer/jsonocel/importer`, `objects/ocel/obj` | `ichnos_io::read_ocel_json` | `ichnos-io` | ported | `read_ocel_json` (OCEL 1.0 layout, pm4py's `classic` variant). Goldens `ocel/model-example-log-jsonocel`, `ocel/model-newocel-jsonocel` and the synthetic `ocel/model-typed-jsonocel`: events, objects, relations, o2o, e2e and object changes equal pm4py's tables. See the ichnos-io (OCEL) Behaviour changes. |
| `pm4py.read_ocel_xml` | `read.py` → `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos_io::read_ocel_xml` | `ichnos-io` | ported | `read_ocel_xml` (OCEL 1.0 layout, pm4py's `classic` variant). Goldens `ocel/model-example-log-xmlocel` and the synthetic `ocel/model-typed-xmlocel`: every table equals pm4py's, apart from the typed values listed in the Behaviour changes. See the ichnos-io (OCEL) Behaviour changes. |
| `pm4py.read_ocel_sqlite` | `read.py` → `objects/ocel/importer/sqlite/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_importer. |
| `pm4py.read_ocel2` | `read.py` → `objects/ocel/importer/bundled/importer`, `objects/ocel/importer/csv/importer`, `objects/ocel/importer/jsonocel/importer`, `objects/ocel/importer/sqlite/importer`, `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_rustxes, ocel20_standard, pandas, pandas_importer. |
| `pm4py.read_ocel2_bundle` | `read.py` → `objects/ocel/importer/bundled/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_bundle` (planned) | `ichnos-io` | todo | Variants: ocel20. |
| `pm4py.read_ocel2_csv` | `read.py` → `objects/ocel/importer/csv/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.read_ocel2_json` | `read.py` → `objects/ocel/importer/jsonocel/importer`, `objects/ocel/obj` | `ichnos_io::read_ocel2_json` | `ichnos-io` | ported | `read_ocel2_json` (OCEL 2.0 standard layout, pm4py's `ocel20_standard` variant). Goldens `ocel/model-ocel20-example-jsonocel` and the synthetic `ocel/model-typed20-jsonocel`: every table equals pm4py's, with relations compared as a set because pm4py orders them by set iteration. See the ichnos-io (OCEL) Behaviour changes. |
| `pm4py.read_ocel2_sqlite` | `read.py` → `objects/ocel/importer/sqlite/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_importer. |
| `pm4py.read_ocel2_xml` | `read.py` → `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos_io::read_ocel2_xml` | `ichnos-io` | ported | `read_ocel2_xml` (pm4py's `ocel20` variant). Goldens `ocel/model-ocel20-example-xmlocel` and the synthetic `ocel/model-typed20-xmlocel`: every table equals pm4py's, apart from the typed values listed in the Behaviour changes. See the ichnos-io (OCEL) Behaviour changes. |

## write

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.write_xes` | `write.py` → `objects/log/exporter/xes/exporter`, `objects/log/obj` | `ichnos_io::write_xes` | `ichnos-io` | ported | Streaming quick-xml; plain/gzip. All readable XES fixtures have io goldens and Rust round trips. See the ichnos-io Behaviour changes list for return types, null handling, validation and XML version differences. |
| `pm4py.write_pnml` | `write.py` → `objects/petri_net/exporter/exporter`, `objects/petri_net/obj` | `ichnos_io::write_pnml` | `ichnos-io` | ported | Weighted normal/inhibitor/reset arcs, final markings and stochastic/data declarations round-trip. Uses deterministic XML ids/layout; graphics and arbitrary tool metadata are not exported. See ichnos-io Behaviour changes. |
| `pm4py.write_ptml` | `write.py` → `objects/process_tree/exporter/exporter`, `objects/process_tree/obj` | `ichnos_io::write_ptml` | `ichnos-io` | ported | Deterministic node ids; emits a silent third loop child for ProM by default. Seven fixture round trips. Interleaving and loops with more than two model children return errors. |
| `pm4py.write_dfg` | `write.py` → `objects/dfg/exporter/exporter` | `ichnos_io::write_dfg` | `ichnos-io` | ported | Deterministic lexical activity indexes, explicit start/end frequencies; optional inferred boundaries. Empty graphs and boundary-only activities round-trip. Unrepresentable whitespace/newline labels error. |
| `pm4py.write_bpmn` | `write.py` → `objects/bpmn/exporter/exporter`, `objects/bpmn/layout/layouter`, `objects/bpmn/obj` | `ichnos_io::write_bpmn` | `ichnos-io` | ported | Ports the etree variant. Goldens `io/bpmn-write-*` compare pm4py's write-then-read of each fixture with the ichnos one; the two subprocess fixtures fail to write in both. Golden `io/bpmn-options-all_kinds` writes with both of pm4py's export switches off (`BpmnWriteOptions::plane` and `incoming_outgoing`). Uses stored bounds and waypoints, or pm4py's default layout; Graphviz auto-layout is not ported, as for `write_pnml`. See ichnos-io Behaviour changes. |
| `pm4py.write_ocel` | `write.py` → `objects/ocel/exporter/csv/exporter`, `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos_io::write_ocel` | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_standard, pandas, pandas_exporter. `ichnos_io::write_ocel` writes names ending in `jsonocel` or `xmlocel`; CSV and SQLite return an error until they are ported. |
| `pm4py.write_ocel_csv` | `write.py` → `objects/ocel/exporter/csv/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.write_ocel_json` | `write.py` → `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/obj` | `ichnos_io::write_ocel_json` | `ichnos-io` | ported | `write_ocel_json` writes pm4py's `classic` layout, or the `ocel20` layout when the log has OCEL 2.0 features (qualifiers, o2o relations or object changes), as pm4py's `is_ocel20` test decides. Golden cases `ocel/write-*`: the nine non-CSV fixtures, an empty log and two synthetic logs (`write-synthetic`, `write-synthetic20`) with typed, unicode, escaped and missing values. See the ichnos-io (OCEL) Behaviour changes. |
| `pm4py.write_ocel_xml` | `write.py` → `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos_io::write_ocel_xml` | `ichnos-io` | ported | `write_ocel_xml` writes the OCEL 1.0 XML layout (pm4py's `classic` variant). Golden cases `ocel/write-*`: the nine non-CSV fixtures, an empty log and two synthetic logs (`write-synthetic`, `write-synthetic20`) with typed, unicode, escaped and missing values. See the ichnos-io (OCEL) Behaviour changes. |
| `pm4py.write_ocel_sqlite` | `write.py` → `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_exporter. |
| `pm4py.write_ocel2` | `write.py` → `objects/ocel/exporter/bundled/exporter`, `objects/ocel/exporter/csv/exporter`, `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos_io::write_ocel2` | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_standard, pandas, pandas_exporter. `ichnos_io::write_ocel2` writes names ending in `xml`, `xmlocel`, `json` or `jsonocel`, each optionally followed by `.gz`; CSV, SQLite and the bundle return an error until they are ported. |
| `pm4py.write_ocel2_bundle` | `write.py` → `objects/ocel/exporter/bundled/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_bundle` (planned) | `ichnos-io` | todo | Variants: ocel20. |
| `pm4py.write_ocel2_csv` | `write.py` → `objects/ocel/exporter/csv/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.write_ocel2_json` | `write.py` → `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/obj` | `ichnos_io::write_ocel2_json` | `ichnos-io` | ported | `write_ocel2_json` writes the OCEL 2.0 JSON layout (pm4py's `ocel20_standard` variant). Golden cases `ocel/write-*`: the nine non-CSV fixtures, an empty log and two synthetic logs (`write-synthetic`, `write-synthetic20`) with typed, unicode, escaped and missing values. See the ichnos-io (OCEL) Behaviour changes. |
| `pm4py.write_ocel2_sqlite` | `write.py` → `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_exporter. |
| `pm4py.write_ocel2_xml` | `write.py` → `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos_io::write_ocel2_xml` | `ichnos-io` | ported | `write_ocel2_xml` writes the OCEL 2.0 XML layout (pm4py's `ocel20` variant). Golden cases `ocel/write-*`: the nine non-CSV fixtures, an empty log and two synthetic logs (`write-synthetic`, `write-synthetic20`) with typed, unicode, escaped and missing values. See the ichnos-io (OCEL) Behaviour changes. |

## discovery

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.discover_dfg` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/algorithm`, `objects/log/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos_discovery::dfg` | `ichnos-discovery` | ported | Seven dfg-mining goldens compare edge and boundary counts exactly, order-free. Input event order follows pm4py’s EventLog path; DataFrame sorting belongs to readers. Window and once-per-case counting are typed options; negative windows are unrepresentable. Core activity stringification and positional errors apply. Covers the EventLog path (`native`, `performance`); `freq_triples`, `case_attributes`, `clean*` and `*_greedy` are not ported. |
| `pm4py.discover_directly_follows_graph` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/algorithm`, `objects/log/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos_discovery::directly_follows_graph` | `ichnos-discovery` | ported | Alias of dfg, covered by the same seven goldens and options. |
| `pm4py.discover_dfg_typed` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/algorithm`, `objects/dfg/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos_discovery::dfg_typed` | `ichnos-discovery` | ported | Uses the same ichnos_model::Dfg as ordinary discovery, over the canonical EventLog. Seven goldens compare pm4py’s public typed DFG, including empty and boundary-only traces. |
| `pm4py.discover_performance_dfg` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/variants/performance`, `objects/dfg/obj`, `objects/log/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos_discovery::performance_dfg` | `ichnos-discovery` | ported | Seven goldens cover all six aggregates, business hours, singleton observations, zero-clipped negative gaps, and optional raw durations; floats use 1e-6 relative / 1e-12 absolute tolerance. Typed summaries replace aggregation strings; weekly schedules and excluded dates replace Python calendar objects. Preserves nanosecond elapsed gaps. Completion timestamps are starts by default, including custom keys; pm4py’s custom-key wrappers require an explicit start-key property. Business hours use input wall clocks; oracle XES inputs are normalized to UTC to match pm4py’s XES reader. |
| `pm4py.discover_petri_net_alpha` | `discovery.py` → `algo/discovery/alpha/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos_discovery::petri_net_alpha` | `ichnos-discovery` | ported | `AlphaOptions` over ordered EventLog input; first-appearance causal-pair processing, deterministic IDs, typed core input errors. Empty traces contribute no boundaries. Petri-net behavior is covered by twelve classic-miner goldens, using complete footprints below a 10,000-marking cap and exact executable/accepted languages through depth 3. |
| `pm4py.discover_petri_net_ilp` | `discovery.py` → `algo/discovery/ilp/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_ilp` | `ichnos-discovery` | ported | Classic binary-region discovery; alpha sequence filtering, typed custom causals and existing model reductions. Thirteen pinned oracle cases use integer HiGHS with lexicographic optimal-region ties; executable/accepted depth-3 language and complete capped footprints. good_lp/microlp, stable IDs and collision-free boundaries; empty input returns a silent workflow shell. |
| `pm4py.discover_petri_net_genetic` | `discovery.py` → `algo/discovery/genetic/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos_discovery::petri_net_genetic` | `ichnos-discovery` | ported | Classic causal-partition search and matrix conversion/fitness. Nineteen pm4py oracle cases cover five real fixtures, non-simple silent bindings and every controlled crossover point; deterministic public examples and seeded search tests. See miner-genetic Behaviour changes for replay order and RNG differences. |
| `pm4py.discover_petri_net_alpha_plus` | `discovery.py` → `algo/discovery/alpha/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos_discovery::petri_net_alpha_plus` | `ichnos-discovery` | ported | `AlphaPlusOptions::remove_unconnected` defaults false. Preserves pinned length-one filtering, replacement of loop predecessors, separate unmarked loop places and single-pass pair merging. Uses typed synthetic boundaries to avoid user-label collisions; empty input returns a workflow shell and initial marking places remain live. Deterministic sorted target iteration; receipt goldens retain all distinct observed public/native outcomes over hash seeds 0–7. Twelve behavioral goldens cover both options. |
| `pm4py.discover_petri_net_inductive` | `discovery.py` → `algo/discovery/inductive/algorithm`, `objects/dfg/obj`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos_discovery::petri_net_inductive` | `ichnos-discovery` | ported | Golden discovery cases `inductive-{im,imf,imd}-*` on running-example, receipt, roadtraffic100traces, receipt_even, receipt_odd and reviewing CSV: tree equal up to XOR/parallel child order, tree and net footprints equal. `inductive-im-nofallthrough-receipt-csv` and `inductive-im-plainsequence-receipt-csv` cover `disable_fallthroughs` and `disable_strict_sequence_cut`; `inductive-im-fallthroughs-synthetic` reaches the activity-once-per-trace fall-through. IM, IMf (`InductiveVariant::Imf`) and IMd (`process_tree_inductive_dfg` on a DFG) share one implementation. Deterministic where pm4py follows set order (XOR groups largest first, so IMf trees can differ from some pm4py runs); no `tree_sort`; `multi_processing` dropped; noise threshold outside [0, 1] is an error. See the ichnos-discovery Behaviour changes. The net is the tree converted with `ProcessTree::to_petri_net`; `petri_net_inductive_dfg` takes a DFG. |
| `pm4py.discover_petri_net_heuristics` | `discovery.py` → `algo/discovery/heuristics/variants/classic`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos_discovery::petri_net_heuristics` | `ichnos-discovery` | ported | Uses the same classic options and `HeuristicsNet::to_petri_net` conversion. Result may be unsound or unbounded. Twelve goldens cover public defaults and three native option sets, checking complete footprints below a 10,000-marking cap and exact executable/accepted visible languages through depth 3; no partial footprint is accepted. |
| `pm4py.discover_process_tree_inductive` | `discovery.py` → `algo/discovery/inductive/algorithm`, `objects/dfg/obj`, `objects/log/obj`, `objects/process_tree/obj` | `ichnos_discovery::process_tree_inductive` | `ichnos-discovery` | ported | Golden discovery cases `inductive-{im,imf,imd}-*` on running-example, receipt, roadtraffic100traces, receipt_even, receipt_odd and reviewing CSV: tree equal up to XOR/parallel child order, tree and net footprints equal. `inductive-im-nofallthrough-receipt-csv` and `inductive-im-plainsequence-receipt-csv` cover `disable_fallthroughs` and `disable_strict_sequence_cut`; `inductive-im-fallthroughs-synthetic` reaches the activity-once-per-trace fall-through. IM, IMf (`InductiveVariant::Imf`) and IMd (`process_tree_inductive_dfg` on a DFG) share one implementation. Deterministic where pm4py follows set order (XOR groups largest first, so IMf trees can differ from some pm4py runs); no `tree_sort`; `multi_processing` dropped; noise threshold outside [0, 1] is an error. See the ichnos-discovery Behaviour changes. |
| `pm4py.discover_heuristics_net` | `discovery.py` → `algo/discovery/heuristics/variants/classic`, `objects/heuristics_net/obj`, `objects/log/obj` | `ichnos_discovery::heuristics_net` | `ichnos-discovery` | ported | Classic `HeuristicsOptions` preserves all six threshold/count defaults; validates finite threshold fractions in [0,1]. Uses input event order and core label conversion, with no timestamp requirement. Reuses model AND/length-two measures; preserves reverse frequencies on loop input edges and fallback nodes when no edge qualifies. Drawing/performance decorations are omitted by the existing model type. Twelve goldens compare DFGs, occurrences, boundaries, all matrices, nodes and connections with three option sets. |
| `pm4py.derive_minimum_self_distance` | `discovery.py` → `algo/discovery/minimum_self_distance/algorithm`, `objects/log/obj` | `ichnos_discovery::derive_minimum_self_distance` | `ichnos-discovery` | ported | Seven goldens compare maps exactly. Reuses the stats implementation: adjacent repetitions have zero distance; activities without within-trace repetition are omitted. Core activity stringification and positional errors apply. |
| `pm4py.discover_footprints` | `discovery.py` → `algo/discovery/footprints/algorithm`, `objects/log/obj`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos_discovery::footprints` | `ichnos-discovery` | ported | `log_footprints`, `trace_footprints` and `dfg_footprints`; tree and POWL footprints are `ProcessTree::footprints` and `Powl::footprints` in `ichnos-model`. Golden discovery cases `footprints-log-*` check the log, per-trace, DFG and discovered-POWL footprints of six logs, `footprints-log-synthetic-emptytraces` covers empty traces, and `footprints-powl-*` covers each model in `tools/golden/cases/powl.py`. Tree footprints are checked in the `inductive-*` goldens. See the ichnos-discovery (footprints) Behaviour changes. |
| `pm4py.discover_eventually_follows_graph` | `discovery.py` → `objects/log/obj`, `statistics/eventually_follows/log/get`, `statistics/eventually_follows/pandas/get`, `statistics/eventually_follows/polars/get` | `ichnos_discovery::eventually_follows_graph` | `ichnos-discovery` | ported | Seven goldens compare temporal pair counts exactly, including overlaps, timestamp ties, and interval starts. Reuses stats temporal relations. Typed first-eligible-follower option exposes the log backend’s extension. Starts use completion timestamps by default, including custom keys; pm4py’s custom-key wrapper needs an explicit start-key property. Missing or non-date timestamps return positional core errors. |
| `pm4py.discover_bpmn_inductive` | `discovery.py` → `algo/discovery/inductive/algorithm`, `objects/bpmn/obj`, `objects/dfg/obj`, `objects/log/obj` | `ichnos_discovery::bpmn_inductive` | `ichnos-discovery` | ported | The tree of `process_tree_inductive`, converted with `ProcessTree::to_bpmn`; `bpmn_inductive_dfg` takes a DFG (IMd). Golden discovery cases `bpmn-inductive-{im,imf,imd}-*` on six logs: tree equal up to XOR and parallel child order, diagram isomorphic to pm4py's. See the ichnos-discovery (inductive miner) Behaviour changes. (`abc` and `instances` are base modules of the miner, not variants.) |
| `pm4py.discover_bpmn_split_miner` | `discovery.py` → `algo/discovery/split_miner/algorithm`, `algo/discovery/split_miner/variants/classic`, `algo/discovery/split_miner/variants/sm2`, `objects/bpmn/obj`, `objects/log/obj` | `ichnos_discovery::bpmn_split_miner` | `ichnos-discovery` | ported | Classic and lifecycle-aware SM2 follow pm4py classic.apply and sm2.apply. 33 pm4py goldens compare complete typed graph isomorphism across nine distinct variant settings, including OR-split promotion. See ichnos-discovery (split miner) Behaviour changes. |
| `pm4py.discover_transition_system` | `discovery.py` → `algo/discovery/transition_system/algorithm`, `objects/log/obj`, `objects/transition_system/obj` | `ichnos::discovery::transition_system` | `ichnos-discovery` | ported | Ten complete graph goldens; see ichnos-discovery (transition system) Behaviour changes for normalization, names and fixture sweeps. |
| `pm4py.discover_prefix_tree` | `discovery.py` → `algo/transformation/log_to_trie/algorithm`, `objects/log/obj`, `objects/trie/obj` | `ichnos_discovery::prefix_tree` | `ichnos-discovery` | ported | Ten pm4py goldens compare every node's path, depth, final flag and children on five real fixtures and synthetic logs, with unlimited, zero, one, two and large limits. See ichnos-discovery (prefix tree) Behaviour changes. |
| `pm4py.discover_temporal_profile` | `discovery.py` → `algo/discovery/temporal_profile/algorithm`, `objects/log/obj` | `ichnos_discovery::discover_temporal_profile` | `ichnos-discovery` | ported | Golden discovery cases `temporal-profile-*` on running-example, receipt, roadtraffic100traces and interval_event_log CSV, with elapsed time and default business hours. One implementation matches both pm4py variants (log and dataframe). `TemporalProfileOptions::use_start_timestamp` picks the start timestamp explicitly; the interval log case covers it. See the ichnos-discovery (temporal profile) Behaviour changes. |
| `pm4py.discover_log_skeleton` | `discovery.py` → `algo/discovery/log_skeleton/algorithm`, `objects/log/obj` | `ichnos_discovery::log_skeleton` | `ichnos-discovery` | ported | `LogSkeletonOptions` over ordered EventLog input; typed `LogSkeleton` with all six relation/frequency components. Preserves source occurrence-count denominators, per-trace after/before incidence, positive-only never-together subtraction, event-count frequency-coverage target and first-variant frequency ties. Validates finite noise in [0,1], preserves core label/error rules, needs no timestamps and handles empty input. Thirteen `skeleton-declare-*` goldens cover five noise levels and every returned relation/count; label-index encoding is lossless. |
| `pm4py.discover_declare` | `discovery.py` → `algo/discovery/declare/algorithm`, `objects/log/obj` | `ichnos_discovery::declare` | `ichnos-discovery` | ported | `DeclareOptions`, eighteen typed `DeclareTemplate` variants and a typed `DeclareModel` of unary/ordered-binary arguments plus support/confidence counts. Preserves projection before evaluation, case-weighted counts, zero/vacuous binary support, unary violations, derived-template prerequisites, source negative-template formulas, automatic 0.8 selection and descending name/activity tie-breaking; a missing single ratio means zero. Validates finite selection fractions in [0,1]; binary empty target labels remain binary instead of the source key collapse. Thirteen `skeleton-declare-*` goldens cover defaults, explicit/partial ratios, multiplier 0/1, projection, absent activities and template subsets; label-index encoding retains every rule/count. |
| `pm4py.discover_powl` | `discovery.py` → `algo/discovery/powl/algorithm`, `algo/discovery/powl/inductive/variants/dynamic_clustering_frequency/dynamic_clustering_frequency_partial_order_cut`, `algo/discovery/powl/inductive/variants/powl_discovery_varaints`, `objects/log/obj`, `objects/powl/obj` | `ichnos_discovery::powl_inductive` | `ichnos-discovery` | ported | `PowlOptions` picks the variant (`PowlVariant::{Tree, BruteForce, Maximal, DynamicClustering}`, default maximal); `powl_inductive_variants` takes `Variants`. Golden discovery cases `powl-*` on six logs (brute force on four) and on synthetic logs: model equal up to child order. A random probe of 6300 runs matched pm4py except brute force with filtering. See the ichnos-discovery (inductive miner) Behaviour changes. |
| `pm4py.discover_batches` | `discovery.py` → `algo/discovery/batches/algorithm`, `objects/log/obj` | `ichnos_discovery::discover_batches` | `ichnos-discovery` | ported | EventLog path; five real logs and nine synthetic inputs cover all five categories and event identities, including 40 equal-endpoint observations with duplicates. Heap comparisons follow pm4py; overlap sorting uses a total order. See "ichnos-discovery (batches and correlation)" for typed inputs, precision and other changes. |
| `pm4py.correlation_miner` | `discovery.py` → `algo/discovery/correlation_mining/algorithm` | `ichnos_discovery::correlation_miner` | `ichnos-discovery` | ported | Classic only, with complete statistics, boundaries, marginal counts and LP objectives. Uses microlp; non-integral solutions return an error. Receipt compares to pm4py with SciPy HiGHS, retaining the default backend result too. See "ichnos-discovery (batches and correlation)" for solver ties and other changes. |
| `pm4py.discover_otg` | `discovery.py` → `algo/discovery/ocel/otg/algorithm`, `objects/ocel/obj` | `ichnos::discovery::otg` (planned) | `ichnos-discovery` | todo | Variants: classic. |
| `pm4py.discover_etot` | `discovery.py` → `algo/discovery/ocel/etot/algorithm`, `objects/ocel/obj` | `ichnos::discovery::etot` (planned) | `ichnos-discovery` | todo | Variants: classic. |

## conformance

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.conformance_diagnostics_token_based_replay` | `conformance.py` → `algo/conformance/tokenreplay/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::token_replay::replay_log` | `ichnos-conformance` | ported | `replay_log`, `TokenReplayer` (variant `token_replay`, all its options); goldens `conformance/token-replay-*` (3 logs, fixture and inductive-miner nets, default and 7 non-default option sets) compare token counts, fitness and fired transitions exactly. Where pm4py walks Python sets in address order (duplicate labels, ties between marked places), ichnos takes nodes in name order. `consider_activities_not_in_model_in_fitness` applies per trace; pm4py shares one record across the log, so one such trace makes every later trace unfit. Place- and transition-level fitness (`enable_pltr_fitness`) and the `backwards` variant, which `conformance.py` does not call, are not ported. Variants: backwards, token_replay. |
| `pm4py.conformance_diagnostics_alignments` | `conformance.py` → `algo/conformance/alignments/dfg/algorithm`, `algo/conformance/alignments/edit_distance/algorithm`, `algo/conformance/alignments/petri_net/algorithm`, `algo/conformance/alignments/process_tree/variants/search_graph_pt`, `objects/log/obj`, `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos::conformance::alignments::align_log` | `ichnos-conformance` | todo | Petri nets done (`align_log`, `Aligner`; state-equation A* and Dijkstra; goldens `conformance/alignments-*`). DFG (`align_log_dfg`, classic), process tree (`align_log_tree`, search_graph_pt, no interleaving) and edit distance (`align_log_edit_distance`) done; goldens `conformance/alignments-{dfg,tree,edit-distance}-*`. Discounted A* done (`Aligner::align_discounted`; pm4py ties by object-id hash order, so goldens `conformance/alignments-discounted-*` list the costs pm4py gives under 32 seeded hash orders, and the synchronous=False path is not ported). Approximate variants done (`ApproximateAligner`: approx_tandem_repeats, approx_sliding_window, approx_fixed_horizon; goldens `conformance/alignments-approx-*`, moves and counters equal), approx_subset done (`align_log_subset`; goldens `conformance/alignments-subset-*`, moves, bounds and counters equal), and recompos_maximal done (`DecomposedAligner`; goldens `conformance/alignments-decomposed-*` list the costs and alignments pm4py gives under 32 seeded hash orders; ichnos's cost and fitness are among them on every variant, and its alignment differs on some variants because of search ties). See the ichnos-conformance (alignments) Behaviour changes. `dijkstra_less_memory` is not reproduced: it makes some silent moves free, which changes costs, and its fitness leaves out the trace's log moves, which looks like a pm4py bug. `Heuristic::None` matches `dijkstra_no_heuristics`. Variants: approx_fixed_horizon, approx_sliding_window, approx_subset, approx_tandem_repeats, classic, dijkstra_less_memory, dijkstra_no_heuristics, dijkstra_semantics, discounted_a_star, edit_distance, generator_dijkstra_less_memory, generator_dijkstra_no_heuristics, state_equation_a_star, version_dijkstra_less_memory, version_dijkstra_no_heuristics, version_dijkstra_semantics, version_discounted_a_star, version_state_equation_a_star. |
| `pm4py.fitness_token_based_replay` | `conformance.py` → `algo/evaluation/replay_fitness/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::token_replay::fitness_token_based_replay` | `ichnos-conformance` | ported | Token-based variant; goldens `conformance/token-replay-*`. Variants: alignment_based, token_based, token_replay. |
| `pm4py.fitness_alignments` | `conformance.py` → `algo/evaluation/replay_fitness/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::alignments::fitness_alignments` | `ichnos-conformance` | ported | Petri nets, state-equation A* (default) or Dijkstra; goldens `conformance/alignments-*` (3 logs, fixture and inductive-miner nets). Dijkstra matches `dijkstra_no_heuristics`, not `dijkstra_less_memory`, whose free silent moves and fitness without the trace's log moves give other values. Variants: alignment_based, token_based, token_replay. |
| `pm4py.precision_token_based_replay` | `conformance.py` → `algo/evaluation/precision/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::token_replay::precision_token_based_replay` | `ichnos-conformance` | ported | ETConformance with token replay; goldens `conformance/token-replay-*`. Uses pm4py's `get_visible_transitions_eventually_enabled_by_marking` with its quirk, so values match pm4py. Prefixes are activity sequences; pm4py joins them with "," and splits them again, which breaks on activity names that contain ",". Variants: align_etconformance, automaton_after_align, etconformance_token. |
| `pm4py.precision_alignments` | `conformance.py` → `algo/evaluation/precision/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::alignments::precision_alignments` | `ichnos-conformance` | ported | Align-ETConformance; goldens `conformance/alignments-*`. Explores every marking reachable through silent transitions, so `receipt` with its inductive net gives 0.16606848450719514 where pm4py gives 0.16610454739785152; the golden records both. Variants: align_etconformance, automaton_after_align, etconformance_token. |
| `pm4py.generalization_tbr` | `conformance.py` → `algo/evaluation/generalization/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::generalization::generalization_tbr` | `ichnos-conformance` | ported | Token-based generalization; goldens `conformance/token-replay-*`. Variants: generalization_token, token_based. |
| `pm4py.replay_prefix_tbr` | `conformance.py` → `algo/conformance/tokenreplay/variants/token_replay`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::token_replay::replay_prefix_tbr` | `ichnos-conformance` | ported | Goldens `conformance/token-replay-*` (prefix of each variant). Single entry point; preserve source defaults. |
| `pm4py.conformance_diagnostics_footprints` | `conformance.py` → `algo/conformance/footprints/algorithm` | `ichnos::conformance::footprints::conformance_diagnostics_footprints` | `ichnos-conformance` | ported | Goldens `conformance/footprints-*` against nets and trees. `conformance_diagnostics_footprints` is pm4py on an `EventLog` (trace_extensive); `conformance_diagnostics_footprints_log` is pm4py on a dataframe (log_extensive); `footprint_violations` is log_model, loose and strict. `ModelFootprints::of_net` follows pm4py's reach_graph footprints, quirk in `get_visible_transitions_eventually_enabled_by_marking` included, so it can differ from `PetriNet::footprints`. `ModelFootprints::of_net` fails with `Error::Reachability` past `ReachabilityOptions::default().max_markings` markings, where pm4py has no marking bound, stops after 86400 seconds with partial footprints, and can loop forever on a token-producing silent cycle. Variants: log_extensive, log_model, trace_extensive. |
| `pm4py.fitness_footprints` | `conformance.py` → `algo/conformance/footprints/algorithm`, `algo/conformance/footprints/util/evaluation` | `ichnos::conformance::footprints::fitness_footprints` | `ichnos-conformance` | ported | Goldens `conformance/footprints-*`. `fitness_footprints` (trace by trace, with the percentage of fitting traces) and `fitness_footprints_log` (whole log). A log without traces gives 0% fitting traces where pm4py divides by zero. Variants: log_extensive, log_model, trace_extensive. |
| `pm4py.precision_footprints` | `conformance.py` → `algo/conformance/footprints/util/evaluation` | `ichnos::conformance::footprints::precision_footprints` | `ichnos-conformance` | ported | Goldens `conformance/footprints-*`, from the `EventLog` and from whole-log footprints (same value). Single entry point; preserve source defaults. |
| `pm4py.check_is_fitting` | `conformance.py` → `objects/log/obj`, `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos::conformance::token_replay::check_is_fitting` | `ichnos-conformance` | ported | Goldens `conformance/footprints-*` (every variant, nets and trees). `check_is_fitting` takes a net, `check_is_fitting_tree` a tree. pm4py first tries to turn a net into a tree; ichnos always checks a net as a net, which gives the same answer. |
| `pm4py.conformance_temporal_profile` | `conformance.py` → `algo/conformance/temporal_profile/algorithm`, `objects/log/obj` | `ichnos_conformance::temporal_profile::conformance_temporal_profile` | `ichnos-conformance` | ported | Golden conformance cases `temporal-profile-*` on running-example, receipt, roadtraffic100traces and interval_event_log CSV: zeta 1 and 6, default business hours, and a profile from every other case, so that pairs with a standard deviation of 0 deviate. One implementation matches pm4py's dataframe variant. The log variant adds deviations from float noise, which the test checks. Zeta defaults to 1, as in the pm4py wrapper. See the ichnos-conformance (temporal profile) Behaviour changes. |
| `pm4py.conformance_declare` | `conformance.py` → `algo/conformance/declare/algorithm`, `objects/log/obj` | `ichnos::conformance::conformance_declare` (planned) | `ichnos-conformance` | todo | Variants: classic. |
| `pm4py.conformance_log_skeleton` | `conformance.py` → `algo/conformance/log_skeleton/algorithm`, `objects/log/obj` | `ichnos::conformance::conformance_log_skeleton` (planned) | `ichnos-conformance` | todo | Variants: classic. |
| `pm4py.conformance_ocdfg` | `conformance.py` → `algo/conformance/ocel/ocdfg/algorithm`, `objects/ocel/obj` | `ichnos::conformance::conformance_ocdfg` (planned) | `ichnos-conformance` | todo | Variants: graph_comparison. |
| `pm4py.conformance_otg` | `conformance.py` → `algo/conformance/ocel/otg/algorithm`, `objects/ocel/obj` | `ichnos::conformance::conformance_otg` (planned) | `ichnos-conformance` | todo | Variants: graph_comparison. |
| `pm4py.conformance_etot` | `conformance.py` → `algo/conformance/ocel/etot/algorithm`, `objects/ocel/obj` | `ichnos::conformance::conformance_etot` (planned) | `ichnos-conformance` | todo | Variants: graph_comparison. |

## filtering

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.filter_log_relative_occurrence_event_attribute` | `filtering.py` → `algo/filtering/log/attributes/attributes_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_log_relative_occurrence_event_attribute` | `ichnos-stats` | ported | Counts each value once per case or per event; both bases retain only qualifying events and drop empty cases. |
| `pm4py.filter_start_activities` | `filtering.py` → `algo/filtering/log/start_activities/start_activities_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_start_activities` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_end_activities` | `filtering.py` → `algo/filtering/log/end_activities/end_activities_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_end_activities` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_event_attribute_values` | `filtering.py` → `algo/filtering/log/attributes/attributes_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_event_attribute_values` | `ichnos-stats` | ported | Missing event attributes are nonmatches rather than raising KeyError in event scope. Event scope drops empty cases; case scope retains complete nonempty cases. |
| `pm4py.filter_trace_attribute_values` | `filtering.py` → `algo/filtering/log/attributes/attributes_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_trace_attribute_values` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_variants` | `filtering.py` → `algo/filtering/log/variants/variants_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_variants` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_directly_follows_relation` | `filtering.py` → `algo/filtering/log/paths/paths_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_directly_follows_relation` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_eventually_follows_relation` | `filtering.py` → `algo/filtering/log/ltl/ltl_checker`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_eventually_follows_relation` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_time_range` | `filtering.py` → `algo/filtering/log/timestamp/timestamp_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_time_range` | `ichnos-stats` | ported | Typed inclusive dates and mode enum replace date strings; exclusion is also supported for event/contained/intersecting modes. |
| `pm4py.filter_between` | `filtering.py` → `algo/filtering/log/between/between_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_between` | `ichnos-stats` | ported | Complete inclusive subcases receive an ID##@@index in the configurable subcase attribute (default concept:name, updating core case IDs as in pm4py’s DataFrame path; use case:concept:name explicitly for EventLog-path parity); absent IDs fall back to the trace index, and numeric IDs stringify rather than raising TypeError. Equal start/end lists share the boundary event. |
| `pm4py.filter_case_size` | `filtering.py` → `algo/filtering/log/cases/case_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_case_size` | `ichnos-stats` | ported | Log metadata is preserved where the source constructs a metadata-free log. |
| `pm4py.filter_case_performance` | `filtering.py` → `algo/filtering/log/cases/case_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_case_performance` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_activities_rework` | `filtering.py` → `algo/filtering/log/rework/rework_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_activities_rework` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_paths_performance` | `filtering.py` → `algo/filtering/log/paths/paths_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_paths_performance` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_variants_top_k` | `filtering.py` → `algo/filtering/log/variants/variants_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_variants_top_k` | `ichnos-stats` | ported | Count ties sort by descending variant text; k is unsigned. |
| `pm4py.filter_variants_by_coverage_percentage` | `filtering.py` → `algo/filtering/log/variants/variants_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_variants_by_coverage_percentage` | `ichnos-stats` | ported | Coverage is tested per variant, not cumulatively. |
| `pm4py.filter_prefixes` | `filtering.py` → `algo/filtering/log/prefixes/prefix_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_prefixes` | `ichnos-stats` | ported | Strict slices exclude the boundary event and retain empty traces; first/last occurrence is typed. |
| `pm4py.filter_suffixes` | `filtering.py` → `algo/filtering/log/suffixes/suffix_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_suffixes` | `ichnos-stats` | ported | Strict slices exclude the boundary event and retain empty traces; first/last occurrence is typed. |
| `pm4py.filter_ocel_event_attribute` | `filtering.py` → `algo/filtering/ocel/event_attributes`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_event_attribute` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_object_attribute` | `filtering.py` → `algo/filtering/ocel/object_attributes`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_object_attribute` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_object_types_allowed_activities` | `filtering.py` → `algo/filtering/ocel/activity_type_matching`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_object_types_allowed_activities` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_object_per_type_count` | `filtering.py` → `algo/filtering/ocel/objects_ot_count`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_object_per_type_count` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_start_events_per_object_type` | `filtering.py` → `algo/filtering/ocel/ot_endpoints`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_start_events_per_object_type` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_end_events_per_object_type` | `filtering.py` → `algo/filtering/ocel/ot_endpoints`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_end_events_per_object_type` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_events_timestamp` | `filtering.py` → `algo/filtering/ocel/event_attributes`, `objects/ocel/obj` | `ichnos::ocel::filter_ocel_events_timestamp` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_four_eyes_principle` | `filtering.py` → `algo/filtering/log/ltl/ltl_checker`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_four_eyes_principle` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_activity_done_different_resources` | `filtering.py` → `algo/filtering/log/ltl/ltl_checker`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_activity_done_different_resources` | `ichnos-stats` | ported | Goldens `filters-log-*`; the log-filters package notes give the shared conventions. |
| `pm4py.filter_trace_segments` | `filtering.py` → `algo/filtering/log/traces/trace_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos_stats::filters::filter_trace_segments` | `ichnos-stats` | ported | Typed activity/wildcard patterns replace comma-delimited regexes; commas and punctuation are literal activities, wildcard-only patterns match any trace, and an empty pattern list matches none. |
| `pm4py.filter_ocel_object_types` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_object_types` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_objects` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_objects` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_events` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_events` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_activities_connected_object_type` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_activities_connected_object_type` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_object` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_cc_object` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_length` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_cc_length` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_otype` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_cc_otype` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_activity` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::filter_ocel_cc_activity` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_dfg_activities_percentage` | `filtering.py` → `algo/filtering/dfg/dfg_filtering` | `ichnos_stats::filters::filter_dfg_activities_percentage` | `ichnos-stats` | ported | Goldens filters-dfg-* cover three logs at percentages 0, 0.2, 0.5 and 1. Thin wrapper over ichnos-model Dfg filtering; maximum incoming/outgoing totals supply activity frequencies. Fractions outside [0,1] return typed errors. |
| `pm4py.filter_dfg_paths_percentage` | `filtering.py` → `algo/filtering/dfg/dfg_filtering` | `ichnos_stats::filters::filter_dfg_paths_percentage` | `ichnos-stats` | ported | Goldens filters-dfg-* cover three logs at percentages 0, 0.2, 0.5 and 1. Thin wrapper over ichnos-model Dfg filtering; maximum incoming/outgoing totals supply activity frequencies. Fractions outside [0,1] return typed errors. |

## stats

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.get_start_activities` | `stats.py` → `objects/log/obj`, `statistics/start_activities/log/get` | `ichnos_stats::attributes::get_start_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_end_activities` | `stats.py` → `objects/log/obj`, `statistics/end_activities/log/get` | `ichnos_stats::attributes::get_end_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_event_attributes` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos_stats::attributes::get_event_attributes` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_trace_attributes` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos_stats::attributes::get_trace_attributes` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_event_attribute_values` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos_stats::attributes::get_event_attribute_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_trace_attribute_values` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos_stats::attributes::get_trace_attribute_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_variants` | `stats.py` → `objects/log/obj`, `statistics/variants/log/get` | `ichnos_stats::variants::get_variants` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed sequence/count map also represents the backend count/set helpers. |
| `pm4py.get_variants_as_tuples` | `stats.py` → `objects/log/obj`, `statistics/variants/log/get` | `ichnos_stats::variants::get_variants_as_tuples` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.split_by_process_variant` | `stats.py` → `objects/log/obj`, `objects/log/util/pandas_numpy_variants` | `ichnos_stats::variants::split_by_process_variant` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Returns metadata-preserving EventLogs rather than DataFrames with utility columns. |
| `pm4py.get_variants_paths_duration` | `stats.py` → `objects/log/obj`, `objects/log/util/pandas_numpy_variants` | `ichnos_stats::variants::get_variants_paths_duration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_stochastic_language` | `stats.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/petri_net/obj`, `objects/process_tree/obj`, `statistics/variants/log/get` | `ichnos_stats::variants::get_stochastic_language` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_minimum_self_distances` | `stats.py` → `algo/discovery/minimum_self_distance/algorithm`, `objects/log/obj` | `ichnos_stats::cases::get_minimum_self_distances` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |
| `pm4py.get_minimum_self_distance_witnesses` | `stats.py` → `algo/discovery/minimum_self_distance/algorithm`, `algo/discovery/minimum_self_distance/utils`, `objects/log/obj` | `ichnos_stats::cases::get_minimum_self_distance_witnesses` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |
| `pm4py.get_case_arrival_average` | `stats.py` → `objects/log/obj`, `statistics/traces/generic/log/case_arrival` | `ichnos_stats::cases::get_case_arrival_average` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |
| `pm4py.get_rework_cases_per_activity` | `stats.py` → `objects/log/obj`, `statistics/rework/log/get` | `ichnos_stats::cases::get_rework_cases_per_activity` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |
| `pm4py.get_case_overlap` | `stats.py` → `objects/log/obj`, `statistics/overlap/cases/log/get` | `ichnos_stats::cases::get_case_overlap` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Includes self and duplicate intervals, expands by epsilon; empty traces contribute zero. |
| `pm4py.get_cycle_time` | `stats.py` → `objects/log/obj`, `statistics/traces/cycle_time/log/get` | `ichnos_stats::cases::get_cycle_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Preserves reference omission of the final merged interval; empty inputs return zero. |
| `pm4py.get_service_time` | `stats.py` → `objects/log/obj`, `statistics/service_time/log/get` | `ichnos_stats::cases::get_service_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |
| `pm4py.get_all_case_durations` | `stats.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics` | `ichnos_stats::cases::get_all_case_durations` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |
| `pm4py.get_case_duration` | `stats.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics` | `ichnos_stats::cases::get_case_duration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |
| `pm4py.get_frequent_trace_segments` | `stats.py` → `objects/log/obj` | `ichnos_stats::variants::get_frequent_trace_segments` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.get_activity_position_summary` | `stats.py` → `objects/log/obj` | `ichnos_stats::cases::get_activity_position_summary` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |
| `pm4py.get_process_cube` | `stats.py` → `statistics/process_cube/pandas/algorithm`, `statistics/process_cube/polars/algorithm` | `ichnos::stats::get_process_cube` (planned) | `ichnos-stats` | todo | Variants: classic. |

## utils

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.utils.Shared` | `utils.py` | `ichnos::core::Shared` (planned) | `ichnos-core` | dropped | Python global warning-state holder, not a process-mining API; Rust diagnostics replace it. |
| `pm4py.utils.is_polars_lazyframe` | `utils.py` | `ichnos::core::is_polars_lazyframe` (planned) | `ichnos-core` | dropped | Python backend type detection; Rust types replace runtime pandas/polars dispatch. |
| `pm4py.format_dataframe` | `utils.py` → `objects/log/util/dataframe_utils` | `ichnos::format_batch` | `ichnos-core` | ported | Arrow `RecordBatch` form. Goldens `core/format-receipt-csv` and `core/format-interval-event-log-csv` check row order; `log/running-example-csv` loads through it. Changes core-20 to core-22. |
| `pm4py.rebase` | `utils.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/dataframe_utils` | `ichnos::EventLog::rebase`, `ichnos::EventStream::rebase` | `ichnos-core` | ported | Goldens `core/rebase-running-example-csv` and `core/rebase-stream-running-example-csv`. |
| `pm4py.parse_process_tree` | `utils.py` → `objects/process_tree/obj`, `objects/process_tree/utils/generic` | `ichnos::model::parse_process_tree` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.parse_powl_model_string` | `utils.py` → `objects/powl/obj`, `objects/powl/parser` | `ichnos_model::Powl::parse` | `ichnos-model` | ported | Also `str::parse`. Goldens `powl/model-*` parse 17 POWL strings and compare the model, `simplify`, `simplify_using_frequent_transitions`, the Petri net and the process tree. |
| `pm4py.serialize` | `utils.py` → `objects/bpmn/exporter/exporter`, `objects/bpmn/obj`, `objects/dfg/exporter/exporter`, `objects/log/exporter/xes/exporter`, `objects/log/obj`, `objects/petri_net/exporter/exporter`, `objects/petri_net/obj`, `objects/process_tree/exporter/exporter`, `objects/process_tree/obj` | none | `ichnos-core` | dropped | Each ichnos-io writer (XES, Parquet, Arrow IPC, PNML and others) already returns the bytes, and Rust callers know the type, so a tagged dispatcher adds nothing. See `docs/design.md`. Change core-30. |
| `pm4py.deserialize` | `utils.py` → `objects/bpmn/importer/importer`, `objects/dfg/importer/importer`, `objects/log/importer/xes/importer`, `objects/petri_net/importer/importer`, `objects/process_tree/importer/importer` | none | `ichnos-core` | dropped | Each ichnos-io reader already reads the bytes into a known type. See `docs/design.md`. Change core-30. |
| `pm4py.utils.get_properties` | `utils.py` | `ichnos::core::get_properties` (planned) | `ichnos-core` | dropped | Python string-keyed parameter-map adapter; typed Rust options replace it. |
| `pm4py.set_classifier` | `utils.py` → `objects/log/obj` | `ichnos::EventLog::insert_classifier_attribute`, `ichnos::EventLog::insert_named_classifier_attribute` | `ichnos-core` | ported | Golden `core/set-classifier-running-example-csv`. Changes core-14, core-29. |
| `pm4py.parse_event_log_string` | `utils.py` → `objects/log/obj` | `ichnos::EventLog::from_trace_strings` | `ichnos-core` | ported | Checked against pm4py output in `log::tests::from_trace_strings_matches_pm4py`. Change core-12. |
| `pm4py.project_on_event_attribute` | `utils.py` → `objects/log/obj`, `streaming/conversion/from_pandas` | `ichnos::EventLog::project` | `ichnos-core` | ported | Golden `core/project-running-example-csv`. Change core-28. |
| `pm4py.sample_cases` | `utils.py` → `objects/log/obj`, `objects/log/util/dataframe_utils`, `objects/log/util/sampling` | `ichnos::EventLog::sample_cases` | `ichnos-core` | ported | Golden `core/sample-running-example-csv` (sizes only). Change core-17. |
| `pm4py.sample_events` | `utils.py` → `objects/log/obj`, `objects/log/util/sampling`, `objects/ocel/obj`, `objects/ocel/util/sampling` | `ichnos::EventStream::sample_events` | `ichnos-core` | ported | Golden `core/sample-running-example-csv` (sizes only). Changes core-17, core-18. |

## vis

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.view_petri_net` | `vis.py` → `objects/log/obj`, `objects/petri_net/obj`, `visualization/petri_net/visualizer` | `ichnos_viz::petri_net_dot` | `ichnos-viz` | ported | Goldens `viz/petri-net-*`: running-example and receipt nets from the inductive miner, a styled run (background, rankdir, title), `inh_res_nets/order_fulfillment.pnml` with inhibitor and reset arcs (plain, with `font_size` and `debug`, and with decorations), `data_petri_net.pnml` with guards, and a sequence net that checks the statement order. Ports the wo_decoration variant that `pm4py.view_petri_net` uses. The token-replay, greedy, frequency, performance and alignments decoration variants are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_petri_net` | `vis.py` → `objects/log/obj`, `objects/petri_net/obj`, `visualization/petri_net/visualizer` | `ichnos_viz::write_petri_net` | `ichnos-viz` | ported | Goldens `viz/petri-net-*`: running-example and receipt nets from the inductive miner, a styled run (background, rankdir, title), `inh_res_nets/order_fulfillment.pnml` with inhibitor and reset arcs (plain, with `font_size` and `debug`, and with decorations), `data_petri_net.pnml` with guards, and a sequence net that checks the statement order. Ports the wo_decoration variant that `pm4py.view_petri_net` uses. The token-replay, greedy, frequency, performance and alignments decoration variants are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.view_performance_dfg` | `vis.py` → `visualization/dfg/variants/performance`, `visualization/dfg/visualizer` | `ichnos_viz::performance_dfg_dot` | `ichnos-viz` | ported | Goldens `viz/performance-dfg-*` on running-example and receipt, the median aggregate, the default business hours and `serv_time`; they check the statement order. Ports the performance variant; the cost and timeline variants are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_performance_dfg` | `vis.py` → `visualization/dfg/variants/performance`, `visualization/dfg/visualizer` | `ichnos_viz::write_performance_dfg` | `ichnos-viz` | ported | Goldens `viz/performance-dfg-*` on running-example and receipt, the median aggregate, the default business hours and `serv_time`; they check the statement order. Ports the performance variant; the cost and timeline variants are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.view_dfg` | `vis.py` → `visualization/dfg/visualizer` | `ichnos_viz::dfg_dot` | `ichnos-viz` | ported | Goldens `viz/dfg-*` on running-example and receipt, `max_num_edges` 8, and `activities_count` with `font_size`; they check the statement order. Ports the frequency variant; the cost and timeline variants are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_dfg` | `vis.py` → `visualization/dfg/visualizer` | `ichnos_viz::write_dfg` | `ichnos-viz` | ported | Goldens `viz/dfg-*` on running-example and receipt, `max_num_edges` 8, and `activities_count` with `font_size`; they check the statement order. Ports the frequency variant; the cost and timeline variants are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.view_process_tree` | `vis.py` → `objects/process_tree/obj`, `visualization/process_tree/visualizer` | `ichnos_viz::process_tree_dot` | `ichnos-viz` | ported | Goldens `viz/process-tree-*`: the inductive trees of running-example and receipt, and one unsorted (`enable_deepcopy` false) with `font_size`. Ports the wo_decoration variant with pm4py's `tree_sort` child order; frequency_annotation and symbolic are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_process_tree` | `vis.py` → `objects/process_tree/obj`, `visualization/process_tree/visualizer` | `ichnos_viz::write_process_tree` | `ichnos-viz` | ported | Goldens `viz/process-tree-*`: the inductive trees of running-example and receipt, and one unsorted (`enable_deepcopy` false) with `font_size`. Ports the wo_decoration variant with pm4py's `tree_sort` child order; frequency_annotation and symbolic are not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_bpmn` | `vis.py` → `objects/bpmn/obj`, `visualization/bpmn/visualizer` | `ichnos_viz::write_bpmn` | `ichnos-viz` | ported | Goldens `viz/bpmn-*` on `running-example.bpmn` and `receipt.bpmn`, which check the node order, and on `synthetic-bpmn/all_kinds.bpmn` (participants) with default options, with `include_name_in_events`, `endpoints_shape`, `swimlanes_margin` and `font_size`, and without swimlanes. Ports the classic variant with swimlanes; dagrejs and bpmnio_auto_layout are HTML renderers and are excluded. See ichnos-viz Behaviour changes. |
| `pm4py.view_bpmn` | `vis.py` → `objects/bpmn/obj`, `visualization/bpmn/visualizer` | `ichnos_viz::bpmn_dot` | `ichnos-viz` | ported | Goldens `viz/bpmn-*` on `running-example.bpmn` and `receipt.bpmn`, which check the node order, and on `synthetic-bpmn/all_kinds.bpmn` (participants) with default options, with `include_name_in_events`, `endpoints_shape`, `swimlanes_margin` and `font_size`, and without swimlanes. Ports the classic variant with swimlanes; dagrejs and bpmnio_auto_layout are HTML renderers and are excluded. See ichnos-viz Behaviour changes. |
| `pm4py.view_heuristics_net` | `vis.py` → `objects/heuristics_net/obj`, `visualization/heuristics_net/visualizer` | `ichnos_viz::heuristics_net_dot` | `ichnos-viz` | ported | Goldens `viz/heuristics-net-*` on running-example and receipt, and a net discovered with `min_dfg_occurrences` 3. Ports pydotplus_vis for frequency nets. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_heuristics_net` | `vis.py` → `objects/heuristics_net/obj`, `visualization/heuristics_net/visualizer` | `ichnos_viz::write_heuristics_net` | `ichnos-viz` | ported | Goldens `viz/heuristics-net-*` on running-example and receipt, and a net discovered with `min_dfg_occurrences` 3. Ports pydotplus_vis for frequency nets. See ichnos-viz Behaviour changes. |
| `pm4py.view_dotted_chart` | `vis.py` → `objects/log/obj`, `visualization/dotted_chart/visualizer` | `ichnos_viz::dotted_chart_dot` | `ichnos-viz` | ported | Goldens `viz/dotted-chart-*` on running-example: time, case and activity with a legend; activity and resource, two text axes, with a title; resource, time and activity without a legend. `dotted_chart_points` reads the points from an `EventLog`. Ports the classic variant; `write_dotted_chart` renders with `neato -n1`. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_dotted_chart` | `vis.py` → `objects/log/obj`, `visualization/dotted_chart/visualizer` | `ichnos_viz::write_dotted_chart` | `ichnos-viz` | ported | Goldens `viz/dotted-chart-*` on running-example: time, case and activity with a legend; activity and resource, two text axes, with a title; resource, time and activity without a legend. `dotted_chart_points` reads the points from an `EventLog`. Ports the classic variant; `write_dotted_chart` renders with `neato -n1`. See ichnos-viz Behaviour changes. |
| `pm4py.view_sna` | `vis.py` → `objects/org/sna/obj`, `visualization/sna/visualizer` | `ichnos::viz::view_sna` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: networkx, pyvis. |
| `pm4py.save_vis_sna` | `vis.py` → `objects/org/sna/obj`, `visualization/sna/visualizer` | `ichnos::viz::save_vis_sna` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: networkx, pyvis. |
| `pm4py.view_case_duration_graph` | `vis.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics`, `statistics/traces/generic/pandas/case_statistics`, `visualization/graphs/visualizer` | `ichnos::viz::view_case_duration_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.save_vis_case_duration_graph` | `vis.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics`, `statistics/traces/generic/pandas/case_statistics`, `visualization/graphs/visualizer` | `ichnos::viz::save_vis_case_duration_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.view_events_per_time_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::view_events_per_time_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.save_vis_events_per_time_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::save_vis_events_per_time_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.view_performance_spectrum` | `vis.py` → `algo/discovery/performance_spectrum/algorithm`, `objects/log/obj`, `visualization/performance_spectrum/variants/neato`, `visualization/performance_spectrum/visualizer` | `ichnos_viz::performance_spectrum_dot` | `ichnos-viz` | ported | Goldens `viz/performance-spectrum-*` on running-example (two activity lists, one with a title) and receipt (sampled to 25 runs) check the runs and the drawing. `performance_spectrum` ports the `log` discovery variant; the dataframe, lazyframe and disconnected variants are not ported. Ports the neato drawing; `write_performance_spectrum` renders with `neato -n1`. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_performance_spectrum` | `vis.py` → `algo/discovery/performance_spectrum/algorithm`, `objects/log/obj`, `visualization/performance_spectrum/variants/neato`, `visualization/performance_spectrum/visualizer` | `ichnos_viz::write_performance_spectrum` | `ichnos-viz` | ported | Goldens `viz/performance-spectrum-*` on running-example (two activity lists, one with a title) and receipt (sampled to 25 runs) check the runs and the drawing. `performance_spectrum` ports the `log` discovery variant; the dataframe, lazyframe and disconnected variants are not ported. Ports the neato drawing; `write_performance_spectrum` renders with `neato -n1`. See ichnos-viz Behaviour changes. |
| `pm4py.view_events_distribution_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::view_events_distribution_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.save_vis_events_distribution_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::save_vis_events_distribution_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.view_ocdfg` | `vis.py` → `visualization/ocel/ocdfg/visualizer` | `ichnos_viz::ocdfg_dot` | `ichnos-viz` | ported | Goldens `viz/ocdfg-*` on example_log and ocel20_example: frequency; performance, also with business hours and the total-objects metric; and other metrics with thresholds, a background colour, `rankdir` and a title. The input is `Ocdfg`, the counts of pm4py's `discover_ocdfg`, which ichnos does not port yet. Ports the classic variant; elkjs writes HTML and is excluded. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_ocdfg` | `vis.py` → `visualization/ocel/ocdfg/visualizer` | `ichnos_viz::write_ocdfg` | `ichnos-viz` | ported | Goldens `viz/ocdfg-*` on example_log and ocel20_example: frequency; performance, also with business hours and the total-objects metric; and other metrics with thresholds, a background colour, `rankdir` and a title. The input is `Ocdfg`, the counts of pm4py's `discover_ocdfg`, which ichnos does not port yet. Ports the classic variant; elkjs writes HTML and is excluded. See ichnos-viz Behaviour changes. |
| `pm4py.view_ocpn` | `vis.py` → `objects/ocpn/obj`, `visualization/ocel/ocpn/visualizer` | `ichnos_viz::ocpn_dot` | `ichnos-viz` | ported | Goldens `viz/ocpn-*` on example_log and ocel20_example, and example_log with replay diagnostics, `rankdir` and a title. The input is `OcPetriNet`, the parts of pm4py's `discover_oc_petri_net` that the drawing reads. Ports the wo_decoration variant; the brachmann variant is not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_ocpn` | `vis.py` → `objects/ocpn/obj`, `visualization/ocel/ocpn/visualizer` | `ichnos_viz::write_ocpn` | `ichnos-viz` | ported | Goldens `viz/ocpn-*` on example_log and ocel20_example, and example_log with replay diagnostics, `rankdir` and a title. The input is `OcPetriNet`, the parts of pm4py's `discover_oc_petri_net` that the drawing reads. Ports the wo_decoration variant; the brachmann variant is not ported. See ichnos-viz Behaviour changes. |
| `pm4py.view_network_analysis` | `vis.py` → `visualization/network_analysis/visualizer` | `ichnos_viz::network_analysis_dot` | `ichnos-viz` | ported | Goldens `viz/network-analysis-*` on running-example (counts; durations, also with business hours; thresholds with a title) and receipt (counts). `network_analysis_performance_dot` and `write_network_analysis_performance` draw the performance variant. The input is a list of `NetworkAnalysisEdge`, the result of pm4py's `discover_network_analysis` in its order. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_network_analysis` | `vis.py` → `visualization/network_analysis/visualizer` | `ichnos_viz::write_network_analysis` | `ichnos-viz` | ported | Goldens `viz/network-analysis-*` on running-example (counts; durations, also with business hours; thresholds with a title) and receipt (counts). `network_analysis_performance_dot` and `write_network_analysis_performance` draw the performance variant. The input is a list of `NetworkAnalysisEdge`, the result of pm4py's `discover_network_analysis` in its order. See ichnos-viz Behaviour changes. |
| `pm4py.view_transition_system` | `vis.py` → `objects/transition_system/obj`, `visualization/transition_system/visualizer` | `ichnos_viz::transition_system_dot` | `ichnos-viz` | ported | Goldens `viz/transition-system-*` on the running-example and receipt transition systems. Ports the view_based variant that `pm4py.view_transition_system` uses; trans_frequency is not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_transition_system` | `vis.py` → `objects/transition_system/obj`, `visualization/transition_system/visualizer` | `ichnos_viz::write_transition_system` | `ichnos-viz` | ported | Goldens `viz/transition-system-*` on the running-example and receipt transition systems. Ports the view_based variant that `pm4py.view_transition_system` uses; trans_frequency is not ported. See ichnos-viz Behaviour changes. |
| `pm4py.view_prefix_tree` | `vis.py` → `objects/trie/obj`, `visualization/trie/visualizer` | `ichnos_viz::prefix_tree_dot` | `ichnos-viz` | ported | Goldens `viz/prefix-tree-*` on the running-example and receipt prefix trees. Ports the classic variant. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_prefix_tree` | `vis.py` → `objects/trie/obj`, `visualization/trie/visualizer` | `ichnos_viz::write_prefix_tree` | `ichnos-viz` | ported | Goldens `viz/prefix-tree-*` on the running-example and receipt prefix trees. Ports the classic variant. See ichnos-viz Behaviour changes. |
| `pm4py.view_alignments` | `vis.py` → `objects/log/obj`, `visualization/align_table/visualizer` | `ichnos_viz::alignments_dot` | `ichnos-viz` | ported | Goldens `viz/alignments-*` on running-example and receipt, with alignments built from each trace (sync, log and silent and visible model moves, and a label pm4py escapes). `alignment_table_dot` and `write_alignment_table` draw rows the caller builds. Ports the classic variant. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_alignments` | `vis.py` → `objects/log/obj`, `visualization/align_table/visualizer` | `ichnos_viz::write_alignments` | `ichnos-viz` | ported | Goldens `viz/alignments-*` on running-example and receipt, with alignments built from each trace (sync, log and silent and visible model moves, and a label pm4py escapes). `alignment_table_dot` and `write_alignment_table` draw rows the caller builds. Ports the classic variant. See ichnos-viz Behaviour changes. |
| `pm4py.view_footprints` | `vis.py` → `visualization/footprints/visualizer` | `ichnos_viz::footprints_dot` | `ichnos-viz` | ported | Goldens `viz/footprints-*`: one footprint for running-example and receipt, and pairs of the log against its inductive net and against the log without one activity. `footprints_comparison_dot` and `write_footprints_comparison` draw a pair. Ports the single and comparison_symmetric variants that `pm4py.save_vis_footprints` uses; the comparison variant is not ported. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_footprints` | `vis.py` → `visualization/footprints/visualizer` | `ichnos_viz::write_footprints` | `ichnos-viz` | ported | Goldens `viz/footprints-*`: one footprint for running-example and receipt, and pairs of the log against its inductive net and against the log without one activity. `footprints_comparison_dot` and `write_footprints_comparison` draw a pair. Ports the single and comparison_symmetric variants that `pm4py.save_vis_footprints` uses; the comparison variant is not ported. See ichnos-viz Behaviour changes. |
| `pm4py.view_powl` | `vis.py` → `objects/powl/obj`, `visualization/powl/visualizer` | `ichnos_viz::powl_dot` | `ichnos-viz` | ported | Goldens `viz/powl-*`: models discovered from running-example and receipt, and a parsed model with tagged activities, a background colour and `rankdir`. Ports the basic variant; the net variant is not ported. The icons ship with the crate (`POWL_ICONS`, `write_powl_icons`). See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_powl` | `vis.py` → `objects/powl/obj`, `visualization/powl/visualizer` | `ichnos_viz::write_powl` | `ichnos-viz` | ported | Goldens `viz/powl-*`: models discovered from running-example and receipt, and a parsed model with tagged activities, a background colour and `rankdir`. Ports the basic variant; the net variant is not ported. The icons ship with the crate (`POWL_ICONS`, `write_powl_icons`). See ichnos-viz Behaviour changes. |
| `pm4py.view_object_graph` | `vis.py` → `objects/ocel/obj`, `visualization/ocel/object_graph/visualizer` | `ichnos_viz::object_graph_dot` | `ichnos-viz` | ported | Goldens `viz/object-graph-*`: interaction graphs of example_log and ocel20_example, a descendants graph with `rankdir` and a title, and an undirected graph. Ports the graphviz variant. See ichnos-viz Behaviour changes. |
| `pm4py.save_vis_object_graph` | `vis.py` → `objects/ocel/obj`, `visualization/ocel/object_graph/visualizer` | `ichnos_viz::write_object_graph` | `ichnos-viz` | ported | Goldens `viz/object-graph-*`: interaction graphs of example_log and ocel20_example, a descendants graph with `rankdir` and a title, and an undirected graph. Ports the graphviz variant. See ichnos-viz Behaviour changes. |

## sim

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.play_out` | `sim.py` → `algo/simulation/playout/declare/algorithm`, `algo/simulation/playout/dfg/algorithm`, `algo/simulation/playout/petri_net/algorithm`, `algo/simulation/playout/process_tree/algorithm`, `objects/log/obj`, `objects/petri_net/inhibitor_reset/semantics`, `objects/petri_net/obj`, `objects/petri_net/semantics`, `objects/process_tree/obj` | `ichnos::sim::play_out` (planned) | `ichnos-sim` | todo | Variants: basic_playout, classic, extensive, performance, stochastic_playout, topbottom. |
| `pm4py.generate_process_tree` | `sim.py` → `algo/simulation/tree_generator/algorithm`, `objects/process_tree/obj` | `ichnos::sim::generate_process_tree` (planned) | `ichnos-sim` | todo | Variants: basic, ptandloggenerator. |

## ml

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.split_train_test` | `ml.py` → `objects/log/obj`, `objects/log/util/split_train_test` | `ichnos_ml::split_train_test` | `ichnos-ml` | ported | Goldens `ml/split-*` on running-example and receipt replay pm4py's seeded random draws and check the train and test case IDs. Ports the `EventLog` path, with the random draws passed in; the data-frame path is not ported. See ichnos-ml Behaviour changes. |
| `pm4py.get_prefixes_from_log` | `ml.py` → `objects/log/obj`, `objects/log/util/get_prefixes` | `ichnos_ml::get_prefixes_from_log` | `ichnos-ml` | ported | Goldens `ml/prefixes-*` on running-example (lengths 1 and 3) and receipt (length 3). Ports the `EventLog` path; the data-frame path is not ported. |
| `pm4py.extract_outcome_enriched_dataframe` | `ml.py` → `algo/transformation/trace_encodings/algorithm`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_ml::extract_outcome_enriched_dataframe` | `ichnos-ml` | ported | Goldens `ml/outcome-*` on running-example and roadtraffic100traces check the six `@@` timing columns and the case features. See ichnos-ml Behaviour changes. |
| `pm4py.extract_features_dataframe` | `ml.py` → `algo/transformation/trace_encodings/algorithm`, `objects/log/obj` | `ichnos_ml::extract_features_dataframe`, `ichnos_ml::trace_features` | `ichnos-ml` | ported | Goldens `ml/features-*`: the data-frame branch on running-example (automatic attribute selection, occurrence counts, numeric statistics, custom aggregations, named attributes) and roadtraffic100traces; the `EventLog` branch with every extra feature on running-example and part of interval_event_log. Ports the trace_based variant; the other variants and polars are not ported. See ichnos-ml Behaviour changes. |
| `pm4py.extract_ocel_features` | `ml.py` → `algo/transformation/ocel/features/objects/algorithm`, `objects/ocel/obj` | `ichnos_ml::extract_ocel_features` | `ichnos-ml` | ported | Goldens `ml/ocel-*` on example_log (three object types; work in progress and object attributes; defaults; without lifecycle paths; an unknown type) and ocel20_example (two object types). Ports the features that `pm4py.extract_ocel_features` can turn on; related events, related activities and object graph features are not ported. See ichnos-ml Behaviour changes. |
| `pm4py.extract_temporal_features_dataframe` | `ml.py` → `algo/transformation/trace_encodings/variants/temporal`, `algo/transformation/trace_encodings/variants/temporal_lazy`, `objects/log/obj` | `ichnos_ml::extract_temporal_features_dataframe` | `ichnos-ml` | ported | Goldens `ml/temporal-*` on running-example with the frequencies W, D, 2D, 3h, MS, ME, YS and YE, and on receipt with W. Ports the temporal variant with frequencies of n hours, n days, W, MS, ME, YS and YE; other pandas frequencies and temporal_lazy are not ported. See ichnos-ml Behaviour changes. |
| `pm4py.extract_target_vector` | `ml.py` → `algo/transformation/log_to_target/algorithm`, `objects/log/obj` | `ichnos_ml::extract_target_vector` | `ichnos-ml` | ported | Goldens `ml/target-*` on running-example and roadtraffic100traces for next_activity, next_time and remaining_time. See ichnos-ml Behaviour changes. |

## org

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.discover_handover_of_work_network` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos_org::discover_handover_of_work_network` | `ichnos-org` | ported | Goldens `org/handover-*` on running-example, receipt and reviewing, and with `beta` 0.5 (running-example) and 1 (receipt). Ports the handover_log variant; handover_pandas is not ported. See ichnos-org Behaviour changes. |
| `pm4py.discover_working_together_network` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos_org::discover_working_together_network` | `ichnos-org` | ported | Goldens `org/working-together-*` on running-example, receipt and reviewing. Ports the working_together_log variant; working_together_pandas is not ported. See ichnos-org Behaviour changes. |
| `pm4py.discover_activity_based_resource_similarity` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos_org::discover_activity_based_resource_similarity` | `ichnos-org` | ported | Goldens `org/similarity-*` on running-example and reviewing; receipt is left out because its 2,256 pairs make a 250 KB file. Ports the jointactivities_log variant; jointactivities_pandas is not ported. See ichnos-org Behaviour changes. |
| `pm4py.discover_subcontracting_network` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos_org::discover_subcontracting_network` | `ichnos-org` | ported | Goldens `org/subcontracting-*` on running-example, receipt and reviewing, and with `n` 3 (running-example, receipt). Ports the subcontracting_log variant; subcontracting_pandas is not ported. See ichnos-org Behaviour changes. |
| `pm4py.discover_organizational_roles` | `org.py` → `algo/organizational_mining/roles/algorithm`, `objects/log/obj`, `objects/org/roles/obj` | `ichnos_org::discover_organizational_roles` | `ichnos-org` | ported | Goldens `org/roles-*` on running-example, receipt and reviewing check the roles in order. Ports the log variant; the pandas variant is not ported. See ichnos-org Behaviour changes. |
| `pm4py.discover_network_analysis` | `org.py` → `algo/organizational_mining/network_analysis/algorithm`, `algo/organizational_mining/network_analysis/variants/dataframe`, `objects/log/obj` | `ichnos_org::discover_network_analysis`, `ichnos_org::discover_network_analysis_performance` | `ichnos-org` | ported | Goldens `org/network-analysis-*` on running-example (counts; durations; edges named by the target event; events linked by resource, with activities as nodes and cases as edges) and receipt (counts). Ports the dataframe variant on an `EventLog`. See ichnos-org Behaviour changes. |

## ocel

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.ocel_get_object_types` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::get_object_types` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_get_attribute_names` | `ocel.py` → `objects/ocel/obj`, `objects/ocel/util/attributes_names` | `ichnos::ocel::get_attribute_names` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_flattening` | `ocel.py` → `objects/ocel/obj`, `objects/ocel/util/flattening` | `ichnos::ocel::flattening` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_object_type_activities` | `ocel.py` → `objects/ocel/obj`, `statistics/ocel/ot_activities` | `ichnos::ocel::object_type_activities` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_objects_ot_count` | `ocel.py` → `objects/ocel/obj`, `statistics/ocel/objects_ot_count` | `ichnos::ocel::objects_ot_count` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_temporal_summary` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::temporal_summary` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_objects_summary` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::objects_summary` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_objects_interactions_summary` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::objects_interactions_summary` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.discover_ocdfg` | `ocel.py` → `algo/discovery/ocel/ocdfg/algorithm`, `objects/ocel/constants`, `objects/ocel/obj` | `ichnos::ocel::discover_ocdfg` (planned) | `ichnos-ocel` | todo | Variants: classic. |
| `pm4py.discover_oc_petri_net` | `ocel.py` → `algo/discovery/ocel/ocpn/algorithm`, `objects/ocel/obj`, `objects/ocpn/obj` | `ichnos::ocel::discover_oc_petri_net` (planned) | `ichnos-ocel` | todo | Variants: classic, wo_annotation. |
| `pm4py.discover_objects_graph` | `ocel.py` → `algo/transformation/ocel/graphs/object_cobirth_graph`, `algo/transformation/ocel/graphs/object_codeath_graph`, `algo/transformation/ocel/graphs/object_descendants_graph`, `algo/transformation/ocel/graphs/object_inheritance_graph`, `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj` | `ichnos::ocel::discover_objects_graph` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_o2o_enrichment` | `ocel.py` → `algo/transformation/ocel/graphs/ocel20_computation`, `objects/ocel/obj` | `ichnos::ocel::o2o_enrichment` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_e2o_lifecycle_enrichment` | `ocel.py` → `objects/ocel/obj`, `objects/ocel/util/e2o_qualification` | `ichnos::ocel::e2o_lifecycle_enrichment` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.sample_ocel_objects` | `ocel.py` → `objects/ocel/obj`, `objects/ocel/util/sampling` | `ichnos::ocel::sample_ocel_objects` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.sample_ocel_connected_components` | `ocel.py` → `algo/transformation/ocel/split_ocel/algorithm`, `objects/ocel/obj` | `ichnos::ocel::sample_ocel_connected_components` (planned) | `ichnos-ocel` | todo | Variants: ancestors_descendants, connected_components. |
| `pm4py.ocel_drop_duplicates` | `ocel.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::ocel::drop_duplicates` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_merge_duplicates` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::merge_duplicates` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_sort_by_additional_column` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::sort_by_additional_column` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.ocel_add_index_based_timedelta` | `ocel.py` → `objects/ocel/obj` | `ichnos::ocel::add_index_based_timedelta` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |
| `pm4py.cluster_equivalent_ocel` | `ocel.py` → `algo/transformation/ocel/description/algorithm`, `algo/transformation/ocel/split_ocel/algorithm`, `objects/ocel/obj`, `objects/ocel/util/rename_objs_ot_tim_lex` | `ichnos::ocel::cluster_equivalent_ocel` (planned) | `ichnos-ocel` | todo | Variants: ancestors_descendants, connected_components, variant1, variant2. |
| `pm4py.ocel_drill_down` | `ocel.py` → `algo/transformation/ocel/olap/drill_down/algorithm`, `algo/transformation/ocel/olap/drill_down/variants/classic`, `objects/ocel/obj` | `ichnos::ocel::drill_down` (planned) | `ichnos-ocel` | todo | Variants: classic. |
| `pm4py.ocel_roll_up` | `ocel.py` → `algo/transformation/ocel/olap/roll_up/algorithm`, `algo/transformation/ocel/olap/roll_up/variants/classic`, `objects/ocel/obj` | `ichnos::ocel::roll_up` (planned) | `ichnos-ocel` | todo | Variants: classic. |
| `pm4py.ocel_unfold` | `ocel.py` → `algo/transformation/ocel/olap/unfold/algorithm`, `algo/transformation/ocel/olap/unfold/variants/classic`, `objects/ocel/obj` | `ichnos::ocel::unfold` (planned) | `ichnos-ocel` | todo | Variants: classic. |
| `pm4py.ocel_fold` | `ocel.py` → `algo/transformation/ocel/olap/fold/algorithm`, `algo/transformation/ocel/olap/fold/variants/classic`, `objects/ocel/obj` | `ichnos::ocel::fold` (planned) | `ichnos-ocel` | todo | Variants: classic. |

## privacy

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.privacy.anonymize_differential_privacy` | `privacy.py` → `algo/anonymization/pripel/algorithm`, `algo/anonymization/trace_variant_query/algorithm`, `objects/log/obj` | `ichnos::privacy::anonymize_differential_privacy` (planned) | `ichnos-privacy` | todo | Variants: laplace, pripel, sacofa. |

## convert

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.convert_to_event_log` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::EventStream::into_event_log`, `ichnos::EventLog::from_arrow` | `ichnos-core` | ported | Golden `core/convert-running-example-csv`. Changes core-5 to core-9. |
| `pm4py.convert_to_event_stream` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::EventLog::to_event_stream`, `ichnos::EventStream::from_arrow` | `ichnos-core` | ported | Golden `core/convert-running-example-csv`. Change core-11. |
| `pm4py.convert_to_dataframe` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::EventLog::to_arrow`, `ichnos::EventStream::to_arrow` | `ichnos-core` | ported | DataFrame is an Arrow `RecordBatch`. Golden `core/convert-running-example-csv` checks column names, types and the first rows' values. Changes core-10, core-16. |
| `pm4py.convert_to_bpmn` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/process_tree/variants/to_bpmn`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/conversion/wf_net/variants/to_bpmn`, `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos_model::ProcessTree::to_bpmn`, `ichnos_model::AcceptingPetriNet::to_bpmn` | `ichnos-model` | ported | One method per source type, as for the other conversions. Goldens `bpmn/tree-to-bpmn-*` (7 trees) and `bpmn/petri-to-bpmn-*` (9 nets). A `Bpmn` needs no conversion. Other sources go through a net, as in pm4py: their `to_petri_net`, then `AcceptingPetriNet::to_bpmn`. pm4py takes a POWL for a process tree here; see ichnos-model Behaviour changes. |
| `pm4py.convert_to_petri_net` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/genetic_matrix/obj`, `objects/heuristics_net/obj`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos_model::ProcessTree::to_petri_net`, `ichnos_model::Bpmn::to_petri_net`, `ichnos_model::Powl::to_petri_net`, `ichnos_model::HeuristicsNet::to_petri_net`, `ichnos_model::Dfg::to_petri_net` | `ichnos-model` | ported | One method per source type. Goldens: `wfnet/tree-*` (7 trees), `bpmn/bpmn-to-petri-*` (6 diagrams), `powl/model-*` and `powl/tree-*`, `model/heuristics-net-*` (4 logs) and `dfg/filters-*` (2 logs). pm4py names visible transitions of tree and POWL nets with random UUIDs, so the tests compare them by label and neighbouring places. A net needs no conversion. pm4py's `GeneticMatrix` source has no ichnos type yet; it belongs with `pm4py.discover_petri_net_genetic`. |
| `pm4py.convert_to_process_tree` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/powl/variants/to_process_tree`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/conversion/wf_net/variants/to_process_tree`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos_model::AcceptingPetriNet::to_process_tree`, `ichnos_model::Bpmn::to_process_tree`, `ichnos_model::Powl::to_process_tree` | `ichnos-model` | ported | Goldens `wfnet/*` (14 PNML fixtures, 7 trees converted to nets and back, and one net that is not a workflow net), `bpmn/bpmn-to-petri-*` (6 diagrams, through their nets) and `powl/model-*` and `powl/tree-*`. pm4py's two `ValueError`s map to `WfNetToTreeError::NotWorkflowNet` and `NotBlockStructured`. Differences: choice and parallel children are sorted by string form, not by a sum of MD5 hashes; transition pairs are tried in id order; labels keep their quotes; the workflow-net check works on node ids, where pm4py's works on names and merges a place and a transition that share a name. A tree needs no conversion. Variants: to_petri_net. |
| `pm4py.convert_to_powl` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/process_tree/variants/to_powl`, `objects/conversion/wf_net/variants/to_powl`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos_model::ProcessTree::to_powl`, `ichnos_model::PetriNet::to_powl`, `ichnos_model::Bpmn::to_powl` | `ichnos-model` | ported | `PetriNet::to_powl` ports pm4py's workflow-net translation (`objects/conversion/wf_net/variants/to_powl`). Goldens `powl/tree-*` (6 trees), `wfnet/*` (22 nets, among them one with inhibitor and reset arcs) and `bpmn/bpmn-to-petri-*` (6 diagrams). pm4py's errors map to `WfNetToPowlError`. pm4py builds choice and partial-order children in set order, so goldens and tests sort them. See ichnos-model Behaviour changes. |
| `pm4py.convert_to_reachability_graph` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/petri_net/obj`, `objects/petri_net/utils/reachability_graph`, `objects/process_tree/obj`, `objects/transition_system/obj` | `ichnos_model::PetriNet::to_transition_system` | `ichnos-model` | ported | Goldens `model/reachability-graph-net-*` (10 nets). Other sources go through a net, as in pm4py: their `to_petri_net` (see `pm4py.convert_to_petri_net`), then `to_transition_system` from the initial marking. See the method docs for inhibitor and reset arcs. Variants: to_petri_net. |
| `pm4py.convert_log_to_ocel` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/ocel/obj`, `objects/ocel/util/log_ocel` | `ichnos::ocel::convert_log_to_ocel` (planned) | `ichnos-ocel` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.convert_ocel_to_networkx` | `convert.py` → `objects/conversion/ocel/converter`, `objects/ocel/obj` | `ichnos::ocel::convert_ocel_to_networkx` (planned) | `ichnos-ocel` | todo | Variants: ocel_features_to_nx, ocel_to_nx. |
| `pm4py.convert_log_to_networkx` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::EventLog::to_graph` | `ichnos-core` | ported | petgraph `DiGraph`. Golden `core/networkx-running-example-csv`. Change core-27. |
| `pm4py.convert_log_to_time_intervals` | `convert.py` → `algo/transformation/log_to_interval_tree/variants/open_paths`, `objects/log/obj` | `ichnos::perf::convert_log_to_time_intervals` (planned) | `ichnos-perf` | todo | Single entry point; preserve source defaults. |
| `pm4py.convert_petri_net_to_networkx` | `convert.py` → `objects/petri_net/obj` | `ichnos_model::AcceptingPetriNet::to_graph` | `ichnos-model` | ported | petgraph `DiGraph` of `PetriGraphNode` and `PetriGraphArc`. Goldens `model/networkx-net-*` (12 nets, one with inhibitor and reset arcs). See ichnos-model Behaviour changes. |
| `pm4py.convert_petri_net_type` | `convert.py` → `objects/petri_net/obj`, `objects/petri_net/utils/petri_utils` | none | `ichnos-model` | dropped | ichnos has one `PetriNet` type, and each arc carries its kind (`ArcKind`). pm4py's four net classes differ only in class; `convert_petri_net_type` copies the net into another class and keeps each arc's type, so there is nothing to convert. |

## analysis

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.construct_synchronous_product_net` | `analysis.py` → `objects/log/obj`, `objects/petri_net/obj`, `objects/petri_net/utils/align_utils`, `objects/petri_net/utils/petri_utils`, `objects/petri_net/utils/synchronous_product` | `ichnos_model::analysis::SynchronousProduct::new` | `ichnos-model` | ported | Goldens `analysis/sync-*`: 6 running-example and 3 receipt variants against their nets. Product nodes are named with the strings `"(x, y)"`, not Python tuples; `SynchronousProduct::move_of` and `standard_cost` give each transition's move and pm4py's standard cost. See the ichnos-model (petri-analysis) Behaviour changes. |
| `pm4py.compute_emd` | `analysis.py` → `algo/evaluation/earth_mover_distance/algorithm` | `ichnos_stats::emd::compute_emd` | `ichnos-stats` | ported | Goldens `analysis_remaining/emd` and `emd-{running-example,receipt,roadtraffic100traces}` compare normalized trace edit distance and transport cost. Both real-log languages use every variant: 6, 116 and 10 respectively. The second language reweights those variants. Masses use NumPy isclose tolerances; a close positive second total is rescaled before solving. Uses microlp. |
| `pm4py.solve_marking_equation` | `analysis.py` → `algo/analysis/marking_equation/algorithm`, `objects/petri_net/obj` | `ichnos_model::AcceptingPetriNet::solve_marking_equation` | `ichnos-model` | ported | Goldens `analysis/net-*` (22 nets, unit costs). Linear program solved with `microlp`; each arc counts once, as in pm4py. Variants: classic. |
| `pm4py.solve_extended_marking_equation` | `analysis.py` → `algo/analysis/extended_marking_equation/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos_model::analysis::SynchronousProduct::solve_extended_marking_equation` | `ichnos-model` | ported | Goldens `analysis/sync-*`, default split points and `split_points=[1]`. One known difference: pm4py truncates GLPK's 3.9999999999963 to 3 on `sync-running-example-3`, where ichnos gives 4. Variants: classic. |
| `pm4py.analysis.check_is_sound` | `analysis.py` → `algo/analysis/woflan/algorithm`, `objects/petri_net/obj` | `ichnos_model::AcceptingPetriNet::is_sound` | `ichnos-model` | ported | Goldens `analysis/net-*` compare pm4py POWL-first results. Successful POWL conversion returns true even for Woflan-unsound nets, including `net-and-split-xor-join` and `net-murata3`. Use `AcceptingPetriNet::check_soundness` for the Woflan verdict. |
| `pm4py.check_soundness` | `analysis.py` → `algo/analysis/woflan/algorithm`, `objects/petri_net/obj` | `ichnos_model::AcceptingPetriNet::check_soundness` | `ichnos-model` | ported | Woflan with pm4py's early stop; returns `SoundnessReport` with pm4py's diagnostic messages. Goldens `analysis/net-*` (21 nets; roadtraffic skipped because pm4py takes minutes). On `big_wf_net` the uncovered places differ because the solvers pick different optimal invariants; the verdict matches. |
| `pm4py.cluster_log` | `analysis.py` → `algo/clustering/profiles/algorithm`, `objects/log/obj` | `ichnos_ml::profiles::{cluster_log, ProfileOptions, KMeans, Clusterer}` | `ichnos-ml` | ported | Goldens `analysis_remaining/clusters-{running-example,receipt,roadtraffic100traces}` compare complete-log activity profiles and explicit-center Lloyd partitions with pm4py. `profiles-numeric` covers categorical and numeric attributes. `clusters-default-running-example` records pm4py default groups; its Rust test reports the partition comparison. Rust defaults can differ because feature selection and center initialization differ. See Behaviour changes. |
| `pm4py.insert_artificial_start_end` | `analysis.py` → `objects/log/obj`, `objects/log/util/artificial`, `objects/log/util/dataframe_utils` | `ichnos::EventLog::insert_artificial_start_end` | `ichnos-core` | ported | Golden `core/artificial-start-end-running-example-csv`. Change core-26. |
| `pm4py.insert_case_service_waiting_time` | `analysis.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_perf::insert_case_service_waiting_time` | `ichnos-perf` | ported | Goldens `analysis_remaining/times-{running-example,receipt,roadtraffic100traces}` and `times-intervals` cover service, sojourn and waiting values. The interval case includes overlap and negative durations. Returns an enriched EventLog copy. |
| `pm4py.insert_case_arrival_finish_rate` | `analysis.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_perf::insert_case_arrival_finish_rate` | `ichnos-perf` | ported | Goldens `analysis_remaining/times-{running-example,receipt,roadtraffic100traces}` and `times-intervals` cover arrival and finish gaps. Ties use typed case identifiers. Uses the preceding finish, following pm4py implementation. |
| `pm4py.check_is_workflow_net` | `analysis.py` → `algo/analysis/workflow_net/algorithm`, `objects/petri_net/obj` | `ichnos_model::PetriNet::is_workflow_net` | `ichnos-model` | ported | Goldens `analysis/net-*` (22 nets). Variants: petri_net. |
| `pm4py.maximal_decomposition` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/utils/decomposition` | `ichnos_model::AcceptingPetriNet::maximal_decomposition` | `ichnos-model` | ported | Goldens `analysis/net-*` (22 nets). For a duplicated label pm4py keeps the joining transition by memory-address order, which changes from run to run; ichnos keeps the one whose name sorts last, and `maximal_decomposition_with` takes the pick as a function. The goldens iterate pm4py's transitions in name order, so its pick is the same. |
| `pm4py.simplicity_petri_net` | `analysis.py` → `algo/evaluation/simplicity/variants/arc_degree`, `algo/evaluation/simplicity/variants/extended_cardoso`, `algo/evaluation/simplicity/variants/extended_cyclomatic`, `objects/petri_net/obj` | `ichnos_model::AcceptingPetriNet::simplicity` | `ichnos-model` | ported | Goldens `analysis/net-*`: arc_degree and extended_cardoso on 22 nets, extended_cyclomatic on the 16 bounded ones. extended_cyclomatic follows pm4py's graph, which counts (state, transition name) pairs. Variants: arc_degree, extended_cardoso, extended_cyclomatic. |
| `pm4py.generate_marking` | `analysis.py` → `objects/petri_net/obj` | `ichnos_model::PetriNet::marking_from_names` | `ichnos-model` | ported | Golden `analysis/generate-marking`. Takes (place name, tokens) pairs; an unknown name is an error, where pm4py raises `KeyError`. |
| `pm4py.reduce_petri_net_invisibles` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/utils/reduction` | `ichnos_model::PetriNet::apply_simple_reduction` | `ichnos-model` | ported | Goldens `analysis/net-*` (22 nets). |
| `pm4py.reduce_petri_net_implicit_places` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/utils/murata` | `ichnos_model::AcceptingPetriNet::reduce_implicit_places` | `ichnos-model` | ported | Goldens `analysis/net-*` (22 nets). Integer programs solved with `microlp`; pm4py uses scipy or PuLP. |
| `pm4py.get_enabled_transitions` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/semantics` | `ichnos_model::PetriNet::enabled_transitions` | `ichnos-model` | ported | Goldens `analysis/net-*` (22 nets, initial marking). |
| `pm4py.get_activity_labels` | `analysis.py` → `objects/log/obj` | `ichnos_model::comparison::Model::activity_labels; ichnos_ml::profiles::activity_labels` | `ichnos-model / ichnos-ml` | ported | Goldens `analysis_remaining/models` and `times-{running-example,receipt,roadtraffic100traces}` cover models and event logs. Returns sorted distinct visible labels after model conversion. |
| `pm4py.replace_activity_labels` | `analysis.py` → `objects/bpmn/obj`, `objects/bpmn/util/label_replacing`, `objects/petri_net/obj`, `objects/petri_net/utils/label_replacing`, `objects/powl/obj`, `objects/powl/utils/label_replacing`, `objects/process_tree/obj`, `objects/process_tree/utils/label_replacing` | `ichnos_model::comparison::Model::replace_activity_labels` | `ichnos-model` | ported | Golden `analysis_remaining/models` covers tree, accepting net, POWL and BPMN relabeling. Returns an owned copy. Direct DFG relabeling returns a typed error. |
| `pm4py.behavioral_similarity` | `analysis.py` → `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos_conformance::footprints::behavioral_similarity; ichnos_model::comparison::behavioral_similarity` | `ichnos-conformance / ichnos-model` | ported | Golden `analysis_remaining/models` compares trees, accepting nets, POWL and BPMN, including BPMN pairs. Sequence and parallel relations have separate Jaccard terms. An empty denominator gives zero. DFG inputs are rejected, as in pm4py. |
| `pm4py.structural_similarity` | `analysis.py` → `objects/process_tree/utils/struct_similarity` | `ichnos_model::comparison::{structural_similarity, structural_features}` | `ichnos-model` | ported | Golden `analysis_remaining/models` compares ten structural features for trees, accepting nets and POWL. Preserves pm4py comparison-specific POWL conversion behavior. See Behaviour changes. |
| `pm4py.embeddings_similarity` | `analysis.py` → `objects/petri_net/utils/embeddings_similarity` | — | `ichnos-ml` | dropped | External Gensim Word2Vec training adapter: independently trained random-walk vectors depend on Python/Gensim internals and net set iteration. No built-in Rust training backend is provided; cosine of arbitrary vectors would not implement this function. |
| `pm4py.label_sets_similarity` | `analysis.py` | `ichnos_model::comparison::{label_sets_similarity, Model::label_sets_similarity}` | `ichnos-model` | ported | Goldens `analysis_remaining/labels` and `models` compare greedy Unicode label matching. Thresholds are typed and validated. See Behaviour changes for lexical ties and zero-score candidates. |
| `pm4py.map_labels_from_second_model` | `analysis.py` → `objects/bpmn/obj`, `objects/bpmn/util/label_replacing`, `objects/petri_net/utils/label_replacing`, `objects/powl/obj`, `objects/powl/utils/label_replacing`, `objects/process_tree/utils/label_replacing` | `ichnos_model::comparison::Model::map_labels_from_second_model` | `ichnos-model` | ported | Goldens `analysis_remaining/labels` and `models` compare one-to-one label mapping and renamed models. Matching uses pm4py SequenceMatcher ratios with lexical tie handling. |

## hof

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.hof.filter_log` | `hof.py` → `objects/log/obj` | `ichnos::EventLog::filter_traces`, `ichnos::EventStream::filter_events` | `ichnos-core` | ported | Moved from `ichnos-stats` to the core lane. Goldens `core/hof-running-example-csv` and `core/hof-stream-running-example-csv`. Change core-24. |
| `pm4py.hof.filter_trace` | `hof.py` → `objects/log/obj` | `ichnos::Trace::filter_events` | `ichnos-core` | ported | Moved from `ichnos-stats` to the core lane. Golden `core/hof-running-example-csv`. Change core-24. |
| `pm4py.hof.sort_log` | `hof.py` → `objects/log/obj` | `ichnos::EventLog::sort_traces_by_key`, `ichnos::EventStream::sort_events_by_key` | `ichnos-core` | ported | Goldens `core/hof-running-example-csv` and `core/hof-stream-running-example-csv`. Change core-24. |
| `pm4py.hof.sort_trace` | `hof.py` → `objects/log/obj` | `ichnos::Trace::sort_events_by_key` | `ichnos-core` | ported | Golden `core/hof-running-example-csv`. Changes core-24, core-25. |

## llm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.llm.openai_query` | `llm.py` → `algo/querying/llm/connectors/openai` | `ichnos::ml::openai_query` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.google_query` | `llm.py` → `algo/querying/llm/connectors/google` | `ichnos::ml::google_query` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.anthropic_query` | `llm.py` → `algo/querying/llm/connectors/anthropic` | `ichnos::ml::anthropic_query` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_dfg` | `llm.py` → `algo/querying/llm/abstractions/log_to_dfg_descr`, `objects/log/obj` | `ichnos::ml::abstract_dfg` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_variants` | `llm.py` → `algo/querying/llm/abstractions/log_to_variants_descr`, `objects/log/obj` | `ichnos::ml::abstract_variants` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_ocel` | `llm.py` → `algo/transformation/ocel/description/algorithm`, `objects/ocel/obj` | `ichnos::ml::abstract_ocel` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_ocel_ocdfg` | `llm.py` → `algo/querying/llm/abstractions/ocel_ocdfg_descr`, `objects/ocel/obj` | `ichnos::ml::abstract_ocel_ocdfg` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_ocel_features` | `llm.py` → `algo/querying/llm/abstractions/ocel_fea_descr`, `objects/ocel/obj` | `ichnos::ml::abstract_ocel_features` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_event_stream` | `llm.py` → `algo/querying/llm/abstractions/stream_to_descr`, `objects/log/obj` | `ichnos::ml::abstract_event_stream` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_petri_net` | `llm.py` → `algo/querying/llm/abstractions/net_to_descr`, `objects/petri_net/obj` | `ichnos::ml::abstract_petri_net` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_log_attributes` | `llm.py` → `algo/querying/llm/abstractions/log_to_cols_descr`, `objects/log/obj` | `ichnos::ml::abstract_log_attributes` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_log_features` | `llm.py` → `algo/querying/llm/abstractions/log_to_fea_descr`, `objects/log/obj` | `ichnos::ml::abstract_log_features` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_temporal_profile` | `llm.py` → `algo/querying/llm/abstractions/tempprofile_to_descr` | `ichnos::ml::abstract_temporal_profile` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_case` | `llm.py` → `algo/querying/llm/abstractions/case_to_descr`, `objects/log/obj` | `ichnos::ml::abstract_case` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_declare` | `llm.py` → `algo/querying/llm/abstractions/declare_to_descr` | `ichnos::ml::abstract_declare` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.abstract_log_skeleton` | `llm.py` → `algo/querying/llm/abstractions/logske_to_descr` | `ichnos::ml::abstract_log_skeleton` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.clustering` | `llm.py` → `algo/querying/llm/connectors/openai` | `ichnos::ml::clustering` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.nlp_to_log_query` | `llm.py` → `algo/querying/llm/connectors/openai`, `algo/querying/llm/injection/algorithm` | `ichnos::ml::nlp_to_log_query` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.nlp_to_log_filter` | `llm.py` → `algo/querying/llm/connectors/openai`, `algo/querying/llm/injection/algorithm` | `ichnos::ml::nlp_to_log_filter` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.automated_hypotheses_formulation` | `llm.py` → `algo/querying/llm/abstractions/log_to_cols_descr`, `algo/querying/llm/abstractions/log_to_dfg_descr`, `algo/querying/llm/connectors/openai`, `algo/querying/llm/injection/algorithm` | `ichnos::ml::automated_hypotheses_formulation` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |
| `pm4py.llm.explain_visualization` | `llm.py` → `algo/querying/llm/connectors/openai` | `ichnos::ml::explain_visualization` (planned) | `ichnos-ml` | dropped | LLM prompts and provider integrations are outside the Rust library scope. |

## connectors

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.connectors.extract_log_outlook_mails` | `connectors.py` → `algo/connectors/variants/outlook_mail_extractor` | `ichnos::io::extract_log_outlook_mails` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_log_outlook_calendar` | `connectors.py` → `algo/connectors/variants/outlook_calendar` | `ichnos::io::extract_log_outlook_calendar` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_log_windows_events` | `connectors.py` → `algo/connectors/variants/windows_events` | `ichnos::io::extract_log_windows_events` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_log_chrome_history` | `connectors.py` → `algo/connectors/variants/chrome_history` | `ichnos::io::extract_log_chrome_history` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_log_firefox_history` | `connectors.py` → `algo/connectors/variants/firefox_history` | `ichnos::io::extract_log_firefox_history` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_log_github` | `connectors.py` → `algo/connectors/variants/github_repo` | `ichnos::io::extract_log_github` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_log_camunda_workflow` | `connectors.py` → `algo/connectors/variants/camunda_workflow` | `ichnos::io::extract_log_camunda_workflow` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_log_sap_o2c` | `connectors.py` → `algo/connectors/variants/sap_o2c` | `ichnos::io::extract_log_sap_o2c` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_log_sap_accounting` | `connectors.py` → `algo/connectors/variants/sap_accounting` | `ichnos::io::extract_log_sap_accounting` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_ocel_outlook_mails` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_outlook_mails` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_ocel_outlook_calendar` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_outlook_calendar` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_ocel_windows_events` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_windows_events` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_ocel_chrome_history` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_chrome_history` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_ocel_firefox_history` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_firefox_history` (planned) | `ichnos-io` | dropped | Desktop application/OS profile extraction is outside the portable Rust library scope. |
| `pm4py.connectors.extract_ocel_github` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_github` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_ocel_camunda_workflow` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_camunda_workflow` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_ocel_sap_o2c` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_sap_o2c` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |
| `pm4py.connectors.extract_ocel_sap_accounting` | `connectors.py` → `objects/ocel/obj` | `ichnos::io::extract_ocel_sap_accounting` (planned) | `ichnos-io` | todo | Single entry point; preserve source defaults. |

## cli

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.cli.cli_interface` | `cli.py` | `ichnos::core::cli_interface` (planned) | `ichnos-core` | dropped | Python argv/file-batch command wrapper; underlying library operations have their own rows. |

## statistics.attributes.common.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.common.get.get_sorted_attributes_list` | `statistics/attributes/common/get.py` | `ichnos_stats::attributes::get_sorted_attributes_list` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.common.get.get_attributes_threshold` | `statistics/attributes/common/get.py` | `ichnos_stats::attributes::get_attributes_threshold` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute` | `statistics/attributes/common/get.py` | `ichnos_stats::attributes::get_kde_numeric_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance. |
| `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute_json` | `statistics/attributes/common/get.py` | `ichnos_stats::attributes::get_kde_numeric_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance. |
| `pm4py.statistics.attributes.common.get.get_kde_date_attribute` | `statistics/attributes/common/get.py` | `ichnos_stats::attributes::get_kde_date_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance.  Date KDE interprets wall-clock values as UTC; pm4py uses the process time zone. The oracle generator pins TZ=UTC. |
| `pm4py.statistics.attributes.common.get.get_kde_date_attribute_json` | `statistics/attributes/common/get.py` | `ichnos_stats::attributes::get_kde_date_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance.  Date KDE interprets wall-clock values as UTC; pm4py uses the process time zone. The oracle generator pins TZ=UTC. |

## statistics.attributes.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.log.get.get_events_distribution` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_events_distribution` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.get.get_all_trace_attributes_from_log` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_all_trace_attributes_from_log` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.get.get_all_event_attributes_from_log` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_all_event_attributes_from_log` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.get.get_attribute_values` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_attribute_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.get.get_trace_attribute_values` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_trace_attribute_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos_stats::attributes::get_kde_numeric_attribute` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance. |
| `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute_json` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos_stats::attributes::get_kde_numeric_attribute` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance. |
| `pm4py.statistics.attributes.log.get.get_kde_date_attribute` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos_stats::attributes::get_kde_date_attribute` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance.  Date KDE interprets wall-clock values as UTC; pm4py uses the process time zone. The oracle generator pins TZ=UTC. |
| `pm4py.statistics.attributes.log.get.get_kde_date_attribute_json` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos_stats::attributes::get_kde_date_attribute` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed Density replaces JSON wrappers; relative 1e-6 / absolute 1e-12 tolerance.  Date KDE interprets wall-clock values as UTC; pm4py uses the process time zone. The oracle generator pins TZ=UTC. |

## statistics.attributes.log.select

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.log.select.select_attributes_from_log_for_tree` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/sampling`, `statistics/attributes/log/get` | `ichnos_stats::attributes::select_attributes_from_log_for_tree` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Seeded sampling; mixed-type attributes require uniform numeric or string types. |
| `pm4py.statistics.attributes.log.select.check_trace_attributes_presence` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::check_trace_attributes_presence` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.select.check_event_attributes_presence` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::check_event_attributes_presence` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.select.verify_if_event_attribute_is_in_each_trace` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::verify_if_event_attribute_is_in_each_trace` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.attributes.log.select.verify_if_trace_attribute_is_in_each_trace` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::verify_if_trace_attribute_is_in_each_trace` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.attributes.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.pandas.get.get_events_distribution` | `statistics/attributes/pandas/get.py` | `ichnos::stats::attributes::pandas::get::get_events_distribution` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_events_distribution; ichnos has one implementation |
| `pm4py.statistics.attributes.pandas.get.get_attribute_values` | `statistics/attributes/pandas/get.py` | `ichnos::stats::attributes::pandas::get::get_attribute_values` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_attribute_values; ichnos has one implementation |
| `pm4py.statistics.attributes.pandas.get.get_kde_numeric_attribute` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_numeric_attribute` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_numeric_attribute; ichnos has one implementation |
| `pm4py.statistics.attributes.pandas.get.get_kde_numeric_attribute_json` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_numeric_attribute_json` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_numeric_attribute_json; ichnos has one implementation |
| `pm4py.statistics.attributes.pandas.get.get_kde_date_attribute` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_date_attribute` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_date_attribute; ichnos has one implementation |
| `pm4py.statistics.attributes.pandas.get.get_kde_date_attribute_json` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_date_attribute_json` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_date_attribute_json; ichnos has one implementation |

## statistics.attributes.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.polars.get.get_events_distribution` | `statistics/attributes/polars/get.py` | `ichnos::stats::attributes::polars::get::get_events_distribution` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_events_distribution; ichnos has one implementation |
| `pm4py.statistics.attributes.polars.get.get_attribute_values` | `statistics/attributes/polars/get.py` | `ichnos::stats::attributes::polars::get::get_attribute_values` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_attribute_values; ichnos has one implementation |
| `pm4py.statistics.attributes.polars.get.get_kde_numeric_attribute` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_numeric_attribute` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_numeric_attribute; ichnos has one implementation |
| `pm4py.statistics.attributes.polars.get.get_kde_numeric_attribute_json` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_numeric_attribute_json` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_numeric_attribute_json; ichnos has one implementation |
| `pm4py.statistics.attributes.polars.get.get_kde_date_attribute` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_date_attribute` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_date_attribute; ichnos has one implementation |
| `pm4py.statistics.attributes.polars.get.get_kde_date_attribute_json` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_date_attribute_json` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.attributes.log.get.get_kde_date_attribute_json; ichnos has one implementation |

## statistics.chaotic_activities.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.chaotic_activities.algorithm.apply` | `statistics/chaotic_activities/algorithm.py` → `objects/log/obj` | `ichnos_stats::variants::get_chaotic_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.chaotic_activities.variants.niek_sidorova

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.chaotic_activities.variants.niek_sidorova.apply` | `statistics/chaotic_activities/variants/niek_sidorova.py` → `objects/log/obj` | `ichnos_stats::variants::get_chaotic_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.chaotic_activities.variants.niek_sidorova.chaotic_metrics` | `statistics/chaotic_activities/variants/niek_sidorova.py` | `ichnos_stats::variants::chaotic_metrics` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.chaotic_activities.variants.niek_sidorova.total_entropy` | `statistics/chaotic_activities/variants/niek_sidorova.py` | `ichnos_stats::variants::total_entropy` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.concurrent_activities.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.concurrent_activities.log.get.apply` | `statistics/concurrent_activities/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/sorting` | `ichnos_stats::time::get_concurrent_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |

## statistics.concurrent_activities.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.concurrent_activities.pandas.get.apply` | `statistics/concurrent_activities/pandas/get.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::concurrent_activities::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.concurrent_activities.log.get.apply; ichnos has one implementation |

## statistics.concurrent_activities.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.concurrent_activities.polars.get.get_concurrent_events_dataframe` | `statistics/concurrent_activities/polars/get.py` | `ichnos_stats::time::get_concurrent_events` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point.  Event-level trace/source/target/duration rows are compared. Concurrency uses elapsed overlap; partial-order flow times honour business slots. Canonical elapsed durations retain subseconds, whereas Polars duration columns truncate to whole seconds. Concurrency rejects negative subsecond gaps that Polars admits as zero after truncation; direct oracle rows include precise endpoint differences to verify this boundary. |
| `pm4py.statistics.concurrent_activities.polars.get.apply` | `statistics/concurrent_activities/polars/get.py` | `ichnos::stats::concurrent_activities::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.concurrent_activities.log.get.apply; ichnos has one implementation |

## statistics.end_activities.common.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.common.get.get_sorted_end_activities_list` | `statistics/end_activities/common/get.py` | `ichnos_stats::attributes::get_sorted_end_activities_list` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.end_activities.common.get.get_end_activities_threshold` | `statistics/end_activities/common/get.py` | `ichnos_stats::attributes::get_end_activities_threshold` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.end_activities.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.log.get.get_end_activities` | `statistics/end_activities/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_end_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.end_activities.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.pandas.get.get_end_activities` | `statistics/end_activities/pandas/get.py` | `ichnos::stats::end_activities::pandas::get::get_end_activities` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.end_activities.log.get.get_end_activities; ichnos has one implementation |

## statistics.end_activities.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.polars.get.get_end_activities` | `statistics/end_activities/polars/get.py` | `ichnos::stats::end_activities::polars::get::get_end_activities` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.end_activities.log.get.get_end_activities; ichnos has one implementation |

## statistics.eventually_follows.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.log.get.apply` | `statistics/eventually_follows/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/sorting` | `ichnos_stats::time::get_eventually_follows` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |

## statistics.eventually_follows.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.pandas.get.apply` | `statistics/eventually_follows/pandas/get.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::eventually_follows::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.eventually_follows.log.get.apply; ichnos has one implementation |

## statistics.eventually_follows.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.polars.get.get_partial_order_dataframe` | `statistics/eventually_follows/polars/get.py` | `ichnos_stats::time::get_partial_order` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point.  Event-level trace/source/target/duration rows are compared. Concurrency uses elapsed overlap; partial-order flow times honour business slots. Canonical elapsed durations retain subseconds, whereas Polars duration columns truncate to whole seconds. Concurrency rejects negative subsecond gaps that Polars admits as zero after truncation; direct oracle rows include precise endpoint differences to verify this boundary. |
| `pm4py.statistics.eventually_follows.polars.get.apply` | `statistics/eventually_follows/polars/get.py` | `ichnos::stats::eventually_follows::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.eventually_follows.log.get.apply; ichnos has one implementation |

## statistics.eventually_follows.uvcl.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.uvcl.get.apply` | `statistics/eventually_follows/uvcl/get.py` → `algo/discovery/inductive/dtypes/im_ds` | `ichnos_stats::time::get_eventually_follows_sequences` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |

## statistics.ocel.act_ot_dependent

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.ocel.act_ot_dependent.aggregate_events` | `statistics/ocel/act_ot_dependent.py` | `ichnos::stats::ocel::act_ot_dependent::aggregate_events` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_ot_dependent.aggregate_unique_objects` | `statistics/ocel/act_ot_dependent.py` | `ichnos::stats::ocel::act_ot_dependent::aggregate_unique_objects` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_ot_dependent.aggregate_total_objects` | `statistics/ocel/act_ot_dependent.py` | `ichnos::stats::ocel::act_ot_dependent::aggregate_total_objects` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_ot_dependent.find_associations_from_ocel` | `statistics/ocel/act_ot_dependent.py` → `objects/ocel/obj`, `statistics/ocel/act_utils` | `ichnos::stats::ocel::act_ot_dependent::find_associations_from_ocel` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.ocel.act_utils

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.ocel.act_utils.aggregate_events` | `statistics/ocel/act_utils.py` | `ichnos::stats::ocel::act_utils::aggregate_events` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_utils.aggregate_unique_objects` | `statistics/ocel/act_utils.py` | `ichnos::stats::ocel::act_utils::aggregate_unique_objects` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_utils.aggregate_total_objects` | `statistics/ocel/act_utils.py` | `ichnos::stats::ocel::act_utils::aggregate_total_objects` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_utils.find_associations_from_relations_df` | `statistics/ocel/act_utils.py` → `objects/ocel/constants` | `ichnos::stats::ocel::act_utils::find_associations_from_relations_df` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.act_utils.find_associations_from_ocel` | `statistics/ocel/act_utils.py` → `objects/ocel/obj` | `ichnos::stats::ocel::act_utils::find_associations_from_ocel` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.ocel.edge_metrics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.ocel.edge_metrics.performance_calculation_ocel_aggregation` | `statistics/ocel/edge_metrics.py` → `objects/ocel/obj` | `ichnos::stats::ocel::edge_metrics::performance_calculation_ocel_aggregation` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.edge_metrics.aggregate_ev_couples` | `statistics/ocel/edge_metrics.py` | `ichnos::stats::ocel::edge_metrics::aggregate_ev_couples` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.edge_metrics.aggregate_unique_objects` | `statistics/ocel/edge_metrics.py` | `ichnos::stats::ocel::edge_metrics::aggregate_unique_objects` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.edge_metrics.aggregate_total_objects` | `statistics/ocel/edge_metrics.py` | `ichnos::stats::ocel::edge_metrics::aggregate_total_objects` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.ocel.edge_metrics.find_associations_per_edge` | `statistics/ocel/edge_metrics.py` → `objects/ocel/obj` | `ichnos::stats::ocel::edge_metrics::find_associations_per_edge` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.ocel.objects_ot_count

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.ocel.objects_ot_count.get_objects_ot_count` | `statistics/ocel/objects_ot_count.py` → `objects/ocel/obj` | `ichnos::stats::ocel::objects_ot_count::get_objects_ot_count` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.ocel.ot_activities

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.ocel.ot_activities.get_object_type_activities` | `statistics/ocel/ot_activities.py` → `objects/ocel/obj` | `ichnos::stats::ocel::ot_activities::get_object_type_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.overlap.cases.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.cases.log.get.apply` | `statistics/overlap/cases/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/overlap/utils/compute` | `ichnos_stats::time::get_case_overlap` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Includes self and duplicate intervals, expands by epsilon; empty traces contribute zero. |

## statistics.overlap.cases.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.cases.pandas.get.apply` | `statistics/overlap/cases/pandas/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::cases::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.overlap.cases.log.get.apply; ichnos has one implementation |

## statistics.overlap.cases.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.cases.polars.get.apply` | `statistics/overlap/cases/polars/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::cases::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.overlap.cases.log.get.apply; ichnos has one implementation |

## statistics.overlap.interval_events.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.interval_events.log.get.apply` | `statistics/overlap/interval_events/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/overlap/utils/compute` | `ichnos_stats::time::get_interval_event_overlap` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Includes self and duplicate intervals, expands by epsilon; empty traces contribute zero. |

## statistics.overlap.interval_events.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.interval_events.pandas.get.apply` | `statistics/overlap/interval_events/pandas/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::interval_events::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.overlap.interval_events.log.get.apply; ichnos has one implementation |

## statistics.overlap.interval_events.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.interval_events.polars.get.apply` | `statistics/overlap/interval_events/polars/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::interval_events::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.overlap.interval_events.log.get.apply; ichnos has one implementation |

## statistics.overlap.utils.compute

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.utils.compute.apply` | `statistics/overlap/utils/compute.py` | `ichnos_stats::time::get_overlap` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Includes self and duplicate intervals, expands by epsilon; empty traces contribute zero. |

## statistics.passed_time.log.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.algorithm.apply` | `statistics/passed_time/log/algorithm.py` → `objects/log/obj` | `ichnos_stats::time::get_passed_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed pre/post neighbor lists and weighted averages; default returns both directions.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point. |

## statistics.passed_time.log.variants.post

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.variants.post.apply` | `statistics/passed_time/log/variants/post.py` → `algo/discovery/dfg/variants/native`, `algo/discovery/dfg/variants/performance`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::time::get_passed_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed pre/post neighbor lists and weighted averages; default returns both directions. |

## statistics.passed_time.log.variants.pre

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.variants.pre.apply` | `statistics/passed_time/log/variants/pre.py` → `algo/discovery/dfg/variants/native`, `algo/discovery/dfg/variants/performance`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::time::get_passed_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed pre/post neighbor lists and weighted averages; default returns both directions. |

## statistics.passed_time.log.variants.prepost

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.variants.prepost.apply` | `statistics/passed_time/log/variants/prepost.py` → `algo/discovery/dfg/variants/native`, `algo/discovery/dfg/variants/performance`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::time::get_passed_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed pre/post neighbor lists and weighted averages; default returns both directions. |

## statistics.passed_time.pandas.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.algorithm.apply` | `statistics/passed_time/pandas/algorithm.py` | `ichnos::stats::passed_time::pandas::algorithm::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.algorithm.apply; ichnos has one implementation |

## statistics.passed_time.pandas.variants.post

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.variants.post.apply` | `statistics/passed_time/pandas/variants/post.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::passed_time::pandas::variants::post::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.variants.post.apply; ichnos has one implementation |

## statistics.passed_time.pandas.variants.pre

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.variants.pre.apply` | `statistics/passed_time/pandas/variants/pre.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::passed_time::pandas::variants::pre::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.variants.pre.apply; ichnos has one implementation |

## statistics.passed_time.pandas.variants.prepost

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.variants.prepost.apply` | `statistics/passed_time/pandas/variants/prepost.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::passed_time::pandas::variants::prepost::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.variants.prepost.apply; ichnos has one implementation |

## statistics.passed_time.polars.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.algorithm.apply` | `statistics/passed_time/polars/algorithm.py` | `ichnos::stats::passed_time::polars::algorithm::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.algorithm.apply; ichnos has one implementation |

## statistics.passed_time.polars.variants.post

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.variants.post.apply` | `statistics/passed_time/polars/variants/post.py` | `ichnos::stats::passed_time::polars::variants::post::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.variants.post.apply; ichnos has one implementation |

## statistics.passed_time.polars.variants.pre

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.variants.pre.apply` | `statistics/passed_time/polars/variants/pre.py` | `ichnos::stats::passed_time::polars::variants::pre::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.variants.pre.apply; ichnos has one implementation |

## statistics.passed_time.polars.variants.prepost

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.variants.prepost.apply` | `statistics/passed_time/polars/variants/prepost.py` → `statistics/passed_time/polars/variants/post`, `statistics/passed_time/polars/variants/pre` | `ichnos::stats::passed_time::polars::variants::prepost::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.passed_time.log.variants.prepost.apply; ichnos has one implementation |

## statistics.process_cube.pandas.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.pandas.algorithm.apply` | `statistics/process_cube/pandas/algorithm.py` | `ichnos::stats::process_cube::pandas::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. No log/common counterpart exists; retain this operation as todo. Rust uses one implementation across dataframe backends. |

## statistics.process_cube.pandas.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.pandas.variants.classic.apply` | `statistics/process_cube/pandas/variants/classic.py` | `ichnos::stats::process_cube::pandas::variants::classic::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. No log/common counterpart exists; retain this operation as todo. Rust uses one implementation across dataframe backends. |

## statistics.process_cube.polars.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.polars.algorithm.apply` | `statistics/process_cube/polars/algorithm.py` | `ichnos::stats::process_cube::polars::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. No log/common counterpart exists; retain this operation as todo. Rust uses one implementation across dataframe backends. |

## statistics.process_cube.polars.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.polars.variants.classic.apply` | `statistics/process_cube/polars/variants/classic.py` | `ichnos::stats::process_cube::polars::variants::classic::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. No log/common counterpart exists; retain this operation as todo. Rust uses one implementation across dataframe backends. |

## statistics.rework.cases.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.cases.log.get.apply` | `statistics/rework/cases/log/get.py` → `objects/log/obj` | `ichnos_stats::variants::get_rework_cases` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.rework.cases.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.cases.pandas.get.apply` | `statistics/rework/cases/pandas/get.py` | `ichnos::stats::rework::cases::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.rework.cases.log.get.apply; ichnos has one implementation |

## statistics.rework.cases.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.cases.polars.get.apply` | `statistics/rework/cases/polars/get.py` | `ichnos::stats::rework::cases::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.rework.cases.log.get.apply; ichnos has one implementation |

## statistics.rework.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.log.get.apply` | `statistics/rework/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::variants::get_rework` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.rework.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.pandas.get.apply` | `statistics/rework/pandas/get.py` | `ichnos::stats::rework::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.rework.log.get.apply; ichnos has one implementation |

## statistics.rework.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.polars.get.apply` | `statistics/rework/polars/get.py` | `ichnos::stats::rework::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.rework.log.get.apply; ichnos has one implementation |

## statistics.service_time.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.service_time.log.get.apply` | `statistics/service_time/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::time::get_service_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |

## statistics.service_time.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.service_time.pandas.get.apply` | `statistics/service_time/pandas/get.py` | `ichnos::stats::service_time::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.service_time.log.get.apply; ichnos has one implementation |

## statistics.service_time.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.service_time.polars.get.apply` | `statistics/service_time/polars/get.py` | `ichnos::stats::service_time::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.service_time.log.get.apply; ichnos has one implementation |

## statistics.start_activities.common.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.common.get.get_sorted_start_activities_list` | `statistics/start_activities/common/get.py` | `ichnos_stats::attributes::get_sorted_start_activities_list` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.start_activities.common.get.get_start_activities_threshold` | `statistics/start_activities/common/get.py` | `ichnos_stats::attributes::get_start_activities_threshold` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.start_activities.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.log.get.get_start_activities` | `statistics/start_activities/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::attributes::get_start_activities` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.start_activities.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.pandas.get.get_start_activities` | `statistics/start_activities/pandas/get.py` | `ichnos::stats::start_activities::pandas::get::get_start_activities` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.start_activities.log.get.get_start_activities; ichnos has one implementation |

## statistics.start_activities.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.polars.get.get_start_activities` | `statistics/start_activities/polars/get.py` | `ichnos::stats::start_activities::polars::get::get_start_activities` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.start_activities.log.get.get_start_activities; ichnos has one implementation |

## statistics.traces.cycle_time.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.log.get.apply` | `statistics/traces/cycle_time/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/traces/cycle_time/util/compute` | `ichnos_stats::time::get_cycle_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Preserves reference omission of the final merged interval; empty inputs return zero. |

## statistics.traces.cycle_time.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.pandas.get.apply` | `statistics/traces/cycle_time/pandas/get.py` → `statistics/traces/cycle_time/util/compute` | `ichnos::stats::traces::cycle_time::pandas::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.cycle_time.log.get.apply; ichnos has one implementation |

## statistics.traces.cycle_time.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.polars.get.apply` | `statistics/traces/cycle_time/polars/get.py` → `statistics/traces/cycle_time/util/compute` | `ichnos::stats::traces::cycle_time::polars::get::apply` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.cycle_time.log.get.apply; ichnos has one implementation |

## statistics.traces.cycle_time.util.compute

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.util.compute.cycle_time` | `statistics/traces/cycle_time/util/compute.py` | `ichnos_stats::time::cycle_time` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Preserves reference omission of the final merged interval; empty inputs return zero. |

## statistics.traces.generic.common.case_duration

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration` | `statistics/traces/generic/common/case_duration.py` | `ichnos_stats::cases::get_kde_case_duration_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed Density replaces JSON wrappers; duplicate grid points retained; singular inputs return a typed error.  graph_points 2 and 3 use the reference two-point grid, checked directly in case-review-*. |
| `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration_json` | `statistics/traces/generic/common/case_duration.py` | `ichnos_stats::cases::get_kde_case_duration_values` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed Density replaces JSON wrappers; duplicate grid points retained; singular inputs return a typed error.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point.  graph_points 2 and 3 use the reference two-point grid, checked directly in case-review-*. |

## statistics.traces.generic.log.case_arrival

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.log.case_arrival.get_case_arrival_avg` | `statistics/traces/generic/log/case_arrival.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_case_arrival_avg` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |
| `pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg` | `statistics/traces/generic/log/case_arrival.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_case_dispersion_avg` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |

## statistics.traces.generic.log.case_statistics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.log.case_statistics.get_variant_statistics` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/variants/log/get` | `ichnos_stats::cases::get_variant_statistics` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_cases_description` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_cases_description` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute.  Case IDs remain typed scalar keys; numeric IDs sort numerically, int 1 and string "1" stay distinct, and incomparable mixed ID types return an error when sorting is requested. The source descriptions sort raw IDs but stringify dictionary keys afterwards; Rust preserves typed keys to prevent collisions (typed-ID oracle case covers the policy). |
| `pm4py.statistics.traces.generic.log.case_statistics.index_log_caseid` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter` | `ichnos_stats::cases::index_log_caseid` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute.  Case IDs remain typed scalar keys; numeric IDs sort numerically, int 1 and string "1" stay distinct, and incomparable mixed ID types return an error when sorting is requested. The source descriptions sort raw IDs but stringify dictionary keys afterwards; Rust preserves typed keys to prevent collisions (typed-ID oracle case covers the policy). |
| `pm4py.statistics.traces.generic.log.case_statistics.get_events` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_events` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_all_case_durations` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_all_case_durations` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Weekly business schedule and excluded dates supported. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_first_quartile_case_duration` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_first_quartile_case_duration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Preserves reference floor(3n/4) index despite the helper name. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_median_case_duration` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::cases::get_median_case_duration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Preserves upper-middle selection for even samples. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `statistics/traces/generic/common/case_duration` | `ichnos_stats::cases::get_kde_caseduration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed Density replaces JSON wrappers; duplicate grid points retained; singular inputs return a typed error.  graph_points 2 and 3 use the reference two-point grid, checked directly in case-review-*. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `statistics/traces/generic/common/case_duration` | `ichnos_stats::cases::get_kde_caseduration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed Density replaces JSON wrappers; duplicate grid points retained; singular inputs return a typed error.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point.  graph_points 2 and 3 use the reference two-point grid, checked directly in case-review-*. |

## statistics.traces.generic.pandas.case_arrival

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.pandas.case_arrival.get_case_arrival_avg` | `statistics/traces/generic/pandas/case_arrival.py` | `ichnos::stats::traces::generic::pandas::case_arrival::get_case_arrival_avg` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_arrival.get_case_arrival_avg; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_arrival.get_case_dispersion_avg` | `statistics/traces/generic/pandas/case_arrival.py` | `ichnos::stats::traces::generic::pandas::case_arrival::get_case_dispersion_avg` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg; ichnos has one implementation |

## statistics.traces.generic.pandas.case_statistics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variant_statistics` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_variant_statistics` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_variant_statistics; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_and_list` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos_stats::cases::get_variants_df_and_list` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_cases_description` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_cases_description` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_cases_description; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos_stats::cases::get_variants_df` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_with_case_duration` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos_stats::cases::get_variants_df_with_case_duration` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_events` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_events` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_events; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_kde_caseduration` | `statistics/traces/generic/pandas/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::pandas::case_statistics::get_kde_caseduration` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_kde_caseduration_json` | `statistics/traces/generic/pandas/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::pandas::case_statistics::get_kde_caseduration_json` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_all_case_durations` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_all_case_durations` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_all_case_durations; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_first_quartile_case_duration` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_first_quartile_case_duration` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_first_quartile_case_duration; ichnos has one implementation |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_median_case_duration` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_median_case_duration` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_median_case_duration; ichnos has one implementation |

## statistics.traces.generic.polars.case_arrival

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.polars.case_arrival.get_case_arrival_avg` | `statistics/traces/generic/polars/case_arrival.py` | `ichnos::stats::traces::generic::polars::case_arrival::get_case_arrival_avg` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_arrival.get_case_arrival_avg; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_arrival.get_case_dispersion_avg` | `statistics/traces/generic/polars/case_arrival.py` | `ichnos::stats::traces::generic::polars::case_arrival::get_case_dispersion_avg` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg; ichnos has one implementation |

## statistics.traces.generic.polars.case_statistics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_variant_statistics` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_variant_statistics` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_variant_statistics; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df_and_list` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos_stats::cases::get_variants_df_and_list` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_cases_description` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_cases_description` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_cases_description; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos_stats::cases::get_variants_df` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt, roadtraffic100traces and interval-event-log; metric tolerance 1e-6 relative / 1e-12 absolute. Typed rows replace backend DataFrames; canonical log implementation.  Direct oracle calls and typed output comparisons in case-review-* cover this entry point. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_all_case_durations` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_all_case_durations` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_all_case_durations; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_median_case_duration` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_median_case_duration` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_median_case_duration; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_first_quartile_case_duration` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_first_quartile_case_duration` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_first_quartile_case_duration; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_kde_caseduration` | `statistics/traces/generic/polars/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::polars::case_statistics::get_kde_caseduration` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration; ichnos has one implementation |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_kde_caseduration_json` | `statistics/traces/generic/polars/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::polars::case_statistics::get_kde_caseduration_json` (planned) | `ichnos-stats` | dropped | collapsed into pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json; ichnos has one implementation |

## statistics.util.times_bipartite_matching

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.util.times_bipartite_matching.exact_match_minimum_average` | `statistics/util/times_bipartite_matching.py` | `ichnos_stats::time::exact_match_minimum_average` | `ichnos-stats` | ported | Golden matching cases; tied optimal assignments compared by cardinality and total gap. Typed result; empty inputs return an empty matching. |

## statistics.variants.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.variants.log.get.get_language` | `statistics/variants/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::variants::get_language` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.variants.log.get.get_variants` | `statistics/variants/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::variants::get_variant_traces` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed sequence/count map also represents the backend count/set helpers. |
| `pm4py.statistics.variants.log.get.get_variants_along_with_case_durations` | `statistics/variants/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos_stats::variants::get_variants_along_with_case_durations` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled).  Nonempty cases with missing endpoint timestamps have duration zero rather than pm4py’s KeyError; empty cases also have duration zero. |
| `pm4py.statistics.variants.log.get.get_variants_from_log_trace_idx` | `statistics/variants/log/get.py` → `objects/conversion/log/converter` | `ichnos_stats::variants::get_variants_from_log_trace_idx` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.variants.log.get.get_variants_sorted_by_count` | `statistics/variants/log/get.py` | `ichnos_stats::variants::get_variants_sorted_by_count` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |
| `pm4py.statistics.variants.log.get.convert_variants_trace_idx_to_trace_obj` | `statistics/variants/log/get.py` → `objects/conversion/log/converter` | `ichnos_stats::variants::convert_variants_trace_idx_to_trace_obj` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). |

## statistics.variants.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.variants.pandas.get.get_variants_count` | `statistics/variants/pandas/get.py` → `objects/log/util/pandas_numpy_variants` | `ichnos_stats::variants::get_variants_count` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed sequence/count map also represents the backend count/set helpers.  Backend-specific table representations collapse into the same typed EventLog implementation; these rows introduce no separate statistical algorithm. |
| `pm4py.statistics.variants.pandas.get.get_variants_set` | `statistics/variants/pandas/get.py` | `ichnos_stats::variants::get_variants_set` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed sequence/count map also represents the backend count/set helpers.  Backend-specific table representations collapse into the same typed EventLog implementation; these rows introduce no separate statistical algorithm. |

## statistics.variants.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.variants.polars.get.pandas_numpy_variants_apply_polars` | `statistics/variants/polars/get.py` | `ichnos_stats::variants::get_variants_from_log_trace_idx` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled).  Backend-specific table representations collapse into the same typed EventLog implementation; these rows introduce no separate statistical algorithm. |
| `pm4py.statistics.variants.polars.get.get_variants_count` | `statistics/variants/polars/get.py` | `ichnos_stats::variants::get_variants_count` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed sequence/count map also represents the backend count/set helpers.  Backend-specific table representations collapse into the same typed EventLog implementation; these rows introduce no separate statistical algorithm. |
| `pm4py.statistics.variants.polars.get.get_variants_set` | `statistics/variants/polars/get.py` | `ichnos_stats::variants::get_variants_set` | `ichnos-stats` | ported | Golden stats cases on running-example, receipt and roadtraffic100traces CSV; Arrow nulls are absent (pm4py stream postprocessing enabled). Typed sequence/count map also represents the backend count/set helpers.  Backend-specific table representations collapse into the same typed EventLog implementation; these rows introduce no separate statistical algorithm. |

## streaming.algo.conformance.alignments.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.alignments.algorithm.apply` | `streaming/algo/conformance/alignments/algorithm.py` | `ichnos_stream::StreamingAlignments::new` | `ichnos-stream` | ported | Default IWS entry point. Golden `stream/iws-automatic-chain` checks model preparation and all prefix/completion diagnostics; real-log cases `iws-running-example`, `iws-receipt`, `iws-roadtraffic100traces` check every event using controlled proxies. |

## streaming.algo.conformance.alignments.variants.approx_iws

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._TrieNode` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `objects/petri_net/obj` | — | `ichnos-stream` | dropped | Private trie representation; internal indexed Rust nodes are not public API. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._State` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `algo/conformance/alignments/petri_net/utils/approx_utils` | — | `ichnos-stream` | dropped | Private candidate representation; typed public alignment results expose diagnostics without internal mutable states. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `algo/conformance/alignments/petri_net/utils/approx_utils`, `objects/petri_net/obj`, `objects/petri_net/utils/align_utils`, `streaming/algo/interface` | `ichnos_stream::StreamingAlignments` | `ichnos-stream` | ported | Goldens `stream/iws-silent-lookahead`, `iws-lookahead-one`, `iws-decay-fallback`, `iws-state-cap`, `iws-branching-proxy` check prefix moves/costs, decay, candidate counts and trie nodes. All three real-log cases check complete ordered state digests and samples; `iws-running-example-pnml` uses a real PNML model with silent transitions; `iws-duplicate-labels` preserves transition identity. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments.finish` | `streaming/algo/conformance/alignments/variants/approx_iws.py` | `ichnos_stream::StreamingAlignments::finish` | `ichnos-stream` | ported | Goldens `stream/iws-silent-lookahead`, `iws-branching-proxy`, `iws-completion-custom` check final suffixes, costs, validity and completed-result overlay; `iws-empty` checks no cases. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.apply` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `objects/petri_net/obj` | `ichnos_stream::StreamingAlignments::{new,from_proxy_traces,with_proxy_sequences}` | `ichnos-stream` | ported | Goldens `stream/iws-automatic-chain`, `iws-branching-proxy`, `iws-silent-lookahead` exercise deterministic generation, merged-aligner proxy preparation and supplied complete runs respectively. |

## streaming.algo.conformance.declare.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.declare.algorithm.apply` | `streaming/algo/conformance/declare/algorithm.py` | `ichnos_stream::StreamingDeclareConformance::new` | `ichnos-stream` | ported | Goldens `stream/declare-running-example`, `declare-receipt`, `declare-roadtraffic100traces` construct pm4py-discovered Declare models, feed every event and compare up to five prefix snapshots plus live delivery. |

## streaming.algo.conformance.declare.variants.automata

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.declare.variants.automata.DeclareStreamingConformance` | `streaming/algo/conformance/declare/variants/automata.py` → `streaming/algo/interface` | `ichnos_stream::StreamingDeclareConformance` | `ichnos-stream` | ported | Goldens `stream/declare-all-templates`, `declare-pending`, `declare-interleaved`, `declare-missing`, `declare-timestamps`, `declare-empty`, `declare-special-labels` check state names, absorbing/immediate deviations, event counts, per-event time fallback and typed constraint identities. Real-log cases compare complete ordered state/history digests and samples. |
| `pm4py.streaming.algo.conformance.declare.variants.automata.apply` | `streaming/algo/conformance/declare/variants/automata.py` | `ichnos_stream::StreamingDeclareConformance::new` | `ichnos-stream` | ported | Goldens `stream/declare-all-templates`, `declare-self-pairs`, `declare-empty-model` check typed model preparation, all eighteen monitor templates, equal binary labels and ignored count metadata. |

## streaming.algo.conformance.footprints.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.footprints.algorithm.apply` | `streaming/algo/conformance/footprints/algorithm.py` | `ichnos_stream::StreamingFootprintsConformance::new` | `ichnos-stream` | ported | `stream/conf-footprints-running-example` constructs the checker from discovered entire-log footprints. |

## streaming.algo.conformance.footprints.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance` | `streaming/algo/conformance/footprints/variants/classic.py` → `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos_stream::StreamingFootprintsConformance::new` | `ichnos-stream` | ported | `stream/conf-footprints-running-example` constructs the checker from discovered entire-log footprints. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.build_dictionaries` | `streaming/algo/conformance/footprints/variants/classic.py` → `streaming/util/dictio/generator` | — | `ichnos-stream` | dropped | Internal dictionary encoding replaced by typed case maps and Marking; no string tuples or eval. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.encode_str` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Internal dictionary encoding replaced by typed case maps and Marking; no string tuples or eval. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_footprints` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::push` | `ichnos-stream` | ported | `stream/conf-footprints-interleaved` compares interleaved cases, unknown activities and incomplete events. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_intra_case` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::push` | `ichnos-stream` | ported | `stream/conf-footprints-interleaved` checks sequence/parallel union; `stream/conf-footprints-deviating-running-example` checks invalid pairs. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_start_case` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::push` | `ichnos-stream` | ported | `stream/conf-footprints-deviating-running-example` checks all-invalid starts; `stream/conf-footprints-interleaved` mixes valid/invalid starts. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.get_status` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::get_status` | `ichnos-stream` | ported | `stream/conf-footprints-interleaved` compares prefix fitness; conformance_edges::unknown_first_footprints_deviations_survive_and_cases_restart covers unknown-only status. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::terminate` | `ichnos-stream` | ported | `stream/conf-footprints-interleaved` checks end-aware fitness and removal; `stream/conf-footprints-deviating-running-example` covers invalid ends. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate_all` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::terminate_all` | `ichnos-stream` | ported | `stream/conf-footprints-running-example` compares all restarted live cases and the empty after_termination snapshot. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_or_activity_not_in_event` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_activity_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_footprints_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_start_activity_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_end_activity_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_not_in_dictionary` | `streaming/algo/conformance/footprints/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.apply` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos_stream::StreamingFootprintsConformance::new` | `ichnos-stream` | ported | `stream/conf-footprints-running-example` constructs the checker from discovered entire-log footprints. |

## streaming.algo.conformance.tbr.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.tbr.algorithm.apply` | `streaming/algo/conformance/tbr/algorithm.py` | `ichnos_stream::StreamingTbrConformance::new` | `ichnos-stream` | ported | `stream/conf-tbr-running-example-pnml` constructs the checker over a real model containing silent transitions. |

## streaming.algo.conformance.tbr.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/obj`, `objects/petri_net/semantics`, `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos_stream::StreamingTbrConformance::new` | `ichnos-stream` | ported | `stream/conf-tbr-running-example-pnml` constructs the checker over a real model containing silent transitions. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.build_dictionaries` | `streaming/algo/conformance/tbr/variants/classic.py` → `streaming/util/dictio/generator` | — | `ichnos-stream` | dropped | Internal dictionary encoding replaced by typed case maps and Marking; no string tuples or eval. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_paths_net` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/obj` | `ichnos_stream::StreamingTbrConformance::new` | `ichnos-stream` | ported | `stream/conf-tbr-two-silent-paths` compares unequal shortest silent paths; `stream/conf-tbr-running-example-pnml` covers a real model. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_str` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Internal dictionary encoding replaced by typed case maps and Marking; no string tuples or eval. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_marking` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Internal dictionary encoding replaced by typed case maps and Marking; no string tuples or eval. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.decode_marking` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/obj` | — | `ichnos-stream` | dropped | Internal dictionary encoding replaced by typed case maps and Marking; no string tuples or eval. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.verify_tbr` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/semantics` | `ichnos_stream::StreamingTbrConformance::push` | `ichnos-stream` | ported | `stream/conf-tbr-weighted` checks insertion counts; `stream/conf-tbr-duplicate-labels` checks enabled-label selection. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.enable_trans_with_invisibles` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/semantics` | `ichnos_stream::StreamingTbrConformance::push` | `ichnos-stream` | ported | `stream/conf-tbr-two-silent-paths` checks the shorter path; `stream/conf-tbr-silent-0`, `-1`, `-2` and `-10` check the iteration cap. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_status` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos_stream::StreamingTbrConformance::get_status` | `ichnos-stream` | ported | `stream/conf-tbr-duplicate-labels` compares open-case markings and missing counts at each prefix. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos_stream::StreamingTbrConformance::terminate` | `ichnos-stream` | ported | `stream/conf-tbr-weighted` compares signed final differences; `stream/conf-tbr-silent-10` confirms no final silent walk. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate_all` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos_stream::StreamingTbrConformance::terminate_all` | `ichnos-stream` | ported | `stream/conf-tbr-running-example-pnml` compares restarted live replay and the empty after_termination snapshot. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.reach_fm_with_invisibles` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/semantics` | — | `ichnos-stream` | dropped | Native terminate passes an encoded string to this place-keyed helper and cannot follow final paths; terminate preserves the observed signed final diagnostics. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_or_activity_not_in_event` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_activity_not_possible` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_missing_tokens` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_not_in_dictionary` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_final_marking_not_reached` | `streaming/algo/conformance/tbr/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.apply` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos_stream::StreamingTbrConformance::new` | `ichnos-stream` | ported | `stream/conf-tbr-running-example-pnml` constructs the checker over a real model containing silent transitions. |

## streaming.algo.conformance.temporal.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.temporal.algorithm.apply` | `streaming/algo/conformance/temporal/algorithm.py` | `ichnos_stream::StreamingTemporalConformance::new` | `ichnos-stream` | ported | `stream/conf-temporal-running-example` constructs a checker from a discovered profile; `stream/conf-temporal-empty` covers an empty profile. |

## streaming.algo.conformance.temporal.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance` | `streaming/algo/conformance/temporal/variants/classic.py` → `objects/log/obj`, `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos_stream::StreamingTemporalConformance::new` | `ichnos-stream` | ported | `stream/conf-temporal-running-example` constructs a checker from a discovered profile; `stream/conf-temporal-empty` covers an empty profile. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.check_conformance` | `streaming/algo/conformance/temporal/variants/classic.py` | `ichnos_stream::StreamingTemporalConformance::push` | `ichnos-stream` | ported | `stream/conf-temporal-interval-0`, `-1` and `-6` compare overlaps, arrival order and zero variance; `stream/conf-temporal-receipt` covers discovered real bounds. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_event_is_not_complete` | `streaming/algo/conformance/temporal/variants/classic.py` → `objects/log/obj` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_deviation` | `streaming/algo/conformance/temporal/variants/classic.py` | — | `ichnos-stream` | dropped | Logging hook; callers inspect typed diagnostics/errors and use application sinks. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.apply` | `streaming/algo/conformance/temporal/variants/classic.py` | `ichnos_stream::StreamingTemporalConformance::new` | `ichnos-stream` | ported | `stream/conf-temporal-running-example` constructs a checker from a discovered profile; `stream/conf-temporal-empty` covers an empty profile. |

## streaming.algo.discovery.dfg.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.discovery.dfg.algorithm.apply` | `streaming/algo/discovery/dfg/algorithm.py` | `ichnos_stream::StreamingDfgDiscovery::new` | `ichnos-stream` | ported | `stream/dfg-running-example` compares initial/final graph counts and intermediate prefixes; `stream/dfg-empty` covers empty construction. |

## streaming.algo.discovery.dfg.variants.frequency

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery` | `streaming/algo/discovery/dfg/variants/frequency.py` → `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos_stream::StreamingDfgDiscovery::new` | `ichnos-stream` | ported | `stream/dfg-running-example` compares initial/final graph counts and intermediate prefixes; `stream/dfg-empty` covers empty construction. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.build_dictionaries` | `streaming/algo/discovery/dfg/variants/frequency.py` → `streaming/util/dictio/generator` | — | `ichnos-stream` | dropped | Internal dictionary/storage encoding helper; typed Dfg maps replace it. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.event_without_activity_or_case` | `streaming/algo/discovery/dfg/variants/frequency.py` | — | `ichnos-stream` | dropped | Python warning helper dropped; MissingEventPolicy controls skip/error behavior. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_str` | `streaming/algo/discovery/dfg/variants/frequency.py` | — | `ichnos-stream` | dropped | Internal dictionary/storage encoding helper; typed Dfg maps replace it. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_tuple` | `streaming/algo/discovery/dfg/variants/frequency.py` | — | `ichnos-stream` | dropped | Internal dictionary/storage encoding helper; typed Dfg maps replace it. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.apply` | `streaming/algo/discovery/dfg/variants/frequency.py` | `ichnos_stream::StreamingDfgDiscovery::new` | `ichnos-stream` | ported | `stream/dfg-running-example` compares initial/final graph counts and intermediate prefixes; `stream/dfg-empty` covers empty construction. |

## streaming.algo.interface

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.interface.StreamingAlgorithm` | `streaming/algo/interface.py` | `ichnos_stream::StreamSink` | `ichnos-stream` | ported | `stream/dfg-interleaved` exercises a StreamSink algorithm registered with a live event stream. |
| `pm4py.streaming.algo.interface.StreamingAlgorithm.get` | `streaming/algo/interface.py` | `ichnos_stream::StreamingDfgDiscovery::get` | `ichnos-stream` | ported | `stream/dfg-running-example` compares the typed get result at up to five prefixes and after live delivery. |
| `pm4py.streaming.algo.interface.StreamingAlgorithm.receive` | `streaming/algo/interface.py` | `ichnos_stream::StreamSink::push` | `ichnos-stream` | ported | `stream/dfg-interleaved` compares push updates across interleaved cases; `stream/dfg-missing` covers incomplete events. |

## streaming.connectors.windows.click_key_logger

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger` | `streaming/connectors/windows/click_key_logger.py` → `objects/log/obj` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.run` | `streaming/connectors/windows/click_key_logger.py` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.stop` | `streaming/connectors/windows/click_key_logger.py` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.get_process_name` | `streaming/connectors/windows/click_key_logger.py` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.record` | `streaming/connectors/windows/click_key_logger.py` → `objects/log/obj` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_click` | `streaming/connectors/windows/click_key_logger.py` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_key_release` | `streaming/connectors/windows/click_key_logger.py` | — | `ichnos-stream` | dropped | Platform click/key capture connector omitted; supply canonical events to StreamSink. |

## streaming.conversion.from_pandas

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable` | `streaming/conversion/from_pandas.py` → `objects/log/obj`, `streaming/stream/live_trace_stream` | `ichnos_stream::TraceIterator::from_record_batch` | `ichnos-stream` | ported | `stream/dataframe-numeric-integers` covers Arrow/event projection and numeric case ordering; `stream/dataframe-interleaved` records the corrected grouping. |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.read_trace` | `streaming/conversion/from_pandas.py` → `objects/log/obj` | `ichnos_stream::TraceIterator::read_trace` | `ichnos-stream` | ported | `stream/dataframe-numeric-integers` covers typed projected traces; `stream/dataframe-interleaved` records the corrected grouping. |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.reset` | `streaming/conversion/from_pandas.py` | `ichnos_stream::TraceIterator::reset` | `ichnos-stream` | ported | `stream/dataframe-numeric-integers` covers reset and unchanged repeated projection; `stream/dataframe-interleaved` records the corrected grouping. |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.to_trace_stream` | `streaming/conversion/from_pandas.py` → `streaming/stream/live_trace_stream` | `ichnos_stream::TraceIterator::to_trace_stream` | `ichnos-stream` | ported | `stream/dataframe-numeric-integers` covers forwarded trace digest; `stream/dataframe-interleaved` records the corrected grouping. |
| `pm4py.streaming.conversion.from_pandas.apply` | `streaming/conversion/from_pandas.py` | `ichnos_stream::TraceIterator::from_record_batch` | `ichnos-stream` | ported | `stream/dataframe-numeric-integers` covers Arrow/event projection and numeric case ordering; `stream/dataframe-interleaved` records the corrected grouping. |

## streaming.conversion.ocel_flatts_distributor

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor` | `streaming/conversion/ocel_flatts_distributor.py` → `objects/ocel/constants`, `streaming/stream/live_event_stream` | `ichnos_stream::OcelFlatteningDistributor` | `ichnos-stream` | ported | Golden `stream/ocel-example` checks all flattened event contents from a real OCEL fixture; `ocel-custom` checks source/destination keys and object prefix; `ocel-empty` checks empty input. |
| `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.register` | `streaming/conversion/ocel_flatts_distributor.py` → `streaming/stream/live_event_stream` | `ichnos_stream::OcelFlatteningDistributor::register` | `ichnos-stream` | ported | Goldens `stream/ocel-example`, `ocel-duplicates` check per-type routing, repeated listener registration and unregistered/empty types through live streams. |
| `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.append` | `streaming/conversion/ocel_flatts_distributor.py` | `ichnos_stream::OcelFlatteningDistributor::append` | `ichnos-stream` | ported | Goldens `stream/ocel-example`, `ocel-duplicates`, `ocel-custom` check copied attributes, renamed activity/timestamp, overwritten case IDs, list order and duplicate object delivery. |

## streaming.importer.csv.importer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.csv.importer.apply` | `streaming/importer/csv/importer.py` | `ichnos_stream::CsvEventReader::open` | `ichnos-stream` | ported | `stream/csv-quoted` compares items content including quotes/newlines; `stream/csv-receipt` covers real-log CSV input. |

## streaming.importer.csv.variants.csv_event_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos_stream::CsvEventReader::open` | `ichnos-stream` | ported | `stream/csv-quoted` compares items content including quotes/newlines; `stream/csv-receipt` covers real-log CSV input. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.reset` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos_stream::CsvEventReader::reset` | `ichnos-stream` | ported | `stream/csv-quoted` compares reset content including quotes/newlines; `stream/csv-receipt` covers real-log CSV input. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.to_event_stream` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos_stream::CsvEventReader::to_event_stream` | `ichnos-stream` | ported | `stream/csv-quoted` compares forwarded content including quotes/newlines; `stream/csv-receipt` covers real-log CSV input. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.read_event` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos_stream::CsvEventReader::read_event` | `ichnos-stream` | ported | `stream/csv-quoted` compares items content including quotes/newlines; `stream/csv-receipt` covers real-log CSV input. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.apply` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos_stream::CsvEventReader::open` | `ichnos-stream` | ported | `stream/csv-quoted` compares items content including quotes/newlines; `stream/csv-receipt` covers real-log CSV input. |

## streaming.importer.xes.importer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.xes.importer.apply` | `streaming/importer/xes/importer.py` | `ichnos_stream::{XesEventReader, XesTraceReader}::open` | `ichnos-stream` | ported | `stream/xes-events-running-example` and `stream/xes-traces-running-example` cover both importer variants. |

## streaming.importer.xes.variants.xes_event_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.parse_attribute` | `streaming/importer/xes/variants/xes_event_stream.py` | — | `ichnos-stream` | dropped | Internal XML helper; incremental readers reuse the canonical ichnos-io parser. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader` | `streaming/importer/xes/variants/xes_event_stream.py` → `objects/log/obj` | `ichnos_stream::XesEventReader::open` | `ichnos-stream` | ported | `stream/xes-events-typed` compares items content including nested attributes; `stream/xes-events-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.to_event_stream` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos_stream::XesEventReader::to_event_stream` | `ichnos-stream` | ported | `stream/xes-events-typed` compares forwarded content including nested attributes; `stream/xes-events-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.reset` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos_stream::XesEventReader::reset` | `ichnos-stream` | ported | `stream/xes-events-typed` compares reset content including nested attributes; `stream/xes-events-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.read_event` | `streaming/importer/xes/variants/xes_event_stream.py` → `objects/log/obj` | `ichnos_stream::XesEventReader::read_event` | `ichnos-stream` | ported | `stream/xes-events-typed` compares items content including nested attributes; `stream/xes-events-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.apply` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos_stream::XesEventReader::open` | `ichnos-stream` | ported | `stream/xes-events-typed` compares items content including nested attributes; `stream/xes-events-receipt` covers real-log input. |

## streaming.importer.xes.variants.xes_trace_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.parse_attribute` | `streaming/importer/xes/variants/xes_trace_stream.py` | — | `ichnos-stream` | dropped | Internal XML helper; incremental readers reuse the canonical ichnos-io parser. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader` | `streaming/importer/xes/variants/xes_trace_stream.py` → `objects/log/obj` | `ichnos_stream::XesTraceReader::open` | `ichnos-stream` | ported | `stream/xes-traces-typed` compares items content including nested attributes; `stream/xes-traces-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.to_trace_stream` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos_stream::XesTraceReader::to_trace_stream` | `ichnos-stream` | ported | `stream/xes-traces-typed` compares forwarded content including nested attributes; `stream/xes-traces-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.reset` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos_stream::XesTraceReader::reset` | `ichnos-stream` | ported | `stream/xes-traces-typed` compares reset content including nested attributes; `stream/xes-traces-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.read_trace` | `streaming/importer/xes/variants/xes_trace_stream.py` → `objects/log/obj` | `ichnos_stream::XesTraceReader::read_trace` | `ichnos-stream` | ported | `stream/xes-traces-typed` compares items content including nested attributes; `stream/xes-traces-receipt` covers real-log input. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.apply` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos_stream::XesTraceReader::open` | `ichnos-stream` | ported | `stream/xes-traces-typed` compares items content including nested attributes; `stream/xes-traces-receipt` covers real-log input. |

## streaming.stream.live_event_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.stream.live_event_stream.StreamState` | `streaming/stream/live_event_stream.py` | `ichnos_stream::StreamState` | `ichnos-stream` | ported | `stream/dfg-running-example` covers inactive/active/finished states. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream` | `streaming/stream/live_event_stream.py` | `ichnos_stream::LiveEventStream::new` | `ichnos-stream` | ported | `stream/dfg-running-example` covers event-stream construction and lifecycle. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.append` | `streaming/stream/live_event_stream.py` | `ichnos_stream::LiveEventStream::append` | `ichnos-stream` | ported | `stream/dfg-running-example` covers queued, active and ignored-after-finish appends. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.start` | `streaming/stream/live_event_stream.py` | `ichnos_stream::LiveEventStream::start` | `ichnos-stream` | ported | `stream/dfg-running-example` covers queued-item delivery at start. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.stop` | `streaming/stream/live_event_stream.py` | `ichnos_stream::LiveEventStream::stop` | `ichnos-stream` | ported | `stream/dfg-running-example` covers final graph and ignored late append. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.register` | `streaming/stream/live_event_stream.py` | `ichnos_stream::LiveEventStream::register` | `ichnos-stream` | ported | `stream/dfg-running-example` covers idempotent shared registration and collector delivery. |

## streaming.stream.live_trace_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.stream.live_trace_stream.StreamState` | `streaming/stream/live_trace_stream.py` | `ichnos_stream::StreamState` | `ichnos-stream` | ported | `stream/dataframe-running-example` covers inactive/active/finished states. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream` | `streaming/stream/live_trace_stream.py` | `ichnos_stream::LiveTraceStream::new` | `ichnos-stream` | ported | `stream/dataframe-running-example` covers trace-stream construction and lifecycle. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.append` | `streaming/stream/live_trace_stream.py` | `ichnos_stream::LiveTraceStream::append` | `ichnos-stream` | ported | `stream/dataframe-running-example` covers queued and active trace delivery. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.start` | `streaming/stream/live_trace_stream.py` | `ichnos_stream::LiveTraceStream::start` | `ichnos-stream` | ported | `stream/dataframe-running-example` covers queued-trace delivery at start. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.stop` | `streaming/stream/live_trace_stream.py` | `ichnos_stream::LiveTraceStream::stop` | `ichnos-stream` | ported | `stream/dataframe-running-example` covers finished state and full forwarded trace digest. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.register` | `streaming/stream/live_trace_stream.py` | `ichnos_stream::LiveTraceStream::register` | `ichnos-stream` | ported | `stream/dataframe-running-example` covers idempotent shared trace-observer registration. |

## streaming.util.dictio.generator

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.generator.apply` | `streaming/util/dictio/generator.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |

## streaming.util.dictio.versions.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.versions.classic.apply` | `streaming/util/dictio/versions/classic.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |

## streaming.util.dictio.versions.redis

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.keys` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.values` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.itervalues` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushdb` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushall` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.redis.apply` | `streaming/util/dictio/versions/redis.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |

## streaming.util.dictio.versions.thread_safe

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict` | `streaming/util/dictio/versions/thread_safe.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.keys` | `streaming/util/dictio/versions/thread_safe.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.values` | `streaming/util/dictio/versions/thread_safe.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.itervalues` | `streaming/util/dictio/versions/thread_safe.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |
| `pm4py.streaming.util.dictio.versions.thread_safe.apply` | `streaming/util/dictio/versions/thread_safe.py` | — | `ichnos-stream` | dropped | Internal classic/thread-safe/Redis storage backend; native typed maps replace it. |

## streaming.util.event_stream_printer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.event_stream_printer.EventStreamPrinter` | `streaming/util/event_stream_printer.py` → `streaming/algo/interface` | — | `ichnos-stream` | dropped | Python stdout diagnostic observer omitted; applications can implement StreamSink logging. |

## streaming.util.live_to_static_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.live_to_static_stream.LiveToStaticStream` | `streaming/util/live_to_static_stream.py` → `objects/log/obj`, `streaming/algo/interface` | `ichnos_stream::Collector` | `ichnos-stream` | ported | `stream/dfg-running-example` compares collected events; `stream/dataframe-running-example` compares collected traces. |

## streaming.util.trace_stream_printer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.trace_stream_printer.TraceStreamPrinter` | `streaming/util/trace_stream_printer.py` → `streaming/algo/interface` | — | `ichnos-stream` | dropped | Python stdout diagnostic observer omitted; applications can implement StreamSink logging. |

## __init__

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.sys` | `__init__.py` | `ichnos::core::exports::sys` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.time` | `__init__.py` | `ichnos::core::exports::time` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.objects` | `__init__.py` | `ichnos::core::exports::objects` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.util` | `__init__.py` | `ichnos::core::exports::util` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.utils` | `__init__.py` | `ichnos::core::exports::utils` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.algo` | `__init__.py` | `ichnos::core::exports::algo` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.statistics` | `__init__.py` | `ichnos::core::exports::statistics` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.visualization` | `__init__.py` | `ichnos::core::exports::visualization` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.llm` | `__init__.py` | `ichnos::core::exports::llm` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.connectors` | `__init__.py` | `ichnos::core::exports::connectors` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.analysis` | `__init__.py` | `ichnos::core::exports::analysis` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.conformance` | `__init__.py` | `ichnos::core::exports::conformance` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.convert` | `__init__.py` | `ichnos::core::exports::convert` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.discovery` | `__init__.py` | `ichnos::core::exports::discovery` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.filtering` | `__init__.py` | `ichnos::core::exports::filtering` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.hof` | `__init__.py` | `ichnos::core::exports::hof` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.ml` | `__init__.py` | `ichnos::core::exports::ml` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.ocel` | `__init__.py` | `ichnos::core::exports::ocel` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.org` | `__init__.py` | `ichnos::core::exports::org` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.read` | `__init__.py` | `ichnos::core::exports::read` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.sim` | `__init__.py` | `ichnos::core::exports::sim` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.stats` | `__init__.py` | `ichnos::core::exports::stats` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.vis` | `__init__.py` | `ichnos::core::exports::vis` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.write` | `__init__.py` | `ichnos::core::exports::write` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |
| `pm4py.meta` | `__init__.py` | `ichnos::core::exports::meta` (planned) | `ichnos-core` | dropped | Python module namespace, not a function; Rust modules replace it. |

## objects.petri_net.obj

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.PetriNet` | `objects/petri_net/obj.py` → `objects/petri_net/utils/petri_utils` | `ichnos::model::PetriNet` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.Marking` | `objects/petri_net/obj.py` | `ichnos::model::Marking` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |

## objects.process_tree.obj

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.ProcessTree` | `objects/process_tree/obj.py` | `ichnos::model::ProcessTree` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |

## objects.ocel.obj

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.OCEL` | `objects/ocel/obj.py` → `objects/ocel/constants` | `ichnos_ocel::Ocel` | `ichnos-ocel` | ported | `Ocel` with `is_ocel20`, `summary` (pm4py's `get_summary` text through `Display`) and `extended_table`; default column names and JSON keys in `ichnos_ocel::constants`. Goldens `ocel/model-*` (6 fixtures and the empty log) compare each against pm4py. The goldens leave `globals` empty; the ichnos-io OCEL readers fill it and test it. See the ichnos-ocel Behaviour changes. |

## objects.bpmn.obj

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.BPMN` | `objects/bpmn/obj.py` | `ichnos_model::Bpmn` | `ichnos-model` | ported | Goldens `bpmn/*` build diagrams with every node kind from pm4py's and compare conversions, token flow and gateway reduction. See ichnos-model (BPMN) Behaviour changes. |

## Behaviour changes

Lanes record each deliberate change from pm4py here.

### ichnos-discovery (inductive miner)

- **The result is deterministic.** pm4py's IMf trees can change with Python's hash seed, because its exclusive-choice groups and sequence groups follow set order. ichnos lists exclusive-choice groups largest first, then by smallest activity name, and ranks a sequence group by its smallest activity name. Each IMf golden holds one pm4py run per distinct tree over hash seeds 0 to 7, and ichnos must match one of them.
- **No `tree_sort`.** pm4py sorts the children of XOR and parallel nodes by the sum of the MD5 hashes of their labels. ichnos keeps the order the cuts produce. The language is the same.
- **No `multi_processing` option.** The activity-concurrent fall-through tries candidates in sorted order and stops at the first, as pm4py does without multiprocessing.
- **A noise threshold outside [0, 1] is an error.** pm4py accepts any value.
- **`InductiveVariant::Imf { noise_threshold: 0.0 }` runs IMf, with nothing filtered.** pm4py's `discover_process_tree_inductive` runs IM for a threshold of 0. `InductiveOptions::from_noise_threshold` applies pm4py's rule.
- **Where pm4py fails, ichnos returns a tree.** A strict sequence cut that merges into one group counts as no cut; pm4py recurses without end. An IMd base case whose only activity is an end activity gives that activity; pm4py raises `IndexError`.
- **POWL brute force adds up projected counts.** When several variants project to the same trace of a group, pm4py's brute-force cut keeps the count of the last one; ichnos adds them. Only the variant filter reads counts, so the models differ only when `filtering_weight_factor` is above 0. With pm4py patched to add counts, a probe of 3600 runs matched.
- **POWL dynamic clustering gives no cut where pm4py recurses without end.** When making the order transitive would merge a cluster with itself, ichnos goes on to the fall-throughs.
- **BPMN diagrams compare up to isomorphism.** pm4py names BPMN nodes with random UUIDs, so the `bpmn-inductive-*` goldens compare node kinds, labels and flows, not ids.
- **POWL option names follow ichnos.** `PowlOptions::filtering_weight_factor` is pm4py's `filtering_weight_factor`, and `PowlVariant::DynamicClustering { order_frequency_ratio }` is its `order_graph_filtering_threshold`.
- **POWL brute force is not checked on receipt and receipt_even.** pm4py takes too long there. The other four logs and the synthetic logs cover it.
- **POWL options are checked before mining.** A `filtering_weight_factor` outside [0, 1) or an `order_frequency_ratio` outside (0.5, 1] is an error up front. pm4py raises only when the miner first reads the option.
- **POWL orders hold the pairs transitivity implies**, as `Powl::simplify` does. pm4py's can leave them out. Children of a choice or a partial order can come in a different order, because pm4py's cuts list groups in Python's set order.

### ichnos-discovery (footprints)

- **Each pm4py variant is its own function.** `log_footprints` covers `entire_event_log`, `entire_dataframe` and `polars_lazyframes`; `trace_footprints` covers `trace_by_trace`; `dfg_footprints` covers `dfg`. `ProcessTree::footprints` (`process_tree`) and `Powl::footprints` (`powl`) are in `ichnos-model`.
- **`petri_reach_graph` is `ichnos_conformance::footprints::ModelFootprints::of_net`,** which follows pm4py. `PetriNet::footprints` covers every marking instead; see ichnos-model.
- **A frequent transition made from two silent steps stays as it is.** pm4py's `simplify_using_frequent_transitions` turns a choice or loop of two silent steps into a frequent transition without an activity; ichnos keeps the choice or loop. The `footprints-powl-*` goldens skip the simplified footprints of those models.
- **Two types share the name `LogFootprints`.** `ichnos_discovery::LogFootprints` groups its sets in a `Footprints` and pairs with `TraceFootprints`; `ichnos_conformance::footprints::LogFootprints` has flat fields and is the input of footprint conformance.

### ichnos-discovery (temporal profile)

- **One implementation for both pm4py variants.** pm4py has a log variant (Python `statistics`) and a dataframe variant (a pandas self-join). They give the same profile on every golden, and ichnos matches both. ichnos and the log variant pair events in log order. The dataframe variant first sorts each case by start and completion timestamp, so on a log whose events are not in time order the two pm4py variants can disagree; the goldens use sorted logs.
- **The start timestamp is an explicit option.** pm4py's log variant reads starts from the completion timestamp unless the caller names a start key. Its dataframe variant reads a `start_timestamp` column whenever the dataframe has one. So `pm4py.discover_temporal_profile` on a dataframe with that column measures from completion to start, and on the same log as an `EventLog` it measures from completion to completion. ichnos reads starts from `EventKeys::start_timestamp` only when `use_start_timestamp` is true.
- **Business hours use `ichnos_stats::time::BusinessHours`.** Its `non_working_dates` replace pm4py's `workcalendar`, as in ichnos-stats.
- **A missing or non-date timestamp is an error.** pm4py's log variant raises `KeyError` on a missing start key. Only a trace's first event may lack a start timestamp, since no pair uses it; pm4py's log variant reads starts from the second event on too. Discovery and conformance share this time measure: `ichnos_stats::time::for_each_event_pair`.

### ichnos-model

- **Conversions are methods on the source type.** pm4py's `convert_to_*` functions take any model and dispatch on its type; ichnos has `to_petri_net`, `to_bpmn`, `to_process_tree`, `to_powl` and `to_transition_system` on the types that support them.
- **No POWL to BPMN, and no POWL to POWL.** pm4py's POWL class extends its process tree class, so `pm4py.convert_to_bpmn` runs the process tree conversion on a POWL and returns a diagram with two nodes, and `pm4py.convert_to_powl` returns an empty partial order. Use `Powl::to_petri_net`, then `AcceptingPetriNet::to_bpmn`.
- **`PetriNet::to_powl` is deterministic and does not change the net.** pm4py follows set order, so its choice and partial-order children come in any order, and it rewrites the net it converts. pm4py names every node it adds `..._id1`, because it makes a new counter for each name; ichnos does not name them.
- **`PetriNet::to_powl` rejects inhibitor and reset arcs** with `WfNetToPowlError::SpecialArcs`. pm4py treats them as normal arcs. On the seven `inh_res_nets` fixtures it finds no structure and fails the same way on every run, with a `KeyError`, an `IndexError` or "Failed to detect a POWL structure". Its other failures are plain `Exception`s with a message, or a `KeyError` or `IndexError`; ichnos returns a `WfNetToPowlError` variant.
- **`AcceptingPetriNet::to_graph` keeps a place and a transition with the same name apart.** pm4py keys its networkx nodes by name and merges them.
- **Footprints of a Petri net cover every marking.** pm4py's `get_visible_transitions_eventually_enabled_by_marking` keeps one marking per silent transition. When two silent paths reach the same transition, it explores only one of them. So `pm4py.discover_footprints(net, im, fm)` can miss sequence and parallel pairs that the net allows, and alignment precision, which uses the same function, can differ. `PetriNet::footprints` explores the whole reachability graph. The discovery golden `inductive-imf-receipt-csv` shows the gap: pm4py misses `T03 -> T02`, which the tree allows when every step between them is skipped. A test that compares net footprints with pm4py can list such cases, check that ichnos keeps every pm4py pair, and check that each extra pair is in pm4py's footprints of the tree, as `PM4PY_NET_FOOTPRINTS_INCOMPLETE` in `crates/ichnos-discovery/tests/golden_inductive.rs` does.

### ichnos-conformance (temporal profile)

- **Times are exact.** pm4py's log variant subtracts float epoch seconds, so on logs with millisecond timestamps its times are off by about 1e-7 seconds. For a pair with a standard deviation of 0, any difference from the mean is a deviation, so this float noise makes the log variant report extra deviations on receipt. ichnos measures time as pm4py's dataframe variant does and matches it.
- **One implementation for both pm4py variants.** Apart from the float noise above, the log and dataframe variants give the same deviations on every golden.
- **The zeta of a pair with a standard deviation of 0 is `f64::INFINITY`.** pm4py uses `sys.maxsize`.
- **Zeta defaults to 1**, the default of `pm4py.conformance_temporal_profile`. pm4py's algorithm module defaults to 6.
- **The start timestamp is an explicit option**, as in temporal profile discovery. pm4py's dataframe variant reads a `start_timestamp` column whenever the dataframe has one.
- **No diagnostics dataframe.** pm4py's `return_diagnostics_dataframe` turns the result into a table with the case ID. ichnos returns one list of `TemporalDeviation` per trace, in log order.
- **A missing or non-date timestamp is an error**, and so is an invalid business schedule. Only a trace's first event may lack a start timestamp, since no pair uses it; pm4py's log variant reads starts from the second event on too. pm4py's dataframe variant falls back to the completion timestamp only when the whole start column is missing.

### ichnos-conformance (alignments)

- **Fixed horizon estimates the rest of a prefix with the LP relaxation of the marking equation.** pm4py solves the integer program and rounds the objective. ichnos solves the relaxation with its own simplex and rounds the objective to the nearest integer, half to even. The estimate can be lower than pm4py's on some nets, which can change the chosen prefix. The goldens match.
- **The approximate methods identify transitions by id.** pm4py looks transitions up by name, which fails or picks the wrong transition when two share a name. ichnos matches pm4py move for move on nets with unique names.
- **Subset selection: `random` and `simulation` are not ported.** Both depend on Python's random generator. ichnos adds `SubsetSelection::Variants`, which aligns the given sequences exactly.
- **Decomposed alignments follow the net's order where pm4py follows object hashes.** pm4py orders components and breaks ties in each component's A* search by object id, so its result can change between runs. ichnos orders components by the net and breaks ties as its own search does. A tie can change which components merge, so on the goldens 114 of 128 variants match one of pm4py's alignments exactly and 126 up to the order of moves; the cost and fitness are among pm4py's on all 128. The golden test fails if fewer match exactly or up to order.
- **Decomposed recomposition stops on cycles.** pm4py's first round does not mark visited components, and its second-round sort repeats until no swap happens. Both can run forever when the merge relation has a cycle. ichnos visits each component once in the first round when a cycle is reachable, and stops the sort after n + 1 passes.
- **Decomposed components keep arc weights.** pm4py's `decompose` builds each component with weight-1 arcs.
- **Decomposed alignments use the standard costs only.** pm4py's `recompos_maximal` accepts `model_cost_function`, `sync_cost_function` and `trace_cost_function`, but cannot apply them. Each component holds new transition objects, which compare by object id, so a caller's model or sync cost map raises `KeyError`. pm4py applies a trace cost list by position in each component's projected trace, not by the original event. `DecomposedOptions` therefore has no cost fields.
- **Decomposed alignments of an empty trace have a fitness and a best worst cost.** pm4py leaves both out for an empty trace. When the denominator is 0, ichnos gives fitness 0.
- **Errors replace exceptions and `None` results.** Nets with reset or inhibitor arcs, unknown marking places and invalid settings return an `Error`.

### ichnos-ocel

- **Tables are typed rows, not data frames.** pm4py's `OCEL` holds pandas data frames whose column names are constructor parameters. `Ocel` holds lists of events, objects, relations, object-to-object and event-to-event relations and object changes. Column names matter only to the readers and writers.
- **Relations keep ids only.** pm4py's `relations` table repeats each event's activity and timestamp and each object's type. `EventObject` keeps the event id, the object id and the qualifier. `summary` and `extended_table` look up the rest, and leave out a relation whose event or object is not in the log.
  - pm4py uses the relation's own activity and type columns, so it keeps such a relation. Two effects follow. First, the summary's `events-objects relationships` count includes every relation, but its `Unique activities per object type` line counts only relations whose event and object are both in the log. Second, a type that appears only on relations to missing objects has no extended-table column and is missing from that line.
- **Repeated event or object ids resolve to the first.** `event_index` and `object_index` map a repeated id to its first event or object. With repeated event ids, pm4py's `get_extended_table` raises and gives no table. `extended_table` gives the first such event all the objects related to that id, and the later events empty lists.
- **The extended table is typed.** `get_extended_table` returns a data frame with one `ocel:type:<type>` column per object type, holding a list or a missing value. `extended_table` returns the object types and, for each event, one list per type, empty where pandas has a missing value.
- **`OcelSummary` is a struct.** Its `Display` gives pm4py's `get_summary` text. `Ocel`'s `Display` gives the same text, as pm4py's `OCEL.__str__` and `__repr__` do.
- **No column-name parameters, `__hash__` or copy methods.** `Ocel` derives `Clone` and `PartialEq`.

### ichnos-io

- PNML preserves alternative final markings; pm4py merges them into one marking on import. `PnmlWriteOptions::include_alternative_final_markings = false` exports only the primary marking for pm4py consumers. Writer goldens read exact ichnos-generated PNML/PTML/DFG bytes with pm4py, including a PNML with two final markings.

- `read_xes` returns an `EventLog`; pm4py returns a DataFrame by default. The oracle uses `return_legacy_log_object=True`.
- Unknown vendor elements (including their subtrees), text and CDATA are ignored. Attributes without a key or scalar value are omitted because the core model has no null key/value; pm4py can retain `None`.
- Invalid numeric/date values return errors rather than being silently dropped. Dates retain their input offset; naive dates use UTC. DOCTYPE is rejected and nesting depth is bounded.
- `write_xes` emits `xes.version="2.0"`; pm4py emits `xes.version="1849-2016"` and `openxes.version`. XML layout and declaration ordering differ, while typed values round-trip.

- Model XML import builds bounded DOM trees with configurable depth/node limits; declarations use UTF-8, DOCTYPE is rejected, and malformed references, counts or structure return typed errors.
- PNML returns `PnmlDocument` with an `AcceptingPetriNet` and typed supplemental metadata. Final-marking inference is opt-in, matching `pm4py.read_pnml`; alternative final markings remain alternatives rather than being merged into one marking. Graphics and unrelated tool declarations are omitted. Distribution parameter text is retained without a sampling engine.
- PTML preserves edge declaration order and clones shared referenced subtrees. It keeps a non-silent three-child loop exit in its original sequence position (pm4py can move it to the end of the parent, or omit a root exit). Identical repeated node declarations are accepted; cycles, conflicting declarations and unreachable nodes error. Export rejects interleaving and loops with more than two model children.
- DFG output retains explicit empty boundary maps by default; inference is opt-in. Duplicate activity declarations, negative counts, invalid indexes and labels that cannot round-trip in the line format error. Unreferenced declarations have no representation in the core DFG.
- Model-I/O goldens compare counts and behavioural footprints, not generated ids. SampleNet is excluded from generation and its footprints are unavailable; import/round-trip still runs. The seven inhibitor/reset fixtures have unbounded counters and record null footprints with status `unbounded`. Petri footprint exploration otherwise caps at 10,000 markings; a42 records null with status `state_space_limit`. Finite ordinary nets use pm4py's ClassicSemantics; imported special arc kinds are checked separately and retain the core's inhibitor/reset firing rules.
- BPMN import keeps edge waypoints; pm4py drops them. Nodes and flows keep document order; pm4py stores them in sets. A diagram without a `process` element gets process id `""`, and so do elements outside any process, where pm4py uses a random UUID. Repeated element ids, flows that name an unknown node and non-numeric bounds are errors; pm4py raises `AttributeError` on an unknown node.
- BPMN export uses the stored bounds and waypoints, else pm4py's default layout (bounds 0, 0, 100, 100 and two waypoints at the origin). There is no Graphviz auto-layout, and the diagram and plane ids are fixed (`id_diagram`, `id_plane`). Several processes need a collaboration node, as in pm4py, but ichnos returns an error where pm4py raises `UnboundLocalError`. Text annotations get no `incoming` or `outgoing` children; pm4py appends them to the node written before the annotation.
- BPMN export writes numbers in Rust's shortest form: a width of 100.0 is written `100`, and 1e20 as `100000000000000000000`, where pm4py writes `100.0` and `1e+20`. The bytes differ from pm4py's; the values read back the same.

### ichnos-io (OCEL)

- **One reader per layout.** `read_ocel_json` and `read_ocel_xml` read the OCEL 1.0 layouts, `read_ocel2_json` and `read_ocel2_xml` the OCEL 2.0 layouts. pm4py's `ocel20_rustxes` variants call an external library for the same layouts and are not ported separately.
- **Choosing a reader by name.** `read_ocel` and `read_ocel2` follow pm4py's tests on the lowercased name. `read_ocel` reads names ending in `jsonocel` or `xmlocel`, without `.gz`. `read_ocel2` reads names ending in `xml`, `xmlocel`, `json` or `jsonocel`, each optionally followed by `.gz`. The CSV, SQLite and OCEL 2.0 bundle formats return an error until they are ported, so both rows stay `todo`.
- **XML limits.** The XML readers take `OcelReadOptions`, with a nesting limit of 128 and an element limit of 1,000,000 by default, as the PNML reader does. pm4py has no limits. A large log needs a higher `max_nodes`.
- **Relation order.** pm4py's OCEL 2.0 JSON reader orders each event's relations by Python set iteration, which depends on string hashing. `read_ocel2_json` keeps the order in which the event's relationships first name each object.
- **Typed XML values.** pm4py's OCEL 1.0 XML reader converts only tags whose name contains `float` or `date`, and keeps every other value as a string. Its OCEL 2.0 XML reader does the same with the declared types, so the standard's `integer`, `boolean` and `time` values stay strings. ichnos also converts `int` and `long` to integers, `bool` to booleans, `double` to floats and `time` to dates, in both layouts. A value that does not convert stays a string, where pm4py's OCEL 1.0 reader raises for a `float` or `date` value. An OCEL 2.0 `float` value of `null` is 0 in pm4py; ichnos keeps the string `null`.
- **JSON values keep their JSON type.** pandas turns an integer column with gaps into floats; ichnos keeps each value's own type.
- **Timestamps.** ichnos ports pm4py's default parser (`strpfromiso`: Python 3.12's `datetime.fromisoformat`, then UTC), with microsecond precision. Three differences remain:
  - When the `ciso8601` package is installed, pm4py uses it instead. ichnos follows `strpfromiso`.
  - pm4py's OCEL 2.0 readers try `dateutil` on text that `fromisoformat` rejects: the XML reader for every time, the JSON reader for `date` and `time` attributes. ichnos rejects such an event or change time as an error, and keeps such an attribute as a string.
  - pm4py parses OCEL 1.0 JSON object-change times with pandas `to_datetime`. ichnos parses them as event times.
- **Missing data.** An object change without a time, which pm4py records with a missing timestamp, is left out of OCEL 2.0 JSON. A repeated OCEL XML object attribute without a time is an error, where pm4py fails to parse it. An XML relation to an unknown object is left out, where pm4py's OCEL 1.0 reader raises `KeyError`. Timestamps that do not parse are errors.
- **Globals.** OCEL 1.0 JSON globals become nested attribute containers, with list items keyed by the empty string.
- **Repeated ids.** OCEL 1.0 JSON keys events and objects by id. A repeated id keeps its first position and its last value, as pm4py's `dict` does. The other layouts list events and objects, and keep every row with a repeated id, as pm4py does. See the ichnos-ocel Behaviour changes for how `Ocel` resolves them.
- **Writers prepare a copy.** Each writer clones the log and runs `make_consistent` and `retain_related` on the clone, as pm4py's writers do on the log itself. pm4py changes the caller's log; ichnos leaves it unchanged.
- **Attribute order.** pm4py writes attributes in pandas column order, which depends on the reader that built the log. ichnos writes them in the order they first appear in the rows. The golden tests compare these outputs modulo attribute order, and require the same bytes where the two orders agree.
- **Attribute types.** pm4py takes each attribute's type from the pandas column dtype. ichnos infers it from the values with pandas' rules: all integers give an integer column, integers with a gap give floats, mixed values give an object column.
- **Attribute names.** pm4py's OCEL 1.0 layouts list every attribute column in `attribute-names`, including a column with no values. ichnos lists only names that have a value.
- **File names.** pm4py appends `.jsonocel`, `.xmlocel`, `.json` or `.xml` to a path that lacks the extension. ichnos writes to the path it is given. A path ending in `.gz` is compressed with gzip.
- **Encoding.** The writers write UTF-8 only; pm4py takes an `encoding` parameter. The XML writers return an error for a control character, U+FFFE or U+FFFF, as lxml does.
- **Text times.** pm4py's OCEL 2.0 XML reader can leave an object-change time as a string. pm4py's `write_ocel2_xml` then raises `AttributeError`, and its JSON writers copy the text. ichnos parses every time when it reads, so its writers always write a time.
- **Read, then write.** The `ocel/write-*` goldens of the fixture files hold pm4py's read-then-write output. ichnos gives the same files when it reads the fixture with its own reader, apart from the reader differences above: the typed XML values change the attribute types in `typed.xmlocel` and `typed20.xmlocel`, and the OCEL 2.0 JSON relation order changes the order of each event's relations.

### ichnos-discovery (DFG)

- Frequency and performance DFGs preserve input event order, following the EventLog path; the DataFrame path sorts by case and timestamp.
- Completion timestamps are the default starts even for custom keys; interval options explicitly enable the start key.
- Typed weekly `BusinessHours` slots and excluded dates replace Python work-calendar objects.
- Elapsed performance gaps retain nanosecond precision.

### ichnos-discovery (batches and correlation)

- Both APIs accept canonical EventLog/EventKeys and typed options/results, with positional errors for missing event fields. Starts default to completions even with custom keys; interval options enable the start key. Timestamp arithmetic shares `ichnos_stats::time::datetime_timestamp`: integer microseconds are converted once, as in Python `datetime.timestamp()`. Submicrosecond digits are truncated; other nanosecond duration APIs keep their precision. Temporal discovery/conformance already share exact duration subtraction and do not convert epoch timestamps.
- Overlap sorting uses endpoints followed by sorted event identities as a total order; Python proper-subset ties remain only in heap operations. Equal positive-length intervals overlap regardless of tie order.
- Batch resources and trace case IDs must be strings or IDs. Case identity defaults to the trace concept:name attribute. Batch distance must be finite/nonnegative and minimum size positive. Empty traces need no case attribute.
- Correlation empty input returns empty graphs instead of a wrapper indexing error. The implemented variant is classic; split and trace-based variants are deferred.
- Correlation uses microlp continuous transportation solving. Uniform-cost matrices choose the diagonal optimal vertex. General tied optima can have backend-dependent edges. Non-integral solver values return `Error::CorrelationSolver` rather than pm4py's unconditional rounding. Receipt goldens retain the default Python backend's higher-cost flow and compare exact maps to the same pm4py miner configured with SciPy HiGHS, whose optimum agrees with ichnos.

### ichnos-core

Rows cite these as `core-N`.

1. **Sorting keeps empty traces.** pm4py's `sort_timestamp_log` drops them. ichnos places them after all other traces.
2. **Sorting fails cleanly.** An event without a date under the key gives `Error::MissingAttribute` or `Error::AttributeType`, and the log is unchanged. pm4py raises `KeyError` or `TypeError` part-way through.
3. **Dates keep their UTC offset.** pm4py converts every date to UTC on read. ichnos stores `DateTime<FixedOffset>`. The instant is the same, and comparison and sorting use the instant.
4. **No `properties` dict on logs.** Keys are always passed as `&EventKeys`.
5. **`from_arrow` sets no log attributes.** pm4py's DataFrame-to-stream conversion sets `attributes={"origin": "csv"}`.
6. **Extension detection order.** `from_arrow` declares standard extensions in the order their prefixes first appear in column names. pm4py adds them from a Python `set`, so its order depends on the hash seed.
7. **Case prefix stripping.** Only the leading `case:` is removed from a column name. pm4py's `str.replace` removes every occurrence.
8. **Case grouping key.** It follows Python dict semantics: a string and an ID with the same text are one case, and so are `1` and `1.0`. Booleans stay separate; in Python `True == 1`.
9. **Nulls.** A null in an Arrow column means the attribute is absent. pm4py keeps NaN values in events unless `stream_postprocessing` is on.
10. **`to_arrow` column types.** A column of incompatible types (for example int and bool) becomes `Utf8`, with values formatted as Python's `str()`; pandas keeps an object column. Lists and containers are an error. Meta-attributes are dropped. Dates outside 1677 to 2262 are an error. Timestamps are stored as nanoseconds; pandas reports `datetime64[us, UTC]` for the running example.
11. **`to_event_stream` does not change the log.** pm4py's `to_data_frame` writes the `case:` attributes into the original events.
12. **`from_trace_strings` timestamps** start at 10,000,000 seconds after the epoch in UTC. pm4py uses the local time zone, then labels the result UTC. The two agree when the local time zone is UTC. pm4py returns a pandas DataFrame; `from_trace_strings` returns an `EventLog`, and `from_trace_strings(...).to_arrow(...)` gives the table (see core-10).
13. **Non-string activities** are formatted as Python's `str()` in `activity_sequences` and `variants`. pm4py keeps the raw value.
14. **`insert_classifier_attribute`** formats every value with `str()`, as `set_classifier` does on a log, and changes nothing if an event lacks a key. pm4py raises part-way through.
15. **Variants come in first-appearance order.** pm4py sorts them by count.
16. **`to_arrow` drops log metadata.** Log attributes, extensions, globals and classifiers are not written. pm4py keeps only `log.properties`, in `df.attrs`.
17. **Sampling uses ChaCha8 with an explicit seed** and returns the sample in input order. pm4py uses Python's `random`, so samples differ; only sizes match.
18. **`sample_events` exists only on `EventStream`.** pm4py's `sample_events` on a log samples traces, which `sample_cases` does.
19. **`to_interval` has no business-hours option.** That belongs with the performance crate.
20. **`format_batch` parses only the timestamp and start-timestamp columns.** pandas tries every string column.
21. **An unparseable timestamp is an error** (`Error::UnparseableTimestamp`). pandas leaves the column as strings. A numeric timestamp column is also an error (`Error::UnsupportedColumn`); pandas leaves it as numbers.
22. **No day-first or month-first guessing.** Without `timestamp_format`, only ISO 8601 and RFC 3339 forms parse.
23. **The lifecycle instance key is fixed to `concept:instance`; pm4py takes it as a parameter.**
24. **The `hof` functions take Rust closures.** Filters return a copy with the metadata; sorts work in place, stably, and need an `Ord` key.
25. **`Trace::sort_events_by_key` keeps the trace attributes.** pm4py's `sort_trace` drops them.
26. **`insert_artificial_start_end` works on `EventLog` only.** pm4py's DataFrame version shifts timestamps by one millisecond and sorts the table; ichnos shifts by one second, as pm4py's log version does. A first or last event whose timestamp is not a date is an error, and the log is unchanged.
27. **`to_graph` returns a petgraph `DiGraph`.** Case, event and attribute nodes are keyed apart, so an attribute value whose text is `CASE=1` does not merge with a case node as it does in NetworkX. The case ID is always the trace's `concept:name`. A list or container attribute is an error.
28. **`project` gives `None` for a missing attribute.** pm4py raises `KeyError`.
29. **`set_classifier` is two methods**: `insert_classifier_attribute` takes attribute keys and `insert_named_classifier_attribute` takes a log classifier name. Neither records the activity key on the log (see core-4).
30. **`serialize` and `deserialize` are not ported.** The `ichnos-io` readers and writers give and take the bytes for each format.

### ichnos-model (BPMN)

1. **`ProcessTree::to_bpmn` rejects interleaving** with `UnsupportedOperator`. pm4py drops the node and its subtree without a warning.
2. **A one-child sequence reaches the end of its block.** `ProcessTree::to_bpmn` connects it to the following node; pm4py's tau chaining leaves it unconnected.
3. **`Bpmn::reduce_xor_gateways` keeps an exclusive gateway whose only flow is a self-loop.** pm4py tries to splice it out on every pass and never ends.
4. **Inclusive-gateway skips break ties by place name.** When two places before a converging inclusive gateway are equally near, `Bpmn::to_petri_net` links the `<place>_skip` transition to the smallest name. pm4py's choice depends on set order.
5. **The element maps of `Bpmn::to_petri_net` leave out what reduction removed.** pm4py turns reduction off when it returns the maps.
6. **Generated ids are counters, not UUIDs.** Nodes are `id_<n>`, flows `flow_<n>`, and the helper transitions of the Petri net conversion are named after their node (`<id>_start`, `<id>_end`, `<place>_skip`). pm4py uses random UUIDs.

### ichnos-model (petri-analysis)

1. **`AcceptingPetriNet::is_sound` tries POWL conversion first.** A successful conversion returns true; otherwise Woflan decides. This preserves the oracle verdict on and-split-xor-join and murata3 even though Woflan finds them unsound.
2. **Woflan stops early, as pm4py's `check_soundness` does.** Only steps 1, 2, 3, 10 and 11 run; the steps for the full diagnosis (not-well-handled pairs, the minimal coverability graph, unboundedness sequences) are not ported, because `check_soundness` and `check_is_sound` never reach them.
3. **Dead tasks come from the reachability graph, not the minimal coverability graph.** Woflan reaches step 10 only for S-coverable nets, which are bounded, so both graphs fire the same transitions.
4. **Linear and integer programs use `microlp`.** pm4py uses scipy (HiGHS), CVXOPT with GLPK, or PuLP. Feasibility and optimal values agree, but the solutions themselves can differ: on `big_wf_net` Woflan's uniform invariants, and so the uncovered places it lists, are not pm4py's.
5. **Marking-equation optima are rounded, not truncated.** ichnos rounds down after adding 1e-6; pm4py truncates the float, so it gives 3 for GLPK's 3.9999999999963 on `sync-running-example-3`, where ichnos gives 4.
6. **`maximal_decomposition` keeps the transition whose name sorts last for a duplicated label.** pm4py's pick follows memory addresses and changes from run to run. `maximal_decomposition_with` takes the pick as a function. Components come in id order.
7. **`extended_cyclomatic` is ported as pm4py computes it.** pm4py's graph joins each state to the name of each transition leaving it, not to the next state, so the metric is the number of distinct (state, transition name) pairs, not the cyclomatic number of the reachability graph.
8. **Synchronous products keep the model's arc weights and kinds.** pm4py copies every arc as a normal arc of weight 1. Product nodes are named `"(x, y)"` strings, not tuples.
9. **`marking_from_names` returns `AnalysisError::UnknownPlace`** for a name no place has; pm4py raises `KeyError`.

### ichnos-discovery (prefix tree)

- Canonical EventLog/EventKeys and core positional activity errors replace pm4py's variant extraction. Input event order is retained; timestamps and case identifiers are unnecessary.
- Optional unsigned path limits express unlimited and nonnegative lengths. Negative limits are unrepresentable: pm4py's negative slice limits drop that many activities from the end.
- Children use stable lexical label order instead of pm4py variant/set iteration. First-creation arena indices replace cyclic parent objects, allowing long traces without recursive construction or drop.
- Distinct prefixes retain parent indices, depth and pm4py final flags. Frequencies are absent. Empty traces and zero limits leave the root non-final, as in pm4py's log_to_trie.apply.

### ichnos-discovery (split miner)

- One native Rust pipeline shares frequency filtering, split hierarchy, ordered triconnected-component/RPST joins and gateway normalization across classic and SM2. No Python runtime dependency. Defaults remain classic, epsilon 0.1, eta 0.4 and inclusive-join minimization enabled; SM2 fixes eta to one and always handles OR gateways.
- Input is canonical EventLog/EventKeys. Core activity coercion and positional errors replace pm4py activity skipping; classic uses recorded event order and ignores empty traces. SM2 uses lifecycle transitions, stable completion-timestamp sorting at Python datetime microsecond precision when every event has a timestamp, and recorded order otherwise. Empty logs are typed errors; SM2 retains empty traces as a start/end path. Thresholds must be finite fractions in [0,1], except eta is ignored for SM2 as in pm4py.
- Arena identities keep synthetic boundaries/gateways separate from equally named activities. Stable arena identities replace pm4py task-label, gateway-counter and loop naming schemes; deterministic flow IDs replace UUID flow IDs (RPST virtual edges also use UUIDs in pm4py). Complete BPMN node kinds, task labels, flow multiplicities and SM2 loop markers are checked up to graph isomorphism, rather than comparing IDs or layout. A detailed result exposes informational self-loop task IDs instead of Python's private _sm_looped attribute; those markers do not add flows.
- Both public variants are covered on all five real fixtures and synthetic empty, sequence, XOR/parallel, self/short-loop, nested/rigid, Unicode/custom-key, lifecycle overlap/OR/sorting/fallback cases and eight deterministic generated trace sets. The precomputed-DFG discovery entry point is not ported; paths can be loaded separately through reader APIs; no soundness or boundedness guarantee is added.

- Thirty-three pm4py goldens compare complete graphs for five classic settings (including eta varied independently at epsilon 0.1) and four distinct SM2 epsilon settings (0, 0.1, 0.5 and 1). A three-branch lifecycle case reaches SM2 OR-split promotion and matching inclusive join; SM2 ignores eta and join-minimization settings.

### ichnos-discovery (transition system)

- Canonical EventLog/EventKeys inputs use positional core errors, typed direction/abstraction and an unsigned window. Event order is retained; timestamps and case IDs are unnecessary. Empty traces contribute no states.
- TransitionView Display supplies stable Python-style names with escaped single-quoted labels; sets and counters use lexical label order. TransitionDiscovery retains structured identities, and typed event positions replace whole-trace references.
- pm4py TransitionSystem.State.__hash__ hashes str(name), so equal Counters with different insertion orders may survive as duplicate states and edges. Its first self-loop may attach target data to a detached equal state. Rust interns views before constructing edges and retains both endpoints' annotations. The oracle merges equal views and aggregates state annotations from edge events, with separate direct state.data comparisons for unique states without self-loops.
- Ten goldens cover 265 option runs. Running-example and synthetics cover direction/view and zero/one/two/large windows; receipt and its two CSV partitions use three forward/two-window abstractions to keep complete graphs compact.

### ichnos-viz (drawings)

- There is no viewer. Each `view_*` function maps to a `*_dot` function that returns the DOT text, and each `save_vis_*` function to a `write_*` function.
- `write_*` takes the format from the file extension. `.dot` and `.gv` get the DOT text itself; pm4py renders `.dot` through Graphviz, which adds layout positions. Other extensions run Graphviz `dot -T<ext>` and fail with `VizError::DotNotFound` when `dot` is not on the `PATH`. A path without an extension is an error. pm4py's `html` output is not supported.
- Node names are stable (`p0`, `t0`, `a0`, `n0`), not Python object ids. Goldens compare graphs up to isomorphism, with every node and edge attribute. Where pm4py's statement order does not depend on Python's set order (DFGs, a tie-free Petri net, BPMN nodes), they also check that ichnos writes nodes and edges in pm4py's order.
- Petri nets: pm4py reads data-net guards from the transitions' properties. `PetriNet` has no guards, so callers pass them in `PetriNetDotOptions::guards`; `ichnos_io::read_pnml` keeps them in `PnmlDocument::transition_data`. Elements at the same distance from the initial marking keep id order; pm4py takes them in Python's set order.
- DFGs: a start or end activity without edges keeps its node; pm4py raises `KeyError`. An edge activity missing from `activities_count` gets a plain node. An empty DFG gives a graph with no nodes; pm4py fails on `min()` of an empty sequence. Start and end edges come in activity order; pm4py follows the insertion order of its start and end dictionaries.
- BPMN: flows that tie in the breadth-first order keep the order of pm4py's networkx graph, by source node, but within a source they follow the file's flow elements. pm4py adds flows in the order of the diagram's edge shapes (`BPMNEdge`), which ichnos does not read, and flows without a shape in Python's set order.
- Heuristics nets: pm4py also takes start and end activities without counts and draws an unlabelled arc to each. `HeuristicsNet` always holds counts, so every start and end arc follows the `min_dfg_occurrences` rule. pm4py reads `min_dfg_occurrences` from the net; `HeuristicsNet` does not store it, so callers pass the discovery value in `HeuristicsNetDotOptions::min_dfg_occurrences`. `HeuristicsNet` has no net names or per-net colours, so merged nets draw black arcs labelled with counts only. Performance heuristics nets are not supported.
- Transition systems: states are named `s0`, `s1`, ... in state order, not by Python object ids.
- Alignment tables: each variant row shows that variant's own alignment. For a dataframe log, pm4py lists the alignments in another trace order than the table's variants, so rows can show another trace's alignment; the golden cases pass an `EventLog`. A variant without an alignment gets an empty row; pm4py fails on it.
- POWL:
  - pm4py's `save_vis_powl` takes a `graph_title` but its drawing never uses it, so `PowlDotOptions` has no title.
  - Nested blocks darken the background colour. pm4py darkens any colour matplotlib knows; ichnos darkens hex colours, `white` and `black`, and leaves other colour names unchanged.
  - pm4py turns a choice or loop between two silent steps into an activity labelled `None`; ichnos keeps and draws it.
  - An empty partial order has no node for an edge to point at, so it gets no edge; pm4py fails on it.
  - The icons ship inside the crate. `PowlDotOptions::icon_dir` sets the directory the image paths point into; pm4py points them into its install directory. `write_powl` writes the icons to a temporary directory for Graphviz and inlines them into `.svg` output.
  - pm4py's `save_vis_powl` renders SVG and converts it to PNG or PDF with `cairosvg`; it accepts no other format. `write_powl` runs `dot -T<format>` for any format. For PNG, PDF and the other non-SVG formats Graphviz needs an SVG image loader, such as the rsvg plugin, to draw the icons. Without one Graphviz warns and draws the model without them, and `write_powl` fails with `VizError::IconsNotLoaded`.
  - The goldens draw a copy of pm4py's model with the children of choices and partial orders sorted, because pm4py orders them by Python's set order and an edge into a cluster joins its first child.
- Object-centric drawings (OC-DFG, OCPN, object graph):
  - pm4py colours object types from Python's string hash, which changes from one process to the next. ichnos takes colours from `object_type_colors` and gives other types `object_type_color`: the first three bytes of the MD5 digest of the name. The goldens give pm4py the same colours.
  - ichnos does not discover OC-DFGs or OCPNs yet, so the drawings take `Ocdfg` and `OcPetriNet`, which hold what pm4py's drawings read from its dictionaries.
  - OC-DFG: `save_vis_ocdfg` passes the aggregation under a key the drawing does not read, so pm4py always takes the mean; ichnos uses `OcdfgDotOptions::aggregation`. pm4py has no standard deviation here and takes the mean, as ichnos does. With the performance annotation and the unique-objects edge metric pm4py raises; ichnos draws no edges between activities.
  - OCPN: a visible transition whose label is not among the activities gets its own box; pm4py fails.
  - Object graph: an object id that the OCEL does not hold is drawn without a colour; pm4py fails.
- Network analysis: two pm4py quirks are kept. Every node gets the same fill colour, since pm4py shades nodes against a range it has not computed yet. The pen-width range comes from a running minimum and maximum that never update both on one value, so it depends on the edge order. `save_vis_network_analysis` always takes the mean; ichnos uses `NetworkAnalysisDotOptions::aggregation`. Means and sums are rounded once from the exact sum; pm4py's `sum` adds in order, so a sum can differ in the last digit.
- Dotted chart:
  - pm4py picks the colours at random. ichnos takes them from `DottedChartDotOptions::colors`, keyed by the Python `str` of the value, and gives other values the MD5 colour. The goldens record pm4py's colours with its random generator seeded.
  - pm4py's default attributes fail: an `EventLog` raises `TypeError`, and a data frame needs a `@@case_index` column that pm4py does not add. `DottedChartAttributes::default()` reads `case:@@index` as the position of the trace in the log. The goldens name the attributes.
  - An event without one of the attributes is left out, and a chart without points draws the axes alone; pm4py fails on both. Values of mixed kinds, which Python cannot sort, are drawn as text.
  - pm4py takes a `bgcolor` but does not use it, so `DottedChartDotOptions` has none.
- Performance spectrum:
  - `performance_spectrum` ports the `log` discovery variant. `save_vis_performance_spectrum` takes the dataframe variant for a data frame, which samples at random above the sample size, so the goldens pass an `EventLog`.
  - pm4py regroups the filtered events by case id, so traces that share a case id merge; ichnos keeps the traces of the log. An event without an activity or timestamp is left out; pm4py fails.
  - Dates are written in UTC; pm4py writes them in the machine's local time zone. The goldens run pm4py in UTC.
  - pm4py fails on a spectrum without runs, on runs that all start and end at one instant, and on steps that all take as long. ichnos draws the activities alone, puts every point at the left, or colours every step grey.

### ichnos-ml (features)

- `split_train_test` takes the random draws as `rand_below(k)`, so a caller can reproduce pm4py's split from Python's `random._randbelow`. A `train_percentage` outside 0 to 1, or NaN, is an error; pm4py accepts any value.
- pm4py orders the feature columns of a data frame by Python's set order, which changes with the hash seed. ichnos puts numeric columns first, then string columns, each sorted by name. Values are rounded to 32-bit floats, as pm4py stores them.
- pm4py orders the per-activity `@@max_concurrent_activities_like_*` features, and the OCEL string-attribute features, by set order; ichnos sorts them.
- pm4py's one feature entry point is split in two: `extract_features_dataframe` for the data-frame branch and `trace_features` for the `EventLog` branch.
- The outcome-enriched result keeps the case features apart from the event attributes. pm4py merges them into one frame, so a feature that shares a name with an event column, such as `Costs`, becomes `Costs_y` and the column `Costs_x`.
- Temporal features are binned in UTC. An empty log gives no bins, and a log without the resource attribute counts zero resources; pm4py fails on both.
- `extract_target_vector` returns empty targets for an empty log; pm4py fails.
- OCEL features: numeric object attributes stored as text are parsed as Python's `float()` parses them. An event without related objects is an error, as in pm4py.

### ichnos-org (organizational mining)

- The social networks and roles port pm4py's `*_log` variants, which `pm4py.discover_*` takes for an `EventLog`; the goldens pass one. For a data frame pm4py takes its pandas variants instead. Those differ: the pandas working-together network divides by the number of events, not the number of traces.
- Resources, activities and the other attributes are compared and reported in their Python `str` form. An event without the resource or activity attribute is an error; pm4py raises `KeyError`.
- When no pair of resources is linked, pm4py raises `ValueError`, because it takes the largest value of an empty network; ichnos returns an empty network. `discover_subcontracting_network` with `n` 0 is an error, where pm4py raises the same `ValueError`.
- Subcontracting: pm4py counts only the first subcontracting window of each resource, in the first trace variant that opens one; later windows of the same resource are skipped. This is kept.
- Resource similarity: the Pearson correlation is NaN when a resource's activity profile is constant, as in pm4py, and when the log has fewer than two activities, where pm4py fails.
- Roles: the merge threshold is pm4py's default, 0.65, which `discover_organizational_roles` does not expose.
- Network analysis:
  - pm4py's `discover_network_analysis` accepts an `EventLog` in its signature but fails on one; ichnos takes an `EventLog`. A column that starts with `case:` reads the trace attribute.
  - pm4py sorts the events with pandas' default sort, which is not stable, so events with equal timestamps can link in any order. ichnos keeps them in log order. The goldens use logs where the order of ties does not change the result.
  - An event without the sent or received attribute links to nothing; pandas matches missing values with each other. A link without both timestamps has a NaN duration.
  - Durations are listed in the order of the source events; pm4py's order follows its table merges.
  - pm4py's business-hours parameters are not reachable from `discover_network_analysis` and are not ported.

### ichnos-stream (live streams and DFG)

- Live streams deliver synchronously in FIFO order instead of using pm4py's thread pool.
- Observers receive items in registration order.
- Appends buffer before start.
- Appends after finish are ignored.
- Stop drains an inactive queue, fixing the native hang in that state.
- Repeated starts return a typed error.
- Repeated stops are harmless.
- Repeated registration of the same shared observer is idempotent.
- Delivery continues after an observer error and returns the first error without rollback or retry.
- Collectors clone canonical items instead of retaining mutable Python references.
- StreamSink replaces the abstract StreamingAlgorithm interface.
- Algorithms expose typed get results.
- Delivering to an already borrowed shared observer returns a typed error.
- Missing DFG activity/case fields are ignored and counted by default.
- MissingEventPolicy::Reject returns an indexed error.
- DFG activity/case values use core display rather than Python str.
- Typed Dfg maps replace dictionary backends and string tuples/eval.
- DFG ends count each case's current last activity rather than completed cases.
- CSV readers retain string fields.
- CSV readers preserve input order.
- Unequal CSV record widths return a typed error instead of DictReader's missing/extra-field placeholders.
- CSV options add a configurable delimiter to the comma default.
- Path-backed reset reopens the file and closes its previous handle.
- Arbitrary reader inputs have no resettable path.
- Readers fuse after EOF or an error.
- Iterator adapters replace transformation and acceptance callbacks.
- CSV transformation adapters can run before filtering.
- XES filters see inherited trace attributes, whereas native event filtering runs before inheritance.
- XES event readers frame one event plus preceding trace metadata.
- XES trace readers frame one complete trace, including empty traces.
- XES attributes use the merged ichnos-io parser.
- Trace attributes overwrite case-prefixed event fields.
- Log declarations and globals are omitted from emitted items.
- XES readers accept plain and gzip files.
- XES readers preserve file order.
- Typed XES options control nesting depth and the case prefix.
- XES readers return errors for invalid XML/core values that native readers may log and skip.
- TraceIterator accepts canonical EventStream or Arrow input instead of pandas.
- TraceIterator projects case IDs and activity/timestamp values under standard XES keys.
- TraceIterator orders homogeneous numeric case IDs numerically, as np.unique does.
- TraceIterator orders homogeneous strings lexically, as np.unique does.
- Mixed core case types retain distinct identities, so integer 1 and string "1" are separate cases.
- Mixed-type ordering is an extension: booleans precede numbers, then strings, IDs, dates, lists, containers and metadata values.
- Numerically equal mixed integer/float IDs remain distinct, with the integer first.
- Floating signed zeros share a case, and NaNs share a case ordered after other floating values.
- Integer case comparisons retain precision above 2^53.
- TraceIterator preserves input order within cases rather than sorting timestamps.
- Canonical grouping corrects native contiguous slicing of interleaved rows after its discarded sort_values result.
- The interleaved golden records raw native output and native output on explicitly grouped input.
- No oracle implementation is patched.
- Missing projection columns return typed indexed errors.
- Pandas attributes and index metadata are not retained.
- Thirty-one goldens compare complete ordered content digests and typed samples.
- Golden dates normalize to UTC microseconds while core timestamps retain their original precision.
- DFG goldens compare up to five prefix snapshots.
- Running-example, receipt and roadtraffic100traces cover real XES, conversion and DFG paths.
- CSV goldens cover running-example, receipt, quoting, transforms and filters.
- Native dictionary, Redis and thread helpers are dropped in favor of typed maps.
- Platform click/key connectors are dropped because applications supply canonical events.
- Python stdout observers are dropped in favor of application sinks.

### ichnos-stream (streaming conformance)

- The three conformance consumers implement StreamSink over canonical events.
- Typed snapshots replace pandas diagnostics.
- Case snapshots are ordered lexically by displayed case ID.
- Missing fields default to counted skip.
- MissingEventPolicy::Reject returns an indexed error without changing case state.
- Case/activity keys use core display rather than Python str.
- TBR displays activity values before matching, whereas native TBR preserves their original types.
- Typed case maps replace dictionary backends and eval-based encoding.
- Application sinks replace native logging hooks.
- TBR retains one Marking per case and reuses merged PetriNet firing rules.
- TBR rejects inhibitor and reset arcs because native classic handling is ambiguous.
- TBR validates marking membership.
- TBR resolves transition and shortest-path ties by node name then ID rather than Python set/object-address order.
- The default ten-iteration silent limit includes the visible-firing attempt.
- A zero silent limit skips exploration.
- All TBR fallback token shortfalls are computed against the original marking.
- Native fallback tests the explored marking but inserts into the original marking, which can undercount or subtract missing tokens after partial silent moves.
- The capped partial-path regression pins missing=1 where native reports 0 for p0:1, p1:1, tau p1→p0, A consuming two p0 tokens and a one-iteration limit.
- A failed silent path falls back without committing partial moves, fixing native null-marking failures.
- Typed place IDs avoid native place-name encoding collisions.
- Unknown TBR activities are counted and ignored without creating a case.
- TBR termination does not implicitly fire final silent transitions.
- TBR termination returns the last replay marking.
- TBR retains native signed final-place missing differences, including negative values for final-place surplus.
- TBR retains native signed remaining differences across all places.
- TBR final fitness requires both signed counts to equal zero.
- These streaming termination diagnostics are distinct from batch replay fitness.
- The native final-path helper is dropped because terminate passes it an encoded string rather than a marking.
- TBR and footprints terminate_all return diagnostics instead of discarding them.
- Nonexistent cases return None instead of logging.
- Completed case IDs can restart with fresh state.
- Footprints uses the merged Footprints type with a separate end-activity set.
- Footprints checks start activities when the first modeled activity arrives.
- Footprints checks subsequent pairs against the sequence/parallel union.
- Footprints defers end checks until termination.
- Unknown footprints activities do not advance the last modeled activity.
- Unknown-first footprints deviations are retained rather than reset by subsequent events.
- Unknown-only footprints cases are visible and unfit rather than omitted.
- Footprints get_status returns None only when no open case is tracked.
- Footprints helper checks are consolidated into push.
- Temporal conformance reuses merged TemporalProfile and TemporalDeviation types.
- The streaming temporal default zeta is six.
- The default temporal start key equals the completion key.
- Each temporal arrival compares with every prior case completion no later than its start.
- Temporal deviations retain arrival-major, then prior-event order.
- Temporal checks require both timestamps even on the first event.
- Overlapping temporal pairs are excluded.
- Temporal histories retain all prior events rather than only adjacent ones.
- Temporal bounds are inclusive.
- Absent temporal profile pairs are ignored.
- Epoch seconds follow the native microsecond calculation plus representable submicrosecond core precision.
- Non-date timestamp values return typed errors rather than logged/swallowed exceptions.
- Nonfinite or negative zeta values are rejected rather than accepted unchecked.
- Profile means must be finite.
- Profile standard deviations must be finite and nonnegative.
- Zero-variance deviation zeta is infinity instead of sys.maxsize.
- Temporal get omits cases without deviations.
- Explicit temporal terminate releases history and deviations, an extension absent from native temporal streaming.
- Thirty conformance goldens compare up to five prefix snapshots.
- The three original real-log TBR cases use native-constructed activity chains.
- The running-example PNML TBR case feeds every real event through a model with two silent transitions.
- Unequal-length silent paths and duplicate labels enabled in different markings have separate deterministic TBR goldens.
- Real footprints cases use entire-log discovery and deliberately empty constraints.
- Real temporal cases use discovered profiles.
- Synthetic goldens cover interleaving, unknown/missing fields, custom keys, empty input, weighted arcs, silent limits, intervals and zero variance.
- Complete ordered digests with samples compact integer real-log snapshots and termination.
- Synthetic states and temporal deviations remain explicit.
- Round-trip JSON float parsing prevents artificial zero-variance deviations from one-ULP profile-mean changes.
- Native algorithm implementations are not patched.

### ichnos-stream (streaming Declare)

The Declare model types are reused from `ichnos-discovery`; moving them to `ichnos-model` is deferred.

- Typed model constraints replace Python template-name and activity-tuple dictionaries.
- Typed automaton states replace serialized tuple keys and state strings.
- Incorrect unary/binary rule arity returns a typed error before monitoring starts.
- Unknown template strings cannot enter the typed model; pm4py uses a dummy monitor for them.
- Ignore and Reject policies extend pm4py's default handling of incomplete events.
- Configurable case/activity/timestamp keys extend pm4py's hardcoded streaming Declare keys.
- Core display strings replace raw Python case/activity identity, so values with the same display text can share a case or label match.
- `remove_case` releases monitor state without end-of-case validation or changing historical totals.
- A removed case ID can be reused as a fresh case; pm4py has no case-removal API.
- `clear_history` releases event-level deviation records while retaining monitors and totals; pm4py has no history-clearing API.
- Typed snapshots and history replace pm4py's logging of violated template names.
- The shared synchronous `StreamSink` contract replaces pm4py's worker locking.

### Analysis entry points

- Model comparison takes a typed `comparison::Model` instead of heterogeneous positional arguments.
- Behavioral comparison lives in conformance so Petri nets can reuse its silent-routing footprint implementation.
- Behavioral similarity rejects DFG models, following pm4py's conversion restriction.
- Structural comparison preserves pm4py's comparison-specific POWL dispatch: partial-order nodes lose their ordering edges and become parallel.
- Frequent-transition annotations become literal leaf labels during structural comparison.
- Direct `Powl::to_process_tree` retains partial orders; the dispatch quirk is confined to comparison.
- Empty behavioral relations give similarity zero, including self-comparison, as in pm4py.
- Label matching uses Unicode SequenceMatcher blocks and its popular-character anchor rule.
- Equal-score label choices use lexical order instead of Python set iteration.
- Label thresholds must be finite and in [0,1].
- At threshold zero, zero-score candidates remain unmatched, avoiding pm4py's `remove(None)` failure.
- DFG label extraction and structural comparison convert through an accepting net.
- Direct DFG relabeling returns a typed error.
- `is_sound` preserves pm4py's POWL-first shortcut: a successful conversion returns true even if Woflan finds the net unsound.
- The `analysis/net-and-split-xor-join` and `analysis/net-murata3` goldens demonstrate that shortcut; `check_soundness` supplies the Woflan verdict.
- EMD totals use NumPy's isclose rule: absolute tolerance 1e-8 plus relative tolerance 1e-5 times the second total.
- A close positive second total is rescaled to the first total before constructing transport constraints.
- If either accepted total is zero, EMD returns zero.
- Larger mass differences return an error; POT can instead normalize unequal languages depending on installed dependencies.
- The transport optimizer uses microlp instead of POT or SciPy.
- Zero-mass support is ignored, and empty-trace self-distance is zero; pm4py can divide by zero for an empty trace.
- Clustering takes typed feature options and a `Clusterer` trait.
- Categorical presence, adjacent-value presence and last numeric event values use lexical feature order.
- Missing categorical values and absent adjacent pairs produce pm4py's `UNDEFINED` feature.
- `ProfileOptions::infer` scans the complete log instead of sampling up to 50 traces.
- Inference excludes case identifiers and lifecycle transitions, retains attributes present in every trace and selects strings with at most 12 distinct values.
- Default K-means uses two clusters, 300 iterations and variance-scaled tolerance 1e-4.
- Initial centers use the first row and successive farthest points instead of sklearn's seed-0 K-means++.
- The default partition can differ from pm4py's because feature selection and seeding differ; the running-example default golden records the comparison.
- Explicit centers reproduce the covered pm4py Lloyd fixtures.
- K-means assignment ties choose the lowest center index.
- Empty-cluster relocation chooses the highest row index among equally distant eligible points.
- Duplicate-only clusters can remain unused.
- Custom clusterers return typed assignments instead of sklearn objects.
- Invalid shapes, nonfinite numbers, out-of-range assignments and missing selected numeric values return errors.
- Cluster logs preserve source metadata; pm4py creates logs with default metadata.
- Cluster traces stay in input order and intermediate empty cluster indices are retained.
- The Gensim Word2Vec trainer is dropped; no alternative similarity replaces its independently trained random-walk vectors.
- Case-time enrichment returns an EventLog copy with aggregate values repeated on events instead of a DataFrame.
- Enrichment retains log metadata and trace/event order.
- Case identifiers support string/ID and integer values, with typed ordering for timestamp ties.
- Other case identifier types return errors.
- Repeated case identifiers are grouped across traces.
- Service time sums all individual intervals, including overlaps and negative durations.
- Waiting time can therefore be negative, as in pm4py.
- Finish enrichment uses the preceding finish, following pm4py's implementation instead of the top-level docstring's next-finish wording.
- Empty traces are preserved without enrichment.
- Empty logs return an empty copy; pm4py's DataFrame arrival helper fails on no cases.
- Enrichment requires date-valued timestamps.
- Computed durations use microsecond resolution; submicrosecond detail is not retained in the enrichment values.

## Proposed lanes

Each short heading is a lane slug. Packages group a coherent model, algorithm family or data operation; no package uses a fixed row limit. Complete foundational models before their I/O, miners and conformance consumers. Core log utilities and statistics can proceed once the log model exists; OCEL consumers depend on the OCEL model. Each listed row occurs in exactly one package. Backend-only dataframe rows preserve their operation through a shared Rust implementation. Reuse source dependencies already implemented by earlier packages; every port adds golden coverage for its rows.


### ichnos-stream (IWS alignments and OCEL distribution)

- IWS aligns against a finite proxy of complete model runs; valid completed paths provide upper bounds without establishing optimality or complete model coverage.
- Bounded deterministic breadth-first exploration ordered by transition name/ID replaces native seeded random simulation, so automatic proxies can differ.
- Rust automatic proxy/path/expansion limits default to 100/100/100000 and must be positive.
- Native path length defaults to max(100, four times the transition count); Rust's default path length is 100.
- Native simulation attempts default to max(1000, twenty times the requested proxy count); Rust bounds expanded and queued paths instead.
- No random seed or simulation-attempt option is exposed.
- `from_proxy_traces` reuses the merged exact Dijkstra aligner and does not reproduce native approximate-search expansion/time cutoffs or equal-cost heap ordering.
- `with_proxy_sequences` accepts validated complete transition runs for reproducible comparisons.
- Proxy preparation retains the first run per distinct visible trace, matching native deduplication.
- Matching trie paths, candidate ties and cheapest final suffixes follow proxy insertion order.
- `StreamingAlignmentStep` uses optional log activities and transition IDs instead of sentinel strings, preserving silent/duplicate transition identity.
- One result cost serves native cost, standard_cost and upper_bound.
- Typed results omit runtime measurements and redundant method/bound strings.
- One constructor family replaces variant dispatch because IWS is the sole alignment variant.
- Direct visible matches reset lifetime; log/look-ahead moves discount it according to native IWS rules.
- Candidate deduplication, caps and exhausted-state fallback retain native IWS rules.
- Floating settings must be finite; Rust allows a positive fractional decay lifetime.
- IWS rejects missing case/activity by default with an indexed typed error; native receive logs/swallows the exception.
- Optional Ignore counts incomplete-event skips.
- Core display canonicalizes case IDs and activities instead of Python value/string conventions.
- Truthy completion attributes finish after consuming the event.
- Completed results shadow later prefixes for that case, and finish retains active candidates, as native IWS does.
- Finishing an unknown case returns None instead of KeyError.
- Alignment histories grow with their case; the `remove_case` extension releases active/completed history and allows a clean restart.
- Invalid marking membership, special arcs and invalid/non-final supplied runs return typed errors.
- An empty complete proxy run is supported when initial equals final marking.
- The OCEL distributor accepts canonical iterator-style Event rows, without importing OCEL tables itself.
- Each object-type field is a typed List whose child values are object IDs; child keys are ignored.
- Object IDs and ordinary payload attributes retain core types.
- Object types route lexically instead of native dictionary insertion order.
- Objects keep List order and listeners keep registration order.
- Repeated IDs and listener registrations deliberately deliver repeatedly.
- Unregistered types produce no events.
- Required source fields and every object-type List are checked before any delivery; native unregistered values are unchecked.
- Destination keys must be distinct and the object prefix nonempty.
- Source activity/timestamp are removed before destination insertion, preserving values when keys coincide; native rename-then-delete loses them.
- Object-type names remove one leading prefix instead of native split indexing, preserving any repeated prefix in the type name.
- Synchronous delivery continues after sink errors and reports the first without rollback.
- Listeners can be shared live streams or any StreamSink.
- Seventeen new goldens cover all three required logs, a real PNML net with silent transitions, IWS boundary cases and OCEL routing.

### log-model-utils

Crate: `ichnos-core`. Rows: `pm4py.format_dataframe`, `pm4py.rebase`, `pm4py.set_classifier`, `pm4py.parse_event_log_string`, `pm4py.project_on_event_attribute`, `pm4py.sample_cases`, `pm4py.sample_events`, `pm4py.convert_to_event_log`, `pm4py.convert_to_event_stream`, `pm4py.convert_to_dataframe`, `pm4py.convert_log_to_networkx`, `pm4py.insert_artificial_start_end`, `pm4py.hof.sort_log`, `pm4py.hof.sort_trace`, `pm4py.hof.filter_log`, `pm4py.hof.filter_trace`.

Port sources: `pm4py/analysis.py`, `pm4py/convert.py`, `pm4py/hof.py`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/artificial`, `pm4py/objects/log/util/dataframe_utils`, `pm4py/objects/log/util/sampling`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/sampling`, `pm4py/streaming/conversion/from_pandas`, `pm4py/utils.py`.

### petri-model

Crate: `ichnos-model`. Rows: `pm4py.PetriNet`, `pm4py.Marking`.

Port sources: `pm4py/objects/petri_net/obj.py`, `pm4py/objects/petri_net/utils/petri_utils`.

### tree-model

Crate: `ichnos-model`. Rows: `pm4py.parse_process_tree`, `pm4py.ProcessTree`.

Port sources: `pm4py/objects/process_tree/obj`, `pm4py/objects/process_tree/obj.py`, `pm4py/objects/process_tree/utils/generic`, `pm4py/utils.py`.

### bpmn-model

Crate: `ichnos-model`. Rows: `pm4py.BPMN`.

Port sources: `pm4py/objects/bpmn/obj.py`.

### powl-model

Crate: `ichnos-model`. Rows: `pm4py.parse_powl_model_string`.

Port sources: `pm4py/objects/powl/obj`, `pm4py/objects/powl/parser`, `pm4py/utils.py`.

### ocel-model

Crate: `ichnos-ocel`. Rows: `pm4py.OCEL`.

Port sources: `pm4py/objects/ocel/constants`, `pm4py/objects/ocel/obj.py`.

### log-io

Ichnos-only table APIs: `ichnos_io::{read_csv, read_csv_from_reader, write_csv, write_csv_to_writer, read_parquet, read_parquet_from_reader, write_parquet, write_parquet_to_writer}`. Readers follow pandas plus `pm4py.format_dataframe`; pm4py has no corresponding public CSV/Parquet functions. CSV dates use RFC 3339 rather than pandas' space-separated form. Both writers reject list/container attributes through the core columnar conversion; metadata and empty traces cannot be represented. Automatic date parsing accepts supported formats per value, so mixed-format columns may convert where pandas would leave strings.

Crate: `ichnos-io`. Rows: `pm4py.read_xes`, `pm4py.write_xes`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/exporter/xes/exporter`, `pm4py/objects/log/importer/xes/importer`, `pm4py/objects/log/obj`, `pm4py/read.py`, `pm4py/write.py`.

### model-io

Crate: `ichnos-io`. Rows: `pm4py.read_pnml`, `pm4py.read_ptml`, `pm4py.read_dfg`, `pm4py.read_bpmn`, `pm4py.write_pnml`, `pm4py.write_ptml`, `pm4py.write_dfg`, `pm4py.write_bpmn`.

Port sources: `pm4py/objects/bpmn/exporter/exporter`, `pm4py/objects/bpmn/importer/importer`, `pm4py/objects/bpmn/layout/layouter`, `pm4py/objects/bpmn/obj`, `pm4py/objects/dfg/exporter/exporter`, `pm4py/objects/dfg/importer/importer`, `pm4py/objects/petri_net/exporter/exporter`, `pm4py/objects/petri_net/importer/importer`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/exporter/exporter`, `pm4py/objects/process_tree/importer/importer`, `pm4py/objects/process_tree/obj`, `pm4py/read.py`, `pm4py/write.py`.

### ocel-io

Crate: `ichnos-io`. Rows: `pm4py.read_ocel`, `pm4py.read_ocel_csv`, `pm4py.read_ocel_json`, `pm4py.read_ocel_xml`, `pm4py.read_ocel_sqlite`, `pm4py.read_ocel2`, `pm4py.read_ocel2_bundle`, `pm4py.read_ocel2_csv`, `pm4py.read_ocel2_json`, `pm4py.read_ocel2_sqlite`, `pm4py.read_ocel2_xml`, `pm4py.write_ocel`, `pm4py.write_ocel_csv`, `pm4py.write_ocel_json`, `pm4py.write_ocel_xml`, `pm4py.write_ocel_sqlite`, `pm4py.write_ocel2`, `pm4py.write_ocel2_bundle`, `pm4py.write_ocel2_csv`, `pm4py.write_ocel2_json`, `pm4py.write_ocel2_sqlite`, `pm4py.write_ocel2_xml`, `pm4py.connectors.extract_ocel_github`, `pm4py.connectors.extract_ocel_camunda_workflow`, `pm4py.connectors.extract_ocel_sap_o2c`, `pm4py.connectors.extract_ocel_sap_accounting`.

Port sources: `pm4py/connectors.py`, `pm4py/objects/ocel/exporter/bundled/exporter`, `pm4py/objects/ocel/exporter/csv/exporter`, `pm4py/objects/ocel/exporter/jsonocel/exporter`, `pm4py/objects/ocel/exporter/sqlite/exporter`, `pm4py/objects/ocel/exporter/xmlocel/exporter`, `pm4py/objects/ocel/importer/bundled/importer`, `pm4py/objects/ocel/importer/csv/importer`, `pm4py/objects/ocel/importer/jsonocel/importer`, `pm4py/objects/ocel/importer/sqlite/importer`, `pm4py/objects/ocel/importer/xmlocel/importer`, `pm4py/objects/ocel/obj`, `pm4py/read.py`, `pm4py/write.py`.

### log-serialization

Crate: `ichnos-core`. Rows: `pm4py.serialize`, `pm4py.deserialize`.

Port sources: `pm4py/objects/bpmn/exporter/exporter`, `pm4py/objects/bpmn/importer/importer`, `pm4py/objects/bpmn/obj`, `pm4py/objects/dfg/exporter/exporter`, `pm4py/objects/dfg/importer/importer`, `pm4py/objects/log/exporter/xes/exporter`, `pm4py/objects/log/importer/xes/importer`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/exporter/exporter`, `pm4py/objects/petri_net/importer/importer`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/exporter/exporter`, `pm4py/objects/process_tree/importer/importer`, `pm4py/objects/process_tree/obj`, `pm4py/utils.py`.

### model-conversion

Crate: `ichnos-model`. Rows: `pm4py.convert_to_bpmn`, `pm4py.convert_to_petri_net`, `pm4py.convert_to_process_tree`, `pm4py.convert_to_powl`, `pm4py.convert_to_reachability_graph`, `pm4py.convert_petri_net_to_networkx`, `pm4py.convert_petri_net_type`.

Port sources: `pm4py/convert.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/conversion/bpmn/variants/to_petri_net`, `pm4py/objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `pm4py/objects/conversion/genetic_matrix/variants/to_petri_net`, `pm4py/objects/conversion/heuristics_net/variants/to_petri_net`, `pm4py/objects/conversion/powl/converter`, `pm4py/objects/conversion/powl/variants/to_process_tree`, `pm4py/objects/conversion/process_tree/variants/to_bpmn`, `pm4py/objects/conversion/process_tree/variants/to_petri_net`, `pm4py/objects/conversion/process_tree/variants/to_powl`, `pm4py/objects/conversion/wf_net/variants/to_bpmn`, `pm4py/objects/conversion/wf_net/variants/to_powl`, `pm4py/objects/conversion/wf_net/variants/to_process_tree`, `pm4py/objects/genetic_matrix/obj`, `pm4py/objects/heuristics_net/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/petri_utils`, `pm4py/objects/petri_net/utils/reachability_graph`, `pm4py/objects/powl/obj`, `pm4py/objects/process_tree/obj`, `pm4py/objects/transition_system/obj`.

### petri-analysis

Crate: `ichnos-model`. Rows: `pm4py.construct_synchronous_product_net`, `pm4py.solve_marking_equation`, `pm4py.solve_extended_marking_equation`, `pm4py.analysis.check_is_sound`, `pm4py.check_soundness`, `pm4py.check_is_workflow_net`, `pm4py.maximal_decomposition`, `pm4py.simplicity_petri_net`, `pm4py.generate_marking`, `pm4py.reduce_petri_net_invisibles`, `pm4py.reduce_petri_net_implicit_places`, `pm4py.get_enabled_transitions`.

Port sources: `pm4py/algo/analysis/extended_marking_equation/algorithm`, `pm4py/algo/analysis/marking_equation/algorithm`, `pm4py/algo/analysis/woflan/algorithm`, `pm4py/algo/analysis/workflow_net/algorithm`, `pm4py/algo/evaluation/simplicity/variants/arc_degree`, `pm4py/algo/evaluation/simplicity/variants/extended_cardoso`, `pm4py/algo/evaluation/simplicity/variants/extended_cyclomatic`, `pm4py/analysis.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/semantics`, `pm4py/objects/petri_net/utils/align_utils`, `pm4py/objects/petri_net/utils/decomposition`, `pm4py/objects/petri_net/utils/murata`, `pm4py/objects/petri_net/utils/petri_utils`, `pm4py/objects/petri_net/utils/reduction`, `pm4py/objects/petri_net/utils/synchronous_product`.

### ocel-conversion

Crate: `ichnos-ocel`. Rows: `pm4py.convert_log_to_ocel`, `pm4py.convert_ocel_to_networkx`.

Port sources: `pm4py/convert.py`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/conversion/ocel/converter`, `pm4py/objects/log/obj`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/log_ocel`.

### log-attributes

Crate: `ichnos-stats`. Rows: `pm4py.get_start_activities`, `pm4py.get_end_activities`, `pm4py.get_event_attributes`, `pm4py.get_trace_attributes`, `pm4py.get_event_attribute_values`, `pm4py.get_trace_attribute_values`, `pm4py.statistics.attributes.common.get.get_sorted_attributes_list`, `pm4py.statistics.attributes.common.get.get_attributes_threshold`, `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute`, `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute_json`, `pm4py.statistics.attributes.common.get.get_kde_date_attribute`, `pm4py.statistics.attributes.common.get.get_kde_date_attribute_json`, `pm4py.statistics.attributes.log.get.get_events_distribution`, `pm4py.statistics.attributes.log.get.get_all_trace_attributes_from_log`, `pm4py.statistics.attributes.log.get.get_all_event_attributes_from_log`, `pm4py.statistics.attributes.log.get.get_attribute_values`, `pm4py.statistics.attributes.log.get.get_trace_attribute_values`, `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute`, `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute_json`, `pm4py.statistics.attributes.log.get.get_kde_date_attribute`, `pm4py.statistics.attributes.log.get.get_kde_date_attribute_json`, `pm4py.statistics.attributes.log.select.select_attributes_from_log_for_tree`, `pm4py.statistics.attributes.log.select.check_trace_attributes_presence`, `pm4py.statistics.attributes.log.select.check_event_attributes_presence`, `pm4py.statistics.attributes.log.select.verify_if_event_attribute_is_in_each_trace`, `pm4py.statistics.attributes.log.select.verify_if_trace_attribute_is_in_each_trace`, `pm4py.statistics.end_activities.common.get.get_sorted_end_activities_list`, `pm4py.statistics.end_activities.common.get.get_end_activities_threshold`, `pm4py.statistics.end_activities.log.get.get_end_activities`, `pm4py.statistics.start_activities.common.get.get_sorted_start_activities_list`, `pm4py.statistics.start_activities.common.get.get_start_activities_threshold`, `pm4py.statistics.start_activities.log.get.get_start_activities`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/sampling`, `pm4py/statistics/attributes/common/get`, `pm4py/statistics/attributes/common/get.py`, `pm4py/statistics/attributes/log/get`, `pm4py/statistics/attributes/log/get.py`, `pm4py/statistics/attributes/log/select.py`, `pm4py/statistics/end_activities/common/get.py`, `pm4py/statistics/end_activities/log/get`, `pm4py/statistics/end_activities/log/get.py`, `pm4py/statistics/start_activities/common/get.py`, `pm4py/statistics/start_activities/log/get`, `pm4py/statistics/start_activities/log/get.py`, `pm4py/stats.py`.

### log-variants

Crate: `ichnos-stats`. Rows: `pm4py.get_variants`, `pm4py.get_variants_as_tuples`, `pm4py.split_by_process_variant`, `pm4py.get_variants_paths_duration`, `pm4py.get_stochastic_language`, `pm4py.get_frequent_trace_segments`, `pm4py.statistics.chaotic_activities.algorithm.apply`, `pm4py.statistics.chaotic_activities.variants.niek_sidorova.apply`, `pm4py.statistics.chaotic_activities.variants.niek_sidorova.chaotic_metrics`, `pm4py.statistics.chaotic_activities.variants.niek_sidorova.total_entropy`, `pm4py.statistics.rework.cases.log.get.apply`, `pm4py.statistics.rework.log.get.apply`, `pm4py.statistics.variants.log.get.get_language`, `pm4py.statistics.variants.log.get.get_variants`, `pm4py.statistics.variants.log.get.get_variants_along_with_case_durations`, `pm4py.statistics.variants.log.get.get_variants_from_log_trace_idx`, `pm4py.statistics.variants.log.get.get_variants_sorted_by_count`, `pm4py.statistics.variants.log.get.convert_variants_trace_idx_to_trace_obj`, `pm4py.statistics.variants.pandas.get.get_variants_count`, `pm4py.statistics.variants.pandas.get.get_variants_set`, `pm4py.statistics.variants.polars.get.pandas_numpy_variants_apply_polars`, `pm4py.statistics.variants.polars.get.get_variants_count`, `pm4py.statistics.variants.polars.get.get_variants_set`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/pandas_numpy_variants`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`, `pm4py/statistics/chaotic_activities/algorithm.py`, `pm4py/statistics/chaotic_activities/variants/niek_sidorova.py`, `pm4py/statistics/rework/cases/log/get.py`, `pm4py/statistics/rework/log/get.py`, `pm4py/statistics/variants/log/get`, `pm4py/statistics/variants/log/get.py`, `pm4py/statistics/variants/pandas/get.py`, `pm4py/statistics/variants/polars/get.py`, `pm4py/stats.py`.

### log-case-stats

Crate: `ichnos-stats`. Rows: `pm4py.get_minimum_self_distances`, `pm4py.get_minimum_self_distance_witnesses`, `pm4py.get_case_arrival_average`, `pm4py.get_rework_cases_per_activity`, `pm4py.get_case_overlap`, `pm4py.get_cycle_time`, `pm4py.get_service_time`, `pm4py.get_all_case_durations`, `pm4py.get_case_duration`, `pm4py.get_activity_position_summary`, `pm4py.statistics.traces.cycle_time.log.get.apply`, `pm4py.statistics.traces.cycle_time.util.compute.cycle_time`, `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration`, `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration_json`, `pm4py.statistics.traces.generic.log.case_arrival.get_case_arrival_avg`, `pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg`, `pm4py.statistics.traces.generic.log.case_statistics.get_variant_statistics`, `pm4py.statistics.traces.generic.log.case_statistics.get_cases_description`, `pm4py.statistics.traces.generic.log.case_statistics.index_log_caseid`, `pm4py.statistics.traces.generic.log.case_statistics.get_events`, `pm4py.statistics.traces.generic.log.case_statistics.get_all_case_durations`, `pm4py.statistics.traces.generic.log.case_statistics.get_first_quartile_case_duration`, `pm4py.statistics.traces.generic.log.case_statistics.get_median_case_duration`, `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration`, `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_and_list`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_with_case_duration`, `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df_and_list`, `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df`.

Port sources: `pm4py/algo/discovery/minimum_self_distance/algorithm`, `pm4py/algo/discovery/minimum_self_distance/utils`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/overlap/cases/log/get`, `pm4py/statistics/rework/log/get`, `pm4py/statistics/service_time/log/get`, `pm4py/statistics/traces/cycle_time/log/get`, `pm4py/statistics/traces/cycle_time/log/get.py`, `pm4py/statistics/traces/cycle_time/util/compute`, `pm4py/statistics/traces/cycle_time/util/compute.py`, `pm4py/statistics/traces/generic/common/case_duration`, `pm4py/statistics/traces/generic/common/case_duration.py`, `pm4py/statistics/traces/generic/log/case_arrival`, `pm4py/statistics/traces/generic/log/case_arrival.py`, `pm4py/statistics/traces/generic/log/case_statistics`, `pm4py/statistics/traces/generic/log/case_statistics.py`, `pm4py/statistics/traces/generic/pandas/case_statistics.py`, `pm4py/statistics/traces/generic/polars/case_statistics.py`, `pm4py/statistics/variants/log/get`, `pm4py/stats.py`.

### log-time-stats

Crate: `ichnos-stats`. Rows: `pm4py.statistics.concurrent_activities.log.get.apply`, `pm4py.statistics.concurrent_activities.polars.get.get_concurrent_events_dataframe`, `pm4py.statistics.eventually_follows.log.get.apply`, `pm4py.statistics.eventually_follows.polars.get.get_partial_order_dataframe`, `pm4py.statistics.eventually_follows.uvcl.get.apply`, `pm4py.statistics.overlap.cases.log.get.apply`, `pm4py.statistics.overlap.interval_events.log.get.apply`, `pm4py.statistics.overlap.utils.compute.apply`, `pm4py.statistics.passed_time.log.algorithm.apply`, `pm4py.statistics.passed_time.log.variants.post.apply`, `pm4py.statistics.passed_time.log.variants.pre.apply`, `pm4py.statistics.passed_time.log.variants.prepost.apply`, `pm4py.statistics.service_time.log.get.apply`.

Port sources: `pm4py/algo/discovery/dfg/variants/native`, `pm4py/algo/discovery/dfg/variants/performance`, `pm4py/algo/discovery/inductive/dtypes/im_ds`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/sorting`, `pm4py/statistics/concurrent_activities/log/get.py`, `pm4py/statistics/concurrent_activities/polars/get.py`, `pm4py/statistics/eventually_follows/log/get.py`, `pm4py/statistics/eventually_follows/polars/get.py`, `pm4py/statistics/eventually_follows/uvcl/get.py`, `pm4py/statistics/overlap/cases/log/get.py`, `pm4py/statistics/overlap/interval_events/log/get.py`, `pm4py/statistics/overlap/utils/compute`, `pm4py/statistics/overlap/utils/compute.py`, `pm4py/statistics/passed_time/log/algorithm.py`, `pm4py/statistics/passed_time/log/variants/post.py`, `pm4py/statistics/passed_time/log/variants/pre.py`, `pm4py/statistics/passed_time/log/variants/prepost.py`, `pm4py/statistics/service_time/log/get.py`.

### stats-process-cube

Crate: `ichnos-stats`. Rows: `pm4py.get_process_cube`, `pm4py.statistics.process_cube.pandas.algorithm.apply`, `pm4py.statistics.process_cube.pandas.variants.classic.apply`, `pm4py.statistics.process_cube.polars.algorithm.apply`, `pm4py.statistics.process_cube.polars.variants.classic.apply`.

Port sources: `pm4py/statistics/process_cube/pandas/algorithm`, `pm4py/statistics/process_cube/pandas/algorithm.py`, `pm4py/statistics/process_cube/pandas/variants/classic.py`, `pm4py/statistics/process_cube/polars/algorithm`, `pm4py/statistics/process_cube/polars/algorithm.py`, `pm4py/statistics/process_cube/polars/variants/classic.py`, `pm4py/stats.py`.

### stats-util

Crate: `ichnos-stats`. Rows: `pm4py.statistics.util.times_bipartite_matching.exact_match_minimum_average`.

Port sources: `pm4py/statistics/util/times_bipartite_matching.py`.

### log-filters

Goldens `filters-log-*` compare every case ID and event index against pm4py’s EventLog path, using sorted-row SHA-256 summaries and samples for larger logs and full rows for running-example and synthetic cases. One EventLog implementation preserves original case order and metadata. Core activity/scalar/null conventions apply; invalid bounds return typed errors. DataFrame-path differences are named in the rows below.

Crate: `ichnos-stats`. Rows: `pm4py.filter_log_relative_occurrence_event_attribute`, `pm4py.filter_start_activities`, `pm4py.filter_end_activities`, `pm4py.filter_event_attribute_values`, `pm4py.filter_trace_attribute_values`, `pm4py.filter_variants`, `pm4py.filter_directly_follows_relation`, `pm4py.filter_eventually_follows_relation`, `pm4py.filter_time_range`, `pm4py.filter_between`, `pm4py.filter_case_size`, `pm4py.filter_case_performance`, `pm4py.filter_activities_rework`, `pm4py.filter_paths_performance`, `pm4py.filter_variants_top_k`, `pm4py.filter_variants_by_coverage_percentage`, `pm4py.filter_prefixes`, `pm4py.filter_suffixes`, `pm4py.filter_four_eyes_principle`, `pm4py.filter_activity_done_different_resources`, `pm4py.filter_trace_segments`, `pm4py.filter_dfg_activities_percentage`, `pm4py.filter_dfg_paths_percentage`.

Port sources: `pm4py/algo/filtering/dfg/dfg_filtering`, `pm4py/algo/filtering/log/attributes/attributes_filter`, `pm4py/algo/filtering/log/between/between_filter`, `pm4py/algo/filtering/log/cases/case_filter`, `pm4py/algo/filtering/log/end_activities/end_activities_filter`, `pm4py/algo/filtering/log/ltl/ltl_checker`, `pm4py/algo/filtering/log/paths/paths_filter`, `pm4py/algo/filtering/log/prefixes/prefix_filter`, `pm4py/algo/filtering/log/rework/rework_filter`, `pm4py/algo/filtering/log/start_activities/start_activities_filter`, `pm4py/algo/filtering/log/suffixes/suffix_filter`, `pm4py/algo/filtering/log/timestamp/timestamp_filter`, `pm4py/algo/filtering/log/traces/trace_filter`, `pm4py/algo/filtering/log/variants/variants_filter`, `pm4py/algo/filtering/pandas`, `pm4py/algo/filtering/polars`, `pm4py/filtering.py`, `pm4py/hof.py`, `pm4py/objects/log/obj`.

### ocel-transforms

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_get_object_types`, `pm4py.ocel_get_attribute_names`, `pm4py.ocel_flattening`, `pm4py.ocel_object_type_activities`, `pm4py.ocel_objects_ot_count`, `pm4py.ocel_temporal_summary`, `pm4py.ocel_objects_summary`, `pm4py.ocel_objects_interactions_summary`, `pm4py.ocel_o2o_enrichment`, `pm4py.ocel_e2o_lifecycle_enrichment`, `pm4py.sample_ocel_objects`, `pm4py.sample_ocel_connected_components`, `pm4py.ocel_drop_duplicates`, `pm4py.ocel_merge_duplicates`, `pm4py.ocel_sort_by_additional_column`, `pm4py.ocel_add_index_based_timedelta`, `pm4py.cluster_equivalent_ocel`, `pm4py.ocel_drill_down`, `pm4py.ocel_roll_up`, `pm4py.ocel_unfold`, `pm4py.ocel_fold`.

Port sources: `pm4py/algo/transformation/ocel/description/algorithm`, `pm4py/algo/transformation/ocel/graphs/ocel20_computation`, `pm4py/algo/transformation/ocel/olap/drill_down/algorithm`, `pm4py/algo/transformation/ocel/olap/drill_down/variants/classic`, `pm4py/algo/transformation/ocel/olap/fold/algorithm`, `pm4py/algo/transformation/ocel/olap/fold/variants/classic`, `pm4py/algo/transformation/ocel/olap/roll_up/algorithm`, `pm4py/algo/transformation/ocel/olap/roll_up/variants/classic`, `pm4py/algo/transformation/ocel/olap/unfold/algorithm`, `pm4py/algo/transformation/ocel/olap/unfold/variants/classic`, `pm4py/algo/transformation/ocel/split_ocel/algorithm`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/attributes_names`, `pm4py/objects/ocel/util/e2o_qualification`, `pm4py/objects/ocel/util/filtering_utils`, `pm4py/objects/ocel/util/flattening`, `pm4py/objects/ocel/util/rename_objs_ot_tim_lex`, `pm4py/objects/ocel/util/sampling`, `pm4py/ocel.py`, `pm4py/statistics/ocel/objects_ot_count`, `pm4py/statistics/ocel/ot_activities`.

### ocel-filters

Crate: `ichnos-ocel`. Rows: `pm4py.filter_ocel_event_attribute`, `pm4py.filter_ocel_object_attribute`, `pm4py.filter_ocel_object_types_allowed_activities`, `pm4py.filter_ocel_object_per_type_count`, `pm4py.filter_ocel_start_events_per_object_type`, `pm4py.filter_ocel_end_events_per_object_type`, `pm4py.filter_ocel_events_timestamp`, `pm4py.filter_ocel_object_types`, `pm4py.filter_ocel_objects`, `pm4py.filter_ocel_events`, `pm4py.filter_ocel_activities_connected_object_type`, `pm4py.filter_ocel_cc_object`, `pm4py.filter_ocel_cc_length`, `pm4py.filter_ocel_cc_otype`, `pm4py.filter_ocel_cc_activity`.

Port sources: `pm4py/algo/filtering/ocel/activity_type_matching`, `pm4py/algo/filtering/ocel/event_attributes`, `pm4py/algo/filtering/ocel/object_attributes`, `pm4py/algo/filtering/ocel/objects_ot_count`, `pm4py/algo/filtering/ocel/ot_endpoints`, `pm4py/algo/transformation/ocel/graphs/object_interaction_graph`, `pm4py/filtering.py`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/filtering_utils`.

### ocel-statistics

Crate: `ichnos-stats`. Rows: `pm4py.statistics.ocel.act_ot_dependent.aggregate_events`, `pm4py.statistics.ocel.act_ot_dependent.aggregate_unique_objects`, `pm4py.statistics.ocel.act_ot_dependent.aggregate_total_objects`, `pm4py.statistics.ocel.act_ot_dependent.find_associations_from_ocel`, `pm4py.statistics.ocel.act_utils.aggregate_events`, `pm4py.statistics.ocel.act_utils.aggregate_unique_objects`, `pm4py.statistics.ocel.act_utils.aggregate_total_objects`, `pm4py.statistics.ocel.act_utils.find_associations_from_relations_df`, `pm4py.statistics.ocel.act_utils.find_associations_from_ocel`, `pm4py.statistics.ocel.edge_metrics.performance_calculation_ocel_aggregation`, `pm4py.statistics.ocel.edge_metrics.aggregate_ev_couples`, `pm4py.statistics.ocel.edge_metrics.aggregate_unique_objects`, `pm4py.statistics.ocel.edge_metrics.aggregate_total_objects`, `pm4py.statistics.ocel.edge_metrics.find_associations_per_edge`, `pm4py.statistics.ocel.objects_ot_count.get_objects_ot_count`, `pm4py.statistics.ocel.ot_activities.get_object_type_activities`.

Port sources: `pm4py/objects/ocel/constants`, `pm4py/objects/ocel/obj`, `pm4py/statistics/ocel/act_ot_dependent.py`, `pm4py/statistics/ocel/act_utils`, `pm4py/statistics/ocel/act_utils.py`, `pm4py/statistics/ocel/edge_metrics.py`, `pm4py/statistics/ocel/objects_ot_count.py`, `pm4py/statistics/ocel/ot_activities.py`.

### dfg-mining

Crate: `ichnos-discovery`. Rows: `pm4py.discover_dfg`, `pm4py.discover_directly_follows_graph`, `pm4py.discover_dfg_typed`, `pm4py.discover_performance_dfg`, `pm4py.derive_minimum_self_distance`, `pm4py.discover_eventually_follows_graph`.

Port sources: `pm4py/algo/discovery/dfg/adapters/pandas/df_statistics`, `pm4py/algo/discovery/dfg/adapters/polars/df_statistics`, `pm4py/algo/discovery/dfg/algorithm`, `pm4py/algo/discovery/dfg/variants/performance`, `pm4py/algo/discovery/minimum_self_distance/algorithm`, `pm4py/discovery.py`, `pm4py/objects/dfg/obj`, `pm4py/objects/log/obj`, `pm4py/statistics/end_activities/log/get`, `pm4py/statistics/end_activities/pandas/get`, `pm4py/statistics/end_activities/polars/get`, `pm4py/statistics/eventually_follows/log/get`, `pm4py/statistics/eventually_follows/pandas/get`, `pm4py/statistics/eventually_follows/polars/get`, `pm4py/statistics/start_activities/log/get`, `pm4py/statistics/start_activities/pandas/get`, `pm4py/statistics/start_activities/polars/get`.

### miner-alpha

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_alpha`, `pm4py.discover_petri_net_alpha_plus`.

Port sources: `pm4py/algo/discovery/alpha/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### miner-batches

Crate: `ichnos-discovery`. Rows: `pm4py.discover_batches`.

Port sources: `pm4py/algo/discovery/batches/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### miner-correlation-mining

Crate: `ichnos-discovery`. Rows: `pm4py.correlation_miner`.

Port sources: `pm4py/algo/discovery/correlation_mining/algorithm`, `pm4py/discovery.py`.

### miner-declare

Crate: `ichnos-discovery`. Rows: `pm4py.discover_declare`.

Port sources: `pm4py/algo/discovery/declare/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### miner-genetic

- Canonical EventLog/EventKeys and typed options/results/errors replace Python parameter maps. Matrix labels must be unique and reciprocal bindings disjoint and nonempty. Token-replay failures retain their conformance error as a source.
- The six pm4py search defaults are preserved. Fractional rates must be finite in [0,1]; a new ChaCha8 seed defaults to zero. Ordered activity/binding/component iteration and the different RNG mean stochastic trajectories and selected models may differ from Python seeds.
- Discovery and genetic_matrix_fitness both require date timestamps and replay a stable timestamp-sorted copy within each canonical trace. pm4py builds directly-follows counts in timestamp order but its tournament replays the original input order, so unsorted logs may change fitness scores and selected models. Equal timestamp order is stable here; pm4py's sort_values default does not guarantee this. Duplicate case IDs never merge traces.
- Caller activity keys reach both discovery and fitness, correcting pm4py's default-key tournament fallback. Identical-parent pools use different population indices and undersized tournament samples clamp to the available candidates.
- Empty input/matrices yield a silent workflow shell. Typed fitness history replaces CSV/progress side effects. Stagnation follows pm4py's available-history rule rather than waiting for a full half-budget window.
- Sophisticated and silent-transition binding conversion is retained with stable model IDs. Goldens compare exact capped visible languages and pm4py-compatible ModelFootprints::of_net results, including three non-simple matrices with silent-transition counts. Those three oracle models receive Rust-style unique transition names before fitness/footprint diagnostics: pm4py's eventually-enabled traversal keys visited nodes by repr, and its original empty silent names collide. Labels, arcs, markings and executable language are unchanged; the diagnostic comparison is to this named model, not the original unnamed traversal. Controlled crossover covers nonempty prefixes and every input/output swap-point combination.
- Fixed-matrix conversion and fitness cover all five real fixtures; deterministic public examples and seeded Rust searches are tested. Stochastic fixture search equality, soundness and boundedness are not claimed.

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_genetic`.

Port sources: `pm4py/algo/discovery/genetic/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### miner-heuristics

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_heuristics`, `pm4py.discover_heuristics_net`.

Port sources: `pm4py/algo/discovery/heuristics/variants/classic`, `pm4py/discovery.py`, `pm4py/objects/heuristics_net/obj`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### miner-ilp

Behaviour changes: canonical EventLog/EventKeys input, typed labels/options/errors and finite alpha in [0,1]. Event order is retained and timestamps are unnecessary. Synthetic boundary identities cannot collide with visible ▶ or ■ labels. Empty input returns a valid silent workflow shell rather than the source indexing error. Regions use the existing good_lp workspace dependency with microlp binary variables; the configured pm4py oracle uses SciPy HiGHS with integer variables. Both select the lexicographically smallest optimal binary region (production then consumption variables), avoiding backend-dependent optimal ties. The source's installed revised-simplex backend ignores integrality; its default result is not the selected oracle configuration. Existing Rust implicit-place and simple reductions are reused, with stable model identifiers and live markings. This miner does not promise soundness or boundedness; reachable footprints are compared only when the complete marking graph fits the stated cap.

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_ilp`.

Port sources: `pm4py/algo/discovery/ilp/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### miner-inductive

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_inductive`, `pm4py.discover_process_tree_inductive`, `pm4py.discover_bpmn_inductive`, `pm4py.discover_powl`.

Port sources: `pm4py/algo/discovery/inductive/algorithm`, `pm4py/algo/discovery/powl/algorithm`, `pm4py/algo/discovery/powl/inductive/variants/dynamic_clustering_frequency/dynamic_clustering_frequency_partial_order_cut`, `pm4py/algo/discovery/powl/inductive/variants/powl_discovery_varaints`, `pm4py/discovery.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/dfg/obj`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/powl/obj`, `pm4py/objects/process_tree/obj`.

### miner-log-skeleton

Crate: `ichnos-discovery`. Rows: `pm4py.discover_log_skeleton`.

Port sources: `pm4py/algo/discovery/log_skeleton/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### miner-powl

Crate: `ichnos-discovery`. Rows: `pm4py.discover_footprints`.

Port sources: `pm4py/algo/discovery/footprints/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/powl/obj`, `pm4py/objects/process_tree/obj`.

### miner-prefix-tree

Crate: `ichnos-discovery`. Rows: `pm4py.discover_prefix_tree`.

Port sources: `pm4py/algo/transformation/log_to_trie/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/trie/obj`.

### miner-split-miner

Crate: `ichnos-discovery`. Rows: `pm4py.discover_bpmn_split_miner`.

Port sources: `pm4py/algo/discovery/split_miner/algorithm`, `pm4py/algo/discovery/split_miner/variants/classic`, `pm4py/algo/discovery/split_miner/variants/sm2`, `pm4py/discovery.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/log/obj`.

### miner-temporal-profile

Crate: `ichnos-discovery`. Rows: `pm4py.discover_temporal_profile`.

Port sources: `pm4py/algo/discovery/temporal_profile/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### miner-transition-system

Crate: `ichnos-discovery`. Rows: `pm4py.discover_transition_system`.

Port sources: `pm4py/algo/discovery/transition_system/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/transition_system/obj`.

### ocel-discovery

Crate: `ichnos-ocel`. Rows: `pm4py.discover_ocdfg`, `pm4py.discover_oc_petri_net`, `pm4py.discover_objects_graph`.

Port sources: `pm4py/algo/discovery/ocel/ocdfg/algorithm`, `pm4py/algo/discovery/ocel/ocpn/algorithm`, `pm4py/algo/transformation/ocel/graphs/object_cobirth_graph`, `pm4py/algo/transformation/ocel/graphs/object_codeath_graph`, `pm4py/algo/transformation/ocel/graphs/object_descendants_graph`, `pm4py/algo/transformation/ocel/graphs/object_inheritance_graph`, `pm4py/algo/transformation/ocel/graphs/object_interaction_graph`, `pm4py/objects/ocel/constants`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocpn/obj`, `pm4py/ocel.py`.

### ocel-temporal-mining

Crate: `ichnos-discovery`. Rows: `pm4py.discover_otg`, `pm4py.discover_etot`.

Port sources: `pm4py/algo/discovery/ocel/etot/algorithm`, `pm4py/algo/discovery/ocel/otg/algorithm`, `pm4py/discovery.py`, `pm4py/objects/ocel/obj`.

### token-replay

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_diagnostics_token_based_replay`, `pm4py.fitness_token_based_replay`, `pm4py.precision_token_based_replay`, `pm4py.generalization_tbr`, `pm4py.replay_prefix_tbr`, `pm4py.check_is_fitting`.

Port sources: `pm4py/algo/conformance/tokenreplay/algorithm`, `pm4py/algo/conformance/tokenreplay/variants/token_replay`, `pm4py/algo/evaluation/generalization/algorithm`, `pm4py/algo/evaluation/precision/algorithm`, `pm4py/algo/evaluation/replay_fitness/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`.

### alignments

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_diagnostics_alignments`, `pm4py.fitness_alignments`, `pm4py.precision_alignments`.

Port sources: `pm4py/algo/conformance/alignments/dfg/algorithm`, `pm4py/algo/conformance/alignments/edit_distance/algorithm`, `pm4py/algo/conformance/alignments/petri_net/algorithm`, `pm4py/algo/conformance/alignments/process_tree/variants/search_graph_pt`, `pm4py/algo/evaluation/precision/algorithm`, `pm4py/algo/evaluation/replay_fitness/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`.

### footprint-conformance

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_diagnostics_footprints`, `pm4py.fitness_footprints`, `pm4py.precision_footprints`.

Port sources: `pm4py/algo/conformance/footprints/algorithm`, `pm4py/algo/conformance/footprints/util/evaluation`, `pm4py/conformance.py`.

### declare-conformance

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_declare`.

Port sources: `pm4py/algo/conformance/declare/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`.

### log-skeleton-conformance

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_log_skeleton`.

Port sources: `pm4py/algo/conformance/log_skeleton/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`.

### temporal-profile-conformance

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_temporal_profile`.

Port sources: `pm4py/algo/conformance/temporal_profile/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`.

### ocel-conformance

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_ocdfg`, `pm4py.conformance_otg`, `pm4py.conformance_etot`.

Port sources: `pm4py/algo/conformance/ocel/etot/algorithm`, `pm4py/algo/conformance/ocel/ocdfg/algorithm`, `pm4py/algo/conformance/ocel/otg/algorithm`, `pm4py/conformance.py`, `pm4py/objects/ocel/obj`.

### organizational-mining

Crate: `ichnos-org`. Rows: `pm4py.discover_handover_of_work_network`, `pm4py.discover_working_together_network`, `pm4py.discover_activity_based_resource_similarity`, `pm4py.discover_subcontracting_network`, `pm4py.discover_organizational_roles`, `pm4py.discover_network_analysis`.

Port sources: `pm4py/algo/organizational_mining/network_analysis/algorithm`, `pm4py/algo/organizational_mining/network_analysis/variants/dataframe`, `pm4py/algo/organizational_mining/roles/algorithm`, `pm4py/algo/organizational_mining/sna/algorithm`, `pm4py/objects/log/obj`, `pm4py/objects/org/roles/obj`, `pm4py/objects/org/sna/obj`, `pm4py/org.py`.

### performance

Crate: `ichnos-perf`. Rows: `pm4py.convert_log_to_time_intervals`, `pm4py.insert_case_service_waiting_time`, `pm4py.insert_case_arrival_finish_rate`.

Port sources: `pm4py/algo/transformation/log_to_interval_tree/variants/open_paths`, `pm4py/analysis.py`, `pm4py/convert.py`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`.

### simulation

Crate: `ichnos-sim`. Rows: `pm4py.play_out`, `pm4py.generate_process_tree`.

Port sources: `pm4py/algo/simulation/playout/declare/algorithm`, `pm4py/algo/simulation/playout/dfg/algorithm`, `pm4py/algo/simulation/playout/petri_net/algorithm`, `pm4py/algo/simulation/playout/process_tree/algorithm`, `pm4py/algo/simulation/tree_generator/algorithm`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/inhibitor_reset/semantics`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/semantics`, `pm4py/objects/process_tree/obj`, `pm4py/sim.py`.

### ml-features

Crate: `ichnos-ml`. Rows: `pm4py.split_train_test`, `pm4py.get_prefixes_from_log`, `pm4py.extract_outcome_enriched_dataframe`, `pm4py.extract_features_dataframe`, `pm4py.extract_ocel_features`, `pm4py.extract_temporal_features_dataframe`, `pm4py.extract_target_vector`, `pm4py.cluster_log`, `pm4py.embeddings_similarity`.

Port sources: `pm4py/algo/clustering/profiles/algorithm`, `pm4py/algo/transformation/log_to_target/algorithm`, `pm4py/algo/transformation/ocel/features/objects/algorithm`, `pm4py/algo/transformation/trace_encodings/algorithm`, `pm4py/algo/transformation/trace_encodings/variants/temporal`, `pm4py/algo/transformation/trace_encodings/variants/temporal_lazy`, `pm4py/analysis.py`, `pm4py/ml.py`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/get_prefixes`, `pm4py/objects/log/util/split_train_test`, `pm4py/objects/ocel/obj`, `pm4py/objects/petri_net/utils/embeddings_similarity`.

### language-distance

Crate: `ichnos-stats`. Rows: `pm4py.compute_emd`.

Port sources: `pm4py/algo/evaluation/earth_mover_distance/algorithm`, `pm4py/analysis.py`.

### model-similarity

Crate: `ichnos-model`. Rows: `pm4py.get_activity_labels`, `pm4py.replace_activity_labels`, `pm4py.behavioral_similarity`, `pm4py.structural_similarity`, `pm4py.label_sets_similarity`, `pm4py.map_labels_from_second_model`.

Port sources: `pm4py/analysis.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/bpmn/util/label_replacing`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/label_replacing`, `pm4py/objects/powl/obj`, `pm4py/objects/powl/utils/label_replacing`, `pm4py/objects/process_tree/obj`, `pm4py/objects/process_tree/utils/label_replacing`, `pm4py/objects/process_tree/utils/struct_similarity`.

### privacy

Crate: `ichnos-privacy`. Rows: `pm4py.privacy.anonymize_differential_privacy`.

Port sources: `pm4py/algo/anonymization/pripel/algorithm`, `pm4py/algo/anonymization/trace_variant_query/algorithm`, `pm4py/objects/log/obj`, `pm4py/privacy.py`.

### stream-io

Crate: `ichnos-stream`. Rows: `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable`, `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.read_trace`, `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.reset`, `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.to_trace_stream`, `pm4py.streaming.conversion.from_pandas.apply`, `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor`, `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.register`, `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.append`, `pm4py.streaming.importer.csv.importer.apply`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.reset`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.to_event_stream`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.read_event`, `pm4py.streaming.importer.csv.variants.csv_event_stream.apply`, `pm4py.streaming.importer.xes.importer.apply`, `pm4py.streaming.importer.xes.variants.xes_event_stream.parse_attribute`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.to_event_stream`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.reset`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.read_event`, `pm4py.streaming.importer.xes.variants.xes_event_stream.apply`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.parse_attribute`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.to_trace_stream`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.reset`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.read_trace`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.apply`.

Port sources: `pm4py/objects/log/obj`, `pm4py/objects/ocel/constants`, `pm4py/streaming/conversion/from_pandas.py`, `pm4py/streaming/conversion/ocel_flatts_distributor.py`, `pm4py/streaming/importer/csv/importer.py`, `pm4py/streaming/importer/csv/variants/csv_event_stream.py`, `pm4py/streaming/importer/xes/importer.py`, `pm4py/streaming/importer/xes/variants/xes_event_stream.py`, `pm4py/streaming/importer/xes/variants/xes_trace_stream.py`, `pm4py/streaming/stream/live_event_stream`, `pm4py/streaming/stream/live_trace_stream`.

### stream-runtime

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.interface.StreamingAlgorithm`, `pm4py.streaming.algo.interface.StreamingAlgorithm.get`, `pm4py.streaming.algo.interface.StreamingAlgorithm.receive`, `pm4py.streaming.stream.live_event_stream.StreamState`, `pm4py.streaming.stream.live_event_stream.LiveEventStream`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.append`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.start`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.stop`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.register`, `pm4py.streaming.stream.live_trace_stream.StreamState`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.append`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.start`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.stop`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.register`, `pm4py.streaming.util.dictio.generator.apply`, `pm4py.streaming.util.dictio.versions.classic.apply`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.keys`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.values`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.itervalues`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushdb`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushall`, `pm4py.streaming.util.dictio.versions.redis.apply`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.keys`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.values`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.itervalues`, `pm4py.streaming.util.dictio.versions.thread_safe.apply`, `pm4py.streaming.util.event_stream_printer.EventStreamPrinter`, `pm4py.streaming.util.live_to_static_stream.LiveToStaticStream`, `pm4py.streaming.util.trace_stream_printer.TraceStreamPrinter`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/algo/interface`, `pm4py/streaming/algo/interface.py`, `pm4py/streaming/stream/live_event_stream.py`, `pm4py/streaming/stream/live_trace_stream.py`, `pm4py/streaming/util/dictio/generator.py`, `pm4py/streaming/util/dictio/versions/classic.py`, `pm4py/streaming/util/dictio/versions/redis.py`, `pm4py/streaming/util/dictio/versions/thread_safe.py`, `pm4py/streaming/util/event_stream_printer.py`, `pm4py/streaming/util/live_to_static_stream.py`, `pm4py/streaming/util/trace_stream_printer.py`.

### stream-dfg

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.discovery.dfg.algorithm.apply`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.build_dictionaries`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.event_without_activity_or_case`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_str`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_tuple`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.apply`.

Port sources: `pm4py/streaming/algo/discovery/dfg/algorithm.py`, `pm4py/streaming/algo/discovery/dfg/variants/frequency.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-alignments

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.alignments.algorithm.apply`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._TrieNode`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._State`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments.finish`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.apply`.

Port sources: `pm4py/algo/conformance/alignments/petri_net/utils/approx_utils`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/align_utils`, `pm4py/streaming/algo/conformance/alignments/algorithm.py`, `pm4py/streaming/algo/conformance/alignments/variants/approx_iws.py`, `pm4py/streaming/algo/interface`.

### stream-declare

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.declare.algorithm.apply`, `pm4py.streaming.algo.conformance.declare.variants.automata.DeclareStreamingConformance`, `pm4py.streaming.algo.conformance.declare.variants.automata.apply`.

Port sources: `pm4py/streaming/algo/conformance/declare/algorithm.py`, `pm4py/streaming/algo/conformance/declare/variants/automata.py`, `pm4py/streaming/algo/interface`.

### stream-footprints

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.footprints.algorithm.apply`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.build_dictionaries`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.encode_str`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_footprints`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_intra_case`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_start_case`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.get_status`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate_all`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_or_activity_not_in_event`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_activity_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_footprints_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_start_activity_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_end_activity_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_not_in_dictionary`, `pm4py.streaming.algo.conformance.footprints.variants.classic.apply`.

Port sources: `pm4py/streaming/algo/conformance/footprints/algorithm.py`, `pm4py/streaming/algo/conformance/footprints/variants/classic.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-tbr

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.tbr.algorithm.apply`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.build_dictionaries`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_paths_net`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_str`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_marking`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.decode_marking`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.verify_tbr`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.enable_trans_with_invisibles`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_status`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate_all`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.reach_fm_with_invisibles`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_or_activity_not_in_event`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_activity_not_possible`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_missing_tokens`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_not_in_dictionary`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_final_marking_not_reached`, `pm4py.streaming.algo.conformance.tbr.variants.classic.apply`.

Port sources: `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/semantics`, `pm4py/streaming/algo/conformance/tbr/algorithm.py`, `pm4py/streaming/algo/conformance/tbr/variants/classic.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-temporal

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.temporal.algorithm.apply`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.check_conformance`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_event_is_not_complete`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_deviation`, `pm4py.streaming.algo.conformance.temporal.variants.classic.apply`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/algo/conformance/temporal/algorithm.py`, `pm4py/streaming/algo/conformance/temporal/variants/classic.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### service-connectors

Crate: `ichnos-io`. Rows: `pm4py.connectors.extract_log_github`, `pm4py.connectors.extract_log_camunda_workflow`, `pm4py.connectors.extract_log_sap_o2c`, `pm4py.connectors.extract_log_sap_accounting`.

Port sources: `pm4py/algo/connectors/variants/camunda_workflow`, `pm4py/algo/connectors/variants/github_repo`, `pm4py/algo/connectors/variants/sap_accounting`, `pm4py/algo/connectors/variants/sap_o2c`, `pm4py/connectors.py`.

### stream-connectors

Crate: `ichnos-stream`. Rows: `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.run`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.stop`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.get_process_name`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.record`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_click`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_key_release`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/connectors/windows/click_key_logger.py`.

### model-dot

Crate: `ichnos-viz`. Rows: `pm4py.view_petri_net`, `pm4py.save_vis_petri_net`, `pm4py.view_dfg`, `pm4py.save_vis_dfg`, `pm4py.view_process_tree`, `pm4py.save_vis_process_tree`, `pm4py.save_vis_bpmn`, `pm4py.view_bpmn`, `pm4py.view_heuristics_net`, `pm4py.save_vis_heuristics_net`, `pm4py.view_transition_system`, `pm4py.save_vis_transition_system`, `pm4py.view_prefix_tree`, `pm4py.save_vis_prefix_tree`, `pm4py.view_alignments`, `pm4py.save_vis_alignments`, `pm4py.view_footprints`, `pm4py.save_vis_footprints`, `pm4py.view_powl`, `pm4py.save_vis_powl`.

Port sources: `pm4py/objects/bpmn/obj`, `pm4py/objects/heuristics_net/obj`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/powl/obj`, `pm4py/objects/process_tree/obj`, `pm4py/objects/transition_system/obj`, `pm4py/objects/trie/obj`, `pm4py/vis.py`, `pm4py/visualization/align_table/visualizer`, `pm4py/visualization/bpmn/visualizer`, `pm4py/visualization/dfg/visualizer`, `pm4py/visualization/footprints/visualizer`, `pm4py/visualization/heuristics_net/visualizer`, `pm4py/visualization/petri_net/visualizer`, `pm4py/visualization/powl/visualizer`, `pm4py/visualization/process_tree/visualizer`, `pm4py/visualization/transition_system/visualizer`, `pm4py/visualization/trie/visualizer`.

### performance-dot

Crate: `ichnos-viz`. Rows: `pm4py.view_performance_dfg`, `pm4py.save_vis_performance_dfg`, `pm4py.view_dotted_chart`, `pm4py.save_vis_dotted_chart`, `pm4py.view_performance_spectrum`, `pm4py.save_vis_performance_spectrum`, `pm4py.view_network_analysis`, `pm4py.save_vis_network_analysis`.

Port sources: `pm4py/algo/discovery/performance_spectrum/algorithm`, `pm4py/objects/log/obj`, `pm4py/vis.py`, `pm4py/visualization/dfg/variants/performance`, `pm4py/visualization/dfg/visualizer`, `pm4py/visualization/dotted_chart/visualizer`, `pm4py/visualization/network_analysis/visualizer`, `pm4py/visualization/performance_spectrum/variants/neato`, `pm4py/visualization/performance_spectrum/visualizer`.

### ocel-dot

Crate: `ichnos-viz`. Rows: `pm4py.view_ocdfg`, `pm4py.save_vis_ocdfg`, `pm4py.view_ocpn`, `pm4py.save_vis_ocpn`, `pm4py.view_object_graph`, `pm4py.save_vis_object_graph`.

Port sources: `pm4py/objects/ocel/obj`, `pm4py/objects/ocpn/obj`, `pm4py/vis.py`, `pm4py/visualization/ocel/object_graph/visualizer`, `pm4py/visualization/ocel/ocdfg/visualizer`, `pm4py/visualization/ocel/ocpn/visualizer`.
