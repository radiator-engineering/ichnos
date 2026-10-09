//! Organizational mining: social networks, roles and resource profiles.
//!
//! Ports pm4py's `pm4py.org` functions. Each takes an
//! [`EventLog`](ichnos_core::EventLog) and the [`EventKeys`](ichnos_core::EventKeys)
//! that name its activity, resource and timestamp attributes.
//!
//! | pm4py | ichnos |
//! | --- | --- |
//! | `discover_handover_of_work_network` | [`discover_handover_of_work_network`] |
//! | `discover_working_together_network` | [`discover_working_together_network`] |
//! | `discover_activity_based_resource_similarity` | [`discover_activity_based_resource_similarity`] |
//! | `discover_subcontracting_network` | [`discover_subcontracting_network`] |
//! | `discover_organizational_roles` | [`discover_organizational_roles`] |
//! | `discover_network_analysis` | [`discover_network_analysis`], [`discover_network_analysis_performance`] |
//!
//! Attribute values are compared and reported in their Python `str` form,
//! so a resource `5` and a resource `"5"` are the same node.

mod error;
mod network_analysis;
mod roles;
mod sna;

pub use error::{Error, Result};
pub use network_analysis::{
    EdgeReference, NetworkAnalysis, NetworkAnalysisOptions, discover_network_analysis,
    discover_network_analysis_performance,
};
pub use roles::{Role, discover_organizational_roles};
pub use sna::{
    Sna, discover_activity_based_resource_similarity, discover_handover_of_work_network,
    discover_subcontracting_network, discover_working_together_network,
};
