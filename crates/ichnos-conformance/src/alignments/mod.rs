//! Alignment-based conformance checking (pm4py's
//! `algo/conformance/alignments`).
//!
//! An alignment pairs the events of a trace with the transitions of a model
//! run. Each step is a *synchronous move* (event and transition share a
//! label), a *log move* (an event the model cannot follow) or a *model move*
//! (a transition with no event). Moves have costs; an optimal alignment is a
//! cheapest one. pm4py's standard costs are 10000 for a log move and for a
//! model move on a visible transition, 1 for a model move on a silent
//! transition and 0 for a synchronous move ([`costs`]).
//!
//! # Petri nets
//!
//! [`Aligner`] prepares an accepting Petri net once and aligns traces
//! against it. The search runs on the synchronous product of trace and net
//! and is exact: [`Heuristic::StateEquation`] (A* with the marking-equation
//! LP heuristic, pm4py's default) and [`Heuristic::None`] (Dijkstra) find
//! alignments of the same, optimal cost.
//!
//! ```
//! use ichnos_conformance::alignments::{Aligner, AlignmentOptions, Move};
//! use ichnos_model::{Marking, PetriNet};
//!
//! // source -> a -> sink
//! let mut net = PetriNet::new("n");
//! let source = net.add_place("source");
//! let sink = net.add_place("sink");
//! let a = net.add_transition("a", Some("a"));
//! net.add_input_arc(source, a).unwrap();
//! net.add_output_arc(a, sink).unwrap();
//! let im = Marking::from([(source, 1)]);
//! let fm = Marking::from([(sink, 1)]);
//!
//! let aligner = Aligner::new(&net, &im, &fm, AlignmentOptions::default()).unwrap();
//! let alignment = aligner.align(&["a", "b"]).unwrap().expect("no time limit");
//! assert_eq!(alignment.cost, 10_000);
//! // Two log moves plus the cheapest model run (one visible move).
//! assert_eq!(alignment.best_worst_cost, 30_000);
//! assert_eq!(alignment.fitness, 1.0 - 1.0 / 3.0);
//! assert_eq!(
//!     alignment.moves,
//!     [Move::Sync { event: 0, transition: a }, Move::Log { event: 1 }],
//! );
//! ```
//!
//! [`align_log`], [`fitness_alignments`] and [`precision_alignments`] are
//! the log-level entry points of pm4py's simplified interface.

pub mod costs;
mod marking;
mod petri_net;
mod precision;
mod result;
mod search;
mod simplex;
mod state_equation;
mod sync_product;

pub use costs::ModelCosts;
pub use petri_net::{Aligner, AlignmentOptions, Heuristic, align_log, fitness_alignments};
pub use precision::precision_alignments;
pub use result::{AlignmentFitness, LogAlignment, Move, TraceAlignment};
