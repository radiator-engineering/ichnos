# pm4py parity inventory

Reference: a checkout of pm4py **2.7.23.8** (commit **24a3bf6**), cross-checked against its installed top-level exports. Use `PM4PY_SRC` for the source checkout and `PM4PY_PYTHON` for its Python interpreter when reproducing the inventory.

## Summary

todo: 586; ported: 0; dropped: 40; total: 626.

Recompute with `tools/parity_count.py`. Completion requires each row to be `ported` with a passing golden test or `dropped` with a reason.

All Rust paths below are **planned**. Lanes replace them with actual public paths when porting. Distinct defined log/pandas/polars functions remain separate rows; pure imported aliases are counted once under their defining module (top-level spelling preferred). All public definitions under statistics are retained conservatively, including common helpers; streaming includes direct functions, classes and public methods, with parameter/variant enums represented by the owning entry point. Top-level namespace exports are explicit compatibility rows. Private names and underscore metadata are excluded. Source paths are relative to `pm4py/`; arrows identify implementation dependencies, and variant lists describe available backends rather than requiring separate Rust implementations.

## read

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.read_xes` | `read.py` → `objects/conversion/log/converter`, `objects/log/importer/xes/importer`, `objects/log/obj` | `ichnos::io::read_xes` (planned) | `ichnos-io` | todo | Variants: chunk_regex, iterparse, iterparse_20, iterparse_mem_compressed, line_by_line, rustxes, to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.read_pnml` | `read.py` → `objects/petri_net/importer/importer`, `objects/petri_net/obj` | `ichnos::io::read_pnml` (planned) | `ichnos-io` | todo | Variants: pnml. |
| `pm4py.read_ptml` | `read.py` → `objects/process_tree/importer/importer`, `objects/process_tree/obj` | `ichnos::io::read_ptml` (planned) | `ichnos-io` | todo | Variants: ptml. |
| `pm4py.read_dfg` | `read.py` → `objects/dfg/importer/importer` | `ichnos::io::read_dfg` (planned) | `ichnos-io` | todo | Variants: classic. |
| `pm4py.read_bpmn` | `read.py` → `objects/bpmn/importer/importer`, `objects/bpmn/obj` | `ichnos::io::read_bpmn` (planned) | `ichnos-io` | todo | Variants: lxml. |
| `pm4py.read_ocel` | `read.py` → `objects/ocel/importer/csv/importer`, `objects/ocel/importer/jsonocel/importer`, `objects/ocel/importer/sqlite/importer`, `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_rustxes, ocel20_standard, pandas, pandas_importer. |
| `pm4py.read_ocel_csv` | `read.py` → `objects/ocel/importer/csv/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.read_ocel_json` | `read.py` → `objects/ocel/importer/jsonocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel_json` (planned) | `ichnos-io` | todo | Variants: classic, ocel20_rustxes, ocel20_standard. |
| `pm4py.read_ocel_xml` | `read.py` → `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel_xml` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_rustxes. |
| `pm4py.read_ocel_sqlite` | `read.py` → `objects/ocel/importer/sqlite/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_importer. |
| `pm4py.read_ocel2` | `read.py` → `objects/ocel/importer/bundled/importer`, `objects/ocel/importer/csv/importer`, `objects/ocel/importer/jsonocel/importer`, `objects/ocel/importer/sqlite/importer`, `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_rustxes, ocel20_standard, pandas, pandas_importer. |
| `pm4py.read_ocel2_bundle` | `read.py` → `objects/ocel/importer/bundled/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_bundle` (planned) | `ichnos-io` | todo | Variants: ocel20. |
| `pm4py.read_ocel2_csv` | `read.py` → `objects/ocel/importer/csv/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.read_ocel2_json` | `read.py` → `objects/ocel/importer/jsonocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_json` (planned) | `ichnos-io` | todo | Variants: classic, ocel20_rustxes, ocel20_standard. |
| `pm4py.read_ocel2_sqlite` | `read.py` → `objects/ocel/importer/sqlite/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_importer. |
| `pm4py.read_ocel2_xml` | `read.py` → `objects/ocel/importer/xmlocel/importer`, `objects/ocel/obj` | `ichnos::io::read_ocel2_xml` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_rustxes. |

## write

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.write_xes` | `write.py` → `objects/log/exporter/xes/exporter`, `objects/log/obj` | `ichnos::io::write_xes` (planned) | `ichnos-io` | todo | Variants: etree, etree_xes_exp, line_by_line. |
| `pm4py.write_pnml` | `write.py` → `objects/petri_net/exporter/exporter`, `objects/petri_net/obj` | `ichnos::io::write_pnml` (planned) | `ichnos-io` | todo | Variants: pnml. |
| `pm4py.write_ptml` | `write.py` → `objects/process_tree/exporter/exporter`, `objects/process_tree/obj` | `ichnos::io::write_ptml` (planned) | `ichnos-io` | todo | Variants: ptml. |
| `pm4py.write_dfg` | `write.py` → `objects/dfg/exporter/exporter` | `ichnos::io::write_dfg` (planned) | `ichnos-io` | todo | Variants: classic. |
| `pm4py.write_bpmn` | `write.py` → `objects/bpmn/exporter/exporter`, `objects/bpmn/layout/layouter`, `objects/bpmn/obj` | `ichnos::io::write_bpmn` (planned) | `ichnos-io` | todo | Variants: etree, graphviz, graphviz_new. |
| `pm4py.write_ocel` | `write.py` → `objects/ocel/exporter/csv/exporter`, `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_standard, pandas, pandas_exporter. |
| `pm4py.write_ocel_csv` | `write.py` → `objects/ocel/exporter/csv/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.write_ocel_json` | `write.py` → `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel_json` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_standard. |
| `pm4py.write_ocel_xml` | `write.py` → `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel_xml` (planned) | `ichnos-io` | todo | Variants: classic, ocel20. |
| `pm4py.write_ocel_sqlite` | `write.py` → `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_exporter. |
| `pm4py.write_ocel2` | `write.py` → `objects/ocel/exporter/bundled/exporter`, `objects/ocel/exporter/csv/exporter`, `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_standard, pandas, pandas_exporter. |
| `pm4py.write_ocel2_bundle` | `write.py` → `objects/ocel/exporter/bundled/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_bundle` (planned) | `ichnos-io` | todo | Variants: ocel20. |
| `pm4py.write_ocel2_csv` | `write.py` → `objects/ocel/exporter/csv/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_csv` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas. |
| `pm4py.write_ocel2_json` | `write.py` → `objects/ocel/exporter/jsonocel/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_json` (planned) | `ichnos-io` | todo | Variants: classic, ocel20, ocel20_standard. |
| `pm4py.write_ocel2_sqlite` | `write.py` → `objects/ocel/exporter/sqlite/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_sqlite` (planned) | `ichnos-io` | todo | Variants: ocel20, pandas_exporter. |
| `pm4py.write_ocel2_xml` | `write.py` → `objects/ocel/exporter/xmlocel/exporter`, `objects/ocel/obj` | `ichnos::io::write_ocel2_xml` (planned) | `ichnos-io` | todo | Variants: classic, ocel20. |

