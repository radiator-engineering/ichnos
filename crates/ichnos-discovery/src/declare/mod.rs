//! Classic DECLARE discovery with typed templates, rules and count summaries.
//! Trace variants are evaluated once and weighted without constructing a
//! per-case rules DataFrame.

mod conformance;

pub use conformance::{DeclareDeviation, DeclareTraceConformance, conformance_declare};

use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::Label;
use std::collections::{BTreeMap, BTreeSet};

/// One of the eighteen templates implemented by the pinned classic miner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclareTemplate {
    /// At least one occurrence.
    Existence,
    /// Exactly one occurrence.
    ExactlyOne,
    /// The first projected activity.
    Init,
    /// An occurrence of the source requires an occurrence of the target.
    RespondedExistence,
    /// Every source occurrence has a later target occurrence.
    Response,
    /// Every target occurrence has an earlier source occurrence.
    Precedence,
    /// Minimum of response and precedence evaluations.
    Succession,
    /// A target occurs between each source and its next occurrence.
    AlternateResponse,
    /// A source occurs between each target and its previous occurrence.
    AlternatePrecedence,
    /// Minimum of alternate response and precedence evaluations.
    AlternateSuccession,
    /// Each source is immediately followed by the target.
    ChainResponse,
    /// Each target is immediately preceded by the source.
    ChainPrecedence,
    /// Minimum of chain response and precedence evaluations.
    ChainSuccession,
    /// No occurrence; generated only with existence enabled.
    Absence,
    /// Minimum of both responded-existence directions.
    Coexistence,
    /// Negation of the coexistence evaluation.
    NonCoexistence,
    /// Negation of the succession evaluation.
    NonSuccession,
    /// Negation of the chain-succession evaluation.
    NonChainSuccession,
}

impl DeclareTemplate {
    /// All supported templates, also the default allowed set.
    pub const ALL: [Self; 18] = [
        Self::Existence,
        Self::ExactlyOne,
        Self::Init,
        Self::RespondedExistence,
        Self::Response,
        Self::Precedence,
        Self::Succession,
        Self::AlternateResponse,
        Self::AlternatePrecedence,
        Self::AlternateSuccession,
        Self::ChainResponse,
        Self::ChainPrecedence,
        Self::ChainSuccession,
        Self::Absence,
        Self::Coexistence,
        Self::NonCoexistence,
        Self::NonSuccession,
        Self::NonChainSuccession,
    ];

    /// The pinned pm4py template name, for interchange and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Existence => "existence",
            Self::ExactlyOne => "exactly_one",
            Self::Init => "init",
            Self::RespondedExistence => "responded_existence",
            Self::Response => "response",
            Self::Precedence => "precedence",
            Self::Succession => "succession",
            Self::AlternateResponse => "altresponse",
            Self::AlternatePrecedence => "altprecedence",
            Self::AlternateSuccession => "altsuccession",
            Self::ChainResponse => "chainresponse",
            Self::ChainPrecedence => "chainprecedence",
            Self::ChainSuccession => "chainsuccession",
            Self::Absence => "absence",
            Self::Coexistence => "coexistence",
            Self::NonCoexistence => "noncoexistence",
            Self::NonSuccession => "nonsuccession",
            Self::NonChainSuccession => "nonchainsuccession",
        }
    }

    /// Whether this template has one activity instead of an ordered pair.
    pub const fn is_unary(self) -> bool {
        matches!(
            self,
            Self::Existence | Self::ExactlyOne | Self::Init | Self::Absence
        )
    }

    fn available(self, allowed: &BTreeSet<Self>) -> bool {
        use DeclareTemplate::*;
        let prerequisites: &[Self] = match self {
            Absence => &[Existence],
            Succession => &[Response, Precedence],
            AlternateSuccession => &[AlternateResponse, AlternatePrecedence],
            ChainSuccession => &[ChainResponse, ChainPrecedence],
            Coexistence => &[RespondedExistence],
            NonCoexistence => &[Coexistence, RespondedExistence],
            NonSuccession => &[Succession, Response, Precedence],
            NonChainSuccession => &[ChainSuccession, ChainResponse, ChainPrecedence],
            _ => &[],
        };
        allowed.contains(&self) && prerequisites.iter().all(|t| allowed.contains(t))
    }
}

/// Typed rule arguments. Binary activities remain an ordered pair, including
/// when an activity has an empty label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclareActivities {
    /// One activity for a unary template.
    Unary(Label),
    /// Ordered source and target for a binary template.
    Binary(Label, Label),
}

