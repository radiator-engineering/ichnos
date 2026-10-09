# ichnos design

This file records design decisions and the reasons for them. Each section names the crate it applies to.

## Event log data model (`ichnos-core`)

`ichnos-core` defines the event log types that every other ichnos crate builds on. It ports the data model parts of pm4py 2.7.23.8 (commit 24a3bf6): `objects/log/obj.py`, the standard keys in `util/xes_constants.py` and `util/constants.py`, timestamp sorting from `objects/log/util/sorting.py`, the conversions in `objects/conversion/log/`, and a few helpers from `utils.py`.

### One canonical type: algorithms take `&EventLog`

pm4py keeps two parallel representations of a log: `EventLog` objects and pandas DataFrames. It implements most algorithms twice, once for each. ichnos keeps one in-memory type, `EventLog`, and every algorithm takes `&EventLog`. Each algorithm then has one implementation and one set of tests.

The Arrow `RecordBatch` is the columnar view of a log. It serves I/O (Parquet, CSV), bulk transforms and interchange with other tools. `EventLog::to_arrow` and `EventLog::from_arrow` (also `TryFrom`) convert between the two.

### Arrow, not polars, for the columnar view

The columnar view uses the `arrow` crate (version 60), not polars.

- Arrow is a stable, specified data format. The `arrow` crate is smaller than polars and its API changes more slowly.
- The `parquet` crate reads and writes Arrow data directly.
- Arrow data passes to pyarrow, polars and DuckDB through the Arrow C data interface without a copy.
- Polars would bring a large dependency tree, long compile times and a fast-changing API into a library crate. Users who want polars can get a DataFrame from the Arrow data without a copy.

`ichnos-core` re-exports `arrow` and `chrono`, so downstream crates and users name the same versions.

### Types

```text
EventLog   { attributes, extensions, globals, classifiers, traces: Vec<Trace> }
Trace      { attributes, events: Vec<Event> }
Event      { attributes }
EventStream{ attributes, extensions, globals, classifiers, events: Vec<Event> }
```

All four are plain structs with public fields. Construction uses struct literals, `FromIterator`, and a few helpers such as `Trace::with_case_id` and `EventLog::from_trace_strings`.

- **Log metadata.** XES extensions (`Vec<Extension>`), globals (`Globals { trace, event }`) and classifiers (`Vec<Classifier { name, keys }>`) are kept so that an XES file can be read and written back. Vectors keep declaration order. `XesExtension` lists the 12 standard extensions that pm4py knows.
- **`EventStream`** is the flat form: events with the trace attributes copied in under the `case:` prefix. It is the step between a table and a log, and the input for streaming and event sampling. `EventLog::into_event_stream` and `EventStream::into_event_log` convert between the two with pm4py's rules.
- **No `properties` map.** pm4py logs carry a `properties` dict that stores parameter keys such as the activity key. ichnos passes `EventKeys` explicitly instead, so no state hides in the log.

### Attribute values

`AttributeValue` has one variant per XES type: `String`, `Int(i64)`, `Float(f64)`, `Bool`, `Date`, `Id`, `List`, `Container`, plus `Meta` for a value that carries XES meta-attributes.

- **Dates are `chrono::DateTime<FixedOffset>`.** The offset is kept so that XES output can repeat the input offset. Comparison and equality use the instant, so dates with different offsets sort correctly. pm4py converts every date to UTC when it reads XES. ichnos keeps the offset, and the instant is the same.
- **Strings and keys are `Arc<str>`.** Logs repeat the same activity names and keys millions of times. A reader can share one allocation between all events that carry the same text, and cloning a value is cheap. `from_arrow` does this sharing per column.
- **Lists** hold ordered `(key, value)` pairs, because XES list children have keys and may repeat them. **Containers** (XES 2.0) hold an `Attributes` map.
- **Meta-attributes** wrap the value: `Meta(Box<MetaValue { value, meta }>)`. They are rare, so the box keeps `AttributeValue` at 32 bytes. The `as_*` accessors look through `Meta`, so a value with meta-attributes reads like a plain one. pm4py instead replaces such a value with a `{"value": ..., "children": ...}` dict.
- **`Display` matches Python's `str()`** for the common cases (`True`, `1.0`, `2020-01-01 10:00:00+00:00`), so joined classifier values and stringified activities match pm4py.

### The attribute map

