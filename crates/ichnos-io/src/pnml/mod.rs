//! PNML Petri nets, accepting markings and stochastic/data-net metadata.

mod read;
mod write;

use ichnos_model::{AcceptingPetriNet, Marking, PlaceId, TransitionId};
use std::collections::BTreeMap;

pub use read::{PnmlReadOptions, read_pnml, read_pnml_from_reader};
pub use write::{PnmlWriteOptions, write_pnml, write_pnml_to_writer};

/// A core accepting net plus PNML information which the model does not store.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PnmlDocument {
    /// The model; the first final marking is its primary final marking.
    pub model: AcceptingPetriNet,
    /// Additional alternative accepting markings, in declaration order.
    pub additional_final_markings: Vec<Marking>,
    /// Transition distributions, keyed by the model's transition ids.
    pub stochastic: BTreeMap<TransitionId, StochasticInfo>,
    /// Place display names, separate from the place id/name in the model.
    pub place_names: BTreeMap<PlaceId, String>,
    /// Transition display names, including the names of silent transitions.
    pub transition_names: BTreeMap<TransitionId, String>,
    /// Data-net guards and read/write declarations.
    pub transition_data: BTreeMap<TransitionId, TransitionData>,
    /// Data-net variable declarations.
    pub variables: Vec<PnmlVariable>,
}

impl From<AcceptingPetriNet> for PnmlDocument {
    fn from(model: AcceptingPetriNet) -> Self {
        Self {
            model,
            ..Default::default()
        }
    }
}

/// StochasticPetriNet tool information. Distribution parameters remain in
/// the format's text representation, without imposing a sampling engine.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StochasticInfo {
    /// Distribution family, for example IMMEDIATE or EXPONENTIAL.
    pub distribution_type: String,
    /// Distribution parameters (usually semicolon-separated numbers).
    pub distribution_parameters: Option<String>,
    /// Transition priority, when declared.
    pub priority: Option<i64>,
    /// Stochastic choice weight, when declared.
    pub weight: Option<f64>,
    /// Other tool properties, retained verbatim.
    pub properties: BTreeMap<String, String>,
}

/// Data Petri net declarations associated with one transition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransitionData {
    /// Guard expression, retained without evaluating it.
    pub guard: Option<String>,
    /// Variables read by this transition.
    pub read_variables: Vec<String>,
    /// Variables written by this transition.
    pub write_variables: Vec<String>,
}

/// A declared data Petri net variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PnmlVariable {
    /// Variable name.
    pub name: String,
    /// The PNML language's type name.
    pub type_name: String,
}
