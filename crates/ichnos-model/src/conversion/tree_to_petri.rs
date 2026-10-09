//! Process tree to Petri net, ported from pm4py's
//! `objects/conversion/process_tree/variants/to_petri_net.py`.

use crate::Label;
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};
use crate::process_tree::{Operator, ProcessTree};

/// Where a subtree attaches: an existing place, or a transition that needs a
/// new place in between.
#[derive(Debug, Clone, Copy)]
enum Entity {
    Place(PlaceId),
    Transition(TransitionId),
}

struct Builder {
    net: PetriNet,
    num_places: usize,
    num_hidden: usize,
    num_visible: usize,
}

impl Builder {
    fn new_place(&mut self) -> PlaceId {
        self.num_places += 1;
        self.net.add_place(format!("p_{}", self.num_places))
    }

    fn new_hidden(&mut self, kind: &str) -> TransitionId {
        self.num_hidden += 1;
        self.net
            .add_transition(format!("{kind}_{}", self.num_hidden), None::<Label>)
    }

    fn new_visible(&mut self, label: &Label) -> TransitionId {
        self.num_visible += 1;
        self.net
            .add_transition(format!("t_{}", self.num_visible), Some(label.clone()))
    }

    fn pt(&mut self, p: PlaceId, t: TransitionId) {
        self.net.add_input_arc(p, t).expect("builder ids are live");
    }

    fn tp(&mut self, t: TransitionId, p: PlaceId) {
        self.net.add_output_arc(t, p).expect("builder ids are live");
    }

    /// pm4py's `recursively_add_tree`. Returns the final place of the subtree.
    fn add(&mut self, tree: &ProcessTree, initial: Entity, fin: Option<Entity>) -> PlaceId {
        let initial_place = match initial {
            Entity::Place(p) => p,
            Entity::Transition(t) => {
                let p = self.new_place();
                self.tp(t, p);
                p
            }
        };
        let final_place = match fin {
            Some(Entity::Place(p)) => p,
            other => {
                let p = self.new_place();
                if let Some(Entity::Transition(t)) = other {
                    self.pt(p, t);
                }
                p
            }
        };
        let (op, children) = match tree {
            ProcessTree::Tau | ProcessTree::Activity(_) => {
                let t = match tree.label() {
                    Some(l) => self.new_visible(l),
                    None => self.new_hidden("skip"),
                };
                self.pt(initial_place, t);
                self.tp(t, final_place);
                return final_place;
            }
            ProcessTree::Node(op, children) => (*op, children),
        };
        match op {
            Operator::Xor => {
                for c in children {
                    self.add(
                        c,
                        Entity::Place(initial_place),
                        Some(Entity::Place(final_place)),
                    );
                }
            }
            Operator::Or => {
                let split = self.new_hidden("tauSplit");
                self.pt(initial_place, split);
                let join = self.new_hidden("tauJoin");
                self.tp(join, final_place);
                let terminal = self.new_place();
                self.pt(terminal, join);
                let first = self.new_place();
                self.tp(split, first);
                for c in children {
                    let init = self.new_place();
                    self.tp(split, init);
                    let start = self.new_place();
                    let end = self.new_place();
                    let t_start = self.new_hidden("inclusiveStart");
                    let t_later = self.new_hidden("inclusiveLater");
                    let t_skip = self.new_hidden("inclusiveSkip");
                    self.pt(first, t_start);
                    self.pt(init, t_start);
                    self.tp(t_start, start);
                    self.tp(t_start, terminal);
                    self.pt(terminal, t_later);
                    self.pt(init, t_later);
                    self.tp(t_later, start);
                    self.tp(t_later, terminal);
                    self.pt(terminal, t_skip);
                    self.pt(init, t_skip);
                    self.tp(t_skip, terminal);
                    self.tp(t_skip, end);
                    self.pt(end, join);
                    self.add(c, Entity::Place(start), Some(Entity::Place(end)));
                }
            }
            Operator::Parallel => {
                let split = self.new_hidden("tauSplit");
                self.pt(initial_place, split);
                let join = self.new_hidden("tauJoin");
                self.tp(join, final_place);
                for c in children {
                    self.add(c, Entity::Transition(split), Some(Entity::Transition(join)));
                }
            }
            Operator::Interleaving => {
                let split = self.new_hidden("tauSplit");
                self.pt(initial_place, split);
                let join = self.new_hidden("tauJoin");
                self.tp(join, final_place);
                let control = self.new_place();
                self.tp(split, control);
                self.pt(control, join);
                for c in children {
                    let place_i = self.new_place();
                    let i_trans = self.new_hidden("iTrans");
                    let place_f = self.new_place();
                    let f_trans = self.new_hidden("fTrans");
                    self.tp(split, place_i);
                    self.pt(place_i, i_trans);
                    self.tp(f_trans, place_f);
                    self.pt(place_f, join);
                    self.pt(control, i_trans);
                    self.tp(f_trans, control);
                    self.add(
                        c,
                        Entity::Transition(i_trans),
                        Some(Entity::Transition(f_trans)),
                    );
                }
            }
            Operator::Sequence => {
                let mut intermediate = initial_place;
                let last = children.len().saturating_sub(1);
                for (i, c) in children.iter().enumerate() {
                    let fin = (i == last).then_some(Entity::Place(final_place));
                    intermediate = self.add(c, Entity::Place(intermediate), fin);
                }
            }
            Operator::Loop => {
                let loop_start = self.new_place();
                let init_loop = self.new_hidden("init_loop");
                self.pt(initial_place, init_loop);
                self.tp(init_loop, loop_start);
                let back = self.new_hidden("loop");
                match children.as_slice() {
                    [] => {}
                    [do_part] => {
                        self.add(
                            do_part,
                            Entity::Place(loop_start),
                            Some(Entity::Place(final_place)),
                        );
                        self.pt(final_place, back);
                        self.tp(back, loop_start);
                    }
                    [do_part, redos @ ..] => {
                        let after_do = self.add(do_part, Entity::Place(loop_start), None);
                        let mut after_redo = None;
                        for r in redos {
                            after_redo = Some(self.add(
                                r,
                                Entity::Place(after_do),
                                after_redo.map(Entity::Place),
                            ));
                        }
                        self.add(
                            &ProcessTree::Tau,
                            Entity::Place(after_do),
                            Some(Entity::Place(final_place)),
                        );
                        let after_redo = after_redo.expect("a loop here has a redo child");
                        self.pt(after_redo, back);
                        self.tp(back, loop_start);
                    }
                }
            }
        }
        final_place
    }
}

