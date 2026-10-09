//! Case features, ported from pm4py's `extract_features_dataframe`: the
//! `trace_based` encoding of `algo/transformation/trace_encodings`, data-frame
//! branch (`objects/log/util/df_features_utils.py`).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use ichnos_core::{AttributeValue, EventLog};

use crate::error::{Error, Result};
use crate::util::{ColumnKind, Table, mean, median};

/// One row of features per case.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeatureTable {
    /// The case of each row.
    pub case_ids: Vec<String>,
    /// The feature names, one per column.
    pub names: Vec<String>,
    /// The rows. A missing value is NaN.
    pub rows: Vec<Vec<f64>>,
}

impl FeatureTable {
    /// The values of the feature `name`, one per row.
    pub fn column(&self, name: &str) -> Option<Vec<f64>> {
        let k = self.names.iter().position(|n| n == name)?;
        Some(self.rows.iter().map(|r| r[k]).collect())
    }
}

/// How a numeric attribute is summarised per case when statistics are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericAggregation {
    /// The last value (`_LAST`).
    Last,
    /// The first value (`_FIRST`).
    First,
    /// The smallest value (`_MIN`).
    Min,
    /// The largest value (`_MAX`).
    Max,
    /// The mean (`_MEAN`).
    Mean,
    /// The median (`_MEDIAN`).
    Median,
    /// The population standard deviation (`_STDEV`).
    Stdev,
    /// The sum (`_SUM`).
    Sum,
}

impl NumericAggregation {
    /// pm4py's default aggregations, in its order.
    pub const ALL: [NumericAggregation; 8] = [
        NumericAggregation::Last,
        NumericAggregation::First,
        NumericAggregation::Min,
        NumericAggregation::Max,
        NumericAggregation::Mean,
        NumericAggregation::Median,
        NumericAggregation::Stdev,
        NumericAggregation::Sum,
    ];

    fn suffix(self) -> &'static str {
        match self {
            NumericAggregation::Last => "LAST",
            NumericAggregation::First => "FIRST",
            NumericAggregation::Min => "MIN",
            NumericAggregation::Max => "MAX",
            NumericAggregation::Mean => "MEAN",
            NumericAggregation::Median => "MEDIAN",
            NumericAggregation::Stdev => "STDEV",
            NumericAggregation::Sum => "SUM",
        }
    }

    fn apply(self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return f64::NAN;
        }
        match self {
            NumericAggregation::Last => values[values.len() - 1],
            NumericAggregation::First => values[0],
            NumericAggregation::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
            NumericAggregation::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            NumericAggregation::Mean => mean(values),
            NumericAggregation::Median => median(values),
            NumericAggregation::Stdev => {
                let m = mean(values);
                let var =
                    values.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / values.len() as f64;
                var.sqrt()
            }
            NumericAggregation::Sum => values.iter().sum(),
        }
    }
}

/// Options for [`extract_features_dataframe`], with pm4py's defaults.
#[derive(Clone, Debug)]
pub struct FeatureOptions {
    /// String trace attributes, read from the prefixed columns.
    pub str_tr_attr: Vec<String>,
    /// Numeric trace attributes, read from the prefixed columns.
    pub num_tr_attr: Vec<String>,
    /// String event attributes.
    pub str_ev_attr: Vec<String>,
    /// Numeric event attributes.
    pub num_ev_attr: Vec<String>,
    /// The activity attribute, always kept by the automatic selection.
    /// Default `concept:name`.
    pub activity_key: String,
    /// The timestamp attribute, never a feature. Default `time:timestamp`.
    pub timestamp_key: String,
    /// The case column. Default `case:concept:name`.
    pub case_id_key: String,
    /// The prefix of trace attributes in the flat table. Default `case:`.
    pub case_prefix: String,
    /// Count each value of a string column instead of marking it with 1.
    pub count_occurrences: bool,
    /// Summarise numeric columns with `numeric_attribute_aggregations`
    /// instead of keeping their last value.
    pub enable_numeric_attribute_statistics: bool,
    /// The summaries of numeric columns. Setting it turns statistics on.
    /// `None` with statistics on means [`NumericAggregation::ALL`].
    pub numeric_attribute_aggregations: Option<Vec<NumericAggregation>>,
    /// The automatic selection keeps a string column with at least this
    /// many distinct values. Default 5.
    pub min_different_occ_str_attr: usize,
    /// The automatic selection keeps a string column with at most this many
    /// distinct values. Default 50.
    pub max_different_occ_str_attr: usize,
    /// The automatic selection also considers columns missing from some
    /// case. Default `true`.
    pub consider_all_attributes: bool,
}

