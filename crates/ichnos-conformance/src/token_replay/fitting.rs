//! Whether one trace fits a model (pm4py's `check_is_fitting`).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use ichnos_model::{Marking, PetriNet, ProcessTree};

use super::{TokenReplayOptions, TokenReplayer};
use crate::alignments::{Aligner, AlignmentOptions, TreeAligner};
use crate::error::Result;
use crate::footprints::{FootprintsDeviations, LogFootprints, ModelFootprints};

/// Whether `trace`, given as its activities, is a run of the accepting net
/// (pm4py's `check_is_fitting` on a Petri net).
///
/// This prepares a [`FittingChecker`] for one trace. To check many traces
/// against one net, build the checker once and call
/// [`FittingChecker::check`] for each.
pub fn check_is_fitting<S: AsRef<str>>(
    trace: &[S],
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
) -> Result<bool> {
    FittingChecker::new(net, initial_marking, final_marking)?.check(trace)
}

/// Checks whether traces are runs of one accepting net (pm4py's
/// `check_is_fitting` on a Petri net), preparing the net once.
///
/// As pm4py does, [`FittingChecker::check`] tries the cheap checks first.
/// A trace with an activity no transition carries does not fit. A trace
/// that token replay finds fit does. Otherwise the trace fits when its
/// optimal alignment has fitness 1. The aligner is built the first time a
/// trace needs it.
///
/// pm4py first tries to convert the net to a process tree and, when that
/// works, takes the tree path ([`check_is_fitting_tree`]). Both paths decide
/// whether the trace is in the model's language, so this always takes the
/// net path.
#[derive(Debug)]
pub struct FittingChecker<'a> {
    net: &'a PetriNet,
    initial_marking: &'a Marking,
    final_marking: &'a Marking,
    labels: BTreeSet<&'a str>,
    replayer: TokenReplayer,
    aligner: OnceLock<Aligner>,
}

impl<'a> FittingChecker<'a> {
    /// Prepares token replay on the accepting net.
    pub fn new(
        net: &'a PetriNet,
        initial_marking: &'a Marking,
        final_marking: &'a Marking,
    ) -> Result<Self> {
        let labels = net
            .transitions()
            .filter_map(|(_, t)| t.label.as_deref())
            .collect();
        let options = TokenReplayOptions::default();
        let replayer = TokenReplayer::new(net, initial_marking, final_marking, options)?;
        Ok(Self {
            net,
            initial_marking,
            final_marking,
            labels,
            replayer,
            aligner: OnceLock::new(),
        })
    }

    /// Whether `trace`, given as its activities, is a run of the net.
    pub fn check<S: AsRef<str>>(&self, trace: &[S]) -> Result<bool> {
        if trace.iter().any(|a| !self.labels.contains(a.as_ref())) {
            return Ok(false);
        }
        if self.replayer.replay(trace)?.is_fit {
            return Ok(true);
        }
        let aligner = match self.aligner.get() {
            Some(a) => a,
            None => {
                let a = Aligner::new(
                    self.net,
                    self.initial_marking,
                    self.final_marking,
                    AlignmentOptions::default(),
                )?;
                self.aligner.get_or_init(|| a)
            }
        };
        Ok(aligner.align(trace)?.is_some_and(|a| a.is_fit()))
    }
}

/// Whether `trace`, given as its activities, is a trace of `tree` (pm4py's
/// `check_is_fitting` on a process tree).
///
/// A trace with footprint deviations does not fit. A trace that token
/// replay finds fit on the tree's Petri net does. Otherwise the trace fits
/// when its optimal alignment against the tree has fitness 1.
pub fn check_is_fitting_tree<S: AsRef<str>>(trace: &[S], tree: &ProcessTree) -> Result<bool> {
    let model = ModelFootprints::of_tree(tree);
    if !FootprintsDeviations::of_trace(&LogFootprints::of_trace(trace), &model).is_footprints_fit {
        return Ok(false);
    }
    let net = tree.to_petri_net();
    let options = TokenReplayOptions::default();
    let replayer = TokenReplayer::new(&net.net, &net.initial_marking, &net.final_marking, options)?;
    if replayer.replay(trace)?.is_fit {
        return Ok(true);
    }
    Ok(TreeAligner::new(tree)?.align(trace).is_fit())
}
