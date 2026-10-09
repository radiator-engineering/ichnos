//! Alpha and alpha+ discovery over ordered event traces.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{AcceptingPetriNet, Label, Marking, PetriNet, TransitionId};

use crate::Result;

/// Options for classic alpha discovery.
#[derive(Debug, Clone, Default)]
pub struct AlphaOptions {}

/// Options for alpha+ discovery.
#[derive(Debug, Clone, Default)]
pub struct AlphaPlusOptions {
    /// Remove transitions with neither input nor output arcs (default false).
    pub remove_unconnected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Activity {
    Start,
    Visible(Label),
    End,
}
type Set = BTreeSet<Activity>;
type Relation = BTreeSet<(Activity, Activity)>;
type Pair = (Set, Set);

fn sequences(log: &EventLog, keys: &EventKeys) -> Result<Vec<Vec<Activity>>> {
    let seqs = log.activity_sequences(keys)?;
    Ok(seqs
        .traces
        .iter()
        .map(|t| {
            t.iter()
                .map(|&a| Activity::Visible(Label::from(seqs.activities.name(a))))
                .collect()
        })
        .collect())
}

fn follows(traces: &[Vec<Activity>]) -> Relation {
    traces
        .iter()
        .flat_map(|t| t.windows(2).map(|p| (p[0].clone(), p[1].clone())))
        .collect()
}

fn pairs(
    causal: &Relation,
    unrelated: impl Fn(&Activity, &Activity) -> bool,
    all_causal: bool,
    traces: &[Vec<Activity>],
) -> Vec<Pair> {
    let mut pairs: Vec<Pair> = causal
        .iter()
        .filter(|(a, b)| unrelated(a, a) && unrelated(b, b))
        .map(|(a, b)| (Set::from([a.clone()]), Set::from([b.clone()])))
        .collect();
    if all_causal {
        let mut order = BTreeMap::new();
        for p in traces.iter().flat_map(|t| t.windows(2)) {
            let next = order.len();
            order.entry((p[0].clone(), p[1].clone())).or_insert(next);
        }
        pairs
            .sort_by_key(|(a, b)| order[&(a.first().unwrap().clone(), b.first().unwrap().clone())]);
    } else {
        let mut order = BTreeMap::new();
        for a in traces.iter().flatten() {
            let next = order.len();
            order.entry(a.clone()).or_insert(next);
        }
        pairs.sort_by_key(|(a, b)| (order[a.first().unwrap()], b.first().unwrap().clone()));
    }
    let initial_len = pairs.len();
    // Both source miners make one pass over their initial causal pairs.
    // The inner pass includes pairs that earlier iterations appended.
    let mut i = 0;
    while i < initial_len {
        let mut j = i + 1;
        let end = pairs.len();
        while j < end {
            let (a, b) = &pairs[i];
            let (c, d) = &pairs[j];
            if (a.is_subset(c) || b.is_subset(d))
                && a.iter().all(|x| c.iter().all(|y| unrelated(x, y)))
                && b.iter().all(|x| d.iter().all(|y| unrelated(x, y)))
            {
                let joined: Pair = (a.union(c).cloned().collect(), b.union(d).cloned().collect());
                if (!all_causal
                    || joined.0.iter().all(|x| {
                        joined
                            .1
                            .iter()
                            .all(|y| causal.contains(&(x.clone(), y.clone())))
                    }))
                    && !pairs.contains(&joined)
                {
                    pairs.push(joined);
                }
            }
            j += 1;
        }
        i += 1;
    }
    pairs
        .iter()
        .filter(|(a, b)| {
            !pairs
                .iter()
                .any(|(c, d)| (a, b) != (c, d) && a.is_subset(c) && b.is_subset(d))
        })
        .cloned()
        .collect()
}

fn build(
    traces: &[Vec<Activity>],
    pairs: &[Pair],
) -> (AcceptingPetriNet, BTreeMap<Activity, TransitionId>) {
    let mut net = PetriNet::new("alpha");
    let labels: Set = traces.iter().flatten().cloned().collect();
    let transitions = labels
        .into_iter()
        .enumerate()
        .map(|(i, a)| {
            let label = match &a {
                Activity::Visible(l) => Some(l.clone()),
                _ => None,
            };
            let t = net.add_transition(format!("activity_{i}"), label);
            (a, t)
        })
        .collect::<BTreeMap<_, _>>();
    let source = net.add_place("start");
    let sink = net.add_place("end");
    let starts: Set = traces.iter().filter_map(|t| t.first().cloned()).collect();
    let ends: Set = traces.iter().filter_map(|t| t.last().cloned()).collect();
    for a in starts {
        net.add_input_arc(source, transitions[&a]).unwrap();
    }
    for a in ends {
        net.add_output_arc(transitions[&a], sink).unwrap();
    }
    for (i, (ins, outs)) in pairs.iter().enumerate() {
        let p = net.add_place(format!("pair_{i}"));
        for a in ins {
            net.add_output_arc(transitions[a], p).unwrap();
        }
        for a in outs {
            net.add_input_arc(p, transitions[a]).unwrap();
        }
    }
    (
        AcceptingPetriNet::new(
            net,
            Marking::from([(source, 1)]),
            Marking::from([(sink, 1)]),
        ),
        transitions,
    )
}

/// Discovers a classic alpha Petri net. Empty traces contribute no boundaries;
/// the final marking is not guaranteed reachable, as in pm4py.
pub fn petri_net_alpha(
    log: &EventLog,
    keys: &EventKeys,
    _options: &AlphaOptions,
) -> Result<AcceptingPetriNet> {
    let traces = sequences(log, keys)?;
    let follows = follows(&traces);
    let causal = follows
        .iter()
        .filter(|(a, b)| !follows.contains(&(b.clone(), a.clone())))
        .cloned()
        .collect();
    let pairs = pairs(
        &causal,
        |a, b| {
            !follows.contains(&(a.clone(), b.clone())) && !follows.contains(&(b.clone(), a.clone()))
        },
        true,
        &traces,
    );
    Ok(build(&traces, &pairs).0)
}

/// Discovers alpha+ with length-one filtering and length-two loop relations.
/// Input traces are never mutated. Synthetic boundary activities are distinct
/// from every user label, including `artificial_start` and `artificial_end`.
pub fn petri_net_alpha_plus(
    log: &EventLog,
    keys: &EventKeys,
    options: &AlphaPlusOptions,
) -> Result<AcceptingPetriNet> {
    let traces: Vec<_> = sequences(log, keys)?
        .into_iter()
        .map(|mut t| {
            t.insert(0, Activity::Start);
            t.push(Activity::End);
            t
        })
        .collect();
    // An empty log yields an empty workflow shell instead of indexing a
    // nonexistent first boundary transition.
    if traces.iter().all(|t| t.len() == 2) {
        return Ok(build(&[], &[]).0);
    }
    let loops: Set = traces
        .iter()
        .flat_map(|t| t.windows(2).filter(|p| p[0] == p[1]).map(|p| p[0].clone()))
        .collect();
    let mut before: BTreeMap<Activity, Set> = BTreeMap::new();
    let mut after: BTreeMap<Activity, Set> = BTreeMap::new();
    for t in &traces {
        for p in t.windows(2) {
            if !loops.contains(&p[0]) && loops.contains(&p[1]) {
                // pm4py replaces this entry on each non-loop predecessor.
                before.insert(p[1].clone(), Set::from([p[0].clone()]));
            }
            if loops.contains(&p[0]) && !loops.contains(&p[1]) {
                after.entry(p[0].clone()).or_default().insert(p[1].clone());
            }
        }
    }
    let filtered: Vec<Vec<_>> = traces
        .iter()
        .map(|t| t.iter().filter(|a| !loops.contains(a)).cloned().collect())
        .collect();
    let follows = follows(&filtered);
    let triangles: Relation = filtered
        .iter()
        .flat_map(|t| {
            t.windows(3)
                .filter(|p| p[0] == p[2])
                .map(|p| (p[0].clone(), p[1].clone()))
        })
        .collect();
    let causal: Relation = follows
        .iter()
        .filter(|(a, b)| {
            !follows.contains(&(b.clone(), a.clone()))
                || (triangles.contains(&(a.clone(), b.clone()))
                    && triangles.contains(&(b.clone(), a.clone())))
        })
        .cloned()
        .collect();
    let has_row = |a: &Activity| follows.iter().any(|(x, _)| x == a);
    let pairs = pairs(
        &causal,
        |a, b| {
            (has_row(a) == has_row(b))
                && !follows.contains(&(a.clone(), b.clone()))
                && !follows.contains(&(b.clone(), a.clone()))
        },
        false,
        &filtered,
    );
    let (mut result, transitions) = build(&filtered, &pairs);
    for a in &loops {
        let Activity::Visible(label) = a else {
            continue;
        };
        let t = result
            .net
            .add_transition(label.as_str(), Some(label.clone()));
        if let (Some(ins), Some(outs)) = (before.get(a), after.get(a)) {
            let ins: Set = ins.difference(outs).cloned().collect();
            let outs: Set = outs.difference(before.get(a).unwrap()).cloned().collect();
            for (i, (left, right)) in pairs.iter().enumerate() {
                if ins.is_subset(left) && outs.is_subset(right) {
                    // pm4py creates a distinct, unmarked place here rather
                    // than reusing the causal place. Preserve that behaviour.
                    let p = result.net.add_place(format!("loop_{label}_{i}"));
                    result.net.add_output_arc(t, p).unwrap();
                    result.net.add_input_arc(p, t).unwrap();
                }
            }
        }
    }
    simplify_boundaries(
        &mut result,
        transitions[&Activity::Start],
        transitions[&Activity::End],
    );
    if options.remove_unconnected {
        let remove: Vec<_> = result
            .net
            .transitions()
            .filter(|(_, t)| t.in_arcs().is_empty() && t.out_arcs().is_empty())
            .map(|(id, _)| id)
            .collect();
        for t in remove {
            result.net.remove_transition(t);
        }
    }
    Ok(result)
}

fn simplify_boundaries(result: &mut AcceptingPetriNet, start: TransitionId, end: TransitionId) {
    let net = &mut result.net;
    let targets: Vec<_> = net.postset(start).collect();
    if targets.len() == 1 && net.place(targets[0]).in_arcs().len() == 1 {
        let source = result.initial_marking.iter().next().unwrap().0;
        result.initial_marking = Marking::from([(targets[0], 1)]);
        net.remove_place(source);
        net.remove_transition(start);
    }
    let inputs: Vec<_> = net.preset(end).collect();
    if inputs.len() == 1
        && net.place(inputs[0]).out_arcs().len() == 1
        && net
            .place_preset(inputs[0])
            .all(|t| net.transition(t).out_arcs().len() == 1)
    {
        let predecessors: Vec<_> = net.place_preset(inputs[0]).collect();
        let sink = result.final_marking.iter().next().unwrap().0;
        net.remove_transition(end);
        for t in predecessors {
            net.add_output_arc(t, sink).unwrap();
        }
        // Preserve a live place for the initial marking when the source
        // simplification already marked this place. pm4py leaves an orphan
        // marking in that case; an isolated place has the same behaviour.
        if result.initial_marking.get(inputs[0]) == 0 {
            net.remove_place(inputs[0]);
        }
    }
}
