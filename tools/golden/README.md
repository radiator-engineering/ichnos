# Golden generator

pm4py is the oracle for ichnos. This tool runs pm4py on the fixture logs in
`fixtures/logs/` and writes what it computes to
`fixtures/golden/<area>/<case>.json`. Rust tests load those files through the
`ichnos-golden` crate and compare.

The goldens are committed. CI does not run Python; it only reads them.

## Running it

You need a checkout of pm4py 2.7.23.8 (commit 24a3bf6) installed in a Python
environment. Set two variables:

- `PM4PY_PYTHON`: the Python interpreter of that environment.
- `PM4PY_SRC`: the pm4py checkout. Optional; the generator reads the commit
  from it, and otherwise from the checkout that pm4py was imported from.

```sh
$PM4PY_PYTHON tools/golden/generate.py                     # every area
$PM4PY_PYTHON tools/golden/generate.py --area log          # one area
$PM4PY_PYTHON tools/golden/generate.py --area log --case receipt-xes
$PM4PY_PYTHON tools/golden/generate.py --check             # write nothing; exit 1 on any change
```

The generator refuses to run against any pm4py other than 2.7.23.8 at commit
24a3bf6.

`--check` exits 1 when a file would change, is missing, or is stale: a file on
disk that no case registers. Delete stale files by hand.

## Adding a case

Each area has one module, `tools/golden/cases/<area>.py`. The module name is
the area name, and a case can only register into the area of the file it is
in. A lane edits its own area module and nothing else, so two lanes never edit
the same file.

1. Open `cases/<area>.py`, or create it for a new area.
2. Register a case with the `case` decorator:

   ```python
   import pm4py

   from harness import case
   from harness.fixtures import load_log


   @case(
       "running-example-rework",
       fixture="running-example.xes",
       functions=["pm4py.get_rework_cases_per_activity"],
   )
   def rework(fixtures):
       log = load_log(fixtures["log"])
       return pm4py.get_rework_cases_per_activity(log)
   ```

   - The id is lowercase letters, digits, `-` and `_`. It becomes the file name.
   - `fixture="x.xes"` names one input under `fixtures/logs/`, with role `log`.
     For several inputs, use `fixtures={"log": "x.xes", "model": "x.pnml"}`.
   - `functions` lists the pm4py functions the case calls. It goes into the
     meta block.
   - `params={...}` is passed to the function as keyword arguments and
     recorded in the meta block. Use it for the pm4py parameters the case
     varies, so the meta block shows them.
   - The function receives `{role: Path}` and returns the expected value.
   - To register one function for several fixtures, call
     `case(id, fixture=..., functions=...)(fn)` in a loop. See `cases/log.py`.
3. Load inputs with `harness.fixtures.load_log` and `load_model`, so every
   case sees the same pm4py object for the same file.
4. Run `generate.py --area <area>`, then `generate.py --check` to confirm a
   second run changes nothing.
5. Add a Rust test that loads the golden with `ichnos_golden::golden(area, id)`.
6. Commit the case module and the generated JSON together.

Mappings in the result must have string keys. Convert results keyed by tuples
(a DFG, variants) into lists of records, sorted, as `cases/log.py` does. The
generator raises an error that names the key otherwise.

That sorted order is the generator's, not pm4py's. pm4py returns variants in
first-appearance order, and so may ichnos. Compare variant lists and DFG
edges as multisets, for example with `ichnos_golden::assert_multiset_eq` or
`LogSummary::variant_counts()` and `dfg_counts()`, never as ordered lists.

## File format

```json
{
  "expected": ...,
  "meta": {
    "area": "log",
    "case": "running-example-xes",
    "fixtures": {"log": "fixtures/logs/running-example.xes"},
    "functions": ["pm4py.get_variants", "..."],
    "loaders": {"log": {"function": "pm4py.read_xes", "params": {"variant": "iterparse"}}},
    "params": {},
    "pm4py_commit": "24a3bf610aea6ecc4938b1864b3ad71fcfb82084",
    "pm4py_version": "2.7.23.8"
  }
}
```

Fixture paths are relative to the workspace root.

Output is deterministic. Running the generator twice gives byte-identical
files:

- Object keys are sorted by code point. Indent is two spaces. Files are UTF-8
  with a trailing newline.
- Sets become lists sorted by their canonical JSON text. Tuples become lists.
- Floats use Python's shortest round-trip form (`repr`). `-0.0` becomes `0.0`.
- NaN and infinities become the strings `"NaN"`, `"Infinity"` and
  `"-Infinity"`, because JSON has no literal for them.
  `ichnos_golden::as_f64` reads both forms.
- Datetimes become RFC 3339 strings in UTC with a `Z` suffix, for example
  `2010-12-30T13:32:00Z`. Fractional seconds appear only when non-zero: 6
  digits, or 9 with nanoseconds. Naive datetimes are taken as UTC.
- Timedeltas become float seconds.
- `None`, `pandas.NA` and `NaT` become `null`.

## Fixture loading

- XES (`.xes`, `.xes.gz`): `pm4py.read_xes(path, variant="iterparse")`. The
  variant is pinned, so that installing a Rust XES backend cannot change the
  goldens.
