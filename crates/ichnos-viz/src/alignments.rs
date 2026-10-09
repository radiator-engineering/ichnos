//! Alignment tables, ported from pm4py's
//! `visualization/align_table/variants/classic.py`.

use ichnos_conformance::alignments::{LogAlignment, Move, TraceAlignment};
use ichnos_model::PetriNet;

use crate::dot::{Dot, title_label};

/// One move of an alignment, as the table shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentStep {
    /// The event and the model move together; the model's label.
    Sync(String),
    /// Only the log moves; the event's activity.
    Log(String),
    /// Only the model moves; the transition's label, `None` for a silent
    /// transition.
    Model(Option<String>),
}

/// One variant of a log and its alignment, a row of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantAlignment {
    /// The activities of the variant.
    pub activities: Vec<String>,
    /// The number of traces with this variant.
    pub count: usize,
    /// The alignment, or `None` when the search found none.
    pub steps: Option<Vec<AlignmentStep>>,
}

/// Options for [`alignments_dot`] and [`alignment_table_dot`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlignmentsDotOptions {
    /// A title above the table. `None` or an empty title draws none.
    pub graph_title: Option<String>,
}

/// The DOT text of an alignment table, as pm4py's `save_vis_alignments`
/// draws it, from the alignments of a log on a Petri net.
///
/// See [`alignment_table_dot`] for the layout.
pub fn alignments_dot(
    alignments: &LogAlignment<TraceAlignment>,
    net: &PetriNet,
    options: &AlignmentsDotOptions,
) -> String {
    let variants = &alignments.variants;
    let rows: Vec<VariantAlignment> = variants
        .iter()
        .zip(&alignments.alignments)
        .map(|(variant, alignment)| {
            let activities: Vec<String> = variants.names(variant).map(str::to_owned).collect();
            let steps = alignment
                .as_ref()
                .map(|a| a.moves.iter().map(|m| step(m, &activities, net)).collect());
            VariantAlignment {
                activities,
                count: variant.count(),
                steps,
            }
        })
        .collect();
    alignment_table_dot(&rows, options)
}

fn step(m: &Move, trace: &[String], net: &PetriNet) -> AlignmentStep {
    let (log, model) = m.labels(trace, net);
    match (log, model) {
        (Some(_), Some(model)) => AlignmentStep::Sync(model.unwrap_or("None").to_owned()),
        (Some(log), None) => AlignmentStep::Log(log.to_owned()),
        (None, model) => AlignmentStep::Model(model.flatten().map(str::to_owned)),
    }
}

/// The DOT text of an alignment table, as pm4py's `save_vis_alignments`
/// draws it.
///
/// A plain-text node holds an HTML table with one row per variant, the
/// most frequent first and ties in activity order. Each row shows the
/// variant's number and count, then its moves: sync moves in green with the
/// model's label, log moves in orange marked `(LM)`, and model moves in
/// violet marked `(MM)`, where a silent transition shows `None`. Labels
/// escape only `>`, as in pm4py. A variant without an alignment gets an
/// empty row; pm4py fails on it.
pub fn alignment_table_dot(rows: &[VariantAlignment], options: &AlignmentsDotOptions) -> String {
    let mut sorted: Vec<&VariantAlignment> = rows.iter().collect();
    sorted.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.activities.cmp(&b.activities))
    });
    let esc = |s: &str| s.replace('>', "&gt;");
    let mut label = String::from(
        "<\n<table border='0' cellborder='1' color='blue' cellspacing='0'>\n\
         <tr><td>Variant</td><td>Alignment</td></tr>\n",
    );
    for (index, row) in sorted.iter().enumerate() {
        label.push_str(&format!(
            "<tr><td><font point-size='9'>Variant {} ({} occurrences)</font></td>\
             <td><font point-size='6'><table border='0'><tr>",
            index + 1,
            row.count
        ));
        for step in row.steps.iter().flatten() {
            label.push_str(&match step {
                AlignmentStep::Sync(model) => {
                    format!("<td bgcolor=\"lightgreen\">{}</td>", esc(model))
                }
                AlignmentStep::Log(log) => {
                    format!("<td bgcolor=\"orange\"><b>(LM)</b>{}</td>", esc(log))
                }
                AlignmentStep::Model(model) => format!(
                    "<td bgcolor=\"violet\"><b>(MM)</b>{}</td>",
                    esc(model.as_deref().unwrap_or("None"))
                ),
            });
        }
        label.push_str("</tr></table></font></td></tr>");
    }
    label.push_str("</table>\n>");

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
