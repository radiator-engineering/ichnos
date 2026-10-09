//! Dotted charts, ported from pm4py's
//! `visualization/dotted_chart/variants/classic.py`.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use chrono::{DateTime, FixedOffset};
use ichnos_core::{AttributeValue, EventLog};

use crate::ocdfg::object_type_color;
use crate::style::{py_datetime, py_float};

/// One attribute value of a point in a chart.
#[derive(Debug, Clone, PartialEq)]
pub enum ChartValue {
    /// A timestamp.
    Date(DateTime<FixedOffset>),
    /// An integer.
    Int(i64),
    /// A float.
    Float(f64),
    /// A Boolean, a number as in Python.
    Bool(bool),
    /// Text.
    Text(String),
}

impl ChartValue {
    /// The chart value of an attribute, or `None` for a list, container
    /// or meta attribute.
    pub fn from_attribute(value: &AttributeValue) -> Option<ChartValue> {
        Some(match value {
            AttributeValue::String(s) | AttributeValue::Id(s) => ChartValue::Text(s.to_string()),
            AttributeValue::Int(i) => ChartValue::Int(*i),
            AttributeValue::Float(f) => ChartValue::Float(*f),
            AttributeValue::Bool(b) => ChartValue::Bool(*b),
            AttributeValue::Date(d) => ChartValue::Date(*d),
            _ => return None,
        })
    }

    /// Python's `str` of the value.
    pub(crate) fn py_str(&self) -> String {
        match self {
            ChartValue::Date(d) => py_datetime(d),
            ChartValue::Int(i) => i.to_string(),
            ChartValue::Float(f) => py_float(*f),
            ChartValue::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
            ChartValue::Text(s) => s.clone(),
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            ChartValue::Int(i) => Some(*i as f64),
            ChartValue::Float(f) => Some(*f),
            ChartValue::Bool(b) => Some(f64::from(u8::from(*b))),
            _ => None,
        }
    }
}

/// The attributes a dotted chart draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DottedChartAttributes {
    /// The attribute along the x axis.
    pub x: String,
    /// The attribute along the y axis.
    pub y: String,
    /// The attribute that colours the dots, if any.
    pub color: Option<String>,
}

impl Default for DottedChartAttributes {
    /// pm4py's default for an event log: time, the position of the case in
    /// the log (`case:@@index`), and the activity.
    fn default() -> Self {
        DottedChartAttributes {
            x: "time:timestamp".to_owned(),
            y: "case:@@index".to_owned(),
            color: Some("concept:name".to_owned()),
        }
    }
}

/// One dot of a dotted chart.
#[derive(Debug, Clone, PartialEq)]
pub struct DottedChartPoint {
    /// The x value.
    pub x: ChartValue,
    /// The y value.
    pub y: ChartValue,
    /// The value that sets the colour, when the chart has a colour
    /// attribute.
    pub color: Option<ChartValue>,
}

/// The points of a dotted chart of `log`: one per event that has all the
/// attributes, in log order.
///
/// A name that starts with `case:` reads that attribute of the trace, and
/// `case:@@index` is the position of the trace in the log, as in pm4py's
/// event stream. pm4py fails on an event without one of the attributes;
/// this leaves the event out.
pub fn dotted_chart_points(
    log: &EventLog,
    attributes: &DottedChartAttributes,
) -> Vec<DottedChartPoint> {
    let mut points = Vec::new();
    for (index, trace) in log.traces.iter().enumerate() {
        for event in &trace.events {
            let value = |key: &str| -> Option<ChartValue> {
                if key == "case:@@index" {
                    return Some(ChartValue::Int(index as i64));
                }
                let attribute = match key.strip_prefix("case:") {
                    Some(name) => trace.attributes.get(name),
                    None => event.get(key),
                };
                attribute.and_then(ChartValue::from_attribute)
            };
            let (Some(x), Some(y)) = (value(&attributes.x), value(&attributes.y)) else {
                continue;
            };
            let color = match &attributes.color {
                Some(key) => match value(key) {
                    Some(c) => Some(c),
                    None => continue,
                },
                None => None,
            };
            points.push(DottedChartPoint { x, y, color });
        }
    }
    points
}

