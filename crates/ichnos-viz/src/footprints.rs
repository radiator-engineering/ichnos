//! Footprint tables, ported from pm4py's `visualization/footprints`
//! (`single` and `comparison_symmetric` variants).

use std::collections::BTreeSet;

use ichnos_model::Footprints;

use crate::dot::{Dot, title_label};

/// Options for [`footprints_dot`] and [`footprints_comparison_dot`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FootprintsDotOptions {
    /// A title above the table. `None` or an empty title draws none.
    pub graph_title: Option<String>,
}

const XOR: &str = "&#35;";
const PREV: &str = "&#60;";
const SEQUENCE: &str = "&#62;";
const PARALLEL: &str = "||";
const UNKNOWN: &str = "?";

/// The activities pm4py's footprint tables list: those in a sequence or
/// parallel pair.
fn activities(fp: &Footprints) -> BTreeSet<&str> {
    fp.sequence
        .iter()
        .chain(&fp.parallel)
        .flat_map(|(a, b)| [a.as_str(), b.as_str()])
        .collect()
}

/// The table's HTML label: a header row of activities, then one row per
/// activity with `cell(row, column)` in each column.
fn table(activities: &BTreeSet<&str>, cell: impl Fn(&str, &str) -> String) -> String {
    let mut out = String::from(
        "<\n<table border='0' cellborder='1' color='blue' cellspacing='0'>\n<tr><td></td>",
    );
    for act in activities {
        out.push_str(&format!("<td><b>{act}</b></td>"));
    }
    out.push_str("</tr>\n");
    for a1 in activities {
        out.push_str(&format!("<tr><td><b>{a1}</b></td>"));
        for a2 in activities {
            out.push_str(&cell(a1, a2));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n>");
    out
}

fn graph(label: String, options: &FootprintsDotOptions) -> String {
    let mut dot = Dot::new(true, false, "", vec![]);
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        // pm4py draws this title at 20 points.
        dot.set(vec![
            ("label", Some(title_label(title, 10))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }
    dot.node(
        "tbl",
        Some(&label),
        vec![("shape", Some("plaintext".to_owned()))],
    );
    dot.finish()
}

/// The DOT text of a footprint table, as pm4py's `save_vis_footprints`
/// draws one footprint.
///
/// Rows and columns list the activities of the sequence and parallel
/// pairs. A cell shows `>` when the row activity is followed by the column
/// activity, `<` for the reverse, `||` when they are parallel and `#`
/// otherwise. Activity names go into the HTML label unescaped, as in
/// pm4py.
///
/// When both orders of a pair are in `sequence` and neither is in
/// `parallel`, both cells show `>`; pm4py's cell then depends on Python's
/// set order. pm4py's own footprints never hold such a pair.
pub fn footprints_dot(fp: &Footprints, options: &FootprintsDotOptions) -> String {
    let symbol = |a1: &str, a2: &str| {
        let pair = |a: &str, b: &str| (a.into(), b.into());
        if fp.parallel.contains(&pair(a1, a2)) {
            PARALLEL
        } else if fp.sequence.contains(&pair(a1, a2)) && !fp.parallel.contains(&pair(a1, a2)) {
            SEQUENCE
        } else if fp.sequence.contains(&pair(a2, a1)) && !fp.parallel.contains(&pair(a2, a1)) {
            PREV
        } else {
            XOR
        }
    };
    let label = table(&activities(fp), |a1, a2| {
        format!("<td>{}</td>", symbol(a1, a2))
    });
    graph(label, options)
}

/// The DOT text comparing two footprints, as pm4py's `save_vis_footprints`
/// draws a pair (the `comparison_symmetric` variant).
///
/// Rows and columns list the activities of both. A cell where the two
/// agree shows the symbol in black; otherwise it shows both symbols in
/// red, with `?` for an activity the footprint does not have.
pub fn footprints_comparison_dot(
    first: &Footprints,
    second: &Footprints,
    options: &FootprintsDotOptions,
) -> String {
    let (acts1, acts2) = (activities(first), activities(second));
    let all: BTreeSet<&str> = acts1.union(&acts2).copied().collect();
    let symbol = |fp: &Footprints, acts: &BTreeSet<&str>, a1: &str, a2: &str| {
        if !(acts.contains(a1) && acts.contains(a2)) {
            return UNKNOWN;
        }
        let pair = |a: &str, b: &str| (a.into(), b.into());
        if fp.parallel.contains(&pair(a1, a2)) {
            PARALLEL
        } else if fp.sequence.contains(&pair(a1, a2)) {
            SEQUENCE
        } else if fp.sequence.contains(&pair(a2, a1)) {
            PREV
        } else {
            XOR
        }
    };
    let label = table(&all, |a1, a2| {
        let (s1, s2) = (
            symbol(first, &acts1, a1, a2),
            symbol(second, &acts2, a1, a2),
        );
        if s1 == s2 {
            format!("<td><font color=\"black\">{s1}</font></td>")
        } else {
            format!("<td><font color=\"red\">{s1}&nbsp;&nbsp;{s2}</font></td>")
        }
    });
    graph(label, options)
}