`Attributes` is a newtype over `Vec<(Arc<str>, AttributeValue)>`. It keeps insertion order, so XES output repeats the input order. Lookup is a linear scan. Events usually have fewer than 20 attributes, and a scan of a short vector beats hashing; it also costs one allocation per event instead of two for a hash map. The newtype hides the representation, so it can change later without breaking callers. Equality ignores order, as pm4py's does.

### Keys: `EventKeys`

pm4py passes `activity_key`, `timestamp_key`, `case_id_key`, `resource_key` and others through parameter dicts. ichnos has one struct, `EventKeys`, that every crate accepts as `&EventKeys`. `EventKeys::default()` gives the XES standard keys: `concept:name`, `time:timestamp`, `start_timestamp`, `case:concept:name`, `org:resource`, `lifecycle:transition`, `org:group`, and the case prefix `case:`.

`case_id` is the case column of a flat table (`case:concept:name`). Inside an `EventLog` the case ID is always the trace attribute `concept:name`, as in pm4py.

### Activity interning

Most miners work on activity sequences, not on full events. `EventLog::activity_sequences(&keys)` returns every trace as a `Vec<ActivityId>`, with an `ActivityIndex` that maps IDs to names. IDs are dense `u32`s in order of first appearance, so miners can index vectors and matrices with them. `EventLog::variants(&keys)` groups the sequences into `Variants`: each distinct sequence with the indices of its traces, in order of first appearance.

### Arrow mapping

The table has one row per event, in log order. Event attributes become columns of the same name. Trace attributes become columns with the case prefix. This matches pm4py's `convert_to_dataframe`. Columns appear in order of first appearance, and a missing attribute is a null.

| Attribute values in a column | Arrow type |
|---|---|
| strings, or strings and IDs | `Utf8` |
| IDs only | `Utf8`, field metadata `ichnos:xes-type = id` |
| ints | `Int64` |
| floats, or ints and floats | `Float64` |
| booleans | `Boolean` |
| dates | `Timestamp(Nanosecond, tz)`; `tz` is the common offset, or `UTC` if offsets differ |
| any other mix | `Utf8`, values formatted as Python's `str()` |
| lists or containers | error: they have no flat column form |

Nanoseconds match pandas, so pm4py DataFrames and ichnos tables use the same unit. `from_arrow` accepts more: large, view and dictionary-encoded strings, all integer and float widths, decimals, timestamps of any unit and time zone, and `Date32`/`Date64`. A timestamp column without a time zone, or with a named zone, reads as UTC; Arrow stores UTC instants, so the instant is right either way. `from_arrow_batches` reads a table split into batches, as Parquet readers return it.

`from_arrow` groups rows into traces as pm4py's `to_event_log` does: by the case column, in order of each case's first row. The trace attributes come from the case's first row.

`to_arrow` writes the case ID (trace `concept:name`) to the column `keys.case_id`, so `from_arrow` with the same keys reads the table back. With the default keys that column is `case:concept:name`.

The table holds traces and events only. `to_arrow` does not write log attributes, extensions, globals or classifiers, so a round trip through Arrow loses them; `from_arrow` declares extensions again from the column names. pm4py's `convert_to_dataframe` loses them too. XES output keeps them.

`from_arrow` keeps the row order and column types of the table. pm4py reads a CSV through `format_dataframe`, which also casts the case ID and activity columns to strings and stable-sorts the rows by case ID, timestamp and input order. A reader that wants pm4py's trace and event order must apply both steps before `from_arrow`.

### Errors

`ichnos_core::Error` is one `thiserror` enum for the crate. Errors that concern an event carry a `Position`: trace and event index in a log, event index in a stream, or event index in a standalone trace. Operations that can fail part-way check first and change nothing on error.

Other ichnos crates follow one pattern. Each crate defines its own `thiserror` `Error` enum with a `Core(#[from] ichnos_core::Error)` variant, so `?` passes core errors through. An error about an event reuses `ichnos_core::Position` rather than defining its own.

### Sorting

`sort_by_timestamp(key, SortOrder)` on `Trace`, `EventLog` and `EventStream` is a stable sort, like Python's `sorted`. Descending order also keeps equal timestamps in input order, as `sorted(..., reverse=True)` does. On a log it sorts each trace's events, then sorts traces by their first event, as pm4py's `sort_timestamp_log` does.

