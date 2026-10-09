//! DECLARE conformance checking, ported from pm4py's
//! `algo/conformance/declare/variants/classic.py`.

use super::{DeclareActivities, DeclareModel, DeclareTemplate};
use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::Label;
use std::collections::BTreeMap;

/// A rule of the model that a trace violates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclareDeviation {
    /// The template of the violated rule.
    pub template: DeclareTemplate,
    /// The activities of the violated rule.
    pub activities: DeclareActivities,
}

/// The DECLARE conformance of one trace, with pm4py's field names.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareTraceConformance {
    /// The violated rules, template by template in pm4py's check order, and
    /// in activity order within a template.
    pub deviations: Vec<DeclareDeviation>,
    /// The number of violated rules.
    pub no_dev_total: usize,
    /// The number of rules in the model.
    pub no_constr_total: usize,
    /// One minus the share of violated rules; 1 for a model without rules.
    pub dev_fitness: f64,
    /// Whether the trace violates no rule.
    pub is_fit: bool,
}

/// The order in which pm4py checks the templates.
const CHECK_ORDER: [DeclareTemplate; 18] = {
    use DeclareTemplate::*;
    [
        Existence,
        ExactlyOne,
        Init,
        RespondedExistence,
        Coexistence,
        NonCoexistence,
        Response,
        Precedence,
        Succession,
        AlternateResponse,
        ChainResponse,
        AlternatePrecedence,
        ChainPrecedence,
        AlternateSuccession,
        ChainSuccession,
        Absence,
        NonSuccession,
        NonChainSuccession,
    ]
};

/// Checks each trace of the log against a DECLARE model, as pm4py's
/// `conformance_declare` does.
///
/// Returns one result per trace, in log order. Rule counts in the model are
/// not used.
///
/// # Errors
///
/// An event without the activity attribute, or a rule whose activities do
/// not match its template: one activity for `existence`, `exactly_one`,
/// `init` and `absence`, a pair for the others.
pub fn conformance_declare(
    log: &EventLog,
    model: &DeclareModel,
    keys: &EventKeys,
) -> Result<Vec<DeclareTraceConformance>> {
    for (template, rules) in &model.rules {
        if rules
            .keys()
            .any(|args| template.is_unary() != matches!(args, DeclareActivities::Unary(_)))
        {
            return Err(Error::InvalidOption(
                "a DECLARE rule has the wrong number of activities for its template",
            ));
        }
    }
    let no_constr_total: usize = model.rules.values().map(BTreeMap::len).sum();
    let sequences = log.activity_sequences(keys)?;
    Ok(sequences
        .traces
        .iter()
        .map(|t| {
            let trace: Vec<Label> = t
                .iter()
                .map(|&a| Label::from(sequences.activities.name(a)))
                .collect();
            let checker = Checker::new(&trace);
            let mut deviations = Vec::new();
            for template in CHECK_ORDER {
                for args in model
                    .rules
                    .get(&template)
                    .into_iter()
                    .flat_map(|r| r.keys())
                {
                    if checker.violates(template, args) {
                        deviations.push(DeclareDeviation {
                            template,
                            activities: args.clone(),
                        });
                    }
                }
            }
            let no_dev_total = deviations.len();
            DeclareTraceConformance {
                deviations,
                no_dev_total,
                no_constr_total,
                dev_fitness: if no_constr_total > 0 {
                    1.0 - no_dev_total as f64 / no_constr_total as f64
                } else {
                    1.0
                },
                is_fit: no_dev_total == 0,
            }
        })
        .collect())
}

/// One trace and the positions of each of its activities.
struct Checker<'a> {
    trace: &'a [Label],
    positions: BTreeMap<&'a str, Vec<usize>>,
}

impl<'a> Checker<'a> {
    fn new(trace: &'a [Label]) -> Self {
        let mut positions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, a) in trace.iter().enumerate() {
            positions.entry(a.as_str()).or_default().push(i);
        }
        Checker { trace, positions }
    }

    fn at(&self, a: &str) -> &[usize] {
        self.positions.get(a).map_or(&[], Vec::as_slice)
    }

    fn has(&self, a: &str) -> bool {
        self.positions.contains_key(a)
    }

    fn violates(&self, template: DeclareTemplate, args: &DeclareActivities) -> bool {
        use DeclareTemplate::*;
        let (a, b) = match args {
            DeclareActivities::Unary(a) => (a.as_str(), ""),
            DeclareActivities::Binary(a, b) => (a.as_str(), b.as_str()),
        };
        let first = |x: &str| self.at(x).first().copied().unwrap_or(0);
        let last = |x: &str| self.at(x).last().copied().unwrap_or(0);
        match template {
            Existence => !self.has(a),
            Absence => self.has(a),
            ExactlyOne => self.at(a).len() != 1,
            Init => self.trace.first().is_none_or(|x| x != a),
            RespondedExistence => self.has(a) && !self.has(b),
            Coexistence => self.has(a) != self.has(b),
            NonCoexistence => self.has(a) && self.has(b),
            Response => self.has(a) && (!self.has(b) || last(a) > last(b)),
            Precedence => self.has(b) && (!self.has(a) || first(a) > first(b)),
            Succession => match (self.has(a), self.has(b)) {
                (false, false) => false,
                (true, true) => first(a) > first(b) || last(a) > last(b),
                _ => true,
            },
            AlternateResponse => !self.alt_response(a, b),
            ChainResponse => !self.chain_response(a, b),
            AlternatePrecedence => !self.alt_precedence(a, b),
            ChainPrecedence => !self.chain_precedence(a, b),
            AlternateSuccession => !(self.alt_response(a, b) && self.alt_precedence(a, b)),
            ChainSuccession => !(self.chain_response(a, b) && self.chain_precedence(a, b)),
            NonSuccession => self.has(a) && self.has(b) && first(a) < first(b) && last(a) < last(b),
            NonChainSuccession => {
                self.has(a)
                    && self.has(b)
                    && self.chain_response(a, b)
                    && self.chain_precedence(a, b)
            }
        }
    }

    /// Each `a` is followed by a `b` before the next `a`.
    fn alt_response(&self, a: &str, b: &str) -> bool {
        let (ais, bis) = (self.at(a), self.at(b));
        if ais.is_empty() {
            return true;
        }
        let mut k = 0;
        for (i, &x) in ais.iter().enumerate() {
            let next = ais.get(i + 1).copied().unwrap_or(self.trace.len());
            while k < bis.len() && bis[k] <= x {
                k += 1;
            }
            if k >= bis.len() || bis[k] >= next {
                return false;
            }
        }
        true
    }

    /// Each `b` is preceded by an `a` after the previous `b`.
    fn alt_precedence(&self, a: &str, b: &str) -> bool {
        let (ais, bis) = (self.at(a), self.at(b));
        if bis.is_empty() {
            return true;
        }
        let mut k = 0;
        let mut previous = None;
        for &y in bis {
            while k < ais.len() && previous.is_some_and(|p| ais[k] <= p) {
                k += 1;
            }
            if k >= ais.len() || ais[k] >= y {
                return false;
            }
            previous = Some(y);
        }
        true
    }

    /// Each `a` is directly followed by `b`.
    fn chain_response(&self, a: &str, b: &str) -> bool {
        self.at(a)
            .iter()
            .all(|&x| self.trace.get(x + 1).is_some_and(|n| n == b))
    }

    /// Each `b` is directly preceded by `a`.
    fn chain_precedence(&self, a: &str, b: &str) -> bool {
        self.at(b).iter().all(|&y| y > 0 && self.trace[y - 1] == a)
    }
}