/// pm4py's `check_initial_loop`. It recurses with `check_terminal_loop`, as
/// pm4py does.
fn initial_loop(tree: &ProcessTree) -> bool {
    match tree.children().first() {
        Some(ProcessTree::Node(Operator::Loop, _)) => true,
        Some(c @ ProcessTree::Node(..)) => terminal_loop(c),
        _ => false,
    }
}

/// pm4py's `check_terminal_loop`.
fn terminal_loop(tree: &ProcessTree) -> bool {
    match tree.children().last() {
        Some(ProcessTree::Node(Operator::Loop, _)) => true,
        Some(c @ ProcessTree::Node(..)) => terminal_loop(c),
        _ => false,
    }
}

/// pm4py's `check_loop_to_first_operator` / `check_loop_to_last_operator`.
fn loop_along(tree: &ProcessTree, pick: fn(&[ProcessTree]) -> Option<&ProcessTree>) -> bool {
    let mut node = tree;
    loop {
        if node.operator() == Some(Operator::Loop) {
            return true;
        }
        match pick(node.children()) {
            Some(c) => node = c,
            None => return false,
        }
    }
}

fn root_is_xor_or_parallel(tree: &ProcessTree) -> bool {
    matches!(tree.operator(), Some(Operator::Xor | Operator::Parallel))
}

impl ProcessTree {
    /// Converts the tree to an accepting Petri net (pm4py's
    /// `process_tree.converter.apply`, default variant).
    ///
    /// The net has a `source` place with the initial token and a `sink`
    /// place for the final marking. Silent transitions are named after their
    /// role (`tauSplit_3`, `skip_5`, ...) as in pm4py. Visible transitions are
    /// named `t_1`, `t_2`, ... where pm4py uses random UUIDs. The net is named
    /// `process_tree_net`; pm4py names it `imdf_net_<timestamp>`.
    pub fn to_petri_net(&self) -> AcceptingPetriNet {
        let mut b = Builder {
            net: PetriNet::new("process_tree_net"),
            num_places: 0,
            num_hidden: 0,
            num_visible: 0,
        };
        b.num_places += 1;
        let source = b.net.add_place("source");
        b.num_places += 1;
        let sink = b.net.add_place("sink");

        let initial_mandatory = initial_loop(self)
            || loop_along(self, <[ProcessTree]>::first)
            || root_is_xor_or_parallel(self);
        let final_mandatory = terminal_loop(self)
            || loop_along(self, <[ProcessTree]>::last)
            || root_is_xor_or_parallel(self);

        let initial_place = if initial_mandatory {
            let p = b.new_place();
            let t = b.new_hidden("tau");
            b.pt(source, t);
            b.tp(t, p);
            p
        } else {
            source
        };
        let final_place = if final_mandatory {
            let p = b.new_place();
            let t = b.new_hidden("tau");
            b.pt(p, t);
            b.tp(t, sink);
            p
        } else {
            sink
        };
        b.add(
            self,
            Entity::Place(initial_place),
            Some(Entity::Place(final_place)),
        );

        let mut net = b.net;
        net.apply_simple_reduction();
        let dangling: Vec<PlaceId> = net
            .places()
            .filter(|&(id, p)| {
                (p.out_arcs().is_empty() && id != sink) || (p.in_arcs().is_empty() && id != source)
            })
            .map(|(id, _)| id)
            .collect();
        for p in dangling {
            net.remove_place(p);
        }
        let map = net.compact();
        AcceptingPetriNet::new(
            net,
            map.marking(&Marking::from([(source, 1)])),
            map.marking(&Marking::from([(sink, 1)])),
        )
    }
}
