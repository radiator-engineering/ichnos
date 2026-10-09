//! Workflow-net check, ported from pm4py's
//! `algo/analysis/workflow_net/variants/petri_net.py`.

use crate::petri::{PetriNet, PlaceId};

/// Why a net cannot be short-circuited: it needs exactly one place without
/// input arcs and one without output arcs. The order of the checks and the
/// messages are woflan's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortCircuitError {
    NoSink,
    NoSource,
    ManySources,
    ManySinks,
}

impl ShortCircuitError {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NoSink => "There is no sink place.",
            Self::NoSource => "There is no source place.",
            Self::ManySources => "There is more than one source place.",
            Self::ManySinks => "There is more than one sink place.",
        }
    }
}

/// The source and sink place of a net with exactly one of each.
pub(crate) fn source_and_sink(net: &PetriNet) -> Result<(PlaceId, PlaceId), ShortCircuitError> {
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
    match (sources.as_slice(), sinks.as_slice()) {
        (&[source], &[sink]) => Ok((source, sink)),
        (_, []) => Err(ShortCircuitError::NoSink),
        ([], _) => Err(ShortCircuitError::NoSource),
        ([_, _, ..], _) => Err(ShortCircuitError::ManySources),
        _ => Err(ShortCircuitError::ManySinks),
    }
}

/// Copies `net` and adds a transition `short_circuited_transition` (with the
/// same label) from the sink to the source.
pub(crate) fn short_circuit(net: &PetriNet) -> Result<PetriNet, ShortCircuitError> {
    let (source, sink) = source_and_sink(net)?;
    let mut sc = net.clone();
    let t = sc.add_transition(
        "short_circuited_transition",
        Some("short_circuited_transition"),
    );
    sc.add_input_arc(sink, t).expect("live ids");
    sc.add_output_arc(t, source).expect("live ids");
    Ok(sc)
}

/// Whether every place and transition reaches every other one.
pub(crate) fn is_strongly_connected(net: &PetriNet) -> bool {
    let places: Vec<PlaceId> = net.place_ids().collect();
    let np = places.len();
    let mut place_index = vec![usize::MAX; net.place_index_bound()];
    for (i, p) in places.iter().enumerate() {
        place_index[p.index()] = i;
    }
    let n = np + net.transition_count();
    if n == 0 {
        return false;
    }
    let mut succ = vec![Vec::new(); n];
    let mut pred = vec![Vec::new(); n];
    for (k, t) in net.transition_ids().enumerate() {
        for p in net.preset(t) {
            succ[place_index[p.index()]].push(np + k);
            pred[np + k].push(place_index[p.index()]);
        }
        for p in net.postset(t) {
            succ[np + k].push(place_index[p.index()]);
            pred[place_index[p.index()]].push(np + k);
        }
    }
    let reaches_all = |adj: &[Vec<usize>]| {
        let mut seen = vec![false; n];
        let mut stack = vec![0];
        seen[0] = true;
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

impl PetriNet {
    /// Returns `true` if the net is a workflow net (pm4py's
    /// `check_is_workflow_net`): it has exactly one place without input
    /// arcs and one without output arcs, and adding a transition from the
    /// sink to the source makes it strongly connected.
    ///
    /// pm4py builds its graph on node names, so a place and a transition
    /// with the same name are one node there; here they stay apart.
    pub fn is_workflow_net(&self) -> bool {
        short_circuit(self).is_ok_and(|sc| is_strongly_connected(&sc))
    }
}
