//! Workflow net to process tree, ported from pm4py's
//! `objects/conversion/wf_net/variants/to_process_tree.py`.

use std::collections::{BTreeMap, BTreeSet};

use crate::petri::{AcceptingPetriNet, PetriNet, PlaceId, TransitionId};
use crate::{Label, Operator, ProcessTree};

/// Why a net could not be converted to a process tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WfNetToTreeError {
    /// The net is not a workflow net: it needs one place without inputs,
    /// one place without outputs, and every node on a path between them.
    #[error("the Petri net is not a workflow net")]
    NotWorkflowNet,
    /// Block reduction stopped with more than one transition left.
    #[error("the workflow net is not block structured")]
    NotBlockStructured,
}

/// pm4py's `workflow_net` check: one source place, one sink place, and the
/// net strongly connected once a transition links the sink to the source.
fn is_workflow_net(net: &PetriNet) -> bool {
    let sources: Vec<PlaceId> = net
        .places()
        .filter(|(_, p)| p.in_arcs().is_empty())
        .map(|(id, _)| id)
        .collect();
    let sinks: Vec<PlaceId> = net
        .places()
        .filter(|(_, p)| p.out_arcs().is_empty())
        .map(|(id, _)| id)
        .collect();
    let (&[source], &[sink]) = (sources.as_slice(), sinks.as_slice()) else {
        return false;
    };
    // Nodes: places, then transitions, then the short-circuit transition.
    let places: Vec<PlaceId> = net.place_ids().collect();
    let transitions: Vec<TransitionId> = net.transition_ids().collect();
    let place_index: BTreeMap<PlaceId, usize> =
        places.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    let np = places.len();
    let n = np + transitions.len() + 1;
    let short = n - 1;
    let mut succ = vec![Vec::new(); n];
    let mut pred = vec![Vec::new(); n];
    let mut edge = |a: usize, b: usize| {
        succ[a].push(b);
        pred[b].push(a);
    };
    for (k, &t) in transitions.iter().enumerate() {
        for p in net.preset(t) {
            edge(place_index[&p], np + k);
        }
        for p in net.postset(t) {
            edge(np + k, place_index[&p]);
        }
    }
    edge(place_index[&sink], short);
    edge(short, place_index[&source]);
    let reaches_all = |adj: &[Vec<usize>]| {
        let mut seen = vec![false; n];
        let mut stack = vec![short];
        seen[short] = true;
        while let Some(i) = stack.pop() {
            for &j in &adj[i] {
                if !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        seen.iter().all(|&s| s)
    };
    reaches_all(&succ) && reaches_all(&pred)
}

/// The net being reduced, with the subtree each transition stands for.
struct Reducer {
    net: PetriNet,
    trees: BTreeMap<TransitionId, ProcessTree>,
    created: usize,
}

impl Reducer {
    fn pre(&self, t: TransitionId) -> BTreeSet<PlaceId> {
        self.net.preset(t).collect()
    }

    fn post(&self, t: TransitionId) -> BTreeSet<PlaceId> {
        self.net.postset(t).collect()
    }

    fn place_pre(&self, p: PlaceId) -> BTreeSet<TransitionId> {
        self.net.place_preset(p).collect()
    }

    fn place_post(&self, p: PlaceId) -> BTreeSet<TransitionId> {
        self.net.place_postset(p).collect()
    }

    fn choice(&self, t1: TransitionId, t2: TransitionId) -> bool {
        let (pre, post) = (self.pre(t1), self.post(t1));
        t1 != t2
            && pre == self.pre(t2)
            && post == self.post(t2)
            && !pre.is_empty()
            && !post.is_empty()
    }

    fn sequence(&self, t1: TransitionId, t2: TransitionId) -> bool {
        if t1 == t2 || self.pre(t2).is_empty() {
            return false;
        }
        let link = |p: PlaceId| {
            let (pre, post) = (self.place_pre(p), self.place_post(p));
            pre.len() == 1 && post.len() == 1 && pre.contains(&t1) && post.contains(&t2)
        };
        self.post(t1).into_iter().all(link) && self.pre(t2).into_iter().all(link)
    }

    fn concurrent(&self, t1: TransitionId, t2: TransitionId) -> bool {
        if t1 == t2 {
            return false;
        }
        let (pre1, post1, pre2, post2) = (self.pre(t1), self.post(t1), self.pre(t2), self.post(t2));
        if pre1.is_empty() || post1.is_empty() || pre2.is_empty() || post2.is_empty() {
            return false;
        }
        let mut pre_pre = BTreeSet::new();
        let mut post_post = BTreeSet::new();
        for (t, pre, post) in [(t1, &pre1, &post1), (t2, &pre2, &post2)] {
            for &p in pre {
                let consumers = self.place_post(p);
                pre_pre.extend(self.place_pre(p));
                if consumers.len() > 1 || !consumers.contains(&t) {
                    return false;
                }
            }
            for &p in post {
                let producers = self.place_pre(p);
                post_post.extend(self.place_post(p));
                if producers.len() > 1 || !producers.contains(&t) {
                    return false;
                }
            }
        }
        pre1.union(&pre2)
            .all(|&p| pre_pre.is_subset(&self.place_pre(p)))
            && post1
                .union(&post2)
                .all(|&p| post_post.is_subset(&self.place_post(p)))
    }

    /// `t1` is the do part and `t2` the redo part of a loop.
    fn looped(&self, t1: TransitionId, t2: TransitionId) -> bool {
        if t1 == t2 {
            return false;
        }
        self.pre(t2).into_iter().all(|p| {
            let pre = self.place_pre(p);
            pre.len() == 1 && pre.contains(&t1)
        }) && self.post(t2).into_iter().all(|p| {
            let post = self.place_post(p);
            post.len() == 1 && post.contains(&t1)
        }) && self.pre(t1).into_iter().all(|p| {
            let post = self.place_post(p);
            post.len() == 1 && post.contains(&t1) && self.place_pre(p).contains(&t2)
        }) && self.post(t1).into_iter().all(|p| {
            let pre = self.place_pre(p);
            pre.len() == 1 && pre.contains(&t1) && self.place_post(p).contains(&t2)
        })
    }

    /// The first pair `(t1, t2)` in transition-id order that `test` accepts.
    fn find(
        &self,
        test: fn(&Self, TransitionId, TransitionId) -> bool,
    ) -> Option<(TransitionId, TransitionId)> {
        let ids: Vec<TransitionId> = self.net.transition_ids().collect();
        ids.iter()
            .flat_map(|&a| ids.iter().map(move |&b| (a, b)))
            .find(|&(a, b)| test(self, a, b))
    }

    /// Replaces `t1` and `t2` by one transition for `op(t1, t2)`, with the
    /// input places of `inputs` and the output places of `outputs`.
    fn merge(
        &mut self,
        op: Operator,
        (t1, t2): (TransitionId, TransitionId),
        inputs: &[TransitionId],
        outputs: &[TransitionId],
    ) {
        self.created += 1;
        let t = self
            .net
            .add_transition(format!("block_{}", self.created), None::<Label>);
        for &s in inputs {
            for p in self.net.preset(s).collect::<Vec<_>>() {
                self.net.add_input_arc(p, t).expect("live ids");
            }
        }
        for &s in outputs {
            for p in self.net.postset(s).collect::<Vec<_>>() {
                self.net.add_output_arc(t, p).expect("live ids");
            }
        }
        let a = self.trees.remove(&t1).expect("tree of t1");
        let b = self.trees.remove(&t2).expect("tree of t2");
        self.trees.insert(t, ProcessTree::Node(op, vec![a, b]));
        self.net.remove_transition(t1);
        self.net.remove_transition(t2);
    }

    /// pm4py's `__group_blocks_internal`: one choice, sequence, concurrency
    /// or loop reduction, tried in that order.
    fn group_once(&mut self) -> bool {
        if let Some(pair) = self.find(Self::choice) {
            self.merge(Operator::Xor, pair, &[pair.0], &[pair.1]);
        } else if let Some(pair) = self.find(Self::sequence) {
            let between: Vec<PlaceId> = self.net.postset(pair.0).collect();
            self.merge(Operator::Sequence, pair, &[pair.0], &[pair.1]);
            for p in between {
                self.net.remove_place(p);
            }
        } else if let Some(pair) = self.find(Self::concurrent) {
            self.merge(
                Operator::Parallel,
                pair,
                &[pair.0, pair.1],
                &[pair.0, pair.1],
            );
        } else if let Some(pair) = self.find(Self::looped) {
            self.merge(Operator::Loop, pair, &[pair.0], &[pair.0]);
        } else {
            return false;
        }
        true
    }

    /// pm4py's `__insert_dummy_invisibles`: splits every original place
    /// that is not a source or sink into two places joined by a silent
    /// transition.
    fn insert_skips(&mut self, original: &BTreeSet<PlaceId>) {
        let inner: Vec<PlaceId> = self
            .net
            .places()
            .filter(|&(id, p)| {
                original.contains(&id) && !p.in_arcs().is_empty() && !p.out_arcs().is_empty()
            })
            .map(|(id, _)| id)
            .collect();
        for p in inner {
            let producers: Vec<TransitionId> = self.net.place_preset(p).collect();
            let consumers: Vec<TransitionId> = self.net.place_postset(p).collect();
            let name = self.net.place(p).name.clone();
            self.net.remove_place(p);
            let before = self.net.add_place(format!("{name}_in"));
            let after = self.net.add_place(format!("{name}_out"));
            self.created += 1;
            let skip = self
                .net
                .add_transition(format!("skip_{}", self.created), None::<Label>);
            self.trees.insert(skip, ProcessTree::Tau);
            self.net.add_input_arc(before, skip).expect("live ids");
            self.net.add_output_arc(skip, after).expect("live ids");
            for t in producers {
                self.net.add_output_arc(t, before).expect("live ids");
            }
            for t in consumers {
                self.net.add_input_arc(after, t).expect("live ids");
            }
        }
    }
}

/// Sorts the children of choice and parallel nodes by their string form.
fn sort_children(tree: &mut ProcessTree) {
    if let ProcessTree::Node(op, children) = tree {
        children.iter_mut().for_each(sort_children);
        if matches!(op, Operator::Xor | Operator::Parallel) {
            children.sort_by_cached_key(ToString::to_string);
        }
    }
}

impl AcceptingPetriNet {
    /// Converts a block-structured workflow net to a process tree (pm4py's
    /// `wf_net.variants.to_process_tree.apply`).
    ///
    /// The conversion merges pairs of transitions into choice, sequence,
    /// parallel and loop blocks until one transition is left. When no pair
    /// merges, it splits every inner place of the original net with a
    /// silent transition and tries again. The result is folded
    /// ([`ProcessTree::fold`]) and the children of choice and parallel nodes
    /// are sorted by their string form. pm4py sorts them by a sum of MD5
    /// hashes of their labels.
    ///
    /// The markings are not used: as in pm4py, the source and sink are the
    /// places without inputs and outputs. Arc weights and kinds are ignored.
    /// Pairs are tried in transition-id order; pm4py tries them in hash-set
    /// order. pm4py also passes the subtrees through their string form,
    /// which drops single quotes from labels; here labels stay as they are.
    ///
    /// # Errors
    ///
    /// - [`WfNetToTreeError::NotWorkflowNet`] when the net is not a
    ///   workflow net.
    /// - [`WfNetToTreeError::NotBlockStructured`] when the reduction gets
    ///   stuck with more than one transition.
    pub fn to_process_tree(&self) -> Result<ProcessTree, WfNetToTreeError> {
        if !is_workflow_net(&self.net) {
            return Err(WfNetToTreeError::NotWorkflowNet);
        }
        let trees = self
            .net
            .transitions()
            .map(|(id, t)| {
                let tree = match &t.label {
                    Some(l) => ProcessTree::Activity(l.clone()),
                    None => ProcessTree::Tau,
                };
                (id, tree)
            })
            .collect();
        let original: BTreeSet<PlaceId> = self.net.place_ids().collect();
        let mut r = Reducer {
            net: self.net.clone(),
            trees,
            created: 0,
        };
        while r.net.transition_count() > 1 {
            let sources = r
                .net
                .places()
                .filter(|(_, p)| p.in_arcs().is_empty())
                .count();
            let sinks = r
                .net
                .places()
                .filter(|(_, p)| p.out_arcs().is_empty())
                .count();
            if sources != 1 && sinks != 1 {
                break;
            }
            if r.group_once() {
                continue;
            }
            r.insert_skips(&original);
            if !r.group_once() {
                break;
            }
        }
        if r.net.transition_count() != 1 {
            return Err(WfNetToTreeError::NotBlockStructured);
        }
        let (_, tree) = r.trees.pop_first().expect("one transition is left");
        let mut tree = tree.fold();
        sort_children(&mut tree);
        Ok(tree)
    }
}