## discovery

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.discover_dfg` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/algorithm`, `objects/log/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos::discovery::dfg` (planned) | `ichnos-discovery` | todo | Variants: case_attributes, clean, clean_polars, clean_time, freq_triples, frequency, frequency_greedy, native, performance, performance_greedy. |
| `pm4py.discover_directly_follows_graph` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/algorithm`, `objects/log/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos::discovery::directly_follows_graph` (planned) | `ichnos-discovery` | todo | Variants: case_attributes, clean, clean_polars, clean_time, freq_triples, frequency, frequency_greedy, native, performance, performance_greedy. |
| `pm4py.discover_dfg_typed` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/algorithm`, `objects/dfg/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos::discovery::dfg_typed` (planned) | `ichnos-discovery` | todo | Variants: case_attributes, clean, clean_polars, clean_time, freq_triples, frequency, frequency_greedy, native, performance, performance_greedy. |
| `pm4py.discover_performance_dfg` | `discovery.py` → `algo/discovery/dfg/adapters/pandas/df_statistics`, `algo/discovery/dfg/adapters/polars/df_statistics`, `algo/discovery/dfg/variants/performance`, `objects/dfg/obj`, `objects/log/obj`, `statistics/end_activities/log/get`, `statistics/end_activities/pandas/get`, `statistics/end_activities/polars/get`, `statistics/start_activities/log/get`, `statistics/start_activities/pandas/get`, `statistics/start_activities/polars/get` | `ichnos::discovery::performance_dfg` (planned) | `ichnos-discovery` | todo | Single entry point; preserve source defaults. |
| `pm4py.discover_petri_net_alpha` | `discovery.py` → `algo/discovery/alpha/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_alpha` (planned) | `ichnos-discovery` | todo | Variants: alpha_version_classic, alpha_version_plus, classic, plus. |
| `pm4py.discover_petri_net_ilp` | `discovery.py` → `algo/discovery/ilp/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_ilp` (planned) | `ichnos-discovery` | todo | Variants: classic. |
| `pm4py.discover_petri_net_genetic` | `discovery.py` → `algo/discovery/genetic/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_genetic` (planned) | `ichnos-discovery` | todo | Variants: classic. |
| `pm4py.discover_petri_net_alpha_plus` | `discovery.py` → `algo/discovery/alpha/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_alpha_plus` (planned) | `ichnos-discovery` | todo | Variants: alpha_version_classic, alpha_version_plus, classic, plus. |
| `pm4py.discover_petri_net_inductive` | `discovery.py` → `algo/discovery/inductive/algorithm`, `objects/dfg/obj`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_inductive` (planned) | `ichnos-discovery` | todo | Variants: abc, im, imd, imf, instances. |
| `pm4py.discover_petri_net_heuristics` | `discovery.py` → `algo/discovery/heuristics/variants/classic`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::discovery::petri_net_heuristics` (planned) | `ichnos-discovery` | todo | Single entry point; preserve source defaults. |
| `pm4py.discover_process_tree_inductive` | `discovery.py` → `algo/discovery/inductive/algorithm`, `objects/dfg/obj`, `objects/log/obj`, `objects/process_tree/obj` | `ichnos::discovery::process_tree_inductive` (planned) | `ichnos-discovery` | todo | Variants: abc, im, imd, imf, instances. |
| `pm4py.discover_heuristics_net` | `discovery.py` → `algo/discovery/heuristics/variants/classic`, `objects/heuristics_net/obj`, `objects/log/obj` | `ichnos::discovery::heuristics_net` (planned) | `ichnos-discovery` | todo | Single entry point; preserve source defaults. |
| `pm4py.derive_minimum_self_distance` | `discovery.py` → `algo/discovery/minimum_self_distance/algorithm`, `objects/log/obj` | `ichnos::discovery::derive_minimum_self_distance` (planned) | `ichnos-discovery` | todo | Variants: log, pandas, polars. |
| `pm4py.discover_footprints` | `discovery.py` → `algo/discovery/footprints/algorithm`, `objects/log/obj`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos::discovery::footprints` (planned) | `ichnos-discovery` | todo | Variants: dfg, entire_dataframe, entire_event_log, petri_reach_graph, polars_lazyframes, powl, process_tree, trace_by_trace. |
| `pm4py.discover_eventually_follows_graph` | `discovery.py` → `objects/log/obj`, `statistics/eventually_follows/log/get`, `statistics/eventually_follows/pandas/get`, `statistics/eventually_follows/polars/get` | `ichnos::discovery::eventually_follows_graph` (planned) | `ichnos-discovery` | todo | Single entry point; preserve source defaults. |
| `pm4py.discover_bpmn_inductive` | `discovery.py` → `algo/discovery/inductive/algorithm`, `objects/bpmn/obj`, `objects/dfg/obj`, `objects/log/obj` | `ichnos::discovery::bpmn_inductive` (planned) | `ichnos-discovery` | todo | Variants: abc, im, imd, imf, instances. |
| `pm4py.discover_bpmn_split_miner` | `discovery.py` → `algo/discovery/split_miner/algorithm`, `algo/discovery/split_miner/variants/classic`, `algo/discovery/split_miner/variants/sm2`, `objects/bpmn/obj`, `objects/log/obj` | `ichnos::discovery::bpmn_split_miner` (planned) | `ichnos-discovery` | todo | Variants: abc, classic, sm2. |
| `pm4py.discover_transition_system` | `discovery.py` → `algo/discovery/transition_system/algorithm`, `objects/log/obj`, `objects/transition_system/obj` | `ichnos::discovery::transition_system` (planned) | `ichnos-discovery` | todo | Variants: view_based. |
| `pm4py.discover_prefix_tree` | `discovery.py` → `algo/transformation/log_to_trie/algorithm`, `objects/log/obj`, `objects/trie/obj` | `ichnos::discovery::prefix_tree` (planned) | `ichnos-discovery` | todo | Single entry point; preserve source defaults. |
| `pm4py.discover_temporal_profile` | `discovery.py` → `algo/discovery/temporal_profile/algorithm`, `objects/log/obj` | `ichnos::discovery::temporal_profile` (planned) | `ichnos-discovery` | todo | Variants: dataframe, log. |
| `pm4py.discover_log_skeleton` | `discovery.py` → `algo/discovery/log_skeleton/algorithm`, `objects/log/obj` | `ichnos::discovery::log_skeleton` (planned) | `ichnos-discovery` | todo | Variants: classic. |
| `pm4py.discover_declare` | `discovery.py` → `algo/discovery/declare/algorithm`, `objects/log/obj` | `ichnos::discovery::declare` (planned) | `ichnos-discovery` | todo | Variants: classic. |
| `pm4py.discover_powl` | `discovery.py` → `algo/discovery/powl/algorithm`, `algo/discovery/powl/inductive/variants/dynamic_clustering_frequency/dynamic_clustering_frequency_partial_order_cut`, `algo/discovery/powl/inductive/variants/powl_discovery_varaints`, `objects/log/obj`, `objects/powl/obj` | `ichnos::discovery::powl` (planned) | `ichnos-discovery` | todo | Variants: brute_force, dynamic_clustering, maximal, tree. |
| `pm4py.discover_batches` | `discovery.py` → `algo/discovery/batches/algorithm`, `objects/log/obj` | `ichnos::discovery::batches` (planned) | `ichnos-discovery` | todo | Variants: log, pandas, polars. |
| `pm4py.correlation_miner` | `discovery.py` → `algo/discovery/correlation_mining/algorithm` | `ichnos::discovery::correlation_miner` (planned) | `ichnos-discovery` | todo | Variants: classic, classic_split, trace_based. |
| `pm4py.discover_otg` | `discovery.py` → `algo/discovery/ocel/otg/algorithm`, `objects/ocel/obj` | `ichnos::discovery::otg` (planned) | `ichnos-discovery` | todo | Variants: classic. |
| `pm4py.discover_etot` | `discovery.py` → `algo/discovery/ocel/etot/algorithm`, `objects/ocel/obj` | `ichnos::discovery::etot` (planned) | `ichnos-discovery` | todo | Variants: classic. |

## conformance

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.conformance_diagnostics_token_based_replay` | `conformance.py` → `algo/conformance/tokenreplay/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::conformance_diagnostics_token_based_replay` (planned) | `ichnos-conformance` | todo | Variants: backwards, token_replay. |
| `pm4py.conformance_diagnostics_alignments` | `conformance.py` → `algo/conformance/alignments/dfg/algorithm`, `algo/conformance/alignments/edit_distance/algorithm`, `algo/conformance/alignments/petri_net/algorithm`, `algo/conformance/alignments/process_tree/variants/search_graph_pt`, `objects/log/obj`, `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos::conformance::conformance_diagnostics_alignments` (planned) | `ichnos-conformance` | todo | Variants: approx_fixed_horizon, approx_sliding_window, approx_subset, approx_tandem_repeats, classic, dijkstra_less_memory, dijkstra_no_heuristics, dijkstra_semantics, discounted_a_star, edit_distance, generator_dijkstra_less_memory, generator_dijkstra_no_heuristics, state_equation_a_star, version_dijkstra_less_memory, version_dijkstra_no_heuristics, version_dijkstra_semantics, version_discounted_a_star, version_state_equation_a_star. |
| `pm4py.fitness_token_based_replay` | `conformance.py` → `algo/evaluation/replay_fitness/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::fitness_token_based_replay` (planned) | `ichnos-conformance` | todo | Variants: alignment_based, token_based, token_replay. |
| `pm4py.fitness_alignments` | `conformance.py` → `algo/evaluation/replay_fitness/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::fitness_alignments` (planned) | `ichnos-conformance` | todo | Variants: alignment_based, token_based, token_replay. |
| `pm4py.precision_token_based_replay` | `conformance.py` → `algo/evaluation/precision/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::precision_token_based_replay` (planned) | `ichnos-conformance` | todo | Variants: align_etconformance, automaton_after_align, etconformance_token. |
| `pm4py.precision_alignments` | `conformance.py` → `algo/evaluation/precision/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::precision_alignments` (planned) | `ichnos-conformance` | todo | Variants: align_etconformance, automaton_after_align, etconformance_token. |
| `pm4py.generalization_tbr` | `conformance.py` → `algo/evaluation/generalization/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::generalization_tbr` (planned) | `ichnos-conformance` | todo | Variants: generalization_token, token_based. |
| `pm4py.replay_prefix_tbr` | `conformance.py` → `algo/conformance/tokenreplay/variants/token_replay`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::conformance::replay_prefix_tbr` (planned) | `ichnos-conformance` | todo | Single entry point; preserve source defaults. |
| `pm4py.conformance_diagnostics_footprints` | `conformance.py` → `algo/conformance/footprints/algorithm` | `ichnos::conformance::conformance_diagnostics_footprints` (planned) | `ichnos-conformance` | todo | Variants: log_extensive, log_model, trace_extensive. |
| `pm4py.fitness_footprints` | `conformance.py` → `algo/conformance/footprints/algorithm`, `algo/conformance/footprints/util/evaluation` | `ichnos::conformance::fitness_footprints` (planned) | `ichnos-conformance` | todo | Variants: log_extensive, log_model, trace_extensive. |
| `pm4py.precision_footprints` | `conformance.py` → `algo/conformance/footprints/util/evaluation` | `ichnos::conformance::precision_footprints` (planned) | `ichnos-conformance` | todo | Single entry point; preserve source defaults. |
| `pm4py.check_is_fitting` | `conformance.py` → `objects/log/obj`, `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos::conformance::check_is_fitting` (planned) | `ichnos-conformance` | todo | Single entry point; preserve source defaults. |
| `pm4py.conformance_temporal_profile` | `conformance.py` → `algo/conformance/temporal_profile/algorithm`, `objects/log/obj` | `ichnos::conformance::conformance_temporal_profile` (planned) | `ichnos-conformance` | todo | Variants: dataframe, log. |
| `pm4py.conformance_declare` | `conformance.py` → `algo/conformance/declare/algorithm`, `objects/log/obj` | `ichnos::conformance::conformance_declare` (planned) | `ichnos-conformance` | todo | Variants: classic. |
| `pm4py.conformance_log_skeleton` | `conformance.py` → `algo/conformance/log_skeleton/algorithm`, `objects/log/obj` | `ichnos::conformance::conformance_log_skeleton` (planned) | `ichnos-conformance` | todo | Variants: classic. |
| `pm4py.conformance_ocdfg` | `conformance.py` → `algo/conformance/ocel/ocdfg/algorithm`, `objects/ocel/obj` | `ichnos::conformance::conformance_ocdfg` (planned) | `ichnos-conformance` | todo | Variants: graph_comparison. |
| `pm4py.conformance_otg` | `conformance.py` → `algo/conformance/ocel/otg/algorithm`, `objects/ocel/obj` | `ichnos::conformance::conformance_otg` (planned) | `ichnos-conformance` | todo | Variants: graph_comparison. |
| `pm4py.conformance_etot` | `conformance.py` → `algo/conformance/ocel/etot/algorithm`, `objects/ocel/obj` | `ichnos::conformance::conformance_etot` (planned) | `ichnos-conformance` | todo | Variants: graph_comparison. |

## filtering

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.filter_log_relative_occurrence_event_attribute` | `filtering.py` → `algo/filtering/log/attributes/attributes_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_log_relative_occurrence_event_attribute` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_start_activities` | `filtering.py` → `algo/filtering/log/start_activities/start_activities_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_start_activities` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_end_activities` | `filtering.py` → `algo/filtering/log/end_activities/end_activities_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_end_activities` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_event_attribute_values` | `filtering.py` → `algo/filtering/log/attributes/attributes_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_event_attribute_values` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_trace_attribute_values` | `filtering.py` → `algo/filtering/log/attributes/attributes_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_trace_attribute_values` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_variants` | `filtering.py` → `algo/filtering/log/variants/variants_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_variants` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_directly_follows_relation` | `filtering.py` → `algo/filtering/log/paths/paths_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_directly_follows_relation` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_eventually_follows_relation` | `filtering.py` → `algo/filtering/log/ltl/ltl_checker`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_eventually_follows_relation` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_time_range` | `filtering.py` → `algo/filtering/log/timestamp/timestamp_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_time_range` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_between` | `filtering.py` → `algo/filtering/log/between/between_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_between` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_case_size` | `filtering.py` → `algo/filtering/log/cases/case_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_case_size` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_case_performance` | `filtering.py` → `algo/filtering/log/cases/case_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_case_performance` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_activities_rework` | `filtering.py` → `algo/filtering/log/rework/rework_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_activities_rework` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_paths_performance` | `filtering.py` → `algo/filtering/log/paths/paths_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_paths_performance` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_variants_top_k` | `filtering.py` → `algo/filtering/log/variants/variants_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_variants_top_k` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_variants_by_coverage_percentage` | `filtering.py` → `algo/filtering/log/variants/variants_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_variants_by_coverage_percentage` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_prefixes` | `filtering.py` → `algo/filtering/log/prefixes/prefix_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_prefixes` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_suffixes` | `filtering.py` → `algo/filtering/log/suffixes/suffix_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_suffixes` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_ocel_event_attribute` | `filtering.py` → `algo/filtering/ocel/event_attributes`, `objects/ocel/obj` | `ichnos::core::filter_ocel_event_attribute` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_object_attribute` | `filtering.py` → `algo/filtering/ocel/object_attributes`, `objects/ocel/obj` | `ichnos::core::filter_ocel_object_attribute` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_object_types_allowed_activities` | `filtering.py` → `algo/filtering/ocel/activity_type_matching`, `objects/ocel/obj` | `ichnos::core::filter_ocel_object_types_allowed_activities` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_object_per_type_count` | `filtering.py` → `algo/filtering/ocel/objects_ot_count`, `objects/ocel/obj` | `ichnos::core::filter_ocel_object_per_type_count` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_start_events_per_object_type` | `filtering.py` → `algo/filtering/ocel/ot_endpoints`, `objects/ocel/obj` | `ichnos::core::filter_ocel_start_events_per_object_type` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_end_events_per_object_type` | `filtering.py` → `algo/filtering/ocel/ot_endpoints`, `objects/ocel/obj` | `ichnos::core::filter_ocel_end_events_per_object_type` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_events_timestamp` | `filtering.py` → `algo/filtering/ocel/event_attributes`, `objects/ocel/obj` | `ichnos::core::filter_ocel_events_timestamp` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_four_eyes_principle` | `filtering.py` → `algo/filtering/log/ltl/ltl_checker`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_four_eyes_principle` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_activity_done_different_resources` | `filtering.py` → `algo/filtering/log/ltl/ltl_checker`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_activity_done_different_resources` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_trace_segments` | `filtering.py` → `algo/filtering/log/traces/trace_filter`, `algo/filtering/pandas`, `algo/filtering/polars`, `objects/log/obj` | `ichnos::core::filter_trace_segments` (planned) | `ichnos-core` | todo | Variants: variants_filter. |
| `pm4py.filter_ocel_object_types` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_object_types` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_objects` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_objects` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_events` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_events` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_activities_connected_object_type` | `filtering.py` → `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_activities_connected_object_type` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_object` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_cc_object` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_length` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_cc_length` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_otype` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_cc_otype` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_ocel_cc_activity` | `filtering.py` → `algo/transformation/ocel/graphs/object_interaction_graph`, `objects/ocel/obj`, `objects/ocel/util/filtering_utils` | `ichnos::core::filter_ocel_cc_activity` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_dfg_activities_percentage` | `filtering.py` → `algo/filtering/dfg/dfg_filtering` | `ichnos::core::filter_dfg_activities_percentage` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.filter_dfg_paths_percentage` | `filtering.py` → `algo/filtering/dfg/dfg_filtering` | `ichnos::core::filter_dfg_paths_percentage` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |

## stats

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.get_start_activities` | `stats.py` → `objects/log/obj`, `statistics/start_activities/log/get` | `ichnos::stats::get_start_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_end_activities` | `stats.py` → `objects/log/obj`, `statistics/end_activities/log/get` | `ichnos::stats::get_end_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_event_attributes` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos::stats::get_event_attributes` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_trace_attributes` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos::stats::get_trace_attributes` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_event_attribute_values` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos::stats::get_event_attribute_values` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_trace_attribute_values` | `stats.py` → `objects/log/obj`, `statistics/attributes/log/get` | `ichnos::stats::get_trace_attribute_values` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_variants` | `stats.py` → `objects/log/obj`, `statistics/variants/log/get` | `ichnos::stats::get_variants` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_variants_as_tuples` | `stats.py` → `objects/log/obj`, `statistics/variants/log/get` | `ichnos::stats::get_variants_as_tuples` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.split_by_process_variant` | `stats.py` → `objects/log/obj`, `objects/log/util/pandas_numpy_variants` | `ichnos::stats::split_by_process_variant` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_variants_paths_duration` | `stats.py` → `objects/log/obj`, `objects/log/util/pandas_numpy_variants` | `ichnos::stats::get_variants_paths_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_stochastic_language` | `stats.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/petri_net/obj`, `objects/process_tree/obj`, `statistics/variants/log/get` | `ichnos::stats::get_stochastic_language` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.get_minimum_self_distances` | `stats.py` → `algo/discovery/minimum_self_distance/algorithm`, `objects/log/obj` | `ichnos::stats::get_minimum_self_distances` (planned) | `ichnos-stats` | todo | Variants: log, pandas, polars. |
| `pm4py.get_minimum_self_distance_witnesses` | `stats.py` → `algo/discovery/minimum_self_distance/algorithm`, `algo/discovery/minimum_self_distance/utils`, `objects/log/obj` | `ichnos::stats::get_minimum_self_distance_witnesses` (planned) | `ichnos-stats` | todo | Variants: log, pandas, polars. |
| `pm4py.get_case_arrival_average` | `stats.py` → `objects/log/obj`, `statistics/traces/generic/log/case_arrival` | `ichnos::stats::get_case_arrival_average` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_rework_cases_per_activity` | `stats.py` → `objects/log/obj`, `statistics/rework/log/get` | `ichnos::stats::get_rework_cases_per_activity` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_case_overlap` | `stats.py` → `objects/log/obj`, `statistics/overlap/cases/log/get` | `ichnos::stats::get_case_overlap` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_cycle_time` | `stats.py` → `objects/log/obj`, `statistics/traces/cycle_time/log/get` | `ichnos::stats::get_cycle_time` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_service_time` | `stats.py` → `objects/log/obj`, `statistics/service_time/log/get` | `ichnos::stats::get_service_time` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_all_case_durations` | `stats.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics` | `ichnos::stats::get_all_case_durations` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_case_duration` | `stats.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics` | `ichnos::stats::get_case_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_frequent_trace_segments` | `stats.py` → `objects/log/obj` | `ichnos::stats::get_frequent_trace_segments` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_activity_position_summary` | `stats.py` → `objects/log/obj` | `ichnos::stats::get_activity_position_summary` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_process_cube` | `stats.py` → `statistics/process_cube/pandas/algorithm`, `statistics/process_cube/polars/algorithm` | `ichnos::stats::get_process_cube` (planned) | `ichnos-stats` | todo | Variants: classic. |

## utils

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.utils.Shared` | `utils.py` | `ichnos::core::Shared` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.utils.is_polars_lazyframe` | `utils.py` | `ichnos::core::is_polars_lazyframe` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.format_dataframe` | `utils.py` → `objects/log/util/dataframe_utils` | `ichnos::core::format_dataframe` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.rebase` | `utils.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/dataframe_utils` | `ichnos::core::rebase` (planned) | `ichnos-core` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.parse_process_tree` | `utils.py` → `objects/process_tree/obj`, `objects/process_tree/utils/generic` | `ichnos::core::parse_process_tree` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.parse_powl_model_string` | `utils.py` → `objects/powl/obj`, `objects/powl/parser` | `ichnos::core::parse_powl_model_string` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.serialize` | `utils.py` → `objects/bpmn/exporter/exporter`, `objects/bpmn/obj`, `objects/dfg/exporter/exporter`, `objects/log/exporter/xes/exporter`, `objects/log/obj`, `objects/petri_net/exporter/exporter`, `objects/petri_net/obj`, `objects/process_tree/exporter/exporter`, `objects/process_tree/obj` | `ichnos::core::serialize` (planned) | `ichnos-core` | todo | Variants: classic, etree, etree_xes_exp, line_by_line, pnml, ptml. |
| `pm4py.deserialize` | `utils.py` → `objects/bpmn/importer/importer`, `objects/dfg/importer/importer`, `objects/log/importer/xes/importer`, `objects/petri_net/importer/importer`, `objects/process_tree/importer/importer` | `ichnos::core::deserialize` (planned) | `ichnos-core` | todo | Variants: chunk_regex, classic, iterparse, iterparse_20, iterparse_mem_compressed, line_by_line, lxml, pnml, ptml, rustxes. |
| `pm4py.utils.get_properties` | `utils.py` | `ichnos::core::get_properties` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.set_classifier` | `utils.py` → `objects/log/obj` | `ichnos::core::set_classifier` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.parse_event_log_string` | `utils.py` → `objects/log/obj` | `ichnos::core::parse_event_log_string` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.project_on_event_attribute` | `utils.py` → `objects/log/obj`, `streaming/conversion/from_pandas` | `ichnos::core::project_on_event_attribute` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.sample_cases` | `utils.py` → `objects/log/obj`, `objects/log/util/dataframe_utils`, `objects/log/util/sampling` | `ichnos::core::sample_cases` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.sample_events` | `utils.py` → `objects/log/obj`, `objects/log/util/sampling`, `objects/ocel/obj`, `objects/ocel/util/sampling` | `ichnos::core::sample_events` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |

## vis

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.view_petri_net` | `vis.py` → `objects/log/obj`, `objects/petri_net/obj`, `visualization/petri_net/visualizer` | `ichnos::viz::view_petri_net` (planned) | `ichnos-viz` | todo | Variants: alignments, frequency, frequency_greedy, greedy_decoration_frequency, greedy_decoration_performance, performance, performance_greedy, token_decoration_frequency, token_decoration_performance, wo_decoration. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_petri_net` | `vis.py` → `objects/log/obj`, `objects/petri_net/obj`, `visualization/petri_net/visualizer` | `ichnos::viz::save_vis_petri_net` (planned) | `ichnos-viz` | todo | Variants: alignments, frequency, frequency_greedy, greedy_decoration_frequency, greedy_decoration_performance, performance, performance_greedy, token_decoration_frequency, token_decoration_performance, wo_decoration. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_performance_dfg` | `vis.py` → `visualization/dfg/variants/performance`, `visualization/dfg/visualizer` | `ichnos::viz::view_performance_dfg` (planned) | `ichnos-viz` | todo | Variants: cost, frequency, performance, timeline. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_performance_dfg` | `vis.py` → `visualization/dfg/variants/performance`, `visualization/dfg/visualizer` | `ichnos::viz::save_vis_performance_dfg` (planned) | `ichnos-viz` | todo | Variants: cost, frequency, performance, timeline. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_dfg` | `vis.py` → `visualization/dfg/visualizer` | `ichnos::viz::view_dfg` (planned) | `ichnos-viz` | todo | Variants: cost, frequency, performance, timeline. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_dfg` | `vis.py` → `visualization/dfg/visualizer` | `ichnos::viz::save_vis_dfg` (planned) | `ichnos-viz` | todo | Variants: cost, frequency, performance, timeline. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_process_tree` | `vis.py` → `objects/process_tree/obj`, `visualization/process_tree/visualizer` | `ichnos::viz::view_process_tree` (planned) | `ichnos-viz` | todo | Variants: frequency_annotation, symbolic, wo_decoration. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_process_tree` | `vis.py` → `objects/process_tree/obj`, `visualization/process_tree/visualizer` | `ichnos::viz::save_vis_process_tree` (planned) | `ichnos-viz` | todo | Variants: frequency_annotation, symbolic, wo_decoration. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_bpmn` | `vis.py` → `objects/bpmn/obj`, `visualization/bpmn/visualizer` | `ichnos::viz::save_vis_bpmn` (planned) | `ichnos-viz` | todo | Variants: bpmnio_auto_layout, classic, dagrejs. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_bpmn` | `vis.py` → `objects/bpmn/obj`, `visualization/bpmn/visualizer` | `ichnos::viz::view_bpmn` (planned) | `ichnos-viz` | todo | Variants: bpmnio_auto_layout, classic, dagrejs. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_heuristics_net` | `vis.py` → `objects/heuristics_net/obj`, `visualization/heuristics_net/visualizer` | `ichnos::viz::view_heuristics_net` (planned) | `ichnos-viz` | todo | Variants: pydotplus, pydotplus_vis. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_heuristics_net` | `vis.py` → `objects/heuristics_net/obj`, `visualization/heuristics_net/visualizer` | `ichnos::viz::save_vis_heuristics_net` (planned) | `ichnos-viz` | todo | Variants: pydotplus, pydotplus_vis. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_dotted_chart` | `vis.py` → `objects/log/obj`, `visualization/dotted_chart/visualizer` | `ichnos::viz::view_dotted_chart` (planned) | `ichnos-viz` | todo | Variants: classic. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_dotted_chart` | `vis.py` → `objects/log/obj`, `visualization/dotted_chart/visualizer` | `ichnos::viz::save_vis_dotted_chart` (planned) | `ichnos-viz` | todo | Variants: classic. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_sna` | `vis.py` → `objects/org/sna/obj`, `visualization/sna/visualizer` | `ichnos::viz::view_sna` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: networkx, pyvis. |
| `pm4py.save_vis_sna` | `vis.py` → `objects/org/sna/obj`, `visualization/sna/visualizer` | `ichnos::viz::save_vis_sna` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: networkx, pyvis. |
| `pm4py.view_case_duration_graph` | `vis.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics`, `statistics/traces/generic/pandas/case_statistics`, `visualization/graphs/visualizer` | `ichnos::viz::view_case_duration_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.save_vis_case_duration_graph` | `vis.py` → `objects/log/obj`, `statistics/traces/generic/log/case_statistics`, `statistics/traces/generic/pandas/case_statistics`, `visualization/graphs/visualizer` | `ichnos::viz::save_vis_case_duration_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.view_events_per_time_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::view_events_per_time_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.save_vis_events_per_time_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::save_vis_events_per_time_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.view_performance_spectrum` | `vis.py` → `algo/discovery/performance_spectrum/algorithm`, `objects/log/obj`, `visualization/performance_spectrum/variants/neato`, `visualization/performance_spectrum/visualizer` | `ichnos::viz::view_performance_spectrum` (planned) | `ichnos-viz` | todo | Variants: dataframe, dataframe_disconnected, lazyframe, lazyframe_disconnected, log, log_disconnected, neato. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_performance_spectrum` | `vis.py` → `algo/discovery/performance_spectrum/algorithm`, `objects/log/obj`, `visualization/performance_spectrum/variants/neato`, `visualization/performance_spectrum/visualizer` | `ichnos::viz::save_vis_performance_spectrum` (planned) | `ichnos-viz` | todo | Variants: dataframe, dataframe_disconnected, lazyframe, lazyframe_disconnected, log, log_disconnected, neato. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_events_distribution_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::view_events_distribution_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.save_vis_events_distribution_graph` | `vis.py` → `objects/log/obj`, `statistics/attributes/log/get`, `statistics/attributes/pandas/get`, `visualization/graphs/visualizer` | `ichnos::viz::save_vis_events_distribution_graph` (planned) | `ichnos-viz` | dropped | HTML/pyvis or matplotlib renderer is out of scope. Variants: attributes, barplot, cases, dates. |
| `pm4py.view_ocdfg` | `vis.py` → `visualization/ocel/ocdfg/visualizer` | `ichnos::viz::view_ocdfg` (planned) | `ichnos-viz` | todo | Variants: classic, elkjs. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_ocdfg` | `vis.py` → `visualization/ocel/ocdfg/visualizer` | `ichnos::viz::save_vis_ocdfg` (planned) | `ichnos-viz` | todo | Variants: classic, elkjs. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_ocpn` | `vis.py` → `objects/ocpn/obj`, `visualization/ocel/ocpn/visualizer` | `ichnos::viz::view_ocpn` (planned) | `ichnos-viz` | todo | Variants: brachmann, wo_decoration. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_ocpn` | `vis.py` → `objects/ocpn/obj`, `visualization/ocel/ocpn/visualizer` | `ichnos::viz::save_vis_ocpn` (planned) | `ichnos-viz` | todo | Variants: brachmann, wo_decoration. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_network_analysis` | `vis.py` → `visualization/network_analysis/visualizer` | `ichnos::viz::view_network_analysis` (planned) | `ichnos-viz` | todo | Variants: frequency, performance. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_network_analysis` | `vis.py` → `visualization/network_analysis/visualizer` | `ichnos::viz::save_vis_network_analysis` (planned) | `ichnos-viz` | todo | Variants: frequency, performance. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_transition_system` | `vis.py` → `objects/transition_system/obj`, `visualization/transition_system/visualizer` | `ichnos::viz::view_transition_system` (planned) | `ichnos-viz` | todo | Variants: trans_frequency, view_based. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_transition_system` | `vis.py` → `objects/transition_system/obj`, `visualization/transition_system/visualizer` | `ichnos::viz::save_vis_transition_system` (planned) | `ichnos-viz` | todo | Variants: trans_frequency, view_based. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_prefix_tree` | `vis.py` → `objects/trie/obj`, `visualization/trie/visualizer` | `ichnos::viz::view_prefix_tree` (planned) | `ichnos-viz` | todo | Variants: classic. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_prefix_tree` | `vis.py` → `objects/trie/obj`, `visualization/trie/visualizer` | `ichnos::viz::save_vis_prefix_tree` (planned) | `ichnos-viz` | todo | Variants: classic. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_alignments` | `vis.py` → `objects/log/obj`, `visualization/align_table/visualizer` | `ichnos::viz::view_alignments` (planned) | `ichnos-viz` | todo | Variants: classic. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_alignments` | `vis.py` → `objects/log/obj`, `visualization/align_table/visualizer` | `ichnos::viz::save_vis_alignments` (planned) | `ichnos-viz` | todo | Variants: classic. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_footprints` | `vis.py` → `visualization/footprints/visualizer` | `ichnos::viz::view_footprints` (planned) | `ichnos-viz` | todo | Variants: comparison, comparison_symmetric, single. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_footprints` | `vis.py` → `visualization/footprints/visualizer` | `ichnos::viz::save_vis_footprints` (planned) | `ichnos-viz` | todo | Variants: comparison, comparison_symmetric, single. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_powl` | `vis.py` → `objects/powl/obj`, `visualization/powl/visualizer` | `ichnos::viz::view_powl` (planned) | `ichnos-viz` | todo | Variants: basic, net. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_powl` | `vis.py` → `objects/powl/obj`, `visualization/powl/visualizer` | `ichnos::viz::save_vis_powl` (planned) | `ichnos-viz` | todo | Variants: basic, net. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.view_object_graph` | `vis.py` → `objects/ocel/obj`, `visualization/ocel/object_graph/visualizer` | `ichnos::viz::view_object_graph` (planned) | `ichnos-viz` | todo | Variants: graphviz. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |
| `pm4py.save_vis_object_graph` | `vis.py` → `objects/ocel/obj`, `visualization/ocel/object_graph/visualizer` | `ichnos::viz::save_vis_object_graph` (planned) | `ichnos-viz` | todo | Variants: graphviz. Keep Graphviz/dot export; renderer opening is a wrapper concern. HTML/matplotlib alternatives excluded. |