/// Options for [`dotted_chart_dot`], with pm4py's defaults.
#[derive(Debug, Clone)]
pub struct DottedChartDotOptions {
    /// The size of a dot, in points. Default 0.07.
    pub dot_size: f64,
    /// The scale of the layout. Default 50.
    pub layout_ext_multiplier: f64,
    /// Draw a legend of the colours. Default `true`.
    pub show_legend: bool,
    /// A title above the chart. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// Colours by the Python `str` of a colour value. Other values get the
    /// first three bytes of the MD5 digest of that string, as `#RRGGBB`;
    /// pm4py picks them at random.
    pub colors: BTreeMap<String, String>,
}

impl Default for DottedChartDotOptions {
    fn default() -> Self {
        DottedChartDotOptions {
            dot_size: 0.07,
            layout_ext_multiplier: 50.0,
            show_legend: true,
            graph_title: None,
            colors: BTreeMap::new(),
        }
    }
}

/// How an axis places its values: each distinct value and its position
/// from 0 to 1.
enum Axis {
    /// Dates and numbers, spread by value.
    Scaled(Vec<(ChartValue, f64)>),
    /// Text, spread evenly in sorted order.
    Text(Vec<(String, f64)>),
}

impl Axis {
    fn new<'a>(values: impl Iterator<Item = &'a ChartValue>) -> Axis {
        let values: Vec<&ChartValue> = values.collect();
        if !values.is_empty() && values.iter().all(|v| matches!(v, ChartValue::Date(_))) {
            let mut stamps: Vec<(i64, &ChartValue)> = values
                .iter()
                .map(|v| match v {
                    ChartValue::Date(d) => (d.timestamp_micros(), *v),
                    _ => unreachable!("all dates"),
                })
                .collect();
            stamps.sort_by_key(|(t, _)| *t);
            stamps.dedup_by_key(|(t, _)| *t);
            // Python's `datetime.timestamp()`.
            let seconds = |micros: i64| micros as f64 / 1e6;
            let (lo, hi) = (seconds(stamps[0].0), seconds(stamps[stamps.len() - 1].0));
            let n = stamps.len() as f64;
            let place = |t: f64| 1.0 / n + (n - 1.0) / n * (t - lo) / (hi - lo + 0.00001);
            return Axis::Scaled(
                stamps
                    .into_iter()
                    .map(|(t, v)| (v.clone(), place(seconds(t))))
                    .collect(),
            );
        }
        if !values.is_empty() && values.iter().all(|v| v.number().is_some()) {
            let mut numbers: Vec<&ChartValue> = values;
            let key = |v: &&ChartValue| v.number().expect("all numbers");
            numbers.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap_or(Ordering::Equal));
            numbers.dedup_by(|a, b| key(a) == key(b));
            let ints = numbers
                .iter()
                .all(|v| matches!(v, ChartValue::Int(_) | ChartValue::Bool(_)));
            let int = |v: &ChartValue| match v {
                ChartValue::Int(i) => i128::from(*i),
                ChartValue::Bool(b) => i128::from(*b),
                _ => 0,
            };
            let (first, last) = (numbers[0], numbers[numbers.len() - 1]);
            let n = numbers.len() as f64;
            // Python subtracts integers exactly, then converts.
            let offset = |v: &ChartValue, from: &ChartValue| {
                if ints {
                    (int(v) - int(from)) as f64
                } else {
                    key(&v) - key(&from)
                }
            };
            let span = offset(last, first);
            return Axis::Scaled(
                numbers
                    .iter()
                    .map(|v| {
                        let place = 1.0 / n + (n - 1.0) / n * offset(v, first) / (span + 0.00001);
                        ((*v).clone(), place)
                    })
                    .collect(),
            );
        }
        let mut texts: Vec<String> = values.iter().map(|v| v.py_str()).collect();
        texts.sort();
        texts.dedup();
        let n = texts.len() as f64;
        Axis::Text(
            texts
                .into_iter()
                .enumerate()
                .map(|(i, t)| (t, (i + 1) as f64 / (n + 1.0)))
                .collect(),
        )
    }

    fn is_text(&self) -> bool {
        matches!(self, Axis::Text(_))
    }

    fn len(&self) -> usize {
        match self {
            Axis::Scaled(v) => v.len(),
            Axis::Text(v) => v.len(),
        }
    }

    fn place(&self, value: &ChartValue) -> f64 {
        match self {
            Axis::Scaled(places) => places
                .iter()
                .find(|(v, _)| match (v, value) {
                    (ChartValue::Date(a), ChartValue::Date(b)) => a == b,
                    _ => v.number() == value.number(),
                })
                .map_or(0.0, |(_, p)| *p),
            Axis::Text(places) => {
                let text = value.py_str();
                places
                    .binary_search_by(|(t, _)| t.as_str().cmp(&text))
                    .map_or(0.0, |i| places[i].1)
            }
        }
    }
}

