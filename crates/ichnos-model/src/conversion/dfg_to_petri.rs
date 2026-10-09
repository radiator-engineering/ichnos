//! DFG to Petri net, ported from pm4py's `objects/conversion/dfg/variants/`.

use std::collections::{BTreeMap, BTreeSet};

use crate::Label;
use crate::dfg::Dfg;
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};

/// pm4py's `constants.DEFAULT_ARTIFICIAL_START_ACTIVITY`.
pub const ARTIFICIAL_START: &str = "▶";
/// pm4py's `constants.DEFAULT_ARTIFICIAL_END_ACTIVITY`.
pub const ARTIFICIAL_END: &str = "■";

impl Dfg {
    /// The start activities, or the inferred ones (activities with an
    /// outgoing edge but no incoming edge) when none are set.
    fn starts_or_inferred(&self) -> BTreeMap<Label, u64> {
        if self.start_activities.is_empty() {
            self.infer_start_activities()
                .into_iter()
                .map(|a| (a, 1))
                .collect()
        } else {
            self.start_activities.clone()
        }
    }

    fn ends_or_inferred(&self) -> BTreeMap<Label, u64> {
        if self.end_activities.is_empty() {
            self.infer_end_activities()
                .into_iter()
                .map(|a| (a, 1))
                .collect()
        } else {
            self.end_activities.clone()
        }
    }

    /// Converts the DFG to a Petri net with one place per activity (pm4py's
    /// default `to_petri_net_activity_defines_place`, used by
    /// `pm4py.convert_to_petri_net`).
    ///
    /// Each edge `a -> b` becomes a transition labelled `b` from the place of
    /// `a` to the place of `b`. Start activities get a transition from
    /// `source`; end activities a silent transition to `sink`. When the DFG
    /// has no start (or end) activities they are inferred from the edges.
    /// pm4py infers them only when the argument is missing: given empty
    /// dicts, as `pm4py.convert_to_petri_net(dfg, {}, {})` passes, it builds
    /// no start or end transitions. Activities without an edge are left out,
    /// as in pm4py.
    pub fn to_petri_net(&self) -> AcceptingPetriNet {
        let mut net = PetriNet::new("");
        let source = net.add_place("source");
        let sink = net.add_place("sink");
        let places: BTreeMap<Label, PlaceId> = self
            .edge_activities()
            .into_iter()
            .map(|a| {
                let p = net.add_place(a.as_str());
                (a, p)
            })
            .collect();
        let mut index = 0usize;
        let mut name = |a: &Label| {
            index += 1;
            format!("{a}_{index}")
        };
        for a in self.starts_or_inferred().keys() {
            if let Some(&p) = places.get(a) {
                let t = net.add_transition(name(a), Some(a.clone()));
                net.add_input_arc(source, t).expect("live ids");
                net.add_output_arc(t, p).expect("live ids");
            }
        }
        for a in self.ends_or_inferred().keys() {
            if let Some(&p) = places.get(a) {
                let t = net.add_transition(name(a), None::<Label>);
                net.add_input_arc(p, t).expect("live ids");
                net.add_output_arc(t, sink).expect("live ids");
            }
        }
        for (a, b) in self.graph.keys() {
            let t = net.add_transition(name(b), Some(b.clone()));
            net.add_input_arc(places[a], t).expect("live ids");
            net.add_output_arc(t, places[b]).expect("live ids");
        }
        AcceptingPetriNet::new(
            net,
            Marking::from([(source, 1)]),
            Marking::from([(sink, 1)]),
        )
    }

    /// Converts the DFG to a Petri net with one transition per activity and
    /// a silent transition per edge (pm4py's
    /// `to_petri_net_invisibles_no_duplicates`).
    ///
    /// The artificial start and end activities [`ARTIFICIAL_START`] and
    /// [`ARTIFICIAL_END`] become silent transitions. Start and end activities
    /// are inferred as in [`Dfg::to_petri_net`].
    ///
    /// When there is still no start (end) activity, for example on an empty
    /// DFG or one where every activity is on a cycle, the initial (final)
    /// marking is empty. pm4py raises `KeyError` then.
    pub fn to_petri_net_invisibles_no_duplicates(&self) -> AcceptingPetriNet {
        let start = Label::from(ARTIFICIAL_START);
        let end = Label::from(ARTIFICIAL_END);
        let mut edges: BTreeSet<(Label, Label)> = self.graph.keys().cloned().collect();
        for a in self.starts_or_inferred().keys() {
            edges.insert((start.clone(), a.clone()));
        }
        for a in self.ends_or_inferred().keys() {
            edges.insert((a.clone(), end.clone()));
        }
        let activities: BTreeSet<&Label> = edges.iter().flat_map(|(a, b)| [a, b]).collect();

        let mut net = PetriNet::new("");
        let mut left: BTreeMap<&Label, PlaceId> = BTreeMap::new();
        let mut right: BTreeMap<&Label, PlaceId> = BTreeMap::new();
        for &a in &activities {
            let l = net.add_place(format!("source_{a}"));
            let r = net.add_place(format!("sink_{a}"));
            let label = (*a != start && *a != end).then(|| a.clone());
            let t: TransitionId = net.add_transition(format!("trans_{a}"), label);
            net.add_input_arc(l, t).expect("live ids");
            net.add_output_arc(t, r).expect("live ids");
            left.insert(a, l);
            right.insert(a, r);
        }
        for (a, b) in &edges {
            let t = net.add_transition(format!("{a}_{b}"), None::<Label>);
            net.add_input_arc(right[a], t).expect("live ids");
            net.add_output_arc(t, left[b]).expect("live ids");
        }
        let im = left
            .get(&start)
            .map(|&p| Marking::from([(p, 1)]))
            .unwrap_or_default();
        let fm = right
            .get(&end)
            .map(|&p| Marking::from([(p, 1)]))
            .unwrap_or_default();
        AcceptingPetriNet::new(net, im, fm)
    }
}