## sim

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.play_out` | `sim.py` → `algo/simulation/playout/declare/algorithm`, `algo/simulation/playout/dfg/algorithm`, `algo/simulation/playout/petri_net/algorithm`, `algo/simulation/playout/process_tree/algorithm`, `objects/log/obj`, `objects/petri_net/inhibitor_reset/semantics`, `objects/petri_net/obj`, `objects/petri_net/semantics`, `objects/process_tree/obj` | `ichnos::sim::play_out` (planned) | `ichnos-sim` | todo | Variants: basic_playout, classic, extensive, performance, stochastic_playout, topbottom. |
| `pm4py.generate_process_tree` | `sim.py` → `algo/simulation/tree_generator/algorithm`, `objects/process_tree/obj` | `ichnos::sim::generate_process_tree` (planned) | `ichnos-sim` | todo | Variants: basic, ptandloggenerator. |

## ml

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.split_train_test` | `ml.py` → `objects/log/obj`, `objects/log/util/split_train_test` | `ichnos::ml::split_train_test` (planned) | `ichnos-ml` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_prefixes_from_log` | `ml.py` → `objects/log/obj`, `objects/log/util/get_prefixes` | `ichnos::ml::get_prefixes_from_log` (planned) | `ichnos-ml` | todo | Single entry point; preserve source defaults. |
| `pm4py.extract_outcome_enriched_dataframe` | `ml.py` → `algo/transformation/trace_encodings/algorithm`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::ml::extract_outcome_enriched_dataframe` (planned) | `ichnos-ml` | todo | Variants: alignments, bert, cases_transformers, count2vec, doc2vec, event_based, events_transformers, n_grams, one_hot, temporal, temporal_lazy, tf_idf, to_data_frame, to_event_log, to_event_stream, to_nx, token_replay, trace_based, word2vec. |
| `pm4py.extract_features_dataframe` | `ml.py` → `algo/transformation/trace_encodings/algorithm`, `objects/log/obj` | `ichnos::ml::extract_features_dataframe` (planned) | `ichnos-ml` | todo | Variants: alignments, bert, cases_transformers, count2vec, doc2vec, event_based, events_transformers, n_grams, one_hot, temporal, temporal_lazy, tf_idf, token_replay, trace_based, word2vec. |
| `pm4py.extract_ocel_features` | `ml.py` → `algo/transformation/ocel/features/objects/algorithm`, `objects/ocel/obj` | `ichnos::ml::extract_ocel_features` (planned) | `ichnos-ml` | todo | Single entry point; preserve source defaults. |
| `pm4py.extract_temporal_features_dataframe` | `ml.py` → `algo/transformation/trace_encodings/variants/temporal`, `algo/transformation/trace_encodings/variants/temporal_lazy`, `objects/log/obj` | `ichnos::ml::extract_temporal_features_dataframe` (planned) | `ichnos-ml` | todo | Single entry point; preserve source defaults. |
| `pm4py.extract_target_vector` | `ml.py` → `algo/transformation/log_to_target/algorithm`, `objects/log/obj` | `ichnos::ml::extract_target_vector` (planned) | `ichnos-ml` | todo | Variants: next_activity, next_time, remaining_time. |

## org

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.discover_handover_of_work_network` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos::org::discover_handover_of_work_network` (planned) | `ichnos-org` | todo | Variants: handover_log, handover_pandas, jointactivities_log, jointactivities_pandas, subcontracting_log, subcontracting_pandas, working_together_log, working_together_pandas. |
| `pm4py.discover_working_together_network` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos::org::discover_working_together_network` (planned) | `ichnos-org` | todo | Variants: handover_log, handover_pandas, jointactivities_log, jointactivities_pandas, subcontracting_log, subcontracting_pandas, working_together_log, working_together_pandas. |
| `pm4py.discover_activity_based_resource_similarity` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos::org::discover_activity_based_resource_similarity` (planned) | `ichnos-org` | todo | Variants: handover_log, handover_pandas, jointactivities_log, jointactivities_pandas, subcontracting_log, subcontracting_pandas, working_together_log, working_together_pandas. |
| `pm4py.discover_subcontracting_network` | `org.py` → `algo/organizational_mining/sna/algorithm`, `objects/log/obj`, `objects/org/sna/obj` | `ichnos::org::discover_subcontracting_network` (planned) | `ichnos-org` | todo | Variants: handover_log, handover_pandas, jointactivities_log, jointactivities_pandas, subcontracting_log, subcontracting_pandas, working_together_log, working_together_pandas. |
| `pm4py.discover_organizational_roles` | `org.py` → `algo/organizational_mining/roles/algorithm`, `objects/log/obj`, `objects/org/roles/obj` | `ichnos::org::discover_organizational_roles` (planned) | `ichnos-org` | todo | Variants: log, pandas. |
| `pm4py.discover_network_analysis` | `org.py` → `algo/organizational_mining/network_analysis/algorithm`, `algo/organizational_mining/network_analysis/variants/dataframe`, `objects/log/obj` | `ichnos::org::discover_network_analysis` (planned) | `ichnos-org` | todo | Variants: dataframe. |

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
| `pm4py.convert_to_event_log` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::model::convert_to_event_log` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.convert_to_event_stream` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::model::convert_to_event_stream` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.convert_to_dataframe` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::model::convert_to_dataframe` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.convert_to_bpmn` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/process_tree/variants/to_bpmn`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/conversion/wf_net/variants/to_bpmn`, `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos::model::convert_to_bpmn` (planned) | `ichnos-model` | todo | Variants: to_petri_net. |
| `pm4py.convert_to_petri_net` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/genetic_matrix/obj`, `objects/heuristics_net/obj`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos::model::convert_to_petri_net` (planned) | `ichnos-model` | todo | Variants: to_petri_net. |
| `pm4py.convert_to_process_tree` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/powl/variants/to_process_tree`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/conversion/wf_net/variants/to_process_tree`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos::model::convert_to_process_tree` (planned) | `ichnos-model` | todo | Variants: to_petri_net. |
| `pm4py.convert_to_powl` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/process_tree/variants/to_powl`, `objects/conversion/wf_net/variants/to_powl`, `objects/petri_net/obj`, `objects/powl/obj`, `objects/process_tree/obj` | `ichnos::model::convert_to_powl` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.convert_to_reachability_graph` | `convert.py` → `objects/bpmn/obj`, `objects/conversion/bpmn/variants/to_petri_net`, `objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `objects/conversion/genetic_matrix/variants/to_petri_net`, `objects/conversion/heuristics_net/variants/to_petri_net`, `objects/conversion/powl/converter`, `objects/conversion/process_tree/variants/to_petri_net`, `objects/petri_net/obj`, `objects/petri_net/utils/reachability_graph`, `objects/process_tree/obj`, `objects/transition_system/obj` | `ichnos::model::convert_to_reachability_graph` (planned) | `ichnos-model` | todo | Variants: to_petri_net. |
| `pm4py.convert_log_to_ocel` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/ocel/obj`, `objects/ocel/util/log_ocel` | `ichnos::model::convert_log_to_ocel` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.convert_ocel_to_networkx` | `convert.py` → `objects/conversion/ocel/converter`, `objects/ocel/obj` | `ichnos::model::convert_ocel_to_networkx` (planned) | `ichnos-model` | todo | Variants: ocel_features_to_nx, ocel_to_nx. |
| `pm4py.convert_log_to_networkx` | `convert.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::model::convert_log_to_networkx` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.convert_log_to_time_intervals` | `convert.py` → `algo/transformation/log_to_interval_tree/variants/open_paths`, `objects/log/obj` | `ichnos::model::convert_log_to_time_intervals` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.convert_petri_net_to_networkx` | `convert.py` → `objects/petri_net/obj` | `ichnos::model::convert_petri_net_to_networkx` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.convert_petri_net_type` | `convert.py` → `objects/petri_net/obj`, `objects/petri_net/utils/petri_utils` | `ichnos::model::convert_petri_net_type` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |

## analysis

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.construct_synchronous_product_net` | `analysis.py` → `objects/log/obj`, `objects/petri_net/obj`, `objects/petri_net/utils/align_utils`, `objects/petri_net/utils/petri_utils`, `objects/petri_net/utils/synchronous_product` | `ichnos::model::construct_synchronous_product_net` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.compute_emd` | `analysis.py` → `algo/evaluation/earth_mover_distance/algorithm` | `ichnos::model::compute_emd` (planned) | `ichnos-model` | todo | Variants: pyemd. |
| `pm4py.solve_marking_equation` | `analysis.py` → `algo/analysis/marking_equation/algorithm`, `objects/petri_net/obj` | `ichnos::model::solve_marking_equation` (planned) | `ichnos-model` | todo | Variants: classic. |
| `pm4py.solve_extended_marking_equation` | `analysis.py` → `algo/analysis/extended_marking_equation/algorithm`, `objects/log/obj`, `objects/petri_net/obj` | `ichnos::model::solve_extended_marking_equation` (planned) | `ichnos-model` | todo | Variants: classic. |
| `pm4py.analysis.check_is_sound` | `analysis.py` → `algo/analysis/woflan/algorithm`, `objects/petri_net/obj` | `ichnos::model::check_is_sound` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.check_soundness` | `analysis.py` → `algo/analysis/woflan/algorithm`, `objects/petri_net/obj` | `ichnos::model::check_soundness` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.cluster_log` | `analysis.py` → `algo/clustering/profiles/algorithm`, `objects/log/obj` | `ichnos::model::cluster_log` (planned) | `ichnos-model` | todo | Variants: sklearn_profiles. |
| `pm4py.insert_artificial_start_end` | `analysis.py` → `objects/log/obj`, `objects/log/util/artificial`, `objects/log/util/dataframe_utils` | `ichnos::model::insert_artificial_start_end` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.insert_case_service_waiting_time` | `analysis.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::model::insert_case_service_waiting_time` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.insert_case_arrival_finish_rate` | `analysis.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::model::insert_case_arrival_finish_rate` (planned) | `ichnos-model` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.check_is_workflow_net` | `analysis.py` → `algo/analysis/workflow_net/algorithm`, `objects/petri_net/obj` | `ichnos::model::check_is_workflow_net` (planned) | `ichnos-model` | todo | Variants: petri_net. |
| `pm4py.maximal_decomposition` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/utils/decomposition` | `ichnos::model::maximal_decomposition` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.simplicity_petri_net` | `analysis.py` → `algo/evaluation/simplicity/variants/arc_degree`, `algo/evaluation/simplicity/variants/extended_cardoso`, `algo/evaluation/simplicity/variants/extended_cyclomatic`, `objects/petri_net/obj` | `ichnos::model::simplicity_petri_net` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.generate_marking` | `analysis.py` → `objects/petri_net/obj` | `ichnos::model::generate_marking` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.reduce_petri_net_invisibles` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/utils/reduction` | `ichnos::model::reduce_petri_net_invisibles` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.reduce_petri_net_implicit_places` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/utils/murata` | `ichnos::model::reduce_petri_net_implicit_places` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_enabled_transitions` | `analysis.py` → `objects/petri_net/obj`, `objects/petri_net/semantics` | `ichnos::model::get_enabled_transitions` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.get_activity_labels` | `analysis.py` → `objects/log/obj` | `ichnos::model::get_activity_labels` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.replace_activity_labels` | `analysis.py` → `objects/bpmn/obj`, `objects/bpmn/util/label_replacing`, `objects/petri_net/obj`, `objects/petri_net/utils/label_replacing`, `objects/powl/obj`, `objects/powl/utils/label_replacing`, `objects/process_tree/obj`, `objects/process_tree/utils/label_replacing` | `ichnos::model::replace_activity_labels` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.behavioral_similarity` | `analysis.py` → `objects/petri_net/obj`, `objects/process_tree/obj` | `ichnos::model::behavioral_similarity` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.structural_similarity` | `analysis.py` → `objects/process_tree/utils/struct_similarity` | `ichnos::model::structural_similarity` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.embeddings_similarity` | `analysis.py` → `objects/petri_net/utils/embeddings_similarity` | `ichnos::model::embeddings_similarity` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.label_sets_similarity` | `analysis.py` | `ichnos::model::label_sets_similarity` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |
| `pm4py.map_labels_from_second_model` | `analysis.py` → `objects/bpmn/obj`, `objects/bpmn/util/label_replacing`, `objects/petri_net/utils/label_replacing`, `objects/powl/obj`, `objects/powl/utils/label_replacing`, `objects/process_tree/utils/label_replacing` | `ichnos::model::map_labels_from_second_model` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |

