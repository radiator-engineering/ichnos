//! Comparison helpers with diff-style failure messages.
//!
//! Each check comes in two forms: `compare_*` returns `Result<(), Mismatch>`,
//! and `assert_*` panics with the mismatch. The message lists every difference
//! by path, with `-` for the expected (pm4py) side and `+` for the actual
//! (ichnos) side.

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display};

use serde_json::Value;

/// How far a float may be from the expected value.
///
/// A value passes when `|actual - expected| <= abs` or
/// `|actual - expected| <= rel * max(|actual|, |expected|)`. NaN equals NaN,
/// and an infinity equals the same infinity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Absolute tolerance.
    pub abs: f64,
    /// Relative tolerance.
    pub rel: f64,
}

impl Tolerance {
    /// Exact equality.
    pub const EXACT: Self = Self { abs: 0.0, rel: 0.0 };
    /// Counts that pass through floats: absolute 1e-9.
    pub const COUNT: Self = Self {
        abs: 1e-9,
        rel: 0.0,
    };
    /// Computed metrics: relative 1e-6, with absolute 1e-12 so values near zero can pass.
    pub const METRIC: Self = Self {
        abs: 1e-12,
        rel: 1e-6,
    };

    /// An absolute-only tolerance.
    pub const fn absolute(abs: f64) -> Self {
        Self { abs, rel: 0.0 }
    }

    /// A relative-only tolerance.
    pub const fn relative(rel: f64) -> Self {
        Self { abs: 0.0, rel }
    }

    /// Whether `actual` is within this tolerance of `expected`.
    pub fn accepts(&self, actual: f64, expected: f64) -> bool {
        if actual.is_nan() || expected.is_nan() {
            return actual.is_nan() && expected.is_nan();
        }
        if actual == expected {
            return true;
        }
        if actual.is_infinite() || expected.is_infinite() {
            return false;
        }
        let diff = (actual - expected).abs();
        diff <= self.abs || diff <= self.rel * actual.abs().max(expected.abs())
    }
}

impl Display for Tolerance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "abs {:e}, rel {:e}", self.abs, self.rel)
    }
}

/// A failed comparison: one line per difference.
#[derive(Debug, Clone, PartialEq)]
pub struct Mismatch {
    /// What was compared, for the header line.
    pub context: String,
    /// One entry per difference, already formatted.
    pub differences: Vec<String>,
}

/// At most this many differences are printed; the rest are counted.
const MAX_SHOWN: usize = 50;

impl Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.differences.len();
        writeln!(
            f,
            "{}: {n} difference{} (- expected, + actual)",
            self.context,
            if n == 1 { "" } else { "s" }
        )?;
        for diff in self.differences.iter().take(MAX_SHOWN) {
            writeln!(f, "{diff}")?;
        }
        if n > MAX_SHOWN {
            writeln!(f, "... and {} more", n - MAX_SHOWN)?;
        }
        Ok(())
    }
}

impl std::error::Error for Mismatch {}

fn check(context: &str, differences: Vec<String>) -> Result<(), Mismatch> {
    if differences.is_empty() {
        Ok(())
    } else {
        Err(Mismatch {
            context: context.to_owned(),
            differences,
        })
    }
}

#[track_caller]
fn or_panic(result: Result<(), Mismatch>) {
    if let Err(mismatch) = result {
        panic!("{mismatch}");
    }
}

fn float_line(path: &str, actual: f64, expected: f64, tol: Tolerance) -> String {
    format!(
        "  {path}:\n    - {expected:?}\n    + {actual:?}\n      (diff {:e}, tolerance {tol})",
        (actual - expected).abs()
    )
}

/// Compare two floats within a tolerance.
pub fn compare_close(actual: f64, expected: f64, tol: Tolerance) -> Result<(), Mismatch> {
    let diffs = if tol.accepts(actual, expected) {
        Vec::new()
    } else {
        vec![float_line("value", actual, expected, tol)]
    };
    check("float mismatch", diffs)
}

/// Assert two floats are within a tolerance.
#[track_caller]
pub fn assert_close(actual: f64, expected: f64, tol: Tolerance) {
    or_panic(compare_close(actual, expected, tol));
}

/// Compare two maps regardless of iteration order.
///
/// Reports keys missing from `actual`, keys `actual` has but `expected` lacks,
/// and keys whose values differ.
pub fn compare_map_eq<K, V>(
    actual: impl IntoIterator<Item = (K, V)>,
    expected: impl IntoIterator<Item = (K, V)>,
) -> Result<(), Mismatch>
where
    K: Ord + Debug,
    V: PartialEq + Debug,
{
    compare_maps(
        actual,
        expected,
        |a, e| a == e,
        |key, a, e| format!("  {key:?}:\n    - {e:?}\n    + {a:?}"),
    )
}