impl Default for FeatureOptions {
    fn default() -> Self {
        FeatureOptions {
            str_tr_attr: Vec::new(),
            num_tr_attr: Vec::new(),
            str_ev_attr: Vec::new(),
            num_ev_attr: Vec::new(),
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            case_id_key: "case:concept:name".to_owned(),
            case_prefix: "case:".to_owned(),
            count_occurrences: false,
            enable_numeric_attribute_statistics: false,
            numeric_attribute_aggregations: None,
            min_different_occ_str_attr: 5,
            max_different_occ_str_attr: 50,
            consider_all_attributes: true,
        }
    }
}

/// One row of features per case, as pm4py's `extract_features_dataframe`
/// computes them for a data frame.
///
/// The log is read as a flat table: one row per event, with trace
/// attributes under the case prefix. With no attribute lists, columns are
/// chosen automatically: every numeric column, every string column with
/// between `min_different_occ_str_attr` and `max_different_occ_str_attr`
/// distinct values, and the activity, but never the case or the
/// timestamp. Otherwise the listed columns are used. Dates and booleans
/// give no features.
///
/// A numeric column gives its last value in the case, or the summaries of
/// `numeric_attribute_aggregations` named `<column>_<SUFFIX>`; columns
/// starting with `@@` always keep their last value. A string column gives
/// one feature `<column>_<value>` per value, 1 when the case has it and 0
/// otherwise; names lose non-ASCII characters and get `__1`, `__2`, ... when
/// they clash. A case without any value of a column gets NaN in its
/// features. Values are rounded to 32-bit floats, as pm4py stores them.
///
/// Rows are sorted by case ID. pm4py orders the columns by Python's set
/// order, which changes between runs; this puts numeric columns first, then
/// string columns, each sorted by name.
pub fn extract_features_dataframe(
    log: &EventLog,
    options: &FeatureOptions,
) -> Result<FeatureTable> {
    let table = Table::new(log, &options.case_prefix);
    let cases = table.cases(&options.case_id_key)?;
    let case_ids: Vec<String> = cases
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let position: BTreeMap<&str, usize> = case_ids
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();
    let row_case: Vec<usize> = cases.iter().map(|c| position[c.as_str()]).collect();

    let explicit = !(options.str_tr_attr.is_empty()
        && options.num_tr_attr.is_empty()
        && options.str_ev_attr.is_empty()
        && options.num_ev_attr.is_empty());
    let mut columns: BTreeSet<String> = BTreeSet::new();
    if explicit {
        for a in options.str_tr_attr.iter().chain(&options.num_tr_attr) {
            columns.insert(format!("{}{a}", options.case_prefix));
        }
        columns.extend(options.str_ev_attr.iter().cloned());
        columns.extend(options.num_ev_attr.iter().cloned());
        if let Some(missing) = columns.iter().find(|c| !table.columns.contains(c)) {
            return Err(Error::MissingColumn(missing.clone()));
        }
    } else {
        for c in [
            &options.case_id_key,
            &options.activity_key,
            &options.timestamp_key,
        ] {
            if table.columns.contains(c) {
                columns.insert(c.clone());
            }
        }
        for c in &table.columns {
            let present: HashSet<usize> = (0..table.rows.len())
                .filter(|&r| table.value(r, c).is_some())
                .map(|r| row_case[r])
                .collect();
            if !(present.len() == case_ids.len() || options.consider_all_attributes) {
                continue;
            }
            match table.kind(c) {
                ColumnKind::Numeric => {
                    columns.insert(c.clone());
                }
                ColumnKind::Text => {
                    let distinct: HashSet<String> = (0..table.rows.len())
                        .filter_map(|r| table.value(r, c))
                        .map(ToString::to_string)
                        .collect();
                    if (options.min_different_occ_str_attr..=options.max_different_occ_str_attr)
                        .contains(&distinct.len())
                    {
                        columns.insert(c.clone());
                    }
                }
                ColumnKind::Other => {}
            }
        }
        columns.remove(&options.case_id_key);
        columns.remove(&options.timestamp_key);
    }

    let numeric: Vec<&String> = columns
        .iter()
        .filter(|c| table.kind(c) == ColumnKind::Numeric)
        .collect();
    let strings: Vec<&String> = columns
        .iter()
        .filter(|c| table.kind(c) == ColumnKind::Text)
        .collect();

    let mut names: Vec<String> = Vec::new();
    let mut values: Vec<Vec<f64>> = Vec::new();
    let statistics = options.enable_numeric_attribute_statistics
        || options.numeric_attribute_aggregations.is_some();
    let aggregations: Vec<NumericAggregation> = {
        let mut seen = Vec::new();
        for a in options
            .numeric_attribute_aggregations
            .as_deref()
            .unwrap_or(&NumericAggregation::ALL)
        {
            if !seen.contains(a) {
                seen.push(*a);
            }
        }
        seen
    };
    for col in &numeric {
        let mut per_case: Vec<Vec<f64>> = vec![Vec::new(); case_ids.len()];
        for r in 0..table.rows.len() {
            if let Some(v) = table.value(r, col).and_then(AttributeValue::as_f64) {
                per_case[row_case[r]].push(v);
            }
        }
        let summaries: Vec<(String, NumericAggregation)> = if !statistics || col.starts_with("@@") {
            vec![((*col).clone(), NumericAggregation::Last)]
        } else {
            aggregations
                .iter()
                .map(|a| (format!("{col}_{}", a.suffix()), *a))
                .collect()
        };
        for (name, a) in summaries {
            names.push(name);
            values.push(per_case.iter().map(|v| f32_round(a.apply(v))).collect());
        }
    }

    let mut used: HashSet<String> = names.iter().cloned().collect();
    used.insert(options.case_id_key.clone());
    for col in &strings {
        let mut per_case: Vec<BTreeMap<String, u64>> = vec![BTreeMap::new(); case_ids.len()];
        let mut distinct: BTreeSet<String> = BTreeSet::new();
        for r in 0..table.rows.len() {
            if let Some(v) = table.value(r, col) {
                let v = v.to_string();
                *per_case[row_case[r]].entry(v.clone()).or_default() += 1;
                distinct.insert(v);
            }
        }
        for value in distinct {
            names.push(sanitize(col, &value, &mut used));
            values.push(
                per_case
                    .iter()
                    .map(|counts| {
                        if counts.is_empty() {
                            f64::NAN
                        } else {
                            let n = counts.get(&value).copied().unwrap_or(0);
                            if options.count_occurrences {
                                n as f64
                            } else {
                                f64::from(u8::from(n > 0))
                            }
                        }
                    })
                    .collect(),
            );
        }
    }

    let rows = (0..case_ids.len())
        .map(|i| values.iter().map(|c| c[i]).collect())
        .collect();
    Ok(FeatureTable {
        case_ids,
        names,
        rows,
    })
}

/// pm4py stores features as `float32`.
fn f32_round(v: f64) -> f64 {
    f64::from(v as f32)
}

/// pm4py's `_sanitize_feature_name`.
fn sanitize(prefix: &str, value: &str, used: &mut HashSet<String>) -> String {
    let mut clean: String = value.chars().filter(char::is_ascii).collect();
    if clean.is_empty() {
        clean = "value".to_owned();
    }
    let base = format!("{prefix}_{clean}");
    let mut candidate = base.clone();
    let mut suffix = 1;
    while used.contains(&candidate) {
        candidate = format!("{base}__{suffix}");
        suffix += 1;
    }
    used.insert(candidate.clone());
    candidate
}