## hof

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.hof.filter_log` | `hof.py` → `objects/log/obj` | `ichnos::core::filter_log` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.hof.filter_trace` | `hof.py` → `objects/log/obj` | `ichnos::core::filter_trace` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.hof.sort_log` | `hof.py` → `objects/log/obj` | `ichnos::core::sort_log` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |
| `pm4py.hof.sort_trace` | `hof.py` → `objects/log/obj` | `ichnos::core::sort_trace` (planned) | `ichnos-core` | todo | Single entry point; preserve source defaults. |

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
| `pm4py.statistics.attributes.common.get.get_sorted_attributes_list` | `statistics/attributes/common/get.py` | `ichnos::stats::attributes::common::get::get_sorted_attributes_list` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.common.get.get_attributes_threshold` | `statistics/attributes/common/get.py` | `ichnos::stats::attributes::common::get::get_attributes_threshold` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute` | `statistics/attributes/common/get.py` | `ichnos::stats::attributes::common::get::get_kde_numeric_attribute` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute_json` | `statistics/attributes/common/get.py` | `ichnos::stats::attributes::common::get::get_kde_numeric_attribute_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.common.get.get_kde_date_attribute` | `statistics/attributes/common/get.py` | `ichnos::stats::attributes::common::get::get_kde_date_attribute` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.common.get.get_kde_date_attribute_json` | `statistics/attributes/common/get.py` | `ichnos::stats::attributes::common::get::get_kde_date_attribute_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.attributes.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.log.get.get_events_distribution` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::get::get_events_distribution` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_all_trace_attributes_from_log` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::get::get_all_trace_attributes_from_log` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_all_event_attributes_from_log` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::get::get_all_event_attributes_from_log` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_attribute_values` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::get::get_attribute_values` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_trace_attribute_values` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::get::get_trace_attribute_values` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos::stats::attributes::log::get::get_kde_numeric_attribute` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute_json` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos::stats::attributes::log::get::get_kde_numeric_attribute_json` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_kde_date_attribute` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos::stats::attributes::log::get::get_kde_date_attribute` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.get.get_kde_date_attribute_json` | `statistics/attributes/log/get.py` → `objects/conversion/log/converter`, `statistics/attributes/common/get` | `ichnos::stats::attributes::log::get::get_kde_date_attribute_json` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.attributes.log.select

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.log.select.select_attributes_from_log_for_tree` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/sampling`, `statistics/attributes/log/get` | `ichnos::stats::attributes::log::select::select_attributes_from_log_for_tree` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.select.check_trace_attributes_presence` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::select::check_trace_attributes_presence` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.select.check_event_attributes_presence` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::select::check_event_attributes_presence` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.select.verify_if_event_attribute_is_in_each_trace` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::select::verify_if_event_attribute_is_in_each_trace` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.attributes.log.select.verify_if_trace_attribute_is_in_each_trace` | `statistics/attributes/log/select.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::attributes::log::select::verify_if_trace_attribute_is_in_each_trace` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.attributes.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.pandas.get.get_events_distribution` | `statistics/attributes/pandas/get.py` | `ichnos::stats::attributes::pandas::get::get_events_distribution` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.pandas.get.get_attribute_values` | `statistics/attributes/pandas/get.py` | `ichnos::stats::attributes::pandas::get::get_attribute_values` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.pandas.get.get_kde_numeric_attribute` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_numeric_attribute` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.pandas.get.get_kde_numeric_attribute_json` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_numeric_attribute_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.pandas.get.get_kde_date_attribute` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_date_attribute` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.pandas.get.get_kde_date_attribute_json` | `statistics/attributes/pandas/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::pandas::get::get_kde_date_attribute_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.attributes.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.attributes.polars.get.get_events_distribution` | `statistics/attributes/polars/get.py` | `ichnos::stats::attributes::polars::get::get_events_distribution` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.polars.get.get_attribute_values` | `statistics/attributes/polars/get.py` | `ichnos::stats::attributes::polars::get::get_attribute_values` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.polars.get.get_kde_numeric_attribute` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_numeric_attribute` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.polars.get.get_kde_numeric_attribute_json` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_numeric_attribute_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.polars.get.get_kde_date_attribute` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_date_attribute` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.attributes.polars.get.get_kde_date_attribute_json` | `statistics/attributes/polars/get.py` → `statistics/attributes/common/get` | `ichnos::stats::attributes::polars::get::get_kde_date_attribute_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.chaotic_activities.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.chaotic_activities.algorithm.apply` | `statistics/chaotic_activities/algorithm.py` → `objects/log/obj` | `ichnos::stats::chaotic_activities::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.chaotic_activities.variants.niek_sidorova

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.chaotic_activities.variants.niek_sidorova.apply` | `statistics/chaotic_activities/variants/niek_sidorova.py` → `objects/log/obj` | `ichnos::stats::chaotic_activities::variants::niek_sidorova::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.chaotic_activities.variants.niek_sidorova.chaotic_metrics` | `statistics/chaotic_activities/variants/niek_sidorova.py` | `ichnos::stats::chaotic_activities::variants::niek_sidorova::chaotic_metrics` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.chaotic_activities.variants.niek_sidorova.total_entropy` | `statistics/chaotic_activities/variants/niek_sidorova.py` | `ichnos::stats::chaotic_activities::variants::niek_sidorova::total_entropy` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.concurrent_activities.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.concurrent_activities.log.get.apply` | `statistics/concurrent_activities/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/sorting` | `ichnos::stats::concurrent_activities::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.concurrent_activities.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.concurrent_activities.pandas.get.apply` | `statistics/concurrent_activities/pandas/get.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::concurrent_activities::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.concurrent_activities.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.concurrent_activities.polars.get.get_concurrent_events_dataframe` | `statistics/concurrent_activities/polars/get.py` | `ichnos::stats::concurrent_activities::polars::get::get_concurrent_events_dataframe` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.concurrent_activities.polars.get.apply` | `statistics/concurrent_activities/polars/get.py` | `ichnos::stats::concurrent_activities::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.end_activities.common.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.common.get.get_sorted_end_activities_list` | `statistics/end_activities/common/get.py` | `ichnos::stats::end_activities::common::get::get_sorted_end_activities_list` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.end_activities.common.get.get_end_activities_threshold` | `statistics/end_activities/common/get.py` | `ichnos::stats::end_activities::common::get::get_end_activities_threshold` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.end_activities.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.log.get.get_end_activities` | `statistics/end_activities/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::end_activities::log::get::get_end_activities` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.end_activities.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.pandas.get.get_end_activities` | `statistics/end_activities/pandas/get.py` | `ichnos::stats::end_activities::pandas::get::get_end_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.end_activities.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.end_activities.polars.get.get_end_activities` | `statistics/end_activities/polars/get.py` | `ichnos::stats::end_activities::polars::get::get_end_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.eventually_follows.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.log.get.apply` | `statistics/eventually_follows/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `objects/log/util/sorting` | `ichnos::stats::eventually_follows::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.eventually_follows.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.pandas.get.apply` | `statistics/eventually_follows/pandas/get.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::eventually_follows::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.eventually_follows.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.polars.get.get_partial_order_dataframe` | `statistics/eventually_follows/polars/get.py` | `ichnos::stats::eventually_follows::polars::get::get_partial_order_dataframe` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.eventually_follows.polars.get.apply` | `statistics/eventually_follows/polars/get.py` | `ichnos::stats::eventually_follows::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.eventually_follows.uvcl.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.eventually_follows.uvcl.get.apply` | `statistics/eventually_follows/uvcl/get.py` → `algo/discovery/inductive/dtypes/im_ds` | `ichnos::stats::eventually_follows::uvcl::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

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
| `pm4py.statistics.overlap.cases.log.get.apply` | `statistics/overlap/cases/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/overlap/utils/compute` | `ichnos::stats::overlap::cases::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.overlap.cases.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.cases.pandas.get.apply` | `statistics/overlap/cases/pandas/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::cases::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.overlap.cases.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.cases.polars.get.apply` | `statistics/overlap/cases/polars/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::cases::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.overlap.interval_events.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.interval_events.log.get.apply` | `statistics/overlap/interval_events/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/overlap/utils/compute` | `ichnos::stats::overlap::interval_events::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.overlap.interval_events.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.interval_events.pandas.get.apply` | `statistics/overlap/interval_events/pandas/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::interval_events::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.overlap.interval_events.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.interval_events.polars.get.apply` | `statistics/overlap/interval_events/polars/get.py` → `statistics/overlap/utils/compute` | `ichnos::stats::overlap::interval_events::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.overlap.utils.compute

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.overlap.utils.compute.apply` | `statistics/overlap/utils/compute.py` | `ichnos::stats::overlap::utils::compute::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.log.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.algorithm.apply` | `statistics/passed_time/log/algorithm.py` → `objects/log/obj` | `ichnos::stats::passed_time::log::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.log.variants.post

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.variants.post.apply` | `statistics/passed_time/log/variants/post.py` → `algo/discovery/dfg/variants/native`, `algo/discovery/dfg/variants/performance`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::passed_time::log::variants::post::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.passed_time.log.variants.pre

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.variants.pre.apply` | `statistics/passed_time/log/variants/pre.py` → `algo/discovery/dfg/variants/native`, `algo/discovery/dfg/variants/performance`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::passed_time::log::variants::pre::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.passed_time.log.variants.prepost

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.log.variants.prepost.apply` | `statistics/passed_time/log/variants/prepost.py` → `algo/discovery/dfg/variants/native`, `algo/discovery/dfg/variants/performance`, `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::passed_time::log::variants::prepost::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.passed_time.pandas.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.algorithm.apply` | `statistics/passed_time/pandas/algorithm.py` | `ichnos::stats::passed_time::pandas::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.pandas.variants.post

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.variants.post.apply` | `statistics/passed_time/pandas/variants/post.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::passed_time::pandas::variants::post::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.pandas.variants.pre

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.variants.pre.apply` | `statistics/passed_time/pandas/variants/pre.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::passed_time::pandas::variants::pre::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.pandas.variants.prepost

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.pandas.variants.prepost.apply` | `statistics/passed_time/pandas/variants/prepost.py` → `algo/discovery/dfg/adapters/pandas/df_statistics` | `ichnos::stats::passed_time::pandas::variants::prepost::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.polars.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.algorithm.apply` | `statistics/passed_time/polars/algorithm.py` | `ichnos::stats::passed_time::polars::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.polars.variants.post

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.variants.post.apply` | `statistics/passed_time/polars/variants/post.py` | `ichnos::stats::passed_time::polars::variants::post::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.polars.variants.pre

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.variants.pre.apply` | `statistics/passed_time/polars/variants/pre.py` | `ichnos::stats::passed_time::polars::variants::pre::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.passed_time.polars.variants.prepost

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.passed_time.polars.variants.prepost.apply` | `statistics/passed_time/polars/variants/prepost.py` → `statistics/passed_time/polars/variants/post`, `statistics/passed_time/polars/variants/pre` | `ichnos::stats::passed_time::polars::variants::prepost::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.process_cube.pandas.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.pandas.algorithm.apply` | `statistics/process_cube/pandas/algorithm.py` | `ichnos::stats::process_cube::pandas::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.process_cube.pandas.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.pandas.variants.classic.apply` | `statistics/process_cube/pandas/variants/classic.py` | `ichnos::stats::process_cube::pandas::variants::classic::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.process_cube.polars.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.polars.algorithm.apply` | `statistics/process_cube/polars/algorithm.py` | `ichnos::stats::process_cube::polars::algorithm::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.process_cube.polars.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.process_cube.polars.variants.classic.apply` | `statistics/process_cube/polars/variants/classic.py` | `ichnos::stats::process_cube::polars::variants::classic::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.rework.cases.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.cases.log.get.apply` | `statistics/rework/cases/log/get.py` → `objects/log/obj` | `ichnos::stats::rework::cases::log::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.rework.cases.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.cases.pandas.get.apply` | `statistics/rework/cases/pandas/get.py` | `ichnos::stats::rework::cases::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.rework.cases.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.cases.polars.get.apply` | `statistics/rework/cases/polars/get.py` | `ichnos::stats::rework::cases::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.rework.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.log.get.apply` | `statistics/rework/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::rework::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.rework.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.pandas.get.apply` | `statistics/rework/pandas/get.py` | `ichnos::stats::rework::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.rework.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.rework.polars.get.apply` | `statistics/rework/polars/get.py` | `ichnos::stats::rework::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.service_time.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.service_time.log.get.apply` | `statistics/service_time/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::service_time::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.service_time.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.service_time.pandas.get.apply` | `statistics/service_time/pandas/get.py` | `ichnos::stats::service_time::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.service_time.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.service_time.polars.get.apply` | `statistics/service_time/polars/get.py` | `ichnos::stats::service_time::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.start_activities.common.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.common.get.get_sorted_start_activities_list` | `statistics/start_activities/common/get.py` | `ichnos::stats::start_activities::common::get::get_sorted_start_activities_list` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.start_activities.common.get.get_start_activities_threshold` | `statistics/start_activities/common/get.py` | `ichnos::stats::start_activities::common::get::get_start_activities_threshold` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.start_activities.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.log.get.get_start_activities` | `statistics/start_activities/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::start_activities::log::get::get_start_activities` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.start_activities.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.pandas.get.get_start_activities` | `statistics/start_activities/pandas/get.py` | `ichnos::stats::start_activities::pandas::get::get_start_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.start_activities.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.start_activities.polars.get.get_start_activities` | `statistics/start_activities/polars/get.py` | `ichnos::stats::start_activities::polars::get::get_start_activities` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.cycle_time.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.log.get.apply` | `statistics/traces/cycle_time/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/traces/cycle_time/util/compute` | `ichnos::stats::traces::cycle_time::log::get::apply` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.traces.cycle_time.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.pandas.get.apply` | `statistics/traces/cycle_time/pandas/get.py` → `statistics/traces/cycle_time/util/compute` | `ichnos::stats::traces::cycle_time::pandas::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.cycle_time.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.polars.get.apply` | `statistics/traces/cycle_time/polars/get.py` → `statistics/traces/cycle_time/util/compute` | `ichnos::stats::traces::cycle_time::polars::get::apply` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.cycle_time.util.compute

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.cycle_time.util.compute.cycle_time` | `statistics/traces/cycle_time/util/compute.py` | `ichnos::stats::traces::cycle_time::util::compute::cycle_time` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.generic.common.case_duration

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration` | `statistics/traces/generic/common/case_duration.py` | `ichnos::stats::traces::generic::common::case_duration::get_kde_caseduration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration_json` | `statistics/traces/generic/common/case_duration.py` | `ichnos::stats::traces::generic::common::case_duration::get_kde_caseduration_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.generic.log.case_arrival

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.log.case_arrival.get_case_arrival_avg` | `statistics/traces/generic/log/case_arrival.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_arrival::get_case_arrival_avg` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg` | `statistics/traces/generic/log/case_arrival.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_arrival::get_case_dispersion_avg` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.traces.generic.log.case_statistics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.log.case_statistics.get_variant_statistics` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj`, `statistics/variants/log/get` | `ichnos::stats::traces::generic::log::case_statistics::get_variant_statistics` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_cases_description` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_statistics::get_cases_description` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.index_log_caseid` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter` | `ichnos::stats::traces::generic::log::case_statistics::index_log_caseid` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_events` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_statistics::get_events` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_all_case_durations` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_statistics::get_all_case_durations` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_first_quartile_case_duration` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_statistics::get_first_quartile_case_duration` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_median_case_duration` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::traces::generic::log::case_statistics::get_median_case_duration` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::log::case_statistics::get_kde_caseduration` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json` | `statistics/traces/generic/log/case_statistics.py` → `objects/conversion/log/converter`, `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::log::case_statistics::get_kde_caseduration_json` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.traces.generic.pandas.case_arrival

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.pandas.case_arrival.get_case_arrival_avg` | `statistics/traces/generic/pandas/case_arrival.py` | `ichnos::stats::traces::generic::pandas::case_arrival::get_case_arrival_avg` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_arrival.get_case_dispersion_avg` | `statistics/traces/generic/pandas/case_arrival.py` | `ichnos::stats::traces::generic::pandas::case_arrival::get_case_dispersion_avg` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.generic.pandas.case_statistics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variant_statistics` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_variant_statistics` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_and_list` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_variants_df_and_list` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_cases_description` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_cases_description` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_variants_df` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_with_case_duration` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_variants_df_with_case_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_events` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_events` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_kde_caseduration` | `statistics/traces/generic/pandas/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::pandas::case_statistics::get_kde_caseduration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_kde_caseduration_json` | `statistics/traces/generic/pandas/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::pandas::case_statistics::get_kde_caseduration_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_all_case_durations` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_all_case_durations` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_first_quartile_case_duration` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_first_quartile_case_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.pandas.case_statistics.get_median_case_duration` | `statistics/traces/generic/pandas/case_statistics.py` | `ichnos::stats::traces::generic::pandas::case_statistics::get_median_case_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.generic.polars.case_arrival

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.polars.case_arrival.get_case_arrival_avg` | `statistics/traces/generic/polars/case_arrival.py` | `ichnos::stats::traces::generic::polars::case_arrival::get_case_arrival_avg` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_arrival.get_case_dispersion_avg` | `statistics/traces/generic/polars/case_arrival.py` | `ichnos::stats::traces::generic::polars::case_arrival::get_case_dispersion_avg` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.traces.generic.polars.case_statistics

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_variant_statistics` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_variant_statistics` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df_and_list` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_variants_df_and_list` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_cases_description` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_cases_description` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_variants_df` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_all_case_durations` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_all_case_durations` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_median_case_duration` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_median_case_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_first_quartile_case_duration` | `statistics/traces/generic/polars/case_statistics.py` | `ichnos::stats::traces::generic::polars::case_statistics::get_first_quartile_case_duration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_kde_caseduration` | `statistics/traces/generic/polars/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::polars::case_statistics::get_kde_caseduration` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.traces.generic.polars.case_statistics.get_kde_caseduration_json` | `statistics/traces/generic/polars/case_statistics.py` → `statistics/traces/generic/common/case_duration` | `ichnos::stats::traces::generic::polars::case_statistics::get_kde_caseduration_json` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.util.times_bipartite_matching

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.util.times_bipartite_matching.exact_match_minimum_average` | `statistics/util/times_bipartite_matching.py` | `ichnos::stats::util::times_bipartite_matching::exact_match_minimum_average` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.variants.log.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.variants.log.get.get_language` | `statistics/variants/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::variants::log::get::get_language` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.variants.log.get.get_variants` | `statistics/variants/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::variants::log::get::get_variants` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.variants.log.get.get_variants_along_with_case_durations` | `statistics/variants/log/get.py` → `objects/conversion/log/converter`, `objects/log/obj` | `ichnos::stats::variants::log::get::get_variants_along_with_case_durations` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.variants.log.get.get_variants_from_log_trace_idx` | `statistics/variants/log/get.py` → `objects/conversion/log/converter` | `ichnos::stats::variants::log::get::get_variants_from_log_trace_idx` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |
| `pm4py.statistics.variants.log.get.get_variants_sorted_by_count` | `statistics/variants/log/get.py` | `ichnos::stats::variants::log::get::get_variants_sorted_by_count` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.variants.log.get.convert_variants_trace_idx_to_trace_obj` | `statistics/variants/log/get.py` → `objects/conversion/log/converter` | `ichnos::stats::variants::log::get::convert_variants_trace_idx_to_trace_obj` (planned) | `ichnos-stats` | todo | Variants: to_data_frame, to_event_log, to_event_stream, to_nx. |