/// Per-case counts of a selected rule, rather than ratios.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeclareCounts {
    /// Number of cases with a nonzero evaluation (satisfied or violated).
    pub support: u64,
    /// Number of cases with a positive evaluation, a subset of support.
    pub confidence: u64,
}

/// Selected DECLARE constraints, grouped by template and typed arguments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclareModel {
    /// Only templates and rules surviving support/confidence selection.
    pub rules: BTreeMap<DeclareTemplate, BTreeMap<DeclareActivities, DeclareCounts>>,
}

/// Options for classic DECLARE discovery.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareOptions {
    /// Allowed templates (default all eighteen). Derived templates retain the
    /// pinned prerequisite behavior; prerequisites are not enabled implicitly.
    pub allowed_templates: BTreeSet<DeclareTemplate>,
    /// Activity universe; `None` uses the log's labels. Events outside an
    /// explicit set are removed before all template evaluations.
    pub considered_activities: Option<BTreeSet<Label>>,
    /// Minimum supported fraction of all cases. If both ratios are absent,
    /// automatic selection is used; if just one is absent it means zero.
    pub min_support_ratio: Option<f64>,
    /// Minimum satisfied fraction of supported cases.
    pub min_confidence_ratio: Option<f64>,
    /// Multiplier of the best rule's support/confidence ratios for automatic
    /// selection (default 0.8). All fractions must be finite and in `[0,1]`.
    pub auto_selection_multiplier: f64,
}

impl Default for DeclareOptions {
    fn default() -> Self {
        Self {
            allowed_templates: DeclareTemplate::ALL.into_iter().collect(),
            considered_activities: None,
            min_support_ratio: None,
            min_confidence_ratio: None,
            auto_selection_multiplier: 0.8,
        }
    }
}

fn signed(holds: bool) -> i8 {
    if holds { 1 } else { -1 }
}

struct TraceInfo {
    trace: Vec<Label>,
    positions: BTreeMap<Label, Vec<usize>>,
}

impl TraceInfo {
    fn new(trace: Vec<Label>) -> Self {
        let mut positions: BTreeMap<Label, Vec<usize>> = BTreeMap::new();
        for (i, a) in trace.iter().enumerate() {
            positions.entry(a.clone()).or_default().push(i);
        }
        Self { trace, positions }
    }
    fn evaluate(&self, template: DeclareTemplate, a: &Label, b: Option<&Label>) -> i8 {
        use DeclareTemplate::*;
        let ap = self.positions.get(a).map(Vec::as_slice).unwrap_or(&[]);
        if template.is_unary() {
            return signed(match template {
                Existence => !ap.is_empty(),
                ExactlyOne => ap.len() == 1,
                Init => self.trace.first() == Some(a),
                Absence => ap.is_empty(),
                _ => unreachable!(),
            });
        }
        let b = b.expect("binary template has a target");
        let bp = self.positions.get(b).map(Vec::as_slice).unwrap_or(&[]);
        let response = |kind: DeclareTemplate| {
            if ap.is_empty() {
                return 0;
            }
            signed(match kind {
                RespondedExistence => !bp.is_empty(),
                Response => bp.last().is_some_and(|last| ap.last().unwrap() < last),
                AlternateResponse => ap.iter().enumerate().all(|(i, &pos)| {
                    let next = ap.get(i + 1).copied().unwrap_or(self.trace.len());
                    let j = bp.partition_point(|&p| p <= pos);
                    bp.get(j).is_some_and(|&p| p < next)
                }),
                ChainResponse => ap.iter().all(|&i| self.trace.get(i + 1) == Some(b)),
                _ => unreachable!(),
            })
        };
        let precedence = |kind: DeclareTemplate| {
            if bp.is_empty() {
                return 0;
            }
            signed(match kind {
                Precedence => ap.first().is_some_and(|first| first < bp.first().unwrap()),
                AlternatePrecedence => bp.iter().enumerate().all(|(i, &pos)| {
                    let j = if i == 0 {
                        0
                    } else {
                        ap.partition_point(|&p| p <= bp[i - 1])
                    };
                    ap.get(j).is_some_and(|&p| p < pos)
                }),
                ChainPrecedence => bp
                    .iter()
                    .all(|&i| i > 0 && self.trace.get(i - 1) == Some(a)),
                _ => unreachable!(),
            })
        };
        let coexistence = || {
            response(RespondedExistence).min(if bp.is_empty() {
                0
            } else {
                signed(!ap.is_empty())
            })
        };
        match template {
            RespondedExistence | Response | AlternateResponse | ChainResponse => response(template),
            Precedence | AlternatePrecedence | ChainPrecedence => precedence(template),
            Succession => response(Response).min(precedence(Precedence)),
            AlternateSuccession => response(AlternateResponse).min(precedence(AlternatePrecedence)),
            ChainSuccession => response(ChainResponse).min(precedence(ChainPrecedence)),
            Coexistence => coexistence(),
            NonCoexistence => -coexistence(),
            NonSuccession => -(response(Response).min(precedence(Precedence))),
            NonChainSuccession => -(response(ChainResponse).min(precedence(ChainPrecedence))),
            _ => unreachable!(),
        }
    }
}