/// Assert two maps are equal regardless of iteration order. See [`compare_map_eq`].
#[track_caller]
pub fn assert_map_eq<K, V>(
    actual: impl IntoIterator<Item = (K, V)>,
    expected: impl IntoIterator<Item = (K, V)>,
) where
    K: Ord + Debug,
    V: PartialEq + Debug,
{
    or_panic(compare_map_eq(actual, expected));
}

/// Compare two maps with float values regardless of order, values within `tol`.
pub fn compare_map_close<K>(
    actual: impl IntoIterator<Item = (K, f64)>,
    expected: impl IntoIterator<Item = (K, f64)>,
    tol: Tolerance,
) -> Result<(), Mismatch>
where
    K: Ord + Debug,
{
    compare_maps(
        actual,
        expected,
        |a, e| tol.accepts(*a, *e),
        |key, a, e| float_line(&format!("{key:?}"), *a, *e, tol),
    )
}

/// Assert two maps with float values are equal within `tol`. See [`compare_map_close`].
#[track_caller]
pub fn assert_map_close<K>(
    actual: impl IntoIterator<Item = (K, f64)>,
    expected: impl IntoIterator<Item = (K, f64)>,
    tol: Tolerance,
) where
    K: Ord + Debug,
{
    or_panic(compare_map_close(actual, expected, tol));
}

fn compare_maps<K, V>(
    actual: impl IntoIterator<Item = (K, V)>,
    expected: impl IntoIterator<Item = (K, V)>,
    same: impl Fn(&V, &V) -> bool,
    line: impl Fn(&K, &V, &V) -> String,
) -> Result<(), Mismatch>
where
    K: Ord + Debug,
    V: Debug,
{
    let mut actual: BTreeMap<K, V> = actual.into_iter().collect();
    let mut diffs = Vec::new();
    for (key, e) in expected {
        match actual.remove(&key) {
            None => diffs.push(format!("  - {key:?}: {e:?}")),
            Some(a) if !same(&a, &e) => diffs.push(line(&key, &a, &e)),
            Some(_) => {}
        }
    }
    diffs.extend(actual.iter().map(|(k, a)| format!("  + {k:?}: {a:?}")));
    check("map mismatch", diffs)
}

/// Compare two multisets: the same items with the same multiplicities, in any order.
pub fn compare_multiset_eq<T>(
    actual: impl IntoIterator<Item = T>,
    expected: impl IntoIterator<Item = T>,
) -> Result<(), Mismatch>
where
    T: Ord + Debug,
{
    let mut counts: BTreeMap<T, i64> = BTreeMap::new();
    for item in expected {
        *counts.entry(item).or_default() += 1;
    }
    for item in actual {
        *counts.entry(item).or_default() -= 1;
    }
    let diffs = counts
        .iter()
        .filter(|(_, n)| **n != 0)
        .map(|(item, n)| {
            let (sign, n) = if *n > 0 { ('-', *n) } else { ('+', -*n) };
            if n == 1 {
                format!("  {sign} {item:?}")
            } else {
                format!("  {sign} {item:?} (x{n})")
            }
        })
        .collect();
    check("multiset mismatch", diffs)
}

/// Assert two multisets are equal. See [`compare_multiset_eq`].
#[track_caller]
pub fn assert_multiset_eq<T>(
    actual: impl IntoIterator<Item = T>,
    expected: impl IntoIterator<Item = T>,
) where
    T: Ord + Debug,
{
    or_panic(compare_multiset_eq(actual, expected));
}

/// Read a JSON number as `f64`, or one of the strings `"NaN"`, `"Infinity"`, `"-Infinity"`.
pub fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => match s.as_str() {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            _ => None,
        },
        _ => None,
    }
}

/// Options for [`compare_json`].
#[derive(Debug, Clone, PartialEq)]
pub struct JsonCompare {
    /// Tolerance for numbers that are not both integers. Default [`Tolerance::METRIC`].
    pub tolerance: Tolerance,
    /// Compare every array as a multiset, ignoring order. Default `false`.
    pub unordered_arrays: bool,
}

impl Default for JsonCompare {
    fn default() -> Self {
        Self {
            tolerance: Tolerance::METRIC,
            unordered_arrays: false,
        }
    }
}

impl JsonCompare {
    /// Use `tolerance` for floats.
    pub fn tolerance(mut self, tolerance: Tolerance) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Compare arrays regardless of order.
    pub fn unordered(mut self) -> Self {
        self.unordered_arrays = true;
        self
    }
}

