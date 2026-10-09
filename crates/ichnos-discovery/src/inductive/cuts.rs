//! The four cuts of the inductive miner, ported from pm4py's
//! `algo/discovery/inductive/cuts/`: exclusive choice, sequence (plain and
//! strict), concurrency and loop. Each cut is detected on a DFG and then
//! projected onto a variant log (IM, IMf) or a DFG (IMd).

use std::collections::BTreeMap;

use ichnos_model::Operator;

use super::data::{Act, Dfg, Group, Uvcl, add_trace, connected_components};

/// A cut: the operator and the activity groups of its children, in child
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cut {
    pub operator: Operator,
    pub groups: Vec<Group>,
}

/// Tries the cuts in pm4py's order (exclusive choice, sequence,
/// concurrency, loop) and returns the first that holds (pm4py's
/// `CutFactory.find_cut`).
pub(crate) fn find_cut(dfg: &Dfg, strict_sequence: bool) -> Option<Cut> {
    let cut = |operator, groups| Cut { operator, groups };
    if let Some(groups) = xor_cut(dfg) {
        return Some(cut(Operator::Xor, groups));
    }
    let sequence = if strict_sequence {
        strict_sequence_cut(dfg)
    } else {
        sequence_cut(dfg)
    };
    if let Some(groups) = sequence {
        return Some(cut(Operator::Sequence, groups));
    }
    if let Some(groups) = concurrency_cut(dfg) {
        return Some(cut(Operator::Parallel, groups));
    }
    loop_cut(dfg).map(|groups| cut(Operator::Loop, groups))
}

/// Exclusive choice: the connected components of the undirected DFG, if
/// there are at least two (pm4py's `ExclusiveChoiceCut.holds`).
///
/// pm4py lists the components in networkx order: each vertex not yet seen,
/// taken in Python's set order, starts the next component. That order can
/// change between runs, but a larger component tends to come earlier. Here
/// the components are ordered by size, largest first, and then by their
/// smallest activity. The order matters to IMf, whose projection breaks
/// ties by group position.
pub(crate) fn xor_cut(dfg: &Dfg) -> Option<Vec<Group>> {
    let mut components = connected_components(&dfg.vertices(), dfg.graph.keys().copied());
    // Stable, so equal sizes keep the smallest-activity order.
    components.sort_by_key(|g| std::cmp::Reverse(g.len()));
    (components.len() > 1).then_some(components)
}

/// Sequence: groups activities that reach each other in both directions or
/// in neither, then orders the groups by reachability (pm4py's
/// `SequenceCut.holds`).
pub(crate) fn sequence_cut(dfg: &Dfg) -> Option<Vec<Group>> {
    let alphabet = dfg.vertices();
    if alphabet.is_empty() {
        return None;
    }
    let (pre, post) = dfg.transitive_relations();
    let reaches = |a: Act, b: Act| post[&a].contains(&b);
    // pm4py merges groups until no pair of groups holds two activities that
    // reach each other in both directions or in neither. That gives the
    // connected components of that relation.
    let acts: Vec<Act> = alphabet.iter().copied().collect();
    let mut merge_edges = Vec::new();
    for (i, &a) in acts.iter().enumerate() {
        for &b in &acts[i + 1..] {
            if reaches(a, b) == reaches(b, a) {
                merge_edges.push((a, b));
            }
        }
    }
    let mut groups = connected_components(&alphabet, merge_edges);
    // pm4py ranks a group by one of its members, taken in Python's set
    // order. Here it is the smallest member, so the order is deterministic.
    let n = alphabet.len();
    groups.sort_by_key(|g| {
        let a = g.first().expect("groups are not empty");
        pre[a].len() + (n - post[a].len())
    });
    (groups.len() > 1).then_some(groups)
}

