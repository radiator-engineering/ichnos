//! Generalization of a Petri net against a log (pm4py's
//! `algo/evaluation/generalization`, variant `token_based`).
//!
//! Following Buijs, van Dongen and van der Aalst, a model generalizes well
//! when its transitions fire often. With `n(t)` the number of times token
//! replay fires transition `t` over all traces,
//!
//! ```text
//! generalization = 1 - sum over transitions t of (1 / sqrt(n(t))) / |transitions|
//! ```
//!
//! where a transition that never fires adds 1.

use std::collections::HashMap;

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{Marking, PetriNet};

use crate::error::Result;
use crate::token_replay::{LogReplay, TokenReplayOptions, replay_log};

/// Generalization of `net` from the token replay of a log on it (pm4py's
/// `generalization.variants.token_based.get_generalization`). 1 for a net
/// without transitions.
///
/// The terms are summed in the order pm4py sums them: fired transitions in
/// order of first firing over the log, then 1 for each transition that
/// never fired.
pub fn generalization(net: &PetriNet, replay: &LogReplay) -> f64 {
    let transitions = net.transition_count();
    if transitions == 0 {
        return 1.0;
    }
    let mut order = Vec::new();
    let mut fired = HashMap::new();
    let mut seen_variant = vec![false; replay.replays.len()];
    for trace in 0..replay.trace_count() {
        let variant = replay.variant_of(trace);
        let count = replay.variants.variants[variant].count() as u64;
        if std::mem::replace(&mut seen_variant[variant], true) {
            continue;
        }
        for &t in &replay.replays[variant].activated_transitions {
            *fired.entry(t).or_insert_with(|| {
                order.push(t);
                0u64
            }) += count;
        }
    }
    let mut sum = 0.0;
    for t in &order {
        sum += 1.0 / (fired[t] as f64).sqrt();
    }
    for _ in net.transition_ids().filter(|t| !fired.contains_key(t)) {
        sum += 1.0;
    }
    1.0 - sum / transitions as f64
}

/// Token-based generalization of `net` against `log` (pm4py's
/// `generalization_tbr`), replaying with pm4py's default options.
pub fn generalization_tbr(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
) -> Result<f64> {
    let options = TokenReplayOptions::default();
    let replay = replay_log(log, net, initial_marking, final_marking, keys, options)?;
    Ok(generalization(net, &replay))
}
