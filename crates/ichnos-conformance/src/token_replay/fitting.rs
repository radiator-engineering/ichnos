//! Whether one trace fits a model (pm4py's `check_is_fitting`).

use std::collections::BTreeSet;

use ichnos_model::{Marking, PetriNet, ProcessTree};

use super::{TokenReplayOptions, TokenReplayer};
use crate::alignments::{Aligner, AlignmentOptions, TreeAligner};
use crate::error::Result;
use crate::footprints::{FootprintsDeviations, LogFootprints, ModelFootprints};

/// Whether `trace`, given as its activities, is a run of the accepting net
/// (pm4py's `check_is_fitting` on a Petri net).
///
/// As pm4py does, this tries the cheap checks first. A trace with an
/// activity no transition carries does not fit. A trace that token replay
/// finds fit does. Otherwise the trace fits when its optimal alignment has
/// fitness 1.
///
/// pm4py first tries to convert the net to a process tree and, when that
/// works, takes the tree path ([`check_is_fitting_tree`]). Both paths decide
/// whether the trace is in the model's language, so this always takes the
/// net path.
pub fn check_is_fitting<S: AsRef<str>>(
    trace: &[S],
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
) -> Result<bool> {
    let labels: BTreeSet<&str> = net
        .transitions()
        .filter_map(|(_, t)| t.label.as_deref())
        .collect();
    if trace.iter().any(|a| !labels.contains(a.as_ref())) {
        return Ok(false);
    }
    let options = TokenReplayOptions::default();
    let replayer = TokenReplayer::new(net, initial_marking, final_marking, options)?;
    if replayer.replay(trace)?.is_fit {
        return Ok(true);
    }
    let aligner = Aligner::new(
        net,
        initial_marking,
        final_marking,
        AlignmentOptions::default(),
    )?;
    Ok(aligner.align(trace)?.is_some_and(|a| a.is_fit()))
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