/// Strict sequence: the sequence cut with groups merged where together they
/// can be skipped (pm4py's `StrictSequenceCut.holds`, after function
/// SKIPPABLE in Leemans' thesis, page 233).
pub(crate) fn strict_sequence_cut(dfg: &Dfg) -> Option<Vec<Group>> {
    let mut c = sequence_cut(dfg)?;
    let start: Group = dfg.start.keys().copied().collect();
    let end: Group = dfg.end.keys().copied().collect();
    let mut mf: Vec<i64> = c
        .iter()
        .map(|g| {
            if g.is_disjoint(&start) {
                i64::MAX
            } else {
                -i64::MAX
            }
        })
        .collect();
    let mut mt: Vec<i64> = c
        .iter()
        .map(|g| {
            if g.is_disjoint(&end) {
                -i64::MAX
            } else {
                i64::MAX
            }
        })
        .collect();
    let cluster: BTreeMap<Act, usize> = c
        .iter()
        .enumerate()
        .flat_map(|(i, g)| g.iter().map(move |&a| (a, i)))
        .collect();
    for &(a, b) in dfg.graph.keys() {
        let (ca, cb) = (cluster[&a], cluster[&b]);
        mf[cb] = mf[cb].min(ca as i64);
        mt[ca] = mt[ca].max(cb as i64);
    }
    for p in 0..c.len() {
        if !skippable(p, dfg, &start, &end, &c) {
            continue;
        }
        let pi = p as i64;
        let mut q = p;
        while q > 0 && mt[q - 1] <= pi {
            let moved = std::mem::take(&mut c[q - 1]);
            c[p].extend(moved);
            q -= 1;
        }
        let mut q = p + 1;
        while q < mf.len() && mf[q] >= pi {
            let moved = std::mem::take(&mut c[q]);
            c[p].extend(moved);
            q += 1;
        }
    }
    c.retain(|g| !g.is_empty());
    // pm4py would return a single group here and recurse on the same log
    // without end; treat it as no cut.
    (c.len() > 1).then_some(c)
}

/// pm4py's `StrictSequenceCut._skippable`: whether group `p` can be skipped.
fn skippable(p: usize, dfg: &Dfg, start: &Group, end: &Group, groups: &[Group]) -> bool {
    let before = &groups[..p];
    let after = &groups[p + 1..];
    let jumps = before.iter().flatten().any(|&a| {
        after
            .iter()
            .flatten()
            .any(|&b| dfg.graph.contains_key(&(a, b)))
    });
    jumps
        || after.iter().flatten().any(|a| start.contains(a))
        || before.iter().flatten().any(|a| end.contains(a))
}

/// Concurrency: groups activities that are not connected in both directions,
/// then merges groups without a start or an end activity into a neighbour
/// (pm4py's `ConcurrencyCut.holds`).
pub(crate) fn concurrency_cut(dfg: &Dfg) -> Option<Vec<Group>> {
    let alphabet = dfg.vertices();
    if alphabet.is_empty() {
        return None;
    }
    let acts: Vec<Act> = alphabet.iter().copied().collect();
    let mut merge_edges = Vec::new();
    for (i, &a) in acts.iter().enumerate() {
        for &b in &acts[i + 1..] {
            if !(dfg.has_edge(a, b) && dfg.has_edge(b, a)) {
                merge_edges.push((a, b));
            }
        }
    }
    // pm4py merges pairwise in sorted order, so its groups are these
    // components, ordered by their smallest activity.
    let mut groups = connected_components(&alphabet, merge_edges);
    groups.sort_by_key(Group::len);
    let has_start = |g: &Group| g.iter().any(|a| dfg.start.contains_key(a));
    let has_end = |g: &Group| g.iter().any(|a| dfg.end.contains_key(a));
    let mut i = 0;
    while i < groups.len() && groups.len() > 1 {
        if has_start(&groups[i]) && has_end(&groups[i]) {
            i += 1;
            continue;
        }
        let group = groups.remove(i);
        let target = if i == 0 { 0 } else { i - 1 };
        groups[target].extend(group);
    }
    (groups.len() > 1).then_some(groups)
}