/// Python's `%d` of a float: truncation.
fn py_d(x: f64) -> i64 {
    x.trunc() as i64
}

/// The DOT text of a dotted chart, as pm4py's `save_vis_dotted_chart`
/// writes it before running `neato -n1` on it.
///
/// Each point is a dot at its x and y value, coloured by its colour value
/// or blue. Dates and numbers are spread by value; text is spread evenly in
/// sorted order, with each value written along its axis. With a colour
/// attribute and `show_legend`, a legend lists the colours. The node
/// positions are fixed, so render the text with `neato -n1`, as
/// [`write_dotted_chart`](crate::write_dotted_chart) does.
///
/// Labels are written as they are, without escaping, as in pm4py. pm4py
/// fails on a chart without points; this draws the axes alone.
pub fn dotted_chart_dot(
    points: &[DottedChartPoint],
    attributes: &DottedChartAttributes,
    options: &DottedChartDotOptions,
) -> String {
    let mult = options.layout_ext_multiplier;
    let x_axis = Axis::new(points.iter().map(|p| &p.x));
    let y_axis = Axis::new(points.iter().map(|p| &p.y));
    let colors: Option<Vec<(String, String)>> = attributes.color.as_ref().map(|_| {
        let values: Vec<&ChartValue> = points.iter().filter_map(|p| p.color.as_ref()).collect();
        let keys: Vec<String> = match Axis::new(values.into_iter()) {
            Axis::Scaled(v) => v.into_iter().map(|(v, _)| v.py_str()).collect(),
            Axis::Text(v) => v.into_iter().map(|(t, _)| t).collect(),
        };
        keys.into_iter()
            .map(|k| {
                let c = options
                    .colors
                    .get(&k)
                    .cloned()
                    .unwrap_or_else(|| object_type_color(&k));
                (k, c)
            })
            .collect()
    });
    let mut x_length = 10.0_f64;
    let mut y_length = 10.0_f64;
    if x_axis.is_text() {
        x_length = x_length.max(x_axis.len() as f64 * 1.8);
    }
    if y_axis.is_text() {
        y_length = y_length.max(y_axis.len() as f64 * 0.75);
    }
    x_length *= mult;
    y_length *= mult;

    let mut lines: Vec<String> = vec!["graph G {".to_owned()];
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        lines.push(format!(
            "label=<<FONT POINT-SIZE=\"20\">{title}</FONT>>;\nlabelloc=\"top\";\n"
        ));
    }
    let fixed = |name: &str, label: &str, pos: String| {
        format!(
            "{name} [label=\"{label}\", shape=none, width=\"0px\", height=\"0px\", pos=\"{pos}!\"];"
        )
    };
    lines.push(fixed("origin", "", "0,0".to_owned()));
    lines.push(fixed("rightX", "", format!("{},0", py_d(x_length))));
    lines.push(fixed("topY", "", format!("0,{}", py_d(y_length))));
    lines.push(fixed(
        "rightXlabel",
        &attributes.x,
        format!("{},0", py_d(x_length + 1.5)),
    ));
    lines.push(fixed(
        "topYlabel",
        &attributes.y,
        format!("0,{}", py_d(y_length + 1.0)),
    ));
    lines.push("origin -- rightX [ color=\"black\" ];".to_owned());
    lines.push("origin -- topY [ color=\"black\" ];".to_owned());
    let mut next = 0;
    let mut id = || {
        next += 1;
        format!("n{next}e")
    };
    if let Axis::Text(places) = &x_axis {
        for (k, v) in places {
            lines.push(format!(
                "{} [label=\"{k}\", shape=none, width=\"0px\", height=\"0px\", pos=\"{:.10},0!\", fontsize=\"6pt\"];",
                id(),
                v * x_length
            ));
        }
    }
    if let Axis::Text(places) = &y_axis {
        for (k, v) in places {
            lines.push(format!(
                "{} [label=\"{k}\", shape=none, width=\"0px\", height=\"0px\", pos=\"0,{:.10}!\", fontsize=\"6pt\"];",
                id(),
                v * y_length
            ));
        }
    }
    let size = options.dot_size;
    for p in points {
        let color = match (&colors, &p.color) {
            (Some(colors), Some(c)) => {
                let key = c.py_str();
                colors
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or("blue", |(_, c)| c.as_str())
                    .to_owned()
            }
            _ => "blue".to_owned(),
        };
        lines.push(format!(
            "{} [label=\"\", shape=circle,  width=\"{size:.10}px\", height=\"{size:.10}px\", pos=\"{:.10},{:.10}!\", fontsize=\"6pt\", style=\"filled\", fillcolor=\"{color}\", penwidth=0];",
            id(),
            x_axis.place(&p.x) * x_length,
            y_axis.place(&p.y) * y_length,
        ));
    }
    if let (Some(colors), Some(attribute), true) = (&colors, &attributes.color, options.show_legend)
    {
        lines.push(format!(
            "Legend [label=\"legend (attribute: {attribute})\", shape=none, width=\"0px\", height=\"0px\", pos=\"0,-{}!\"]",
            py_d(mult)
        ));
        let mut row = -1.0;
        for (k, c) in colors {
            row -= 1.0;
            let mut line = String::new();
            let _ = write!(
                line,
                "{} [label=\"\", shape=circle, width=\"{size:.10}px\", height=\"{size:.10}px\", fontsize=\"6pt\", style=\"filled\", fillcolor=\"{c}\", pos=\"0,{}!\"]",
                id(),
                py_d(mult * row)
            );
            lines.push(line);
            lines.push(format!(
                "{} [label=\"{k}\", shape=none, width=\"0px\", height=\"0px\", pos=\"1.5,{}!\", fontsize=\"9pt\"];",
                id(),
                py_d(mult * row)
            ));
        }
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_axes_spread_values_evenly() {
        let p = |x: &str, y: i64| DottedChartPoint {
            x: ChartValue::Text(x.to_owned()),
            y: ChartValue::Int(y),
            color: None,
        };
        let attributes = DottedChartAttributes {
            x: "concept:name".to_owned(),
            y: "n".to_owned(),
            color: None,
        };
        let dot = dotted_chart_dot(
            &[p("b", 1), p("a", 3)],
            &attributes,
            &DottedChartDotOptions::default(),
        );
        // Two names: x_length is max(10, 2 * 1.8) * 50 = 500.
        assert!(dot.contains("pos=\"500,0!\""), "{dot}");
        assert!(dot.contains("[label=\"a\", shape=none, width=\"0px\", height=\"0px\", pos=\"166.6666666667,0!\", fontsize=\"6pt\"];"), "{dot}");
        // y = 1: 1/2 + 1/2 * 0 / (2 + 1e-5) of 500.
        assert!(
            dot.contains("pos=\"333.3333333333,250.0000000000!\""),
            "{dot}"
        );
        assert!(dot.contains("fillcolor=\"blue\""), "{dot}");
        assert!(!dot.contains("Legend"), "{dot}");
    }
}