## statistics.variants.pandas.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.variants.pandas.get.get_variants_count` | `statistics/variants/pandas/get.py` → `objects/log/util/pandas_numpy_variants` | `ichnos::stats::variants::pandas::get::get_variants_count` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.variants.pandas.get.get_variants_set` | `statistics/variants/pandas/get.py` | `ichnos::stats::variants::pandas::get::get_variants_set` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## statistics.variants.polars.get

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.statistics.variants.polars.get.pandas_numpy_variants_apply_polars` | `statistics/variants/polars/get.py` | `ichnos::stats::variants::polars::get::pandas_numpy_variants_apply_polars` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.variants.polars.get.get_variants_count` | `statistics/variants/polars/get.py` | `ichnos::stats::variants::polars::get::get_variants_count` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |
| `pm4py.statistics.variants.polars.get.get_variants_set` | `statistics/variants/polars/get.py` | `ichnos::stats::variants::polars::get::get_variants_set` (planned) | `ichnos-stats` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.alignments.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.alignments.algorithm.apply` | `streaming/algo/conformance/alignments/algorithm.py` | `ichnos::stream::algo::conformance::alignments::algorithm::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.alignments.variants.approx_iws

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._TrieNode` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `objects/petri_net/obj` | `ichnos::stream::algo::conformance::alignments::variants::approx_iws::_TrieNode` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._State` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `algo/conformance/alignments/petri_net/utils/approx_utils` | `ichnos::stream::algo::conformance::alignments::variants::approx_iws::_State` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `algo/conformance/alignments/petri_net/utils/approx_utils`, `objects/petri_net/obj`, `objects/petri_net/utils/align_utils`, `streaming/algo/interface` | `ichnos::stream::algo::conformance::alignments::variants::approx_iws::IWSStreamingAlignments` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments.finish` | `streaming/algo/conformance/alignments/variants/approx_iws.py` | `ichnos::stream::algo::conformance::alignments::variants::approx_iws::IWSStreamingAlignments::finish` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.apply` | `streaming/algo/conformance/alignments/variants/approx_iws.py` → `objects/petri_net/obj` | `ichnos::stream::algo::conformance::alignments::variants::approx_iws::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.declare.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.declare.algorithm.apply` | `streaming/algo/conformance/declare/algorithm.py` | `ichnos::stream::algo::conformance::declare::algorithm::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.declare.variants.automata

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.declare.variants.automata.DeclareStreamingConformance` | `streaming/algo/conformance/declare/variants/automata.py` → `streaming/algo/interface` | `ichnos::stream::algo::conformance::declare::variants::automata::DeclareStreamingConformance` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.declare.variants.automata.apply` | `streaming/algo/conformance/declare/variants/automata.py` | `ichnos::stream::algo::conformance::declare::variants::automata::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.footprints.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.footprints.algorithm.apply` | `streaming/algo/conformance/footprints/algorithm.py` | `ichnos::stream::algo::conformance::footprints::algorithm::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.footprints.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance` | `streaming/algo/conformance/footprints/variants/classic.py` → `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.build_dictionaries` | `streaming/algo/conformance/footprints/variants/classic.py` → `streaming/util/dictio/generator` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::build_dictionaries` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.encode_str` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::encode_str` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_footprints` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::verify_footprints` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_intra_case` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::verify_intra_case` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_start_case` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::verify_start_case` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.get_status` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::get_status` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::terminate` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate_all` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::terminate_all` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_or_activity_not_in_event` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::message_case_or_activity_not_in_event` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_activity_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::message_activity_not_possible` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_footprints_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::message_footprints_not_possible` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_start_activity_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::message_start_activity_not_possible` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_end_activity_not_possible` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::message_end_activity_not_possible` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_not_in_dictionary` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::FootprintsStreamingConformance::message_case_not_in_dictionary` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.footprints.variants.classic.apply` | `streaming/algo/conformance/footprints/variants/classic.py` | `ichnos::stream::algo::conformance::footprints::variants::classic::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.tbr.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.tbr.algorithm.apply` | `streaming/algo/conformance/tbr/algorithm.py` | `ichnos::stream::algo::conformance::tbr::algorithm::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.tbr.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/obj`, `objects/petri_net/semantics`, `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.build_dictionaries` | `streaming/algo/conformance/tbr/variants/classic.py` → `streaming/util/dictio/generator` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::build_dictionaries` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_paths_net` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/obj` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::get_paths_net` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_str` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::encode_str` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_marking` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::encode_marking` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.decode_marking` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/obj` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::decode_marking` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.verify_tbr` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/semantics` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::verify_tbr` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.enable_trans_with_invisibles` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/semantics` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::enable_trans_with_invisibles` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_status` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::get_status` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::terminate` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate_all` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::terminate_all` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.reach_fm_with_invisibles` | `streaming/algo/conformance/tbr/variants/classic.py` → `objects/petri_net/semantics` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::reach_fm_with_invisibles` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_or_activity_not_in_event` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::message_case_or_activity_not_in_event` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_activity_not_possible` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::message_activity_not_possible` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_missing_tokens` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::message_missing_tokens` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_not_in_dictionary` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::message_case_not_in_dictionary` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_final_marking_not_reached` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::TbrStreamingConformance::message_final_marking_not_reached` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.tbr.variants.classic.apply` | `streaming/algo/conformance/tbr/variants/classic.py` | `ichnos::stream::algo::conformance::tbr::variants::classic::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.temporal.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.temporal.algorithm.apply` | `streaming/algo/conformance/temporal/algorithm.py` | `ichnos::stream::algo::conformance::temporal::algorithm::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.conformance.temporal.variants.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance` | `streaming/algo/conformance/temporal/variants/classic.py` → `objects/log/obj`, `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos::stream::algo::conformance::temporal::variants::classic::TemporalProfileStreamingConformance` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.check_conformance` | `streaming/algo/conformance/temporal/variants/classic.py` | `ichnos::stream::algo::conformance::temporal::variants::classic::TemporalProfileStreamingConformance::check_conformance` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_event_is_not_complete` | `streaming/algo/conformance/temporal/variants/classic.py` → `objects/log/obj` | `ichnos::stream::algo::conformance::temporal::variants::classic::TemporalProfileStreamingConformance::message_event_is_not_complete` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_deviation` | `streaming/algo/conformance/temporal/variants/classic.py` | `ichnos::stream::algo::conformance::temporal::variants::classic::TemporalProfileStreamingConformance::message_deviation` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.conformance.temporal.variants.classic.apply` | `streaming/algo/conformance/temporal/variants/classic.py` | `ichnos::stream::algo::conformance::temporal::variants::classic::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.discovery.dfg.algorithm

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.discovery.dfg.algorithm.apply` | `streaming/algo/discovery/dfg/algorithm.py` | `ichnos::stream::algo::discovery::dfg::algorithm::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.discovery.dfg.variants.frequency

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery` | `streaming/algo/discovery/dfg/variants/frequency.py` → `streaming/algo/interface`, `streaming/util/dictio/generator` | `ichnos::stream::algo::discovery::dfg::variants::frequency::StreamingDfgDiscovery` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.build_dictionaries` | `streaming/algo/discovery/dfg/variants/frequency.py` → `streaming/util/dictio/generator` | `ichnos::stream::algo::discovery::dfg::variants::frequency::StreamingDfgDiscovery::build_dictionaries` (planned) | `ichnos-stream` | todo | Variants: classic, redis, thread_safe. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.event_without_activity_or_case` | `streaming/algo/discovery/dfg/variants/frequency.py` | `ichnos::stream::algo::discovery::dfg::variants::frequency::StreamingDfgDiscovery::event_without_activity_or_case` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_str` | `streaming/algo/discovery/dfg/variants/frequency.py` | `ichnos::stream::algo::discovery::dfg::variants::frequency::StreamingDfgDiscovery::encode_str` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_tuple` | `streaming/algo/discovery/dfg/variants/frequency.py` | `ichnos::stream::algo::discovery::dfg::variants::frequency::StreamingDfgDiscovery::encode_tuple` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.discovery.dfg.variants.frequency.apply` | `streaming/algo/discovery/dfg/variants/frequency.py` | `ichnos::stream::algo::discovery::dfg::variants::frequency::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.algo.interface

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.algo.interface.StreamingAlgorithm` | `streaming/algo/interface.py` | `ichnos::stream::algo::interface::StreamingAlgorithm` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.interface.StreamingAlgorithm.get` | `streaming/algo/interface.py` | `ichnos::stream::algo::interface::StreamingAlgorithm::get` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.algo.interface.StreamingAlgorithm.receive` | `streaming/algo/interface.py` | `ichnos::stream::algo::interface::StreamingAlgorithm::receive` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.connectors.windows.click_key_logger

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger` | `streaming/connectors/windows/click_key_logger.py` → `objects/log/obj` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.run` | `streaming/connectors/windows/click_key_logger.py` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger::run` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.stop` | `streaming/connectors/windows/click_key_logger.py` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger::stop` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.get_process_name` | `streaming/connectors/windows/click_key_logger.py` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger::get_process_name` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.record` | `streaming/connectors/windows/click_key_logger.py` → `objects/log/obj` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger::record` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_click` | `streaming/connectors/windows/click_key_logger.py` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger::on_click` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_key_release` | `streaming/connectors/windows/click_key_logger.py` | `ichnos::stream::connectors::windows::click_key_logger::WindowsEventLogger::on_key_release` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.conversion.from_pandas

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable` | `streaming/conversion/from_pandas.py` → `objects/log/obj`, `streaming/stream/live_trace_stream` | `ichnos::stream::conversion::from_pandas::PandasDataframeAsIterable` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.read_trace` | `streaming/conversion/from_pandas.py` → `objects/log/obj` | `ichnos::stream::conversion::from_pandas::PandasDataframeAsIterable::read_trace` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.reset` | `streaming/conversion/from_pandas.py` | `ichnos::stream::conversion::from_pandas::PandasDataframeAsIterable::reset` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.to_trace_stream` | `streaming/conversion/from_pandas.py` → `streaming/stream/live_trace_stream` | `ichnos::stream::conversion::from_pandas::PandasDataframeAsIterable::to_trace_stream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.conversion.from_pandas.apply` | `streaming/conversion/from_pandas.py` | `ichnos::stream::conversion::from_pandas::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.conversion.ocel_flatts_distributor

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor` | `streaming/conversion/ocel_flatts_distributor.py` → `objects/ocel/constants`, `streaming/stream/live_event_stream` | `ichnos::stream::conversion::ocel_flatts_distributor::OcelFlattsDistributor` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.register` | `streaming/conversion/ocel_flatts_distributor.py` → `streaming/stream/live_event_stream` | `ichnos::stream::conversion::ocel_flatts_distributor::OcelFlattsDistributor::register` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.append` | `streaming/conversion/ocel_flatts_distributor.py` | `ichnos::stream::conversion::ocel_flatts_distributor::OcelFlattsDistributor::append` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.importer.csv.importer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.csv.importer.apply` | `streaming/importer/csv/importer.py` | `ichnos::stream::importer::csv::importer::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.importer.csv.variants.csv_event_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos::stream::importer::csv::variants::csv_event_stream::CSVEventStreamReader` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.reset` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos::stream::importer::csv::variants::csv_event_stream::CSVEventStreamReader::reset` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.to_event_stream` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos::stream::importer::csv::variants::csv_event_stream::CSVEventStreamReader::to_event_stream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.read_event` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos::stream::importer::csv::variants::csv_event_stream::CSVEventStreamReader::read_event` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.csv.variants.csv_event_stream.apply` | `streaming/importer/csv/variants/csv_event_stream.py` | `ichnos::stream::importer::csv::variants::csv_event_stream::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.importer.xes.importer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.xes.importer.apply` | `streaming/importer/xes/importer.py` | `ichnos::stream::importer::xes::importer::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.importer.xes.variants.xes_event_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.parse_attribute` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos::stream::importer::xes::variants::xes_event_stream::parse_attribute` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader` | `streaming/importer/xes/variants/xes_event_stream.py` → `objects/log/obj` | `ichnos::stream::importer::xes::variants::xes_event_stream::StreamingEventXesReader` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.to_event_stream` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos::stream::importer::xes::variants::xes_event_stream::StreamingEventXesReader::to_event_stream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.reset` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos::stream::importer::xes::variants::xes_event_stream::StreamingEventXesReader::reset` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.read_event` | `streaming/importer/xes/variants/xes_event_stream.py` → `objects/log/obj` | `ichnos::stream::importer::xes::variants::xes_event_stream::StreamingEventXesReader::read_event` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_event_stream.apply` | `streaming/importer/xes/variants/xes_event_stream.py` | `ichnos::stream::importer::xes::variants::xes_event_stream::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.importer.xes.variants.xes_trace_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.parse_attribute` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos::stream::importer::xes::variants::xes_trace_stream::parse_attribute` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader` | `streaming/importer/xes/variants/xes_trace_stream.py` → `objects/log/obj` | `ichnos::stream::importer::xes::variants::xes_trace_stream::StreamingTraceXesReader` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.to_trace_stream` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos::stream::importer::xes::variants::xes_trace_stream::StreamingTraceXesReader::to_trace_stream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.reset` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos::stream::importer::xes::variants::xes_trace_stream::StreamingTraceXesReader::reset` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.read_trace` | `streaming/importer/xes/variants/xes_trace_stream.py` → `objects/log/obj` | `ichnos::stream::importer::xes::variants::xes_trace_stream::StreamingTraceXesReader::read_trace` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.importer.xes.variants.xes_trace_stream.apply` | `streaming/importer/xes/variants/xes_trace_stream.py` | `ichnos::stream::importer::xes::variants::xes_trace_stream::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.stream.live_event_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.stream.live_event_stream.StreamState` | `streaming/stream/live_event_stream.py` | `ichnos::stream::stream::live_event_stream::StreamState` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream` | `streaming/stream/live_event_stream.py` | `ichnos::stream::stream::live_event_stream::LiveEventStream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.append` | `streaming/stream/live_event_stream.py` | `ichnos::stream::stream::live_event_stream::LiveEventStream::append` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.start` | `streaming/stream/live_event_stream.py` | `ichnos::stream::stream::live_event_stream::LiveEventStream::start` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.stop` | `streaming/stream/live_event_stream.py` | `ichnos::stream::stream::live_event_stream::LiveEventStream::stop` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_event_stream.LiveEventStream.register` | `streaming/stream/live_event_stream.py` | `ichnos::stream::stream::live_event_stream::LiveEventStream::register` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.stream.live_trace_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.stream.live_trace_stream.StreamState` | `streaming/stream/live_trace_stream.py` | `ichnos::stream::stream::live_trace_stream::StreamState` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream` | `streaming/stream/live_trace_stream.py` | `ichnos::stream::stream::live_trace_stream::LiveTraceStream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.append` | `streaming/stream/live_trace_stream.py` | `ichnos::stream::stream::live_trace_stream::LiveTraceStream::append` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.start` | `streaming/stream/live_trace_stream.py` | `ichnos::stream::stream::live_trace_stream::LiveTraceStream::start` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.stop` | `streaming/stream/live_trace_stream.py` | `ichnos::stream::stream::live_trace_stream::LiveTraceStream::stop` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.register` | `streaming/stream/live_trace_stream.py` | `ichnos::stream::stream::live_trace_stream::LiveTraceStream::register` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.dictio.generator

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.generator.apply` | `streaming/util/dictio/generator.py` | `ichnos::stream::util::dictio::generator::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.dictio.versions.classic

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.versions.classic.apply` | `streaming/util/dictio/versions/classic.py` | `ichnos::stream::util::dictio::versions::classic::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.dictio.versions.redis

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::ThreadSafeRedisDict` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.keys` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::ThreadSafeRedisDict::keys` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.values` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::ThreadSafeRedisDict::values` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.itervalues` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::ThreadSafeRedisDict::itervalues` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushdb` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::ThreadSafeRedisDict::flushdb` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushall` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::ThreadSafeRedisDict::flushall` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.redis.apply` | `streaming/util/dictio/versions/redis.py` | `ichnos::stream::util::dictio::versions::redis::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.dictio.versions.thread_safe

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict` | `streaming/util/dictio/versions/thread_safe.py` | `ichnos::stream::util::dictio::versions::thread_safe::ThreadSafeDict` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.keys` | `streaming/util/dictio/versions/thread_safe.py` | `ichnos::stream::util::dictio::versions::thread_safe::ThreadSafeDict::keys` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.values` | `streaming/util/dictio/versions/thread_safe.py` | `ichnos::stream::util::dictio::versions::thread_safe::ThreadSafeDict::values` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.itervalues` | `streaming/util/dictio/versions/thread_safe.py` | `ichnos::stream::util::dictio::versions::thread_safe::ThreadSafeDict::itervalues` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |
| `pm4py.streaming.util.dictio.versions.thread_safe.apply` | `streaming/util/dictio/versions/thread_safe.py` | `ichnos::stream::util::dictio::versions::thread_safe::apply` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.event_stream_printer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.event_stream_printer.EventStreamPrinter` | `streaming/util/event_stream_printer.py` → `streaming/algo/interface` | `ichnos::stream::util::event_stream_printer::EventStreamPrinter` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.live_to_static_stream

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.live_to_static_stream.LiveToStaticStream` | `streaming/util/live_to_static_stream.py` → `objects/log/obj`, `streaming/algo/interface` | `ichnos::stream::util::live_to_static_stream::LiveToStaticStream` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## streaming.util.trace_stream_printer

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.streaming.util.trace_stream_printer.TraceStreamPrinter` | `streaming/util/trace_stream_printer.py` → `streaming/algo/interface` | `ichnos::stream::util::trace_stream_printer::TraceStreamPrinter` (planned) | `ichnos-stream` | todo | Single entry point; preserve source defaults. |

## __init__

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.sys` | `__init__.py` | `ichnos::core::exports::sys` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Python sys/time implementation leakage; lane must document compatibility decision. |
| `pm4py.time` | `__init__.py` | `ichnos::core::exports::time` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Python sys/time implementation leakage; lane must document compatibility decision. |
| `pm4py.objects` | `__init__.py` | `ichnos::core::exports::objects` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.util` | `__init__.py` | `ichnos::core::exports::util` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.utils` | `__init__.py` | `ichnos::core::exports::utils` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.algo` | `__init__.py` | `ichnos::core::exports::algo` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.statistics` | `__init__.py` | `ichnos::core::exports::statistics` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.visualization` | `__init__.py` | `ichnos::core::exports::visualization` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.llm` | `__init__.py` | `ichnos::core::exports::llm` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.connectors` | `__init__.py` | `ichnos::core::exports::connectors` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.analysis` | `__init__.py` | `ichnos::core::exports::analysis` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.conformance` | `__init__.py` | `ichnos::core::exports::conformance` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.convert` | `__init__.py` | `ichnos::core::exports::convert` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.discovery` | `__init__.py` | `ichnos::core::exports::discovery` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.filtering` | `__init__.py` | `ichnos::core::exports::filtering` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.hof` | `__init__.py` | `ichnos::core::exports::hof` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.ml` | `__init__.py` | `ichnos::core::exports::ml` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.ocel` | `__init__.py` | `ichnos::core::exports::ocel` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.org` | `__init__.py` | `ichnos::core::exports::org` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.read` | `__init__.py` | `ichnos::core::exports::read` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.sim` | `__init__.py` | `ichnos::core::exports::sim` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.stats` | `__init__.py` | `ichnos::core::exports::stats` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.vis` | `__init__.py` | `ichnos::core::exports::vis` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.write` | `__init__.py` | `ichnos::core::exports::write` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |
| `pm4py.meta` | `__init__.py` | `ichnos::core::exports::meta` (planned) | `ichnos-core` | todo | Namespace compatibility export; map to Rust modules, not a duplicate algorithm. Contents covered by defining-module rows where in inventory scope. |

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
| `pm4py.OCEL` | `objects/ocel/obj.py` → `objects/ocel/constants` | `ichnos::ocel::OCEL` (planned) | `ichnos-ocel` | todo | Single entry point; preserve source defaults. |

## objects.bpmn.obj

| pm4py | Source | ichnos | Crate | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `pm4py.BPMN` | `objects/bpmn/obj.py` | `ichnos::model::BPMN` (planned) | `ichnos-model` | todo | Single entry point; preserve source defaults. |

## Behaviour changes

Lanes record each deliberate change from pm4py here.

## Proposed lanes

Packages below cover every todo row exactly once. Each targets one to two agent-days; split a package if its first source inspection reveals a larger algorithm. Complete core/model packages before dependent I/O and miners, then conformance; statistics precede performance, organizational, ML and visualization consumers. Source dependencies may overlap: reuse earlier packages rather than porting them twice. Every package adds golden coverage for its listed rows.

### core---init---1

Crate: `ichnos-core`. Rows: `pm4py.sys`, `pm4py.time`, `pm4py.objects`, `pm4py.util`, `pm4py.utils`, `pm4py.algo`, `pm4py.statistics`, `pm4py.visualization`.

Port sources: `pm4py/__init__.py`.

### core---init---2

Crate: `ichnos-core`. Rows: `pm4py.llm`, `pm4py.connectors`, `pm4py.analysis`, `pm4py.conformance`, `pm4py.convert`, `pm4py.discovery`, `pm4py.filtering`, `pm4py.hof`.

Port sources: `pm4py/__init__.py`.

### core---init---3

Crate: `ichnos-core`. Rows: `pm4py.ml`, `pm4py.ocel`, `pm4py.org`, `pm4py.read`, `pm4py.sim`, `pm4py.stats`, `pm4py.vis`, `pm4py.write`.

Port sources: `pm4py/__init__.py`.

### core---init---4

Crate: `ichnos-core`. Rows: `pm4py.meta`.

Port sources: `pm4py/__init__.py`.

### core-filtering-1

Crate: `ichnos-core`. Rows: `pm4py.filter_log_relative_occurrence_event_attribute`, `pm4py.filter_start_activities`, `pm4py.filter_end_activities`, `pm4py.filter_event_attribute_values`, `pm4py.filter_trace_attribute_values`, `pm4py.filter_variants`, `pm4py.filter_directly_follows_relation`, `pm4py.filter_eventually_follows_relation`.

Port sources: `pm4py/algo/filtering/log/attributes/attributes_filter`, `pm4py/algo/filtering/log/end_activities/end_activities_filter`, `pm4py/algo/filtering/log/ltl/ltl_checker`, `pm4py/algo/filtering/log/paths/paths_filter`, `pm4py/algo/filtering/log/start_activities/start_activities_filter`, `pm4py/algo/filtering/log/variants/variants_filter`, `pm4py/algo/filtering/pandas`, `pm4py/algo/filtering/polars`, `pm4py/filtering.py`, `pm4py/objects/log/obj`.

### core-filtering-2

Crate: `ichnos-core`. Rows: `pm4py.filter_time_range`, `pm4py.filter_between`, `pm4py.filter_case_size`, `pm4py.filter_case_performance`, `pm4py.filter_activities_rework`, `pm4py.filter_paths_performance`, `pm4py.filter_variants_top_k`, `pm4py.filter_variants_by_coverage_percentage`.

Port sources: `pm4py/algo/filtering/log/between/between_filter`, `pm4py/algo/filtering/log/cases/case_filter`, `pm4py/algo/filtering/log/paths/paths_filter`, `pm4py/algo/filtering/log/rework/rework_filter`, `pm4py/algo/filtering/log/timestamp/timestamp_filter`, `pm4py/algo/filtering/log/variants/variants_filter`, `pm4py/algo/filtering/pandas`, `pm4py/algo/filtering/polars`, `pm4py/filtering.py`, `pm4py/objects/log/obj`.

### core-filtering-3

Crate: `ichnos-core`. Rows: `pm4py.filter_prefixes`, `pm4py.filter_suffixes`, `pm4py.filter_ocel_event_attribute`, `pm4py.filter_ocel_object_attribute`, `pm4py.filter_ocel_object_types_allowed_activities`, `pm4py.filter_ocel_object_per_type_count`, `pm4py.filter_ocel_start_events_per_object_type`, `pm4py.filter_ocel_end_events_per_object_type`.

Port sources: `pm4py/algo/filtering/log/prefixes/prefix_filter`, `pm4py/algo/filtering/log/suffixes/suffix_filter`, `pm4py/algo/filtering/ocel/activity_type_matching`, `pm4py/algo/filtering/ocel/event_attributes`, `pm4py/algo/filtering/ocel/object_attributes`, `pm4py/algo/filtering/ocel/objects_ot_count`, `pm4py/algo/filtering/ocel/ot_endpoints`, `pm4py/algo/filtering/pandas`, `pm4py/algo/filtering/polars`, `pm4py/filtering.py`, `pm4py/objects/log/obj`, `pm4py/objects/ocel/obj`.

### core-filtering-4

Crate: `ichnos-core`. Rows: `pm4py.filter_ocel_events_timestamp`, `pm4py.filter_four_eyes_principle`, `pm4py.filter_activity_done_different_resources`, `pm4py.filter_trace_segments`, `pm4py.filter_ocel_object_types`, `pm4py.filter_ocel_objects`, `pm4py.filter_ocel_events`, `pm4py.filter_ocel_activities_connected_object_type`.

Port sources: `pm4py/algo/filtering/log/ltl/ltl_checker`, `pm4py/algo/filtering/log/traces/trace_filter`, `pm4py/algo/filtering/ocel/event_attributes`, `pm4py/algo/filtering/pandas`, `pm4py/algo/filtering/polars`, `pm4py/filtering.py`, `pm4py/objects/log/obj`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/filtering_utils`.

### core-filtering-5

Crate: `ichnos-core`. Rows: `pm4py.filter_ocel_cc_object`, `pm4py.filter_ocel_cc_length`, `pm4py.filter_ocel_cc_otype`, `pm4py.filter_ocel_cc_activity`, `pm4py.filter_dfg_activities_percentage`, `pm4py.filter_dfg_paths_percentage`.

Port sources: `pm4py/algo/filtering/dfg/dfg_filtering`, `pm4py/algo/transformation/ocel/graphs/object_interaction_graph`, `pm4py/filtering.py`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/filtering_utils`.

### core-hof-1

Crate: `ichnos-core`. Rows: `pm4py.hof.filter_log`, `pm4py.hof.filter_trace`, `pm4py.hof.sort_log`, `pm4py.hof.sort_trace`.

Port sources: `pm4py/hof.py`, `pm4py/objects/log/obj`.

### core-utils-1

Crate: `ichnos-core`. Rows: `pm4py.utils.Shared`, `pm4py.utils.is_polars_lazyframe`, `pm4py.format_dataframe`, `pm4py.rebase`, `pm4py.parse_process_tree`, `pm4py.parse_powl_model_string`, `pm4py.serialize`, `pm4py.deserialize`.

Port sources: `pm4py/objects/bpmn/exporter/exporter`, `pm4py/objects/bpmn/importer/importer`, `pm4py/objects/bpmn/obj`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/dfg/exporter/exporter`, `pm4py/objects/dfg/importer/importer`, `pm4py/objects/log/exporter/xes/exporter`, `pm4py/objects/log/importer/xes/importer`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/dataframe_utils`, `pm4py/objects/petri_net/exporter/exporter`, `pm4py/objects/petri_net/importer/importer`, `pm4py/objects/petri_net/obj`, `pm4py/objects/powl/obj`, `pm4py/objects/powl/parser`, `pm4py/objects/process_tree/exporter/exporter`, `pm4py/objects/process_tree/importer/importer`, `pm4py/objects/process_tree/obj`, `pm4py/objects/process_tree/utils/generic`, `pm4py/utils.py`.

### core-utils-2

Crate: `ichnos-core`. Rows: `pm4py.utils.get_properties`, `pm4py.set_classifier`, `pm4py.parse_event_log_string`, `pm4py.project_on_event_attribute`, `pm4py.sample_cases`, `pm4py.sample_events`.

Port sources: `pm4py/objects/log/obj`, `pm4py/objects/log/util/dataframe_utils`, `pm4py/objects/log/util/sampling`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/sampling`, `pm4py/streaming/conversion/from_pandas`, `pm4py/utils.py`.

### model-objects-bpmn-obj-1

Crate: `ichnos-model`. Rows: `pm4py.BPMN`, `pm4py.replace_activity_labels`, `pm4py.map_labels_from_second_model`, `pm4py.convert_to_bpmn`, `pm4py.convert_to_petri_net`, `pm4py.convert_to_process_tree`, `pm4py.convert_to_powl`, `pm4py.convert_to_reachability_graph`.

Port sources: `pm4py/analysis.py`, `pm4py/convert.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/bpmn/obj.py`, `pm4py/objects/bpmn/util/label_replacing`, `pm4py/objects/conversion/bpmn/variants/to_petri_net`, `pm4py/objects/conversion/dfg/variants/to_petri_net_activity_defines_place`, `pm4py/objects/conversion/genetic_matrix/variants/to_petri_net`, `pm4py/objects/conversion/heuristics_net/variants/to_petri_net`, `pm4py/objects/conversion/powl/converter`, `pm4py/objects/conversion/powl/variants/to_process_tree`, `pm4py/objects/conversion/process_tree/variants/to_bpmn`, `pm4py/objects/conversion/process_tree/variants/to_petri_net`, `pm4py/objects/conversion/process_tree/variants/to_powl`, `pm4py/objects/conversion/wf_net/variants/to_bpmn`, `pm4py/objects/conversion/wf_net/variants/to_powl`, `pm4py/objects/conversion/wf_net/variants/to_process_tree`, `pm4py/objects/genetic_matrix/obj`, `pm4py/objects/heuristics_net/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/label_replacing`, `pm4py/objects/petri_net/utils/reachability_graph`, `pm4py/objects/powl/obj`, `pm4py/objects/powl/utils/label_replacing`, `pm4py/objects/process_tree/obj`, `pm4py/objects/process_tree/utils/label_replacing`, `pm4py/objects/transition_system/obj`.