/// Loop: a do group (start and end activities plus what must stay with
/// them) and one redo group (pm4py's `LoopCut.holds`).
pub(crate) fn loop_cut(dfg: &Dfg) -> Option<Vec<Group>> {
    if dfg.graph.is_empty() {
        return None;
    }
    let start: Group = dfg.start.keys().copied().collect();
    let end: Group = dfg.end.keys().copied().collect();
    let boundary: Group = start.union(&end).copied().collect();
    let inner: Group = dfg.vertices().difference(&boundary).copied().collect();
    let inner_edges = dfg
        .graph
        .keys()
        .copied()
        .filter(|(a, b)| inner.contains(a) && inner.contains(b));
    let mut redo = connected_components(&inner, inner_edges);
    let mut do_group = boundary;

    // The order of pm4py's checks matters: each one sees the groups the
    // previous one left. Every check moves whole redo groups into the do
    // group, so the redo groups can be kept in a list.
    let mut merge_into_do = |redo: &mut Vec<Group>, act: Act| {
        if let Some(i) = redo.iter().position(|g| g.contains(&act)) {
            do_group.extend(redo.remove(i));
        }
    };
    // Activities directly after a start activity that is not an end
    // activity belong to the do part (`_exclude_sets_non_reachable_from_start`).
    for &a in start.difference(&end) {
        for &(x, b) in dfg.graph.keys() {
            if x == a {
                merge_into_do(&mut redo, b);
            }
        }
    }
    // Likewise for activities directly before an end activity that is not a
    // start activity (`_exclude_sets_no_reachable_from_end`).
    for &b in end.difference(&start) {
        for &(a, x) in dfg.graph.keys() {
            if x == b {
                merge_into_do(&mut redo, a);
            }
        }
    }
    // A redo group that enters the start activities must reach all of them
    // (`_check_start_completeness`).
    redo.retain(|g| {
        let incomplete = g.iter().any(|&a| {
            dfg.graph.keys().any(|&(x, b)| x == a && start.contains(&b))
                && start.iter().any(|&s| !dfg.has_edge(a, s))
        });
        if incomplete {
            do_group.extend(g);
        }
        !incomplete
    });
    // A redo group entered from the end activities must be reached from all
    // of them (`_check_end_completeness`).
    redo.retain(|g| {
        let incomplete = g.iter().any(|&a| {
            dfg.graph.keys().any(|&(b, x)| x == a && end.contains(&b))
                && end.iter().any(|&e| !dfg.has_edge(e, a))
        });
        if incomplete {
            do_group.extend(g);
        }
        !incomplete
    });
    // pm4py drops empty groups, then merges all groups after the first
    // into one redo group. The do group is only empty when the DFG has no
    // start and no end activities; then the first redo group takes its
    // place.
    let mut groups = std::iter::once(do_group)
        .chain(redo)
        .filter(|g| !g.is_empty());
    let first = groups.next()?;
    let rest: Group = groups.flatten().collect();
    (!rest.is_empty()).then(|| vec![first, rest])
}

/// Projects a variant log onto the groups of `cut` (pm4py's
/// `<Cut>UVCL.project`), one sublog per group.
pub(crate) fn project_log(log: &Uvcl, cut: &Cut) -> Vec<Uvcl> {
    match cut.operator {
        Operator::Xor => project_xor(log, &cut.groups),
        Operator::Sequence => project_sequence(log, &cut.groups),
        Operator::Parallel => project_parallel(log, &cut.groups),
        Operator::Loop => project_loop(log, &cut.groups),
        Operator::Or | Operator::Interleaving => {
            unreachable!("the inductive miner finds no {} cuts", cut.operator)
        }
    }
}

