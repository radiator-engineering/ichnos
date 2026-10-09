//! The recursion shared by IM, IMf and IMd, ported from pm4py's
//! `InductiveMinerFramework` and its subclasses `IMUVCL`, `IMFUVCL` and
//! `IMD`.

use std::collections::BTreeMap;

use ichnos_model::{Label, ProcessTree};

use super::cuts::{Cut, find_cut, project_dfg, project_log};
use super::data::{Act, Dfg, Uvcl, trace_count};
use super::fall_through::{Split, empty_traces, fall_through};

/// One run of the miner: the activity names and the options.
#[derive(Debug)]
pub(crate) struct Miner<'a> {
    /// Activity names, indexed by [`Act`].
    pub labels: &'a [Label],
    pub strict_sequence: bool,
    pub fall_throughs: bool,
}

impl Miner<'_> {
    fn leaf(&self, a: Act) -> ProcessTree {
        ProcessTree::Activity(self.labels[a as usize].clone())
    }

    /// IM on a variant log (pm4py's `IMUVCL.apply`).
    pub fn im(&self, log: Uvcl) -> ProcessTree {
        if let Some(split) = empty_traces(&log) {
            return self.mine_split(split, |l| self.im(l));
        }
        if let Some(tree) = self.base_case(&log) {
            return tree;
        }
        if let Some(cut) = find_cut(&Dfg::from_log(&log), self.strict_sequence) {
            return self.mine_cut(&log, &cut, |l| self.im(l));
        }
        let split = fall_through(&log, self.fall_throughs, self.strict_sequence);
        self.mine_split(split, |l| self.im(l))
    }

    /// IMf on a variant log (pm4py's `IMFUVCL.apply`).
    ///
    /// Empty traces are kept as a skip only if they are more than
    /// `noise_threshold` of the traces. When no cut holds, the cuts are tried
    /// again on the DFG without infrequent edges and start activities; the
    /// fall-throughs run only if that fails too.
    pub fn imf(&self, log: Uvcl, noise_threshold: f64) -> ProcessTree {
        let mine = |l| self.imf(l, noise_threshold);
        let log = match empty_traces(&log) {
            Some(Split::Node(op, mut parts)) => {
                let all = trace_count(&log);
                let rest = parts.pop().expect("empty-traces split has two parts");
                if (all - trace_count(&rest)) as f64 > noise_threshold * all as f64 {
                    parts.push(rest);
                    return self.mine_split(Split::Node(op, parts), mine);
                }
                rest
            }
            _ => log,
        };
        if let Some(tree) = self.base_case(&log) {
            return tree;
        }
        let dfg = Dfg::from_log(&log);
        let cut = find_cut(&dfg, self.strict_sequence)
            .or_else(|| find_cut(&filter_noise(&dfg, noise_threshold), self.strict_sequence));
        if let Some(cut) = cut {
            return self.mine_cut(&log, &cut, mine);
        }
        let split = fall_through(&log, self.fall_throughs, self.strict_sequence);
        self.mine_split(split, mine)
    }

    /// IMd on a DFG (pm4py's `IMD.apply`). `skip` says the empty trace is in
    /// the language, as pm4py's `InductiveDFG.skip` does.
    pub fn imd(&self, dfg: Dfg, skip: bool) -> ProcessTree {
        if skip {
            if dfg.is_empty() {
                return ProcessTree::Tau;
            }
            return ProcessTree::xor([self.imd(Dfg::default(), false), self.imd(dfg, false)]);
        }
        if dfg.is_empty() {
            return ProcessTree::Tau;
        }
        if dfg.graph.is_empty() {
            let mut boundary = dfg.start.keys().chain(dfg.end.keys());
            let first = *boundary.next().expect("a non-empty DFG without edges");
            if boundary.all(|&a| a == first) {
                return self.leaf(first);
            }
        }
        if let Some(cut) = find_cut(&dfg, self.strict_sequence) {
            let parts = project_dfg(&dfg, &cut);
            return ProcessTree::Node(
                cut.operator,
                parts.into_iter().map(|(d, s)| self.imd(d, s)).collect(),
            );
        }
        // The flower model (pm4py's `FlowerModelDFG`).
        let all: BTreeMap<Act, u64> = dfg.vertices().into_iter().map(|a| (a, 1)).collect();
        let redo = Dfg {
            graph: BTreeMap::new(),
            start: all.clone(),
            end: all,
        };
        ProcessTree::looped(self.imd(Dfg::default(), false), self.imd(redo, false))
    }

    /// The empty log gives tau; a log with one variant of at most one
    /// activity gives that activity, or tau (pm4py's `EmptyLogBaseCaseUVCL`
    /// and `SingleActivityBaseCaseUVCL`).
    fn base_case(&self, log: &Uvcl) -> Option<ProcessTree> {
        let mut variants = log.keys();
        let Some(first) = variants.next() else {
            return Some(ProcessTree::Tau);
        };
        if variants.next().is_some() || first.len() > 1 {
            return None;
        }
        Some(first.first().map_or(ProcessTree::Tau, |&a| self.leaf(a)))
    }

    fn mine_cut(
        &self,
        log: &Uvcl,
        cut: &Cut,
        mine: impl FnMut(Uvcl) -> ProcessTree,
    ) -> ProcessTree {
        let parts = project_log(log, cut);
        ProcessTree::Node(cut.operator, parts.into_iter().map(mine).collect())
    }

    fn mine_split(&self, split: Split, mine: impl FnMut(Uvcl) -> ProcessTree) -> ProcessTree {
        match split {
            Split::Tau => ProcessTree::Tau,
            Split::Node(op, parts) => ProcessTree::Node(op, parts.into_iter().map(mine).collect()),
        }
    }
}

/// IMf's noise filter (pm4py's `IMFUVCL.__filter_dfg_noise`). Keeps an edge
/// `a -> b` if its frequency is above `noise_threshold` times the largest
/// outgoing frequency of `a`, where ending a trace counts as an outgoing
/// edge. Keeps a start activity if its frequency is at least
/// `noise_threshold` times the largest start frequency. Keeps every end
/// activity.
pub(crate) fn filter_noise(dfg: &Dfg, noise_threshold: f64) -> Dfg {
    let mut max_out: BTreeMap<Act, u64> = BTreeMap::new();
    for (&(a, _), &n) in &dfg.graph {
        let m = max_out.entry(a).or_insert(n);
        *m = (*m).max(n).max(dfg.end.get(&a).copied().unwrap_or(0));
    }
    let max_start = dfg.start.values().copied().max().unwrap_or(0);
    Dfg {
        graph: dfg
            .graph
            .iter()
            .filter(|&(&(a, _), &n)| n as f64 > noise_threshold * max_out[&a] as f64)
            .map(|(&e, &n)| (e, n))
            .collect(),
        start: dfg
            .start
            .iter()
            .filter(|&(_, &n)| n as f64 >= max_start as f64 * noise_threshold)
            .map(|(&a, &n)| (a, n))
            .collect(),
        end: dfg.end.clone(),
    }
}