### model-objects-petri-net-utils-petri-utils-1

Crate: `ichnos-model`. Rows: `pm4py.PetriNet`.

Port sources: `pm4py/objects/petri_net/obj.py`, `pm4py/objects/petri_net/utils/petri_utils`.

### model-objects-petri-net-obj-1

Crate: `ichnos-model`. Rows: `pm4py.Marking`, `pm4py.maximal_decomposition`, `pm4py.generate_marking`, `pm4py.reduce_petri_net_invisibles`, `pm4py.reduce_petri_net_implicit_places`, `pm4py.get_enabled_transitions`, `pm4py.behavioral_similarity`, `pm4py.convert_petri_net_to_networkx`.

Port sources: `pm4py/analysis.py`, `pm4py/convert.py`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/obj.py`, `pm4py/objects/petri_net/semantics`, `pm4py/objects/petri_net/utils/decomposition`, `pm4py/objects/petri_net/utils/murata`, `pm4py/objects/petri_net/utils/reduction`, `pm4py/objects/process_tree/obj`.

### model-objects-petri-net-obj-2

Crate: `ichnos-model`. Rows: `pm4py.convert_petri_net_type`.

Port sources: `pm4py/convert.py`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/petri_utils`.

### model-objects-process-tree-obj-1

Crate: `ichnos-model`. Rows: `pm4py.ProcessTree`.

Port sources: `pm4py/objects/process_tree/obj.py`.

### model-objects-log-obj-1

Crate: `ichnos-model`. Rows: `pm4py.construct_synchronous_product_net`, `pm4py.insert_artificial_start_end`, `pm4py.get_activity_labels`.

Port sources: `pm4py/analysis.py`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/artificial`, `pm4py/objects/log/util/dataframe_utils`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/align_utils`, `pm4py/objects/petri_net/utils/petri_utils`, `pm4py/objects/petri_net/utils/synchronous_product`.

### model-algo-evaluation-earth-mover-distance-algorithm-1

Crate: `ichnos-model`. Rows: `pm4py.compute_emd`.

Port sources: `pm4py/algo/evaluation/earth_mover_distance/algorithm`, `pm4py/analysis.py`.

### model-algo-analysis-marking-equation-algorithm-1

Crate: `ichnos-model`. Rows: `pm4py.solve_marking_equation`.

Port sources: `pm4py/algo/analysis/marking_equation/algorithm`, `pm4py/analysis.py`, `pm4py/objects/petri_net/obj`.

### model-algo-analysis-extended-marking-equation-algorithm-1

Crate: `ichnos-model`. Rows: `pm4py.solve_extended_marking_equation`.

Port sources: `pm4py/algo/analysis/extended_marking_equation/algorithm`, `pm4py/analysis.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### model-algo-analysis-woflan-algorithm-1

Crate: `ichnos-model`. Rows: `pm4py.analysis.check_is_sound`, `pm4py.check_soundness`.

Port sources: `pm4py/algo/analysis/woflan/algorithm`, `pm4py/analysis.py`, `pm4py/objects/petri_net/obj`.

### model-algo-clustering-profiles-algorithm-1

Crate: `ichnos-model`. Rows: `pm4py.cluster_log`.

Port sources: `pm4py/algo/clustering/profiles/algorithm`, `pm4py/analysis.py`, `pm4py/objects/log/obj`.

### model-objects-conversion-log-converter-1

Crate: `ichnos-model`. Rows: `pm4py.insert_case_service_waiting_time`, `pm4py.insert_case_arrival_finish_rate`, `pm4py.convert_to_event_log`, `pm4py.convert_to_event_stream`, `pm4py.convert_to_dataframe`, `pm4py.convert_log_to_ocel`, `pm4py.convert_log_to_networkx`.

Port sources: `pm4py/analysis.py`, `pm4py/convert.py`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/log_ocel`.

### model-algo-analysis-workflow-net-algorithm-1

Crate: `ichnos-model`. Rows: `pm4py.check_is_workflow_net`.

Port sources: `pm4py/algo/analysis/workflow_net/algorithm`, `pm4py/analysis.py`, `pm4py/objects/petri_net/obj`.

### model-algo-evaluation-simplicity-variants-arc-degree-1

Crate: `ichnos-model`. Rows: `pm4py.simplicity_petri_net`.

Port sources: `pm4py/algo/evaluation/simplicity/variants/arc_degree`, `pm4py/algo/evaluation/simplicity/variants/extended_cardoso`, `pm4py/algo/evaluation/simplicity/variants/extended_cyclomatic`, `pm4py/analysis.py`, `pm4py/objects/petri_net/obj`.

### model-objects-process-tree-utils-struct-similarity-1

Crate: `ichnos-model`. Rows: `pm4py.structural_similarity`.

Port sources: `pm4py/analysis.py`, `pm4py/objects/process_tree/utils/struct_similarity`.

### model-objects-petri-net-utils-embeddings-similarity-1

Crate: `ichnos-model`. Rows: `pm4py.embeddings_similarity`.

Port sources: `pm4py/analysis.py`, `pm4py/objects/petri_net/utils/embeddings_similarity`.

### model-analysis-1

Crate: `ichnos-model`. Rows: `pm4py.label_sets_similarity`.

Port sources: `pm4py/analysis.py`.

### model-objects-conversion-ocel-converter-1

Crate: `ichnos-model`. Rows: `pm4py.convert_ocel_to_networkx`.

Port sources: `pm4py/convert.py`, `pm4py/objects/conversion/ocel/converter`, `pm4py/objects/ocel/obj`.

### model-algo-transformation-log-to-interval-tree-variants-open-paths-1

Crate: `ichnos-model`. Rows: `pm4py.convert_log_to_time_intervals`.

Port sources: `pm4py/algo/transformation/log_to_interval_tree/variants/open_paths`, `pm4py/convert.py`, `pm4py/objects/log/obj`.

### ocel-objects-ocel-constants-1

Crate: `ichnos-ocel`. Rows: `pm4py.OCEL`.

Port sources: `pm4py/objects/ocel/constants`, `pm4py/objects/ocel/obj.py`.

### ocel-objects-ocel-obj-1

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_get_object_types`, `pm4py.ocel_get_attribute_names`, `pm4py.ocel_flattening`, `pm4py.ocel_object_type_activities`, `pm4py.ocel_objects_ot_count`, `pm4py.ocel_temporal_summary`, `pm4py.ocel_objects_summary`, `pm4py.ocel_objects_interactions_summary`.

Port sources: `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/attributes_names`, `pm4py/objects/ocel/util/flattening`, `pm4py/ocel.py`, `pm4py/statistics/ocel/objects_ot_count`, `pm4py/statistics/ocel/ot_activities`.

### ocel-objects-ocel-obj-2

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_e2o_lifecycle_enrichment`, `pm4py.sample_ocel_objects`, `pm4py.ocel_drop_duplicates`, `pm4py.ocel_merge_duplicates`, `pm4py.ocel_sort_by_additional_column`, `pm4py.ocel_add_index_based_timedelta`.

Port sources: `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/e2o_qualification`, `pm4py/objects/ocel/util/filtering_utils`, `pm4py/objects/ocel/util/sampling`, `pm4py/ocel.py`.

### ocel-algo-discovery-ocel-ocdfg-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.discover_ocdfg`.

Port sources: `pm4py/algo/discovery/ocel/ocdfg/algorithm`, `pm4py/objects/ocel/constants`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-discovery-ocel-ocpn-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.discover_oc_petri_net`.

Port sources: `pm4py/algo/discovery/ocel/ocpn/algorithm`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocpn/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-graphs-object-cobirth-graph-1

Crate: `ichnos-ocel`. Rows: `pm4py.discover_objects_graph`.

Port sources: `pm4py/algo/transformation/ocel/graphs/object_cobirth_graph`, `pm4py/algo/transformation/ocel/graphs/object_codeath_graph`, `pm4py/algo/transformation/ocel/graphs/object_descendants_graph`, `pm4py/algo/transformation/ocel/graphs/object_inheritance_graph`, `pm4py/algo/transformation/ocel/graphs/object_interaction_graph`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-graphs-ocel20-computation-1

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_o2o_enrichment`.

Port sources: `pm4py/algo/transformation/ocel/graphs/ocel20_computation`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-split-ocel-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.sample_ocel_connected_components`.

Port sources: `pm4py/algo/transformation/ocel/split_ocel/algorithm`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-description-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.cluster_equivalent_ocel`.

Port sources: `pm4py/algo/transformation/ocel/description/algorithm`, `pm4py/algo/transformation/ocel/split_ocel/algorithm`, `pm4py/objects/ocel/obj`, `pm4py/objects/ocel/util/rename_objs_ot_tim_lex`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-olap-drill-down-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_drill_down`.

Port sources: `pm4py/algo/transformation/ocel/olap/drill_down/algorithm`, `pm4py/algo/transformation/ocel/olap/drill_down/variants/classic`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-olap-roll-up-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_roll_up`.

Port sources: `pm4py/algo/transformation/ocel/olap/roll_up/algorithm`, `pm4py/algo/transformation/ocel/olap/roll_up/variants/classic`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-olap-unfold-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_unfold`.

Port sources: `pm4py/algo/transformation/ocel/olap/unfold/algorithm`, `pm4py/algo/transformation/ocel/olap/unfold/variants/classic`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### ocel-algo-transformation-ocel-olap-fold-algorithm-1

Crate: `ichnos-ocel`. Rows: `pm4py.ocel_fold`.

Port sources: `pm4py/algo/transformation/ocel/olap/fold/algorithm`, `pm4py/algo/transformation/ocel/olap/fold/variants/classic`, `pm4py/objects/ocel/obj`, `pm4py/ocel.py`.

### io-algo-connectors-variants-github-repo-1

Crate: `ichnos-io`. Rows: `pm4py.connectors.extract_log_github`.

Port sources: `pm4py/algo/connectors/variants/github_repo`, `pm4py/connectors.py`.

### io-algo-connectors-variants-camunda-workflow-1

Crate: `ichnos-io`. Rows: `pm4py.connectors.extract_log_camunda_workflow`.

Port sources: `pm4py/algo/connectors/variants/camunda_workflow`, `pm4py/connectors.py`.

### io-algo-connectors-variants-sap-o2c-1

Crate: `ichnos-io`. Rows: `pm4py.connectors.extract_log_sap_o2c`.

Port sources: `pm4py/algo/connectors/variants/sap_o2c`, `pm4py/connectors.py`.

### io-algo-connectors-variants-sap-accounting-1

Crate: `ichnos-io`. Rows: `pm4py.connectors.extract_log_sap_accounting`.

Port sources: `pm4py/algo/connectors/variants/sap_accounting`, `pm4py/connectors.py`.

### io-objects-ocel-obj-1

Crate: `ichnos-io`. Rows: `pm4py.connectors.extract_ocel_github`, `pm4py.connectors.extract_ocel_camunda_workflow`, `pm4py.connectors.extract_ocel_sap_o2c`, `pm4py.connectors.extract_ocel_sap_accounting`.

Port sources: `pm4py/connectors.py`, `pm4py/objects/ocel/obj`.

### io-objects-conversion-log-converter-1

Crate: `ichnos-io`. Rows: `pm4py.read_xes`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/importer/xes/importer`, `pm4py/objects/log/obj`, `pm4py/read.py`.

### io-objects-petri-net-importer-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_pnml`.

Port sources: `pm4py/objects/petri_net/importer/importer`, `pm4py/objects/petri_net/obj`, `pm4py/read.py`.

### io-objects-process-tree-importer-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_ptml`.

Port sources: `pm4py/objects/process_tree/importer/importer`, `pm4py/objects/process_tree/obj`, `pm4py/read.py`.

### io-objects-dfg-importer-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_dfg`.

Port sources: `pm4py/objects/dfg/importer/importer`, `pm4py/read.py`.

### io-objects-bpmn-importer-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_bpmn`.

Port sources: `pm4py/objects/bpmn/importer/importer`, `pm4py/objects/bpmn/obj`, `pm4py/read.py`.

### io-objects-ocel-importer-csv-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_ocel`, `pm4py.read_ocel_csv`, `pm4py.read_ocel2_csv`.

Port sources: `pm4py/objects/ocel/importer/csv/importer`, `pm4py/objects/ocel/importer/jsonocel/importer`, `pm4py/objects/ocel/importer/sqlite/importer`, `pm4py/objects/ocel/importer/xmlocel/importer`, `pm4py/objects/ocel/obj`, `pm4py/read.py`.

### io-objects-ocel-importer-jsonocel-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_ocel_json`, `pm4py.read_ocel2_json`.

Port sources: `pm4py/objects/ocel/importer/jsonocel/importer`, `pm4py/objects/ocel/obj`, `pm4py/read.py`.

### io-objects-ocel-importer-xmlocel-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_ocel_xml`, `pm4py.read_ocel2_xml`.

Port sources: `pm4py/objects/ocel/importer/xmlocel/importer`, `pm4py/objects/ocel/obj`, `pm4py/read.py`.

### io-objects-ocel-importer-sqlite-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_ocel_sqlite`, `pm4py.read_ocel2_sqlite`.

Port sources: `pm4py/objects/ocel/importer/sqlite/importer`, `pm4py/objects/ocel/obj`, `pm4py/read.py`.

### io-objects-ocel-importer-bundled-importer-1

Crate: `ichnos-io`. Rows: `pm4py.read_ocel2`, `pm4py.read_ocel2_bundle`.

Port sources: `pm4py/objects/ocel/importer/bundled/importer`, `pm4py/objects/ocel/importer/csv/importer`, `pm4py/objects/ocel/importer/jsonocel/importer`, `pm4py/objects/ocel/importer/sqlite/importer`, `pm4py/objects/ocel/importer/xmlocel/importer`, `pm4py/objects/ocel/obj`, `pm4py/read.py`.

### io-objects-log-exporter-xes-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_xes`.

Port sources: `pm4py/objects/log/exporter/xes/exporter`, `pm4py/objects/log/obj`, `pm4py/write.py`.

### io-objects-petri-net-exporter-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_pnml`.

Port sources: `pm4py/objects/petri_net/exporter/exporter`, `pm4py/objects/petri_net/obj`, `pm4py/write.py`.

### io-objects-process-tree-exporter-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_ptml`.

Port sources: `pm4py/objects/process_tree/exporter/exporter`, `pm4py/objects/process_tree/obj`, `pm4py/write.py`.

### io-objects-dfg-exporter-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_dfg`.

Port sources: `pm4py/objects/dfg/exporter/exporter`, `pm4py/write.py`.

### io-objects-bpmn-exporter-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_bpmn`.

Port sources: `pm4py/objects/bpmn/exporter/exporter`, `pm4py/objects/bpmn/layout/layouter`, `pm4py/objects/bpmn/obj`, `pm4py/write.py`.

### io-objects-ocel-exporter-csv-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_ocel`, `pm4py.write_ocel_csv`, `pm4py.write_ocel2_csv`.

Port sources: `pm4py/objects/ocel/exporter/csv/exporter`, `pm4py/objects/ocel/exporter/jsonocel/exporter`, `pm4py/objects/ocel/exporter/sqlite/exporter`, `pm4py/objects/ocel/exporter/xmlocel/exporter`, `pm4py/objects/ocel/obj`, `pm4py/write.py`.

### io-objects-ocel-exporter-jsonocel-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_ocel_json`, `pm4py.write_ocel2_json`.

Port sources: `pm4py/objects/ocel/exporter/jsonocel/exporter`, `pm4py/objects/ocel/obj`, `pm4py/write.py`.

### io-objects-ocel-exporter-xmlocel-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_ocel_xml`, `pm4py.write_ocel2_xml`.

Port sources: `pm4py/objects/ocel/exporter/xmlocel/exporter`, `pm4py/objects/ocel/obj`, `pm4py/write.py`.

### io-objects-ocel-exporter-sqlite-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_ocel_sqlite`, `pm4py.write_ocel2_sqlite`.

Port sources: `pm4py/objects/ocel/exporter/sqlite/exporter`, `pm4py/objects/ocel/obj`, `pm4py/write.py`.

### io-objects-ocel-exporter-bundled-exporter-1

Crate: `ichnos-io`. Rows: `pm4py.write_ocel2`, `pm4py.write_ocel2_bundle`.

Port sources: `pm4py/objects/ocel/exporter/bundled/exporter`, `pm4py/objects/ocel/exporter/csv/exporter`, `pm4py/objects/ocel/exporter/jsonocel/exporter`, `pm4py/objects/ocel/exporter/sqlite/exporter`, `pm4py/objects/ocel/exporter/xmlocel/exporter`, `pm4py/objects/ocel/obj`, `pm4py/write.py`.

### stats-attributes-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.attributes.common.get.get_sorted_attributes_list`, `pm4py.statistics.attributes.common.get.get_attributes_threshold`, `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute`, `pm4py.statistics.attributes.common.get.get_kde_numeric_attribute_json`, `pm4py.statistics.attributes.common.get.get_kde_date_attribute`, `pm4py.statistics.attributes.common.get.get_kde_date_attribute_json`, `pm4py.statistics.attributes.log.get.get_events_distribution`, `pm4py.statistics.attributes.log.get.get_all_trace_attributes_from_log`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/attributes/common/get.py`, `pm4py/statistics/attributes/log/get.py`.

### stats-attributes-2

Crate: `ichnos-stats`. Rows: `pm4py.statistics.attributes.log.get.get_all_event_attributes_from_log`, `pm4py.statistics.attributes.log.get.get_attribute_values`, `pm4py.statistics.attributes.log.get.get_trace_attribute_values`, `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute`, `pm4py.statistics.attributes.log.get.get_kde_numeric_attribute_json`, `pm4py.statistics.attributes.log.get.get_kde_date_attribute`, `pm4py.statistics.attributes.log.get.get_kde_date_attribute_json`, `pm4py.statistics.attributes.log.select.select_attributes_from_log_for_tree`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/sampling`, `pm4py/statistics/attributes/common/get`, `pm4py/statistics/attributes/log/get`, `pm4py/statistics/attributes/log/get.py`, `pm4py/statistics/attributes/log/select.py`.

### stats-attributes-3

Crate: `ichnos-stats`. Rows: `pm4py.statistics.attributes.log.select.check_trace_attributes_presence`, `pm4py.statistics.attributes.log.select.check_event_attributes_presence`, `pm4py.statistics.attributes.log.select.verify_if_event_attribute_is_in_each_trace`, `pm4py.statistics.attributes.log.select.verify_if_trace_attribute_is_in_each_trace`, `pm4py.statistics.attributes.pandas.get.get_events_distribution`, `pm4py.statistics.attributes.pandas.get.get_attribute_values`, `pm4py.statistics.attributes.pandas.get.get_kde_numeric_attribute`, `pm4py.statistics.attributes.pandas.get.get_kde_numeric_attribute_json`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/attributes/common/get`, `pm4py/statistics/attributes/log/select.py`, `pm4py/statistics/attributes/pandas/get.py`.

### stats-attributes-4

Crate: `ichnos-stats`. Rows: `pm4py.statistics.attributes.pandas.get.get_kde_date_attribute`, `pm4py.statistics.attributes.pandas.get.get_kde_date_attribute_json`, `pm4py.statistics.attributes.polars.get.get_events_distribution`, `pm4py.statistics.attributes.polars.get.get_attribute_values`, `pm4py.statistics.attributes.polars.get.get_kde_numeric_attribute`, `pm4py.statistics.attributes.polars.get.get_kde_numeric_attribute_json`, `pm4py.statistics.attributes.polars.get.get_kde_date_attribute`, `pm4py.statistics.attributes.polars.get.get_kde_date_attribute_json`.

Port sources: `pm4py/statistics/attributes/common/get`, `pm4py/statistics/attributes/pandas/get.py`, `pm4py/statistics/attributes/polars/get.py`.

### stats-chaotic-activities-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.chaotic_activities.algorithm.apply`, `pm4py.statistics.chaotic_activities.variants.niek_sidorova.apply`, `pm4py.statistics.chaotic_activities.variants.niek_sidorova.chaotic_metrics`, `pm4py.statistics.chaotic_activities.variants.niek_sidorova.total_entropy`.

Port sources: `pm4py/objects/log/obj`, `pm4py/statistics/chaotic_activities/algorithm.py`, `pm4py/statistics/chaotic_activities/variants/niek_sidorova.py`.