/// Each trace goes to the group holding most of its events, the last such
/// group on a tie, and keeps only that group's events.
fn project_xor(log: &Uvcl, groups: &[Group]) -> Vec<Uvcl> {
    let mut logs = vec![Uvcl::new(); groups.len()];
    for (trace, &count) in log {
        let best = groups
            .iter()
            .enumerate()
            .map(|(i, g)| (trace.iter().filter(|a| g.contains(a)).count(), i))
            .max()
            .map(|(_, i)| i)
            .expect("a cut has groups");
        let projected = trace
            .iter()
            .copied()
            .filter(|a| groups[best].contains(a))
            .collect();
        add_trace(&mut logs[best], projected, count);
    }
    logs
}

/// Splits each trace at the points that fit the groups best (pm4py's
/// `SequenceCutUVCL.project` and `_find_split_point`).
fn project_sequence(log: &Uvcl, groups: &[Group]) -> Vec<Uvcl> {
    let mut logs = vec![Uvcl::new(); groups.len()];
    for (trace, &count) in log {
        let mut split = 0;
        let mut seen = Group::new();
        for (group, sublog) in groups.iter().zip(&mut logs) {
            let next = find_split_point(trace, group, split, &seen);
            let projected = trace[split..next]
                .iter()
                .copied()
                .filter(|a| group.contains(a))
                .collect();
            add_trace(sublog, projected, count);
            split = next;
            seen.extend(group);
        }
    }
    logs
}

/// The position after which the rest of `trace` fits `group` least: each
/// event of the group lowers the cost, each event of a later group raises
/// it, and events of earlier groups (`ignore`) are free.
fn find_split_point(trace: &[Act], group: &Group, start: usize, ignore: &Group) -> usize {
    let mut least_cost = 0i64;
    let mut position = start;
    let mut cost = 0i64;
    for (i, a) in trace.iter().enumerate().skip(start) {
        if group.contains(a) {
            cost -= 1;
        } else if !ignore.contains(a) {
            cost += 1;
        }
        if cost < least_cost {
            least_cost = cost;
            position = i + 1;
        }
    }
    position
}

/// Each group gets every trace, filtered to the group's activities.
fn project_parallel(log: &Uvcl, groups: &[Group]) -> Vec<Uvcl> {
    groups
        .iter()
        .map(|g| {
            let mut sublog = Uvcl::new();
            for (trace, &count) in log {
                let projected = trace.iter().copied().filter(|a| g.contains(a)).collect();
                add_trace(&mut sublog, projected, count);
            }
            sublog
        })
        .collect()
}

/// Cuts each trace into do and redo slices (pm4py's `LoopCutUVCL.project`).
/// Events in neither group, which only IMf's filtered DFG can produce, end
/// the current slice and are dropped. Every trace adds its last do slice to
/// the do log, even when it is empty.
fn project_loop(log: &Uvcl, groups: &[Group]) -> Vec<Uvcl> {
    let (do_group, redo_group) = (&groups[0], &groups[1]);
    let mut do_log = Uvcl::new();
    let mut redo_log = Uvcl::new();
    for (trace, &count) in log {
        let mut do_slice = Vec::new();
        let mut redo_slice = Vec::new();
        for &a in trace {
            if do_group.contains(&a) {
                do_slice.push(a);
                if !redo_slice.is_empty() {
                    add_trace(&mut redo_log, std::mem::take(&mut redo_slice), count);
                }
            } else if redo_group.contains(&a) {
                redo_slice.push(a);
                if !do_slice.is_empty() {
                    add_trace(&mut do_log, std::mem::take(&mut do_slice), count);
                }
            } else {
                if !do_slice.is_empty() {
                    add_trace(&mut do_log, std::mem::take(&mut do_slice), count);
                }
                if !redo_slice.is_empty() {
                    add_trace(&mut redo_log, std::mem::take(&mut redo_slice), count);
                }
            }
        }
        if !redo_slice.is_empty() {
            add_trace(&mut redo_log, redo_slice, count);
        }
        add_trace(&mut do_log, do_slice, count);
    }
    vec![do_log, redo_log]
}