/// Discovers a classic DECLARE model, with support/confidence counts weighted
/// by case multiplicity. Vacuous binary evaluations are zero and do not count
/// towards support; unary violations count towards support. Automatic ties
/// use the source's descending template-name/activity tuple order.
pub fn declare(log: &EventLog, keys: &EventKeys, options: &DeclareOptions) -> Result<DeclareModel> {
    for (name, value) in [
        ("min_support_ratio", options.min_support_ratio),
        ("min_confidence_ratio", options.min_confidence_ratio),
        (
            "auto_selection_multiplier",
            Some(options.auto_selection_multiplier),
        ),
    ] {
        if let Some(value) = value
            && (!value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(Error::DeclareThreshold {
                option: name,
                value,
            });
        }
    }
    let sequences = log.activity_sequences(keys)?;
    let traces: Vec<Vec<Label>> = sequences
        .traces
        .iter()
        .map(|t| {
            t.iter()
                .map(|&a| Label::from(sequences.activities.name(a)))
                .collect()
        })
        .collect();
    let activities = options
        .considered_activities
        .clone()
        .unwrap_or_else(|| traces.iter().flatten().cloned().collect());
    let cases = traces.len() as u64;
    if cases == 0 || activities.is_empty() {
        return Ok(DeclareModel::default());
    }
    let mut variants = BTreeMap::<Vec<Label>, u64>::new();
    for trace in traces {
        *variants
            .entry(
                trace
                    .into_iter()
                    .filter(|a| activities.contains(a))
                    .collect(),
            )
            .or_default() += 1;
    }
    let variants: Vec<_> = variants
        .into_iter()
        .map(|(trace, n)| (TraceInfo::new(trace), n))
        .collect();
    let mut candidates = Vec::new();
    for &template in &options.allowed_templates {
        if !template.available(&options.allowed_templates) {
            continue;
        }
        for a in &activities {
            let targets: Vec<_> = if template.is_unary() {
                vec![None]
            } else {
                activities.iter().filter(|b| *b != a).map(Some).collect()
            };
            for b in targets {
                let mut counts = DeclareCounts::default();
                for (trace, weight) in &variants {
                    let value = trace.evaluate(template, a, b);
                    if value != 0 {
                        counts.support += weight;
                    }
                    if value == 1 {
                        counts.confidence += weight;
                    }
                }
                let arguments = match b {
                    Some(b) => DeclareActivities::Binary(a.clone(), b.clone()),
                    None => DeclareActivities::Unary(a.clone()),
                };
                candidates.push((template, arguments, counts));
            }
        }
    }
    let (support, confidence) =
        if options.min_support_ratio.is_none() && options.min_confidence_ratio.is_none() {
            let best = candidates.iter().filter(|(_, _, n)| n.support > 0).max_by(
                |(t, a, n), (u, b, m)| {
                    // The source multiplies these ratios rather than simplifying to
                    // confidence/cases; retain its floating-point rounding and tie order.
                    let score = |v: &DeclareCounts| {
                        (v.support as f64 / cases as f64) * (v.confidence as f64 / v.support as f64)
                    };
                    score(n)
                        .total_cmp(&score(m))
                        .then_with(|| t.as_str().cmp(u.as_str()))
                        .then_with(|| a.cmp(b))
                },
            );
            let Some((_, _, n)) = best else {
                return Ok(DeclareModel::default());
            };
            (
                n.support as f64 / cases as f64 * options.auto_selection_multiplier,
                n.confidence as f64 / n.support as f64 * options.auto_selection_multiplier,
            )
        } else {
            (
                options.min_support_ratio.unwrap_or(0.0),
                options.min_confidence_ratio.unwrap_or(0.0),
            )
        };
    let mut model = DeclareModel::default();
    for (template, arguments, counts) in candidates {
        if counts.support > 0
            && counts.support as f64 >= cases as f64 * support
            && counts.confidence as f64 >= counts.support as f64 * confidence
        {
            model
                .rules
                .entry(template)
                .or_default()
                .insert(arguments, counts);
        }
    }
    Ok(model)
}