### Sampling

`EventLog::sample_cases(n, seed)` and `EventStream::sample_events(n, seed)` draw without replacement. They use ChaCha8 (`rand_chacha`) seeded from an explicit `u64`, so a seed gives the same sample on every platform. pm4py uses Python's `random` module, whose sequence ichnos does not reproduce. ichnos also returns the sample in input order; pm4py's `random.sample` returns it in random order. pm4py's `sample_events` on an `EventLog` samples traces, so ichnos offers `sample_events` only on `EventStream`.

### Interval and lifecycle logs

`EventLog::to_interval` and `EventLog::to_lifecycle` port pm4py's `interval_lifecycle` helpers. They use the same marker attribute (`PM4PY_TYPE`) and helper attributes (`@@duration`, `@@startevent_*`, `@@custom_lif_id`, `@@origin_ev_idx`), so output matches pm4py's. The business-hours option of `to_interval` belongs with the performance crate and is not ported here.

### Formatting a raw table

`format_batch(batch, &keys, timestamp_format)` is the Arrow form of pm4py's `format_dataframe`. It copies the case, activity and timestamp columns to the standard names, converts timestamps to UTC, drops rows without a case ID, activity or timestamp, casts the case ID and activity to strings, sorts by case, timestamp and input order, and adds `@@index` and `@@case_index`. Running `format_batch` before `from_arrow` gives pm4py's trace and event order for a CSV table. `EventLog::rebase` and `EventStream::rebase` flatten, format and regroup, as pm4py's `rebase` does.

Two differences from pandas:

- pandas tries to parse every string column as a date. `format_batch` parses only the timestamp and start-timestamp columns, so a column of short numeric strings never turns into dates by accident.
- pandas leaves a column as strings when a value fails to parse. `format_batch` returns `Error::UnparseableTimestamp`. Without an explicit format it accepts ISO 8601 and RFC 3339 forms. Other layouts need `timestamp_format` in chrono syntax, which avoids pandas's day-first and month-first guessing.

### Closures over traces and events

pm4py's `hof` module filters and sorts with Python callables. ichnos takes Rust closures over `&Trace` and `&Event`: `EventLog::filter_traces`, `EventStream::filter_events` and `Trace::filter_events` return a new value with the metadata copied, as pm4py does. `sort_traces_by_key` and `sort_events_by_key` sort in place, stably in both orders, like `sort_by_timestamp`. The sort key must be `Ord`. Floats are not, so wrap a float key in a type that orders with `f64::total_cmp`. pm4py's `sort_trace` drops the trace attributes; ichnos keeps them. With an owned log, `log.traces.retain(...)` filters in place without cloning.

### Artificial start and end events

`EventLog::insert_artificial_start_end(&keys, start, end)` ports pm4py's event-log version: one event before and one after each trace, one second before the first and after the last timestamp. `artificial::ARTIFICIAL_START` (`▶`) and `ARTIFICIAL_END` (`■`) are pm4py's defaults. pm4py's DataFrame version differs: it shifts by one millisecond and sorts the table. ichnos has one version, on `EventLog`; a table goes through `from_arrow` first.

### The log as a graph

`EventLog::to_graph` ports `convert_log_to_networkx` with a petgraph `DiGraph<LogNode, LogEdge>` (`LogGraph`). Nodes are cases, events and attribute values; edges are `BelongsTo`, `DirectlyFollows` and `Attribute(key)`. `LogNode::id` gives pm4py's node names (`CASE=1`, `EVENT=1_0`, the value's `str()`), so graphs can be compared. NetworkX keys every node by its name, so in pm4py an attribute value whose text is `CASE=1` merges with that case node. ichnos keys cases, events and attribute values apart. The case ID is always the trace's `concept:name`; pm4py lets the caller name another trace attribute.

### Not ported, and why

- `serialize` / `deserialize`. pm4py's versions dispatch on the object type and return a `(type tag, bytes)` pair: XES for logs, Parquet for DataFrames, PNML, PTML, BPMN or DFG files for models. In Rust the caller knows the type, so each writer and reader in `ichnos-io` (XES, Parquet, Arrow IPC, PNML and the others) already gives the bytes, and values move between threads without serialising. A `serde` feature adds a second, unversioned format for every type and is not worth its cost until a lane needs it.
- `parse_process_tree` belongs to the model crate.
