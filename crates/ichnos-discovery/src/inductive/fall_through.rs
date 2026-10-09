//! Fall-throughs for variant logs, ported from pm4py's
//! `algo/discovery/inductive/fall_through/`. The miner uses them when no
//! base case and no cut applies.

use ichnos_model::Operator;

use super::cuts::find_cut;
use super::data::{Act, Dfg, Group, Uvcl, add_trace, alphabet, trace_count};

/// What a fall-through produces: a tau leaf, or an operator whose children
/// are mined from the given sublogs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Split {
    Tau,
    Node(Operator, Vec<Uvcl>),
}

/// Tries the fall-throughs in pm4py's order and returns the first that
/// applies (pm4py's `FallThroughFactory.fall_through`). With
/// `fall_throughs` off, only the empty-traces split and the flower model
/// remain, as with pm4py's `disable_fallthroughs`.
pub(crate) fn fall_through(log: &Uvcl, fall_throughs: bool, strict_sequence: bool) -> Split {
    if let Some(split) = empty_traces(log) {
        return split;
    }
    if fall_throughs {
        let candidate =
            activity_once_per_trace(log).or_else(|| activity_concurrent(log, strict_sequence));
        if let Some(a) = candidate {
            return split_activity(log, a);
        }
        if let Some(sublog) = tau_loop(log, true).or_else(|| tau_loop(log, false)) {
            return Split::Node(Operator::Loop, vec![sublog, Uvcl::new()]);
        }
    }
    flower(log)
}

/// If the log has empty traces: `X( tau, <the rest> )`, or tau when every
/// trace is empty (pm4py's `EmptyTracesUVCL`).
pub(crate) fn empty_traces(log: &Uvcl) -> Option<Split> {
    if !log.contains_key(&[][..]) {
        return None;
    }
    let mut rest = log.clone();
    rest.remove(&[][..]);
    Some(if rest.is_empty() {
        Split::Tau
    } else {
        Split::Node(Operator::Xor, vec![Uvcl::new(), rest])
    })
}

/// The first activity, in sorted order, that occurs exactly once in every
/// trace (pm4py's `ActivityOncePerTraceUVCL`).
fn activity_once_per_trace(log: &Uvcl) -> Option<Act> {
    let mut candidates = alphabet(log);
    for trace in log.keys() {
        candidates.retain(|a| trace.iter().filter(|&b| b == a).count() == 1);
        if candidates.is_empty() {
            return None;
        }
    }
    candidates.first().copied()
}

/// The first activity, in sorted order, whose removal lets a cut of IM
/// apply (pm4py's `ActivityConcurrentUVCL`).
fn activity_concurrent(log: &Uvcl, strict_sequence: bool) -> Option<Act> {
    alphabet(log).into_iter().find(|&a| {
        let without: Uvcl = log
            .keys()
            .map(|t| (t.iter().copied().filter(|&b| b != a).collect(), 1))
            .collect();
        find_cut(&Dfg::from_log(&without), strict_sequence).is_some()
    })
}

/// `+( <a only>, <everything but a> )`.
fn split_activity(log: &Uvcl, a: Act) -> Split {
    let mut only = Uvcl::new();
    let mut other = Uvcl::new();
    for (trace, &count) in log {
        let (with, without): (Vec<Act>, Vec<Act>) = trace.iter().partition(|&&b| b == a);
        add_trace(&mut only, with, count);
        add_trace(&mut other, without, count);
    }
    Split::Node(Operator::Parallel, vec![only, other])
}

/// Splits traces where a start activity follows an end activity (`strict`,
/// pm4py's `StrictTauLoopUVCL`) or before every start activity after the
/// first event (pm4py's `TauLoopUVCL`). Returns the split log if it has more
/// traces than `log`.
fn tau_loop(log: &Uvcl, strict: bool) -> Option<Uvcl> {
    let start: Group = log.keys().filter_map(|t| t.first().copied()).collect();
    let end: Group = log.keys().filter_map(|t| t.last().copied()).collect();
    let mut split = Uvcl::new();
    for (trace, &count) in log {
        let mut from = 0;
        for i in 1..trace.len() {
            if start.contains(&trace[i]) && (!strict || end.contains(&trace[i - 1])) {
                add_trace(&mut split, trace[from..i].to_vec(), count);
                from = i;
            }
        }
        add_trace(&mut split, trace[from..].to_vec(), count);
    }
    (trace_count(&split) > trace_count(log)).then_some(split)
}

/// `*( tau, <every activity once> )`: the flower model (pm4py's
/// `FlowerModelUVCL`).
fn flower(log: &Uvcl) -> Split {
    let redo = alphabet(log).into_iter().map(|a| (vec![a], 1)).collect();
    Split::Node(Operator::Loop, vec![Uvcl::new(), redo])
}