### stats-concurrent-activities-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.concurrent_activities.log.get.apply`, `pm4py.statistics.concurrent_activities.pandas.get.apply`, `pm4py.statistics.concurrent_activities.polars.get.get_concurrent_events_dataframe`, `pm4py.statistics.concurrent_activities.polars.get.apply`.

Port sources: `pm4py/algo/discovery/dfg/adapters/pandas/df_statistics`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/sorting`, `pm4py/statistics/concurrent_activities/log/get.py`, `pm4py/statistics/concurrent_activities/pandas/get.py`, `pm4py/statistics/concurrent_activities/polars/get.py`.

### stats-end-activities-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.end_activities.common.get.get_sorted_end_activities_list`, `pm4py.statistics.end_activities.common.get.get_end_activities_threshold`, `pm4py.statistics.end_activities.log.get.get_end_activities`, `pm4py.statistics.end_activities.pandas.get.get_end_activities`, `pm4py.statistics.end_activities.polars.get.get_end_activities`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/end_activities/common/get.py`, `pm4py/statistics/end_activities/log/get.py`, `pm4py/statistics/end_activities/pandas/get.py`, `pm4py/statistics/end_activities/polars/get.py`.

### stats-eventually-follows-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.eventually_follows.log.get.apply`, `pm4py.statistics.eventually_follows.pandas.get.apply`, `pm4py.statistics.eventually_follows.polars.get.get_partial_order_dataframe`, `pm4py.statistics.eventually_follows.polars.get.apply`, `pm4py.statistics.eventually_follows.uvcl.get.apply`.

Port sources: `pm4py/algo/discovery/dfg/adapters/pandas/df_statistics`, `pm4py/algo/discovery/inductive/dtypes/im_ds`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/sorting`, `pm4py/statistics/eventually_follows/log/get.py`, `pm4py/statistics/eventually_follows/pandas/get.py`, `pm4py/statistics/eventually_follows/polars/get.py`, `pm4py/statistics/eventually_follows/uvcl/get.py`.

### stats-ocel-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.ocel.act_ot_dependent.aggregate_events`, `pm4py.statistics.ocel.act_ot_dependent.aggregate_unique_objects`, `pm4py.statistics.ocel.act_ot_dependent.aggregate_total_objects`, `pm4py.statistics.ocel.act_ot_dependent.find_associations_from_ocel`, `pm4py.statistics.ocel.act_utils.aggregate_events`, `pm4py.statistics.ocel.act_utils.aggregate_unique_objects`, `pm4py.statistics.ocel.act_utils.aggregate_total_objects`, `pm4py.statistics.ocel.act_utils.find_associations_from_relations_df`.

Port sources: `pm4py/objects/ocel/constants`, `pm4py/objects/ocel/obj`, `pm4py/statistics/ocel/act_ot_dependent.py`, `pm4py/statistics/ocel/act_utils`, `pm4py/statistics/ocel/act_utils.py`.

### stats-ocel-2

Crate: `ichnos-stats`. Rows: `pm4py.statistics.ocel.act_utils.find_associations_from_ocel`, `pm4py.statistics.ocel.edge_metrics.performance_calculation_ocel_aggregation`, `pm4py.statistics.ocel.edge_metrics.aggregate_ev_couples`, `pm4py.statistics.ocel.edge_metrics.aggregate_unique_objects`, `pm4py.statistics.ocel.edge_metrics.aggregate_total_objects`, `pm4py.statistics.ocel.edge_metrics.find_associations_per_edge`, `pm4py.statistics.ocel.objects_ot_count.get_objects_ot_count`, `pm4py.statistics.ocel.ot_activities.get_object_type_activities`.

Port sources: `pm4py/objects/ocel/obj`, `pm4py/statistics/ocel/act_utils.py`, `pm4py/statistics/ocel/edge_metrics.py`, `pm4py/statistics/ocel/objects_ot_count.py`, `pm4py/statistics/ocel/ot_activities.py`.

### stats-overlap-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.overlap.cases.log.get.apply`, `pm4py.statistics.overlap.cases.pandas.get.apply`, `pm4py.statistics.overlap.cases.polars.get.apply`, `pm4py.statistics.overlap.interval_events.log.get.apply`, `pm4py.statistics.overlap.interval_events.pandas.get.apply`, `pm4py.statistics.overlap.interval_events.polars.get.apply`, `pm4py.statistics.overlap.utils.compute.apply`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/overlap/cases/log/get.py`, `pm4py/statistics/overlap/cases/pandas/get.py`, `pm4py/statistics/overlap/cases/polars/get.py`, `pm4py/statistics/overlap/interval_events/log/get.py`, `pm4py/statistics/overlap/interval_events/pandas/get.py`, `pm4py/statistics/overlap/interval_events/polars/get.py`, `pm4py/statistics/overlap/utils/compute`, `pm4py/statistics/overlap/utils/compute.py`.

### stats-passed-time-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.passed_time.log.algorithm.apply`, `pm4py.statistics.passed_time.log.variants.post.apply`, `pm4py.statistics.passed_time.log.variants.pre.apply`, `pm4py.statistics.passed_time.log.variants.prepost.apply`, `pm4py.statistics.passed_time.pandas.algorithm.apply`, `pm4py.statistics.passed_time.pandas.variants.post.apply`, `pm4py.statistics.passed_time.pandas.variants.pre.apply`, `pm4py.statistics.passed_time.pandas.variants.prepost.apply`.

Port sources: `pm4py/algo/discovery/dfg/adapters/pandas/df_statistics`, `pm4py/algo/discovery/dfg/variants/native`, `pm4py/algo/discovery/dfg/variants/performance`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/passed_time/log/algorithm.py`, `pm4py/statistics/passed_time/log/variants/post.py`, `pm4py/statistics/passed_time/log/variants/pre.py`, `pm4py/statistics/passed_time/log/variants/prepost.py`, `pm4py/statistics/passed_time/pandas/algorithm.py`, `pm4py/statistics/passed_time/pandas/variants/post.py`, `pm4py/statistics/passed_time/pandas/variants/pre.py`, `pm4py/statistics/passed_time/pandas/variants/prepost.py`.

### stats-passed-time-2

Crate: `ichnos-stats`. Rows: `pm4py.statistics.passed_time.polars.algorithm.apply`, `pm4py.statistics.passed_time.polars.variants.post.apply`, `pm4py.statistics.passed_time.polars.variants.pre.apply`, `pm4py.statistics.passed_time.polars.variants.prepost.apply`.

Port sources: `pm4py/statistics/passed_time/polars/algorithm.py`, `pm4py/statistics/passed_time/polars/variants/post`, `pm4py/statistics/passed_time/polars/variants/post.py`, `pm4py/statistics/passed_time/polars/variants/pre`, `pm4py/statistics/passed_time/polars/variants/pre.py`, `pm4py/statistics/passed_time/polars/variants/prepost.py`.

### stats-process-cube-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.process_cube.pandas.algorithm.apply`, `pm4py.statistics.process_cube.pandas.variants.classic.apply`, `pm4py.statistics.process_cube.polars.algorithm.apply`, `pm4py.statistics.process_cube.polars.variants.classic.apply`.

Port sources: `pm4py/statistics/process_cube/pandas/algorithm.py`, `pm4py/statistics/process_cube/pandas/variants/classic.py`, `pm4py/statistics/process_cube/polars/algorithm.py`, `pm4py/statistics/process_cube/polars/variants/classic.py`.

### stats-rework-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.rework.cases.log.get.apply`, `pm4py.statistics.rework.cases.pandas.get.apply`, `pm4py.statistics.rework.cases.polars.get.apply`, `pm4py.statistics.rework.log.get.apply`, `pm4py.statistics.rework.pandas.get.apply`, `pm4py.statistics.rework.polars.get.apply`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/rework/cases/log/get.py`, `pm4py/statistics/rework/cases/pandas/get.py`, `pm4py/statistics/rework/cases/polars/get.py`, `pm4py/statistics/rework/log/get.py`, `pm4py/statistics/rework/pandas/get.py`, `pm4py/statistics/rework/polars/get.py`.

### stats-service-time-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.service_time.log.get.apply`, `pm4py.statistics.service_time.pandas.get.apply`, `pm4py.statistics.service_time.polars.get.apply`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/service_time/log/get.py`, `pm4py/statistics/service_time/pandas/get.py`, `pm4py/statistics/service_time/polars/get.py`.

### stats-start-activities-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.start_activities.common.get.get_sorted_start_activities_list`, `pm4py.statistics.start_activities.common.get.get_start_activities_threshold`, `pm4py.statistics.start_activities.log.get.get_start_activities`, `pm4py.statistics.start_activities.pandas.get.get_start_activities`, `pm4py.statistics.start_activities.polars.get.get_start_activities`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/start_activities/common/get.py`, `pm4py/statistics/start_activities/log/get.py`, `pm4py/statistics/start_activities/pandas/get.py`, `pm4py/statistics/start_activities/polars/get.py`.

### stats-traces-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.traces.cycle_time.log.get.apply`, `pm4py.statistics.traces.cycle_time.pandas.get.apply`, `pm4py.statistics.traces.cycle_time.polars.get.apply`, `pm4py.statistics.traces.cycle_time.util.compute.cycle_time`, `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration`, `pm4py.statistics.traces.generic.common.case_duration.get_kde_caseduration_json`, `pm4py.statistics.traces.generic.log.case_arrival.get_case_arrival_avg`, `pm4py.statistics.traces.generic.log.case_arrival.get_case_dispersion_avg`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/traces/cycle_time/log/get.py`, `pm4py/statistics/traces/cycle_time/pandas/get.py`, `pm4py/statistics/traces/cycle_time/polars/get.py`, `pm4py/statistics/traces/cycle_time/util/compute`, `pm4py/statistics/traces/cycle_time/util/compute.py`, `pm4py/statistics/traces/generic/common/case_duration.py`, `pm4py/statistics/traces/generic/log/case_arrival.py`.

### stats-traces-2

Crate: `ichnos-stats`. Rows: `pm4py.statistics.traces.generic.log.case_statistics.get_variant_statistics`, `pm4py.statistics.traces.generic.log.case_statistics.get_cases_description`, `pm4py.statistics.traces.generic.log.case_statistics.index_log_caseid`, `pm4py.statistics.traces.generic.log.case_statistics.get_events`, `pm4py.statistics.traces.generic.log.case_statistics.get_all_case_durations`, `pm4py.statistics.traces.generic.log.case_statistics.get_first_quartile_case_duration`, `pm4py.statistics.traces.generic.log.case_statistics.get_median_case_duration`, `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/statistics/traces/generic/common/case_duration`, `pm4py/statistics/traces/generic/log/case_statistics.py`, `pm4py/statistics/variants/log/get`.

### stats-traces-3

Crate: `ichnos-stats`. Rows: `pm4py.statistics.traces.generic.log.case_statistics.get_kde_caseduration_json`, `pm4py.statistics.traces.generic.pandas.case_arrival.get_case_arrival_avg`, `pm4py.statistics.traces.generic.pandas.case_arrival.get_case_dispersion_avg`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variant_statistics`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_and_list`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_cases_description`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_variants_df_with_case_duration`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/statistics/traces/generic/common/case_duration`, `pm4py/statistics/traces/generic/log/case_statistics.py`, `pm4py/statistics/traces/generic/pandas/case_arrival.py`, `pm4py/statistics/traces/generic/pandas/case_statistics.py`.

### stats-traces-4

Crate: `ichnos-stats`. Rows: `pm4py.statistics.traces.generic.pandas.case_statistics.get_events`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_kde_caseduration`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_kde_caseduration_json`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_all_case_durations`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_first_quartile_case_duration`, `pm4py.statistics.traces.generic.pandas.case_statistics.get_median_case_duration`, `pm4py.statistics.traces.generic.polars.case_arrival.get_case_arrival_avg`, `pm4py.statistics.traces.generic.polars.case_arrival.get_case_dispersion_avg`.

Port sources: `pm4py/statistics/traces/generic/common/case_duration`, `pm4py/statistics/traces/generic/pandas/case_statistics.py`, `pm4py/statistics/traces/generic/polars/case_arrival.py`.

### stats-traces-5

Crate: `ichnos-stats`. Rows: `pm4py.statistics.traces.generic.polars.case_statistics.get_variant_statistics`, `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df_and_list`, `pm4py.statistics.traces.generic.polars.case_statistics.get_cases_description`, `pm4py.statistics.traces.generic.polars.case_statistics.get_variants_df`, `pm4py.statistics.traces.generic.polars.case_statistics.get_all_case_durations`, `pm4py.statistics.traces.generic.polars.case_statistics.get_median_case_duration`, `pm4py.statistics.traces.generic.polars.case_statistics.get_first_quartile_case_duration`, `pm4py.statistics.traces.generic.polars.case_statistics.get_kde_caseduration`.

Port sources: `pm4py/statistics/traces/generic/common/case_duration`, `pm4py/statistics/traces/generic/polars/case_statistics.py`.

### stats-traces-6

Crate: `ichnos-stats`. Rows: `pm4py.statistics.traces.generic.polars.case_statistics.get_kde_caseduration_json`.

Port sources: `pm4py/statistics/traces/generic/common/case_duration`, `pm4py/statistics/traces/generic/polars/case_statistics.py`.

### stats-util-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.util.times_bipartite_matching.exact_match_minimum_average`.

Port sources: `pm4py/statistics/util/times_bipartite_matching.py`.

### stats-variants-1

Crate: `ichnos-stats`. Rows: `pm4py.statistics.variants.log.get.get_language`, `pm4py.statistics.variants.log.get.get_variants`, `pm4py.statistics.variants.log.get.get_variants_along_with_case_durations`, `pm4py.statistics.variants.log.get.get_variants_from_log_trace_idx`, `pm4py.statistics.variants.log.get.get_variants_sorted_by_count`, `pm4py.statistics.variants.log.get.convert_variants_trace_idx_to_trace_obj`, `pm4py.statistics.variants.pandas.get.get_variants_count`, `pm4py.statistics.variants.pandas.get.get_variants_set`.

Port sources: `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/pandas_numpy_variants`, `pm4py/statistics/variants/log/get.py`, `pm4py/statistics/variants/pandas/get.py`.

### stats-variants-2

Crate: `ichnos-stats`. Rows: `pm4py.statistics.variants.polars.get.pandas_numpy_variants_apply_polars`, `pm4py.statistics.variants.polars.get.get_variants_count`, `pm4py.statistics.variants.polars.get.get_variants_set`.

Port sources: `pm4py/statistics/variants/polars/get.py`.

### stats-stats-1

Crate: `ichnos-stats`. Rows: `pm4py.get_start_activities`, `pm4py.get_end_activities`, `pm4py.get_event_attributes`, `pm4py.get_trace_attributes`, `pm4py.get_event_attribute_values`, `pm4py.get_trace_attribute_values`, `pm4py.get_variants`, `pm4py.get_variants_as_tuples`.

Port sources: `pm4py/objects/log/obj`, `pm4py/statistics/attributes/log/get`, `pm4py/statistics/end_activities/log/get`, `pm4py/statistics/start_activities/log/get`, `pm4py/statistics/variants/log/get`, `pm4py/stats.py`.

### stats-stats-2

Crate: `ichnos-stats`. Rows: `pm4py.split_by_process_variant`, `pm4py.get_variants_paths_duration`, `pm4py.get_stochastic_language`, `pm4py.get_minimum_self_distances`, `pm4py.get_minimum_self_distance_witnesses`, `pm4py.get_case_arrival_average`, `pm4py.get_rework_cases_per_activity`, `pm4py.get_case_overlap`.

Port sources: `pm4py/algo/discovery/minimum_self_distance/algorithm`, `pm4py/algo/discovery/minimum_self_distance/utils`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/pandas_numpy_variants`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`, `pm4py/statistics/overlap/cases/log/get`, `pm4py/statistics/rework/log/get`, `pm4py/statistics/traces/generic/log/case_arrival`, `pm4py/statistics/variants/log/get`, `pm4py/stats.py`.

### stats-stats-3

Crate: `ichnos-stats`. Rows: `pm4py.get_cycle_time`, `pm4py.get_service_time`, `pm4py.get_all_case_durations`, `pm4py.get_case_duration`, `pm4py.get_frequent_trace_segments`, `pm4py.get_activity_position_summary`, `pm4py.get_process_cube`.

Port sources: `pm4py/objects/log/obj`, `pm4py/statistics/process_cube/pandas/algorithm`, `pm4py/statistics/process_cube/polars/algorithm`, `pm4py/statistics/service_time/log/get`, `pm4py/statistics/traces/cycle_time/log/get`, `pm4py/statistics/traces/generic/log/case_statistics`, `pm4py/stats.py`.

### discovery-algo-discovery-dfg-adapters-pandas-df-statistics-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_dfg`, `pm4py.discover_directly_follows_graph`, `pm4py.discover_dfg_typed`, `pm4py.discover_performance_dfg`.

Port sources: `pm4py/algo/discovery/dfg/adapters/pandas/df_statistics`, `pm4py/algo/discovery/dfg/adapters/polars/df_statistics`, `pm4py/algo/discovery/dfg/algorithm`, `pm4py/algo/discovery/dfg/variants/performance`, `pm4py/discovery.py`, `pm4py/objects/dfg/obj`, `pm4py/objects/log/obj`, `pm4py/statistics/end_activities/log/get`, `pm4py/statistics/end_activities/pandas/get`, `pm4py/statistics/end_activities/polars/get`, `pm4py/statistics/start_activities/log/get`, `pm4py/statistics/start_activities/pandas/get`, `pm4py/statistics/start_activities/polars/get`.

### discovery-algo-discovery-alpha-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_alpha`, `pm4py.discover_petri_net_alpha_plus`.

Port sources: `pm4py/algo/discovery/alpha/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### discovery-algo-discovery-ilp-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_ilp`.

Port sources: `pm4py/algo/discovery/ilp/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### discovery-algo-discovery-genetic-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_genetic`.

Port sources: `pm4py/algo/discovery/genetic/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### discovery-algo-discovery-inductive-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_inductive`, `pm4py.discover_process_tree_inductive`, `pm4py.discover_bpmn_inductive`.

Port sources: `pm4py/algo/discovery/inductive/algorithm`, `pm4py/discovery.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/dfg/obj`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`.

### discovery-algo-discovery-heuristics-variants-classic-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_petri_net_heuristics`, `pm4py.discover_heuristics_net`.

Port sources: `pm4py/algo/discovery/heuristics/variants/classic`, `pm4py/discovery.py`, `pm4py/objects/heuristics_net/obj`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### discovery-algo-discovery-minimum-self-distance-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.derive_minimum_self_distance`.

Port sources: `pm4py/algo/discovery/minimum_self_distance/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### discovery-algo-discovery-footprints-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_footprints`.

Port sources: `pm4py/algo/discovery/footprints/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/powl/obj`, `pm4py/objects/process_tree/obj`.

### discovery-objects-log-obj-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_eventually_follows_graph`.

Port sources: `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/statistics/eventually_follows/log/get`, `pm4py/statistics/eventually_follows/pandas/get`, `pm4py/statistics/eventually_follows/polars/get`.

### discovery-algo-discovery-split-miner-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_bpmn_split_miner`.

Port sources: `pm4py/algo/discovery/split_miner/algorithm`, `pm4py/algo/discovery/split_miner/variants/classic`, `pm4py/algo/discovery/split_miner/variants/sm2`, `pm4py/discovery.py`, `pm4py/objects/bpmn/obj`, `pm4py/objects/log/obj`.

### discovery-algo-discovery-transition-system-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_transition_system`.

Port sources: `pm4py/algo/discovery/transition_system/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/transition_system/obj`.

### discovery-algo-transformation-log-to-trie-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_prefix_tree`.

Port sources: `pm4py/algo/transformation/log_to_trie/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/trie/obj`.

### discovery-algo-discovery-temporal-profile-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_temporal_profile`.

Port sources: `pm4py/algo/discovery/temporal_profile/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### discovery-algo-discovery-log-skeleton-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_log_skeleton`.

Port sources: `pm4py/algo/discovery/log_skeleton/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### discovery-algo-discovery-declare-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_declare`.

Port sources: `pm4py/algo/discovery/declare/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### discovery-algo-discovery-powl-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_powl`.

Port sources: `pm4py/algo/discovery/powl/algorithm`, `pm4py/algo/discovery/powl/inductive/variants/dynamic_clustering_frequency/dynamic_clustering_frequency_partial_order_cut`, `pm4py/algo/discovery/powl/inductive/variants/powl_discovery_varaints`, `pm4py/discovery.py`, `pm4py/objects/log/obj`, `pm4py/objects/powl/obj`.