/// Compare two JSON values deeply.
///
/// Objects compare by key. Integers compare exactly; other numbers compare
/// within `opts.tolerance`. A non-finite marker string (see [`as_f64`]) equals
/// the matching float. With `opts.unordered_arrays`, arrays compare as
/// multisets.
pub fn compare_json(actual: &Value, expected: &Value, opts: &JsonCompare) -> Result<(), Mismatch> {
    let mut diffs = Vec::new();
    diff_json("$", actual, expected, opts, &mut diffs);
    check("JSON mismatch", diffs)
}

/// Assert two JSON values are equal. See [`compare_json`].
#[track_caller]
pub fn assert_json_eq(actual: &Value, expected: &Value, opts: &JsonCompare) {
    or_panic(compare_json(actual, expected, opts));
}

fn json_equal(actual: &Value, expected: &Value, opts: &JsonCompare) -> bool {
    let mut diffs = Vec::new();
    diff_json("$", actual, expected, opts, &mut diffs);
    diffs.is_empty()
}

fn diff_json(
    path: &str,
    actual: &Value,
    expected: &Value,
    opts: &JsonCompare,
    out: &mut Vec<String>,
) {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => {
            for (key, ev) in e {
                let child = format!("{path}.{key}");
                match a.get(key) {
                    Some(av) => diff_json(&child, av, ev, opts, out),
                    None => out.push(format!("  {child}:\n    - {ev}\n    + (missing)")),
                }
            }
            for (key, av) in a {
                if !e.contains_key(key) {
                    out.push(format!("  {path}.{key}:\n    - (missing)\n    + {av}"));
                }
            }
        }
        (Value::Array(a), Value::Array(e)) if opts.unordered_arrays => {
            let mut unmatched: Vec<&Value> = a.iter().collect();
            let mut missing = Vec::new();
            for ev in e {
                match unmatched.iter().position(|av| json_equal(av, ev, opts)) {
                    Some(i) => {
                        unmatched.swap_remove(i);
                    }
                    None => missing.push(ev),
                }
            }
            out.extend(missing.iter().map(|ev| format!("  {path}[]:\n    - {ev}")));
            out.extend(
                unmatched
                    .iter()
                    .map(|av| format!("  {path}[]:\n    + {av}")),
            );
        }
        (Value::Array(a), Value::Array(e)) => {
            if a.len() != e.len() {
                out.push(format!(
                    "  {path}: length\n    - {}\n    + {}",
                    e.len(),
                    a.len()
                ));
            }
            for (i, (av, ev)) in a.iter().zip(e).enumerate() {
                diff_json(&format!("{path}[{i}]"), av, ev, opts, out);
            }
            out.extend(
                e.iter()
                    .enumerate()
                    .skip(a.len())
                    .map(|(i, ev)| format!("  {path}[{i}]:\n    - {ev}\n    + (missing)")),
            );
            out.extend(
                a.iter()
                    .enumerate()
                    .skip(e.len())
                    .map(|(i, av)| format!("  {path}[{i}]:\n    - (missing)\n    + {av}")),
            );
        }
        (Value::Number(a), Value::Number(e)) if a.is_f64() || e.is_f64() => {
            float_or_diff(path, a.as_f64(), e.as_f64(), actual, expected, opts, out);
        }
        (Value::Number(_) | Value::String(_), Value::Number(_) | Value::String(_))
            if actual != expected
                && (actual.is_string() || expected.is_string())
                && as_f64(actual).is_some()
                && as_f64(expected).is_some() =>
        {
            float_or_diff(
                path,
                as_f64(actual),
                as_f64(expected),
                actual,
                expected,
                opts,
                out,
            );
        }
        _ if actual != expected => {
            out.push(format!("  {path}:\n    - {expected}\n    + {actual}"));
        }
        _ => {}
    }
}