/// A DFG for one child of an IMd cut, and whether the child can be skipped.
pub(crate) type DfgPart = (Dfg, bool);

/// Projects a DFG onto the groups of `cut` (pm4py's `<Cut>DFG.project`).
pub(crate) fn project_dfg(dfg: &Dfg, cut: &Cut) -> Vec<DfgPart> {
    match cut.operator {
        Operator::Xor | Operator::Parallel => cut
            .groups
            .iter()
            .map(|g| (dfg.restrict(g), false))
            .collect(),
        Operator::Sequence => project_dfg_sequence(dfg, &cut.groups),
        Operator::Loop => project_dfg_loop(dfg, &cut.groups),
        Operator::Or | Operator::Interleaving => {
            unreachable!("the inductive miner finds no {} cuts", cut.operator)
        }
    }
}

/// pm4py's `SequenceCutDFG.project`. The start activities of a later group
/// are the targets of edges from the group just before it, weighted by
/// those edges; end activities likewise. A group is skippable when an edge,
/// a start activity or an end activity jumps over it.
fn project_dfg_sequence(dfg: &Dfg, groups: &[Group]) -> Vec<DfgPart> {
    let index: BTreeMap<Act, usize> = groups
        .iter()
        .enumerate()
        .flat_map(|(i, g)| g.iter().map(move |&a| (a, i)))
        .collect();
    let last = groups.len() - 1;
    let mut skippable = vec![false; groups.len()];
    let mut parts = Vec::with_capacity(groups.len());
    for (i, group) in groups.iter().enumerate() {
        let mut part = dfg.restrict(group);
        if i == 0 {
            for &a in dfg.start.keys() {
                skippable[..index[&a]].iter_mut().for_each(|s| *s = true);
            }
        } else {
            part.start.clear();
            for (&(a, b), &n) in &dfg.graph {
                if groups[i - 1].contains(&a) && group.contains(&b) {
                    *part.start.entry(b).or_insert(0) += n;
                }
            }
        }
        if i == last {
            for &a in dfg.end.keys() {
                skippable[index[&a] + 1..=i]
                    .iter_mut()
                    .for_each(|s| *s = true);
            }
        } else {
            part.end.clear();
            for (&(a, b), &n) in &dfg.graph {
                if group.contains(&a) && groups[i + 1].contains(&b) {
                    *part.end.entry(a).or_insert(0) += n;
                }
            }
        }
        parts.push(part);
    }
    for &(a, b) in dfg.graph.keys() {
        let (from, to) = (index[&a] + 1, index[&b]);
        if from < to {
            skippable[from..to].iter_mut().for_each(|s| *s = true);
        }
    }
    parts.into_iter().zip(skippable).collect()
}

/// pm4py's `LoopCutDFG.project`. The do part keeps its start and end
/// activities; every redo activity starts and ends the redo part with
/// frequency 1. The redo part is skippable when an end activity leads
/// straight back to a start activity.
fn project_dfg_loop(dfg: &Dfg, groups: &[Group]) -> Vec<DfgPart> {
    let (do_group, redo_group) = (&groups[0], &groups[1]);
    let do_part = dfg.restrict(do_group);
    let do_skippable = dfg.start.len() != do_part.start.len() || dfg.end.len() != do_part.end.len();
    let mut redo_part = dfg.restrict(redo_group);
    redo_part.start = redo_group.iter().map(|&a| (a, 1)).collect();
    redo_part.end = redo_part.start.clone();
    let redo_skippable = dfg
        .graph
        .keys()
        .any(|(a, b)| dfg.end.contains_key(a) && dfg.start.contains_key(b));
    vec![(do_part, do_skippable), (redo_part, redo_skippable)]
}