- CSV: `pandas.read_csv(path)`, then `pm4py.format_dataframe` with pm4py's
  default column names (`case:concept:name`, `concept:name`,
  `time:timestamp`), as pm4py's own tests do.
- Parquet: `pandas.read_parquet(path)`, then `pm4py.format_dataframe`. This
  needs `pyarrow` or `fastparquet` in the Python environment; pm4py does not
  install either.
- Models: `pm4py.read_pnml`, `pm4py.read_ptml`, `pm4py.read_bpmn`.

`format_dataframe` changes the table before any case sees it. An ichnos CSV
or Parquet reader must give the same result to match these goldens.
`format_dataframe` does four things:

- It tries to parse every text column as a UTC datetime and keeps each
  column that parses.
- It drops rows with no case ID, activity or timestamp.
- It casts the case ID and the activity to strings.
- It sorts events by (case ID as a string, timestamp, original row
  position). Case IDs therefore sort as text, so `"10"` comes before `"2"`.
  Events with equal timestamps keep their file order.

## Compact log I/O goldens

The `io` area stores case/event counts, global attribute type sets, and counted
per-trace type profiles (trace attributes and the union of event attribute types).
Activity variants use SHA-256 fingerprints with counts; an additional fingerprint
of the full ordered sequence list checks trace and event order, including missing
activity names. Fingerprints use UTF-8 JSON with sorted object keys, no whitespace,
and literal Unicode. Repeated sequences are not stored in full.

The first three traces retain every trace/event attribute for value comparisons,
including nested attributes. Dates are UTC ISO strings with six fractional digits;
null pandas values and attributes without text keys are omitted to match the core
model. Lists retain ordered key/value pairs, containers retain maps, and scalar
meta-attributes retain both their value and children. Rust tests compare these
samples with the oracle before checking the full XES write/read round trip.

## Models: emit behaviour, not structure

Some pm4py outputs are models: Petri nets, process trees, BPMN graphs. Their
node ids, place names and child order are pm4py implementation details. Rust
must not match them. A case that produces a model emits its behaviour instead,
through `harness.behaviour.model_behaviour(log, model)`:

```json
{
  "model_kind": "petri_net",
  "footprints": {
    "activities": ["a", "b"],
    "parallel": [["b", "c"], ["c", "b"]],
    "sequence": [["a", "b"]],
    "start_activities": ["a"]
  },
  "fitness_tbr": {"average_trace_fitness": 1.0, "log_fitness": 1.0, "perc_fit_traces": 100.0, "percentage_of_fitting_traces": 100.0},
  "precision_tbr": 0.753
}
```

- `model_kind` is `petri_net`, `process_tree` or `bpmn`.
- `footprints` is `pm4py.discover_footprints` on the model as given. Relations
  are sorted lists of `[a, b]` pairs. Its keys depend on the model kind,
  because pm4py computes different footprints for each:
  - Petri net and BPMN: `sequence`, `parallel`, `activities`,
    `start_activities`. pm4py does not compute end activities for a Petri net.
  - Process tree: those keys plus `end_activities`, `skippable`,
    `activities_always_happening`, `min_trace_length` and
    `max_trace_length_wo_loops`.
- `fitness_tbr` is `pm4py.fitness_token_based_replay` and `precision_tbr` is
  `pm4py.precision_token_based_replay`, both on the log the model came from.
  Process trees and BPMN graphs are converted with
  `pm4py.convert_to_petri_net` for replay.
- With `model_behaviour(log, model, alignments=True)`, the result also holds
  `fitness_alignments` and `precision_alignments`. Alignments are exact but
  slow; use them on small logs only.

List `harness.behaviour.FUNCTIONS` (and `ALIGNMENT_FUNCTIONS` when you use
alignments) in the case's `functions`.

A model lane compares its own model's behaviour against these values: the same
footprint relations and start activities, and fitness and precision within
`Tolerance::METRIC`.

## Tolerances

`ichnos-golden` compares floats with `Tolerance`. A value passes when
`|actual - expected| <= abs` or `|actual - expected| <= rel * max(|actual|, |expected|)`.

| Preset | abs | rel | Use for |
|---|---|---|---|
| `Tolerance::EXACT` | 0 | 0 | values that must match bit for bit |
| `Tolerance::COUNT` | 1e-9 | 0 | counts that pass through floats |
| `Tolerance::METRIC` | 1e-12 | 1e-6 | computed metrics: fitness, precision, durations |

Integers in JSON always compare exactly. NaN equals NaN, and an infinity
equals the same infinity.

## pm4py output that depends on the hash seed

Some pm4py algorithms iterate Python sets, so their output can change with
`PYTHONHASHSEED`. The generator does not pin the seed, so such a case fails
`--check` at random. pm4py's IMf is one: the order of its exclusive-choice
groups follows set order, and IMf breaks ties by that order.

For such a case, mine the input once per seed, each in a fresh interpreter,
and emit every distinct result. `cases/discovery.py` does this in
`_inductive_seeds`: it runs the module as a script under seeds 0–7 and emits
`runs`, one entry per distinct tree with the `seeds` that produced it, sorted
by tree. The Rust test then accepts a result that equals any run. Other lanes
may reuse `_inductive_seeds` or copy its pattern. Use it only where you have
shown that the output changes with the seed, because each seed costs one
interpreter start.