fn float_or_diff(
    path: &str,
    a: Option<f64>,
    e: Option<f64>,
    actual: &Value,
    expected: &Value,
    opts: &JsonCompare,
    out: &mut Vec<String>,
) {
    match (a, e) {
        (Some(a), Some(e)) if opts.tolerance.accepts(a, e) => {}
        (Some(a), Some(e)) => out.push(float_line(path, a, e, opts.tolerance)),
        _ => out.push(format!("  {path}:\n    - {expected}\n    + {actual}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tolerance_presets() {
        assert!(Tolerance::EXACT.accepts(1.0, 1.0));
        assert!(!Tolerance::EXACT.accepts(1.0, 1.0 + f64::EPSILON));
        assert!(Tolerance::COUNT.accepts(3.0 + 1e-10, 3.0));
        assert!(!Tolerance::COUNT.accepts(3.0 + 1e-8, 3.0));
        assert!(Tolerance::METRIC.accepts(1000.0005, 1000.0));
        assert!(!Tolerance::METRIC.accepts(1000.01, 1000.0));
        assert!(Tolerance::METRIC.accepts(0.0, 1e-13));
        assert!(Tolerance::METRIC.accepts(f64::NAN, f64::NAN));
        assert!(!Tolerance::METRIC.accepts(f64::NAN, 0.0));
        assert!(Tolerance::METRIC.accepts(f64::INFINITY, f64::INFINITY));
        assert!(!Tolerance::METRIC.accepts(f64::INFINITY, f64::NEG_INFINITY));
    }

    #[test]
    fn close_message_names_both_sides() {
        let err = compare_close(0.75, 0.753, Tolerance::METRIC).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("- 0.753"), "{text}");
        assert!(text.contains("+ 0.75"), "{text}");
        assert!(text.contains("tolerance abs 1e-12, rel 1e-6"), "{text}");
    }

    #[test]
    fn map_ignores_order_and_reports_each_key() {
        let expected = [("a", 1), ("b", 2), ("c", 3)];
        assert!(compare_map_eq([("c", 3), ("a", 1), ("b", 2)], expected).is_ok());
        let err = compare_map_eq([("a", 1), ("b", 5), ("d", 4)], expected).unwrap_err();
        assert_eq!(
            err.differences,
            vec![
                "  \"b\":\n    - 2\n    + 5".to_owned(),
                "  - \"c\": 3".to_owned(),
                "  + \"d\": 4".to_owned(),
            ]
        );
    }

    #[test]
    fn map_close_uses_tolerance() {
        let expected = [("x", 0.5), ("y", 1.0)];
        assert!(
            compare_map_close([("y", 1.0 + 1e-9), ("x", 0.5)], expected, Tolerance::METRIC).is_ok()
        );
        assert!(compare_map_close([("y", 1.1), ("x", 0.5)], expected, Tolerance::METRIC).is_err());
    }

    #[test]
    fn multiset_counts_multiplicity() {
        assert!(compare_multiset_eq(["b", "a", "b"], ["b", "b", "a"]).is_ok());
        let err = compare_multiset_eq(["a", "a", "a", "c"], ["a", "b"]).unwrap_err();
        assert_eq!(
            err.differences,
            vec![
                "  + \"a\" (x2)".to_owned(),
                "  - \"b\"".to_owned(),
                "  + \"c\"".to_owned()
            ]
        );
    }

    #[test]
    #[should_panic(expected = "multiset mismatch: 1 difference")]
    fn assert_multiset_panics() {
        assert_multiset_eq([1], [2, 1]);
    }

    #[test]
    fn json_paths_and_tolerance() {
        let expected = json!({"n": 3, "f": 0.5, "xs": [1, 2], "o": {"k": "v"}, "nan": "NaN"});
        let ok = json!({"n": 3, "f": 0.5000000001, "xs": [1, 2], "o": {"k": "v"}, "nan": "NaN"});
        assert!(compare_json(&ok, &expected, &JsonCompare::default()).is_ok());

        let bad = json!({"n": 4, "f": 0.6, "xs": [1], "o": {"k": "w", "extra": true}, "nan": 1.0});
        let err = compare_json(&bad, &expected, &JsonCompare::default()).unwrap_err();
        let text = err.to_string();
        for needle in [
            "$.n:\n    - 3\n    + 4",
            "$.f:\n    - 0.5\n    + 0.6",
            "$.xs: length\n    - 2\n    + 1",
            "$.xs[1]:\n    - 2\n    + (missing)",
            "$.o.k:\n    - \"v\"\n    + \"w\"",
            "$.o.extra:\n    - (missing)\n    + true",
            "$.nan:\n    - NaN\n    + 1.0",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
        }
    }

    #[test]
    fn json_integers_are_exact() {
        let opts = JsonCompare::default().tolerance(Tolerance::absolute(10.0));
        assert!(compare_json(&json!(3), &json!(4), &opts).is_err());
        assert!(compare_json(&json!(3.0), &json!(4), &opts).is_ok());
    }

    #[test]
    fn json_unordered_arrays() {
        let expected = json!([{"a": 1}, {"a": 2}, {"a": 2}]);
        let opts = JsonCompare::default().unordered();
        assert!(compare_json(&json!([{"a": 2}, {"a": 1}, {"a": 2}]), &expected, &opts).is_ok());
        let err =
            compare_json(&json!([{"a": 2}, {"a": 1}, {"a": 3}]), &expected, &opts).unwrap_err();
        assert_eq!(err.differences.len(), 2);
        assert!(
            compare_json(
                &json!([{"a": 2}, {"a": 1}, {"a": 2}]),
                &expected,
                &JsonCompare::default()
            )
            .is_err()
        );
    }
}