### discovery-algo-discovery-batches-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_batches`.

Port sources: `pm4py/algo/discovery/batches/algorithm`, `pm4py/discovery.py`, `pm4py/objects/log/obj`.

### discovery-algo-discovery-correlation-mining-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.correlation_miner`.

Port sources: `pm4py/algo/discovery/correlation_mining/algorithm`, `pm4py/discovery.py`.

### discovery-algo-discovery-ocel-otg-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_otg`.

Port sources: `pm4py/algo/discovery/ocel/otg/algorithm`, `pm4py/discovery.py`, `pm4py/objects/ocel/obj`.

### discovery-algo-discovery-ocel-etot-algorithm-1

Crate: `ichnos-discovery`. Rows: `pm4py.discover_etot`.

Port sources: `pm4py/algo/discovery/ocel/etot/algorithm`, `pm4py/discovery.py`, `pm4py/objects/ocel/obj`.

### conformance-algo-conformance-tokenreplay-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_diagnostics_token_based_replay`.

Port sources: `pm4py/algo/conformance/tokenreplay/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### conformance-algo-conformance-alignments-dfg-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_diagnostics_alignments`.

Port sources: `pm4py/algo/conformance/alignments/dfg/algorithm`, `pm4py/algo/conformance/alignments/edit_distance/algorithm`, `pm4py/algo/conformance/alignments/petri_net/algorithm`, `pm4py/algo/conformance/alignments/process_tree/variants/search_graph_pt`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`.

### conformance-algo-evaluation-replay-fitness-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.fitness_token_based_replay`, `pm4py.fitness_alignments`.

Port sources: `pm4py/algo/evaluation/replay_fitness/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### conformance-algo-evaluation-precision-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.precision_token_based_replay`, `pm4py.precision_alignments`.

Port sources: `pm4py/algo/evaluation/precision/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### conformance-algo-evaluation-generalization-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.generalization_tbr`.

Port sources: `pm4py/algo/evaluation/generalization/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### conformance-algo-conformance-tokenreplay-variants-token-replay-1

Crate: `ichnos-conformance`. Rows: `pm4py.replay_prefix_tbr`.

Port sources: `pm4py/algo/conformance/tokenreplay/variants/token_replay`, `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`.

### conformance-algo-conformance-footprints-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_diagnostics_footprints`, `pm4py.fitness_footprints`.

Port sources: `pm4py/algo/conformance/footprints/algorithm`, `pm4py/algo/conformance/footprints/util/evaluation`, `pm4py/conformance.py`.

### conformance-algo-conformance-footprints-util-evaluation-1

Crate: `ichnos-conformance`. Rows: `pm4py.precision_footprints`.

Port sources: `pm4py/algo/conformance/footprints/util/evaluation`, `pm4py/conformance.py`.

### conformance-objects-log-obj-1

Crate: `ichnos-conformance`. Rows: `pm4py.check_is_fitting`.

Port sources: `pm4py/conformance.py`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/objects/process_tree/obj`.

### conformance-algo-conformance-temporal-profile-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_temporal_profile`.

Port sources: `pm4py/algo/conformance/temporal_profile/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`.

### conformance-algo-conformance-declare-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_declare`.

Port sources: `pm4py/algo/conformance/declare/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`.

### conformance-algo-conformance-log-skeleton-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_log_skeleton`.

Port sources: `pm4py/algo/conformance/log_skeleton/algorithm`, `pm4py/conformance.py`, `pm4py/objects/log/obj`.

### conformance-algo-conformance-ocel-ocdfg-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_ocdfg`.

Port sources: `pm4py/algo/conformance/ocel/ocdfg/algorithm`, `pm4py/conformance.py`, `pm4py/objects/ocel/obj`.

### conformance-algo-conformance-ocel-otg-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_otg`.

Port sources: `pm4py/algo/conformance/ocel/otg/algorithm`, `pm4py/conformance.py`, `pm4py/objects/ocel/obj`.

### conformance-algo-conformance-ocel-etot-algorithm-1

Crate: `ichnos-conformance`. Rows: `pm4py.conformance_etot`.

Port sources: `pm4py/algo/conformance/ocel/etot/algorithm`, `pm4py/conformance.py`, `pm4py/objects/ocel/obj`.

### org-org-1

Crate: `ichnos-org`. Rows: `pm4py.discover_handover_of_work_network`, `pm4py.discover_working_together_network`, `pm4py.discover_activity_based_resource_similarity`, `pm4py.discover_subcontracting_network`, `pm4py.discover_organizational_roles`, `pm4py.discover_network_analysis`.

Port sources: `pm4py/algo/organizational_mining/network_analysis/algorithm`, `pm4py/algo/organizational_mining/network_analysis/variants/dataframe`, `pm4py/algo/organizational_mining/roles/algorithm`, `pm4py/algo/organizational_mining/sna/algorithm`, `pm4py/objects/log/obj`, `pm4py/objects/org/roles/obj`, `pm4py/objects/org/sna/obj`, `pm4py/org.py`.

### sim-sim-1

Crate: `ichnos-sim`. Rows: `pm4py.play_out`, `pm4py.generate_process_tree`.

Port sources: `pm4py/algo/simulation/playout/declare/algorithm`, `pm4py/algo/simulation/playout/dfg/algorithm`, `pm4py/algo/simulation/playout/petri_net/algorithm`, `pm4py/algo/simulation/playout/process_tree/algorithm`, `pm4py/algo/simulation/tree_generator/algorithm`, `pm4py/objects/log/obj`, `pm4py/objects/petri_net/inhibitor_reset/semantics`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/semantics`, `pm4py/objects/process_tree/obj`, `pm4py/sim.py`.

### ml-ml-1

Crate: `ichnos-ml`. Rows: `pm4py.split_train_test`, `pm4py.get_prefixes_from_log`, `pm4py.extract_outcome_enriched_dataframe`, `pm4py.extract_features_dataframe`, `pm4py.extract_ocel_features`, `pm4py.extract_temporal_features_dataframe`, `pm4py.extract_target_vector`.

Port sources: `pm4py/algo/transformation/log_to_target/algorithm`, `pm4py/algo/transformation/ocel/features/objects/algorithm`, `pm4py/algo/transformation/trace_encodings/algorithm`, `pm4py/algo/transformation/trace_encodings/variants/temporal`, `pm4py/algo/transformation/trace_encodings/variants/temporal_lazy`, `pm4py/ml.py`, `pm4py/objects/conversion/log/converter`, `pm4py/objects/log/obj`, `pm4py/objects/log/util/get_prefixes`, `pm4py/objects/log/util/split_train_test`, `pm4py/objects/ocel/obj`.

### stream-algo-1

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.alignments.algorithm.apply`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._TrieNode`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws._State`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.IWSStreamingAlignments.finish`, `pm4py.streaming.algo.conformance.alignments.variants.approx_iws.apply`, `pm4py.streaming.algo.conformance.declare.algorithm.apply`, `pm4py.streaming.algo.conformance.declare.variants.automata.DeclareStreamingConformance`.

Port sources: `pm4py/algo/conformance/alignments/petri_net/utils/approx_utils`, `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/utils/align_utils`, `pm4py/streaming/algo/conformance/alignments/algorithm.py`, `pm4py/streaming/algo/conformance/alignments/variants/approx_iws.py`, `pm4py/streaming/algo/conformance/declare/algorithm.py`, `pm4py/streaming/algo/conformance/declare/variants/automata.py`, `pm4py/streaming/algo/interface`.

### stream-algo-2

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.declare.variants.automata.apply`, `pm4py.streaming.algo.conformance.footprints.algorithm.apply`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.build_dictionaries`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.encode_str`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_footprints`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_intra_case`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.verify_start_case`.

Port sources: `pm4py/streaming/algo/conformance/declare/variants/automata.py`, `pm4py/streaming/algo/conformance/footprints/algorithm.py`, `pm4py/streaming/algo/conformance/footprints/variants/classic.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-algo-3

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.get_status`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.terminate_all`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_or_activity_not_in_event`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_activity_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_footprints_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_start_activity_not_possible`, `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_end_activity_not_possible`.

Port sources: `pm4py/streaming/algo/conformance/footprints/variants/classic.py`.

### stream-algo-4

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.footprints.variants.classic.FootprintsStreamingConformance.message_case_not_in_dictionary`, `pm4py.streaming.algo.conformance.footprints.variants.classic.apply`, `pm4py.streaming.algo.conformance.tbr.algorithm.apply`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.build_dictionaries`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_paths_net`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_str`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.encode_marking`.

Port sources: `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/semantics`, `pm4py/streaming/algo/conformance/footprints/variants/classic.py`, `pm4py/streaming/algo/conformance/tbr/algorithm.py`, `pm4py/streaming/algo/conformance/tbr/variants/classic.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-algo-5

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.decode_marking`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.verify_tbr`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.enable_trans_with_invisibles`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.get_status`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.terminate_all`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.reach_fm_with_invisibles`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_or_activity_not_in_event`.

Port sources: `pm4py/objects/petri_net/obj`, `pm4py/objects/petri_net/semantics`, `pm4py/streaming/algo/conformance/tbr/variants/classic.py`.

### stream-algo-6

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_activity_not_possible`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_missing_tokens`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_case_not_in_dictionary`, `pm4py.streaming.algo.conformance.tbr.variants.classic.TbrStreamingConformance.message_final_marking_not_reached`, `pm4py.streaming.algo.conformance.tbr.variants.classic.apply`, `pm4py.streaming.algo.conformance.temporal.algorithm.apply`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.check_conformance`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/algo/conformance/tbr/variants/classic.py`, `pm4py/streaming/algo/conformance/temporal/algorithm.py`, `pm4py/streaming/algo/conformance/temporal/variants/classic.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-algo-7

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_event_is_not_complete`, `pm4py.streaming.algo.conformance.temporal.variants.classic.TemporalProfileStreamingConformance.message_deviation`, `pm4py.streaming.algo.conformance.temporal.variants.classic.apply`, `pm4py.streaming.algo.discovery.dfg.algorithm.apply`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.build_dictionaries`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.event_without_activity_or_case`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_str`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/algo/conformance/temporal/variants/classic.py`, `pm4py/streaming/algo/discovery/dfg/algorithm.py`, `pm4py/streaming/algo/discovery/dfg/variants/frequency.py`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/generator`.

### stream-algo-8

Crate: `ichnos-stream`. Rows: `pm4py.streaming.algo.discovery.dfg.variants.frequency.StreamingDfgDiscovery.encode_tuple`, `pm4py.streaming.algo.discovery.dfg.variants.frequency.apply`, `pm4py.streaming.algo.interface.StreamingAlgorithm`, `pm4py.streaming.algo.interface.StreamingAlgorithm.get`, `pm4py.streaming.algo.interface.StreamingAlgorithm.receive`.

Port sources: `pm4py/streaming/algo/discovery/dfg/variants/frequency.py`, `pm4py/streaming/algo/interface.py`.

### stream-connectors-1

Crate: `ichnos-stream`. Rows: `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.run`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.stop`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.get_process_name`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.record`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_click`, `pm4py.streaming.connectors.windows.click_key_logger.WindowsEventLogger.on_key_release`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/connectors/windows/click_key_logger.py`.

### stream-conversion-1

Crate: `ichnos-stream`. Rows: `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable`, `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.read_trace`, `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.reset`, `pm4py.streaming.conversion.from_pandas.PandasDataframeAsIterable.to_trace_stream`, `pm4py.streaming.conversion.from_pandas.apply`, `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor`, `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.register`, `pm4py.streaming.conversion.ocel_flatts_distributor.OcelFlattsDistributor.append`.

Port sources: `pm4py/objects/log/obj`, `pm4py/objects/ocel/constants`, `pm4py/streaming/conversion/from_pandas.py`, `pm4py/streaming/conversion/ocel_flatts_distributor.py`, `pm4py/streaming/stream/live_event_stream`, `pm4py/streaming/stream/live_trace_stream`.

### stream-importer-1

Crate: `ichnos-stream`. Rows: `pm4py.streaming.importer.csv.importer.apply`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.reset`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.to_event_stream`, `pm4py.streaming.importer.csv.variants.csv_event_stream.CSVEventStreamReader.read_event`, `pm4py.streaming.importer.csv.variants.csv_event_stream.apply`, `pm4py.streaming.importer.xes.importer.apply`, `pm4py.streaming.importer.xes.variants.xes_event_stream.parse_attribute`.

Port sources: `pm4py/streaming/importer/csv/importer.py`, `pm4py/streaming/importer/csv/variants/csv_event_stream.py`, `pm4py/streaming/importer/xes/importer.py`, `pm4py/streaming/importer/xes/variants/xes_event_stream.py`.

### stream-importer-2

Crate: `ichnos-stream`. Rows: `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.to_event_stream`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.reset`, `pm4py.streaming.importer.xes.variants.xes_event_stream.StreamingEventXesReader.read_event`, `pm4py.streaming.importer.xes.variants.xes_event_stream.apply`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.parse_attribute`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.to_trace_stream`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/importer/xes/variants/xes_event_stream.py`, `pm4py/streaming/importer/xes/variants/xes_trace_stream.py`.

### stream-importer-3

Crate: `ichnos-stream`. Rows: `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.reset`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.StreamingTraceXesReader.read_trace`, `pm4py.streaming.importer.xes.variants.xes_trace_stream.apply`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/importer/xes/variants/xes_trace_stream.py`.

### stream-stream-1

Crate: `ichnos-stream`. Rows: `pm4py.streaming.stream.live_event_stream.StreamState`, `pm4py.streaming.stream.live_event_stream.LiveEventStream`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.append`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.start`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.stop`, `pm4py.streaming.stream.live_event_stream.LiveEventStream.register`, `pm4py.streaming.stream.live_trace_stream.StreamState`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream`.

Port sources: `pm4py/streaming/stream/live_event_stream.py`, `pm4py/streaming/stream/live_trace_stream.py`.

### stream-stream-2

Crate: `ichnos-stream`. Rows: `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.append`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.start`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.stop`, `pm4py.streaming.stream.live_trace_stream.LiveTraceStream.register`.

Port sources: `pm4py/streaming/stream/live_trace_stream.py`.

### stream-util-1

Crate: `ichnos-stream`. Rows: `pm4py.streaming.util.dictio.generator.apply`, `pm4py.streaming.util.dictio.versions.classic.apply`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.keys`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.values`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.itervalues`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushdb`, `pm4py.streaming.util.dictio.versions.redis.ThreadSafeRedisDict.flushall`.

Port sources: `pm4py/streaming/util/dictio/generator.py`, `pm4py/streaming/util/dictio/versions/classic.py`, `pm4py/streaming/util/dictio/versions/redis.py`.

### stream-util-2

Crate: `ichnos-stream`. Rows: `pm4py.streaming.util.dictio.versions.redis.apply`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.keys`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.values`, `pm4py.streaming.util.dictio.versions.thread_safe.ThreadSafeDict.itervalues`, `pm4py.streaming.util.dictio.versions.thread_safe.apply`, `pm4py.streaming.util.event_stream_printer.EventStreamPrinter`, `pm4py.streaming.util.live_to_static_stream.LiveToStaticStream`.

Port sources: `pm4py/objects/log/obj`, `pm4py/streaming/algo/interface`, `pm4py/streaming/util/dictio/versions/redis.py`, `pm4py/streaming/util/dictio/versions/thread_safe.py`, `pm4py/streaming/util/event_stream_printer.py`, `pm4py/streaming/util/live_to_static_stream.py`.

### stream-util-3

Crate: `ichnos-stream`. Rows: `pm4py.streaming.util.trace_stream_printer.TraceStreamPrinter`.

Port sources: `pm4py/streaming/algo/interface`, `pm4py/streaming/util/trace_stream_printer.py`.

### privacy-privacy-1

Crate: `ichnos-privacy`. Rows: `pm4py.privacy.anonymize_differential_privacy`.

Port sources: `pm4py/algo/anonymization/pripel/algorithm`, `pm4py/algo/anonymization/trace_variant_query/algorithm`, `pm4py/objects/log/obj`, `pm4py/privacy.py`.

### viz-objects-log-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_petri_net`, `pm4py.save_vis_petri_net`, `pm4py.view_dotted_chart`, `pm4py.save_vis_dotted_chart`, `pm4py.view_alignments`, `pm4py.save_vis_alignments`.

Port sources: `pm4py/objects/log/obj`, `pm4py/objects/petri_net/obj`, `pm4py/vis.py`, `pm4py/visualization/align_table/visualizer`, `pm4py/visualization/dotted_chart/visualizer`, `pm4py/visualization/petri_net/visualizer`.

### viz-visualization-dfg-variants-performance-1

Crate: `ichnos-viz`. Rows: `pm4py.view_performance_dfg`, `pm4py.save_vis_performance_dfg`.

Port sources: `pm4py/vis.py`, `pm4py/visualization/dfg/variants/performance`, `pm4py/visualization/dfg/visualizer`.

### viz-visualization-dfg-visualizer-1

Crate: `ichnos-viz`. Rows: `pm4py.view_dfg`, `pm4py.save_vis_dfg`.

Port sources: `pm4py/vis.py`, `pm4py/visualization/dfg/visualizer`.

### viz-objects-process-tree-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_process_tree`, `pm4py.save_vis_process_tree`.

Port sources: `pm4py/objects/process_tree/obj`, `pm4py/vis.py`, `pm4py/visualization/process_tree/visualizer`.

### viz-objects-bpmn-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.save_vis_bpmn`, `pm4py.view_bpmn`.

Port sources: `pm4py/objects/bpmn/obj`, `pm4py/vis.py`, `pm4py/visualization/bpmn/visualizer`.

### viz-objects-heuristics-net-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_heuristics_net`, `pm4py.save_vis_heuristics_net`.

Port sources: `pm4py/objects/heuristics_net/obj`, `pm4py/vis.py`, `pm4py/visualization/heuristics_net/visualizer`.

### viz-algo-discovery-performance-spectrum-algorithm-1

Crate: `ichnos-viz`. Rows: `pm4py.view_performance_spectrum`, `pm4py.save_vis_performance_spectrum`.

Port sources: `pm4py/algo/discovery/performance_spectrum/algorithm`, `pm4py/objects/log/obj`, `pm4py/vis.py`, `pm4py/visualization/performance_spectrum/variants/neato`, `pm4py/visualization/performance_spectrum/visualizer`.

### viz-visualization-ocel-ocdfg-visualizer-1

Crate: `ichnos-viz`. Rows: `pm4py.view_ocdfg`, `pm4py.save_vis_ocdfg`.

Port sources: `pm4py/vis.py`, `pm4py/visualization/ocel/ocdfg/visualizer`.

### viz-objects-ocpn-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_ocpn`, `pm4py.save_vis_ocpn`.

Port sources: `pm4py/objects/ocpn/obj`, `pm4py/vis.py`, `pm4py/visualization/ocel/ocpn/visualizer`.

### viz-visualization-network-analysis-visualizer-1

Crate: `ichnos-viz`. Rows: `pm4py.view_network_analysis`, `pm4py.save_vis_network_analysis`.

Port sources: `pm4py/vis.py`, `pm4py/visualization/network_analysis/visualizer`.

### viz-objects-transition-system-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_transition_system`, `pm4py.save_vis_transition_system`.

Port sources: `pm4py/objects/transition_system/obj`, `pm4py/vis.py`, `pm4py/visualization/transition_system/visualizer`.

### viz-objects-trie-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_prefix_tree`, `pm4py.save_vis_prefix_tree`.

Port sources: `pm4py/objects/trie/obj`, `pm4py/vis.py`, `pm4py/visualization/trie/visualizer`.

### viz-visualization-footprints-visualizer-1

Crate: `ichnos-viz`. Rows: `pm4py.view_footprints`, `pm4py.save_vis_footprints`.

Port sources: `pm4py/vis.py`, `pm4py/visualization/footprints/visualizer`.

### viz-objects-powl-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_powl`, `pm4py.save_vis_powl`.

Port sources: `pm4py/objects/powl/obj`, `pm4py/vis.py`, `pm4py/visualization/powl/visualizer`.

### viz-objects-ocel-obj-1

Crate: `ichnos-viz`. Rows: `pm4py.view_object_graph`, `pm4py.save_vis_object_graph`.

Port sources: `pm4py/objects/ocel/obj`, `pm4py/vis.py`, `pm4py/visualization/ocel/object_graph/visualizer`.
