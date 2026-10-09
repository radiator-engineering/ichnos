//! Petri net reachability graph to transition system, ported from pm4py's
//! `petri_net/utils/reachability_graph.py` (`construct_reachability_graph`).

use crate::petri::{Marking, PetriNet, ReachabilityError, ReachabilityOptions, TransitionId};
use crate::transition_system::{StateId, TransitionSystem};

/// How [`PetriNet::to_transition_system`] names its edges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EdgeNaming {
    /// pm4py's `repr` of the transition: `(name, 'label')`, or
    /// `(name, None)` for a silent transition. pm4py's default.
    #[default]
    Repr,
    /// The transition name (pm4py's `use_trans_name=True`).
    Name,
}

/// pm4py's `staterep`: drops every character that is not a letter, digit or
/// `_`.
fn state_name(net: &PetriNet, m: &Marking) -> String {
    m.display(net)
        .to_string()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn edge_name(net: &PetriNet, t: TransitionId, naming: EdgeNaming) -> String {
    let tr = net.transition(t);
    match (naming, &tr.label) {
        (EdgeNaming::Name, _) => tr.name.clone(),
        (EdgeNaming::Repr, Some(l)) => format!("({}, '{l}')", tr.name),
        (EdgeNaming::Repr, None) => format!("({}, None)", tr.name),
    }
}

impl PetriNet {
    /// Builds the reachability graph from `initial` as a transition system
    /// (pm4py's `construct_reachability_graph`, used by
    /// `pm4py.convert_to_reachability_graph`).
    ///
    /// There is one state per reachable marking, in discovery order, so
    /// state 0 is the initial marking. A state is named after its marking
    /// as pm4py does: the marking's `repr` with every character that is not
    /// a letter, digit or `_` removed, so `['p1:1', 'p2:2']` becomes
    /// `p11p22`. Two markings can get the same name; they stay separate
    /// states here.
    ///
    /// Fails if the net has more than `options.max_markings` reachable
    /// markings.
    pub fn to_transition_system(
        &self,
        initial: &Marking,
        options: ReachabilityOptions,
        naming: EdgeNaming,
    ) -> Result<TransitionSystem, ReachabilityError> {
        let graph = self.reachability_graph(initial, options)?;
        let mut ts = TransitionSystem::new("");
        let states: Vec<StateId> = graph
            .markings()
            .iter()
            .map(|m| ts.add_state(state_name(self, m)))
            .collect();
        for (s, t, d) in graph.edges() {
            ts.add_edge(edge_name(self, t, naming), states[s], states[d])
                .expect("states were just added");
        }
        Ok(ts)
    }
}
