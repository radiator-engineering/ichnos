//! Simplicity of Petri nets, ported from pm4py's
//! `algo/evaluation/simplicity/variants`.

use std::collections::BTreeSet;

use super::AnalysisError;
use crate::petri::{AcceptingPetriNet, PetriNet, ReachabilityOptions};

/// The simplicity measures of `pm4py.simplicity_petri_net`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SimplicityVariant {
    /// [`PetriNet::arc_degree_simplicity`] with `k = 2` (the default).
    #[default]
    ArcDegree,
    /// [`PetriNet::extended_cardoso`].
    ExtendedCardoso,
    /// [`AcceptingPetriNet::extended_cyclomatic`].
    ExtendedCyclomatic,
}

impl PetriNet {
    /// pm4py's `arc_degree` simplicity: `1 / (1 + max(d - k, 0))`, where `d`
    /// is the mean number of arcs per place and transition (0 for an empty
    /// net).
    pub fn arc_degree_simplicity(&self, k: f64) -> f64 {
        let degrees: Vec<usize> = self
            .places()
            .map(|(_, p)| p.in_arcs().len() + p.out_arcs().len())
            .chain(
                self.transitions()
                    .map(|(_, t)| t.in_arcs().len() + t.out_arcs().len()),
            )
            .collect();
        let mean = if degrees.is_empty() {
            0.0
        } else {
            degrees.iter().sum::<usize>() as f64 / degrees.len() as f64
        };
        1.0 / (1.0 + (mean - k).max(0.0))
    }

    /// pm4py's `extended_cardoso` metric: for each place, the number of
    /// distinct sets of places that its output transitions produce into,
    /// summed over places.
    pub fn extended_cardoso(&self) -> usize {
        self.place_ids()
            .map(|p| {
                self.place_postset(p)
                    .map(|t| {
                        self.postset(t)
                            .map(|q| self.place(q).name.clone())
                            .collect::<BTreeSet<_>>()
                    })
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .sum()
    }
}

impl AcceptingPetriNet {
    /// pm4py's `extended_cyclomatic` metric on the reachability graph from
    /// the initial marking.
    ///
    /// pm4py computes edges minus nodes plus strongly connected components
    /// on a graph meant to be the reachability graph. Its graph, though,
    /// joins each state to the *name* of each transition that leaves it,
    /// not to the next state. The transition-name nodes have no outgoing
    /// edges, so each node is its own component, and the metric comes to
    /// the number of distinct (state, transition name) pairs. This returns
    /// that number.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Reachability`] when the reachability graph has more
    /// markings than `options` allows. pm4py does not end on an unbounded
    /// net.
    pub fn extended_cyclomatic(&self, options: ReachabilityOptions) -> Result<i64, AnalysisError> {
        let rg = self
            .net
            .reachability_graph(&self.initial_marking, options)?;
        let pairs: usize = (0..rg.len())
            .map(|i| {
                rg.outgoing(i)
                    .iter()
                    .map(|&(t, _)| self.net.transition(t).name.as_str())
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .sum();
        Ok(i64::try_from(pairs).expect("graph size fits in i64"))
    }

    /// pm4py's `simplicity_petri_net`: the chosen measure as a number.
    ///
    /// # Errors
    ///
    /// As [`AcceptingPetriNet::extended_cyclomatic`], with default options.
    pub fn simplicity(&self, variant: SimplicityVariant) -> Result<f64, AnalysisError> {
        Ok(match variant {
            SimplicityVariant::ArcDegree => self.net.arc_degree_simplicity(2.0),
            SimplicityVariant::ExtendedCardoso => self.net.extended_cardoso() as f64,
            SimplicityVariant::ExtendedCyclomatic => {
                self.extended_cyclomatic(ReachabilityOptions::default())? as f64
            }
        })
    }
}
