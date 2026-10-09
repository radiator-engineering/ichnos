//! Object-centric Petri nets (pm4py's `discover_oc_petri_net`, from
//! `algo/discovery/ocel/ocpn`).
//!
//! The miner flattens the log on each object type, mines that log with the
//! inductive miner, and converts the tree to a Petri net. The nets share the
//! log's activities as visible transitions. An activity gets double arcs for
//! a type when its events often hold several objects of that type.
//!
//! Reference: W. M. P. van der Aalst and A. Berti, "Discovering
//! object-centric Petri nets", Fundamenta Informaticae 175 (2020).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use ichnos_core::{ActivityIndex, ActivitySequences, Variants};
use ichnos_model::{AcceptingPetriNet, Dfg, ProcessTree};
use ichnos_ocel::Ocel;
use ichnos_stats::ocel::{ActivityAssociations, Prefilter, act_ot_dependent, edge_metrics};

use crate::Result;
use crate::inductive::{
    InductiveOptions, InductiveVariant, process_tree_inductive_dfg, process_tree_inductive_variants,
};

/// Options for [`discover_oc_petri_net`], with pm4py's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OcpnOptions {
    /// The inductive miner run on each object type. The default is IM with
    /// fall-throughs and the strict sequence cut disabled, as in pm4py's
    /// `discover_oc_petri_net`.
    ///
    /// pm4py's `inductive_miner_variant` and `noise_threshold` map to
    /// [`InductiveOptions::variant`]: `"im"` with a threshold of 0 is
    /// [`InductiveVariant::Im`]; `"imf"`, or `"im"` with a threshold above 0,
    /// is [`InductiveVariant::Imf`]; `"imd"` is [`InductiveVariant::Imd`].
    /// IM and IMf mine each type's flattened log; IMd mines the type's
    /// directly-follows graph.
    pub inductive: InductiveOptions,
    /// An activity gets double arcs for an object type when the share of its
    /// events that hold exactly one object of that type is below this value
    /// (pm4py's `double_arc_threshold`). Default 0.8.
    pub double_arc_threshold: f64,
}

impl Default for OcpnOptions {
    fn default() -> Self {
        Self::new(InductiveVariant::Im)
    }
}

impl OcpnOptions {
    /// Options that run `variant` on each object type, with pm4py's other
    /// defaults.
    pub fn new(variant: InductiveVariant) -> Self {
        Self {
            inductive: InductiveOptions::new(variant)
                .with_fallthroughs_disabled(true)
                .with_strict_sequence_cut_disabled(true),
            double_arc_threshold: 0.8,
        }
    }

    /// Sets [`OcpnOptions::inductive`].
    pub fn with_inductive(mut self, inductive: InductiveOptions) -> Self {
        self.inductive = inductive;
        self
    }

    /// Sets [`OcpnOptions::double_arc_threshold`].
    pub fn with_double_arc_threshold(mut self, threshold: f64) -> Self {
        self.double_arc_threshold = threshold;
        self
    }
}

/// An object-centric Petri net: one accepting Petri net per object type,
/// whose visible transitions share the log's activities.
#[derive(Debug, Clone, PartialEq)]
pub struct OcPetriNet {
    /// The activities of the log's events (pm4py's `activities`).
    pub activities: BTreeSet<String>,
    /// The net of each object type in the log's objects table. The keys are
    /// pm4py's `object_types`.
    pub nets: BTreeMap<String, ObjectTypeNet>,
}

/// The part of an [`OcPetriNet`] for one object type.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTypeNet {
    /// The process tree the inductive miner found for this type. pm4py
    /// converts it and does not keep it.
    pub tree: ProcessTree,
    /// The tree as an accepting Petri net (pm4py's `petri_nets`).
    pub net: AcceptingPetriNet,
    /// For each activity whose events hold objects of this type, whether
    /// its arcs are double (pm4py's `double_arcs_on_activity`). They are
    /// when the share of the activity's events with exactly one object of
    /// the type is below [`OcpnOptions::double_arc_threshold`].
    pub double_arcs: BTreeMap<String, bool>,
    /// The objects of this type that some event relates to (pm4py's
    /// `object_ids`).
    pub object_ids: BTreeSet<String>,
}

/// Discovers an object-centric Petri net (pm4py's `discover_oc_petri_net`).
///
/// For each object type, IM and IMf mine the log flattened on that type:
/// one trace per object, holding its events by timestamp, with ties in
/// event order. IMd mines the type's directly-follows graph, with start and
/// end activities counted in events. A type with no relations gets the
/// miner's tree for an empty log, where pm4py raises `KeyError`.
///
/// pm4py's `diagnostics_with_tbr` is not ported: it needs token replay with
/// place-level counts, which `ichnos-conformance` does not have.
///
/// Fails if a relation names an event or object the log does not hold, or if
/// the IMf noise threshold is not in `[0, 1]`.
pub fn discover_oc_petri_net(ocel: &Ocel, options: &OcpnOptions) -> Result<OcPetriNet> {
    let associations = act_ot_dependent::find_associations_from_ocel(ocel, Prefilter::None)?;
    let mut event_counts: HashMap<&str, usize> = HashMap::new();
    for e in &ocel.events {
        *event_counts.entry(&*e.activity).or_default() += 1;
    }
    let mut dfgs = match options.inductive.variant {
        InductiveVariant::Imd => object_type_dfgs(ocel)?,
        _ => BTreeMap::new(),
    };
    let mut flat = match options.inductive.variant {
        InductiveVariant::Imd => BTreeMap::new(),
        _ => flattened_traces(ocel),
    };

    let none = ActivityAssociations::new();
    let types: BTreeSet<&str> = ocel.objects.iter().map(|o| &*o.object_type).collect();
    let mut nets = BTreeMap::new();
    for ot in types {
        let associations = associations.get(ot).unwrap_or(&none);
        let tree = match options.inductive.variant {
            InductiveVariant::Imd => {
                process_tree_inductive_dfg(&dfgs.remove(ot).unwrap_or_default(), &options.inductive)
            }
            _ => process_tree_inductive_variants(
                &variants(ocel, flat.remove(ot).unwrap_or_default()),
                &options.inductive,
            )?,
        };
        let net = tree.to_petri_net();
        nets.insert(
            ot.to_owned(),
            ObjectTypeNet {
                tree,
                net,
                double_arcs: double_arcs(associations, &event_counts, options.double_arc_threshold),
                object_ids: associations
                    .values()
                    .flatten()
                    .map(|(_, o)| o.to_string())
                    .collect(),
            },
        );
    }
    Ok(OcPetriNet {
        activities: ocel.events.iter().map(|e| e.activity.to_string()).collect(),
        nets,
    })
}

/// Whether each activity's arcs are double for one type: the share of the
/// activity's events with exactly one related object of the type, among all
/// its events, is below `threshold`.
fn double_arcs(
    associations: &ActivityAssociations,
    event_counts: &HashMap<&str, usize>,
    threshold: f64,
) -> BTreeMap<String, bool> {
    associations
        .iter()
        .map(|(activity, pairs)| {
            let mut objects: HashMap<&str, usize> = HashMap::new();
            for (event, _) in pairs {
                *objects.entry(&**event).or_default() += 1;
            }
            let single = objects.values().filter(|&&n| n == 1).count();
            let total = event_counts.get(&**activity).copied().unwrap_or(0);
            let score = if total == 0 {
                0.0
            } else {
                single as f64 / total as f64
            };
            (activity.to_string(), score < threshold)
        })
        .collect()
}

/// The event rows of each object, by object type and object id, as pm4py's
/// `flattening.flatten` and the inductive miner's projection order them: by
/// timestamp, ties in event order. A repeated relation counts once.
fn flattened_traces(ocel: &Ocel) -> BTreeMap<&str, BTreeMap<&str, Vec<usize>>> {
    let types = ocel.object_index();
    let mut rows: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in ocel.events.iter().enumerate() {
        rows.entry(&*e.id).or_default().push(i);
    }
    let mut seen = HashSet::new();
    let mut traces: BTreeMap<&str, BTreeMap<&str, Vec<usize>>> = BTreeMap::new();
    for r in &ocel.relations {
        let (Some(&o), Some(rows)) = (types.get(&*r.object), rows.get(&*r.event)) else {
            continue;
        };
        if seen.insert((&*r.object, &*r.event)) {
            traces
                .entry(&*ocel.objects[o].object_type)
                .or_default()
                .entry(&*r.object)
                .or_default()
                .extend(rows);
        }
    }
    for trace in traces.values_mut().flat_map(BTreeMap::values_mut) {
        trace.sort_by_key(|&i| (ocel.events[i].timestamp, i));
    }
    traces
}

/// The variants of one type's flattened traces.
fn variants(ocel: &Ocel, traces: BTreeMap<&str, Vec<usize>>) -> Variants {
    let mut activities = ActivityIndex::new();
    let traces = traces
        .into_values()
        .map(|rows| {
            rows.into_iter()
                .map(|i| activities.intern(&ocel.events[i].activity))
                .collect()
        })
        .collect();
    Variants::from_sequences(ActivitySequences { activities, traces })
}

/// The directly-follows graph of each object type, as pm4py's OC-DFG gives
/// it to IMd: edges counted in distinct event pairs, start and end
/// activities in events.
fn object_type_dfgs(ocel: &Ocel) -> Result<BTreeMap<String, Dfg>> {
    let mut dfgs: BTreeMap<String, Dfg> = BTreeMap::new();
    let edges = edge_metrics::find_associations_per_edge(ocel)?;
    for (ot, edges) in edge_metrics::aggregate_ev_couples(&edges) {
        let dfg = dfgs.entry(ot.to_string()).or_default();
        for ((a, b), couples) in edges {
            dfg.add_edge(&*a, &*b, couples.len() as u64);
        }
    }
    for prefilter in [Prefilter::Start, Prefilter::End] {
        let found = act_ot_dependent::find_associations_from_ocel(ocel, prefilter)?;
        for (ot, activities) in act_ot_dependent::aggregate_events(&found) {
            let dfg = dfgs.entry(ot.to_string()).or_default();
            for (a, events) in activities {
                let n = events.len() as u64;
                match prefilter {
                    Prefilter::Start => dfg.add_start(&*a, n),
                    _ => dfg.add_end(&*a, n),
                }
            }
        }
    }
    Ok(dfgs)
}

#[cfg(test)]
mod tests {
    use ichnos_core::chrono::{DateTime, TimeDelta};
    use ichnos_ocel::{EventObject, OcelEvent, OcelObject};

    use super::*;

    /// A log with events `(id, activity, objects)`, one second apart, and
    /// objects `(id, type)`.
    fn ocel(events: &[(&str, &str, &[&str])], objects: &[(&str, &str)]) -> Ocel {
        let mut log = Ocel::default();
        for (i, (id, activity, objs)) in events.iter().enumerate() {
            log.events.push(OcelEvent {
                id: (*id).into(),
                activity: (*activity).into(),
                timestamp: DateTime::from_timestamp(i64::try_from(i).unwrap(), 0)
                    .unwrap()
                    .fixed_offset(),
                attributes: Default::default(),
            });
            for o in *objs {
                log.relations.push(EventObject {
                    event: (*id).into(),
                    object: (*o).into(),
                    qualifier: None,
                });
            }
        }
        for (id, ot) in objects {
            log.objects.push(OcelObject {
                id: (*id).into(),
                object_type: (*ot).into(),
                attributes: Default::default(),
            });
        }
        log
    }

    /// Two orders, each placed and then shipped together with its items.
    fn orders() -> Ocel {
        ocel(
            &[
                ("e1", "place", &["o1", "i1", "i2"]),
                ("e2", "pick", &["i1"]),
                ("e3", "pick", &["i2"]),
                ("e4", "ship", &["o1", "i1", "i2"]),
                ("e5", "place", &["o2", "i3"]),
                ("e6", "pick", &["i3"]),
                ("e7", "ship", &["o2", "i3"]),
            ],
            &[
                ("o1", "order"),
                ("o2", "order"),
                ("i1", "item"),
                ("i2", "item"),
                ("i3", "item"),
                ("c1", "customer"),
            ],
        )
    }

    fn tree(s: &str) -> ProcessTree {
        ProcessTree::parse(s).unwrap()
    }

    #[test]
    fn mines_each_object_type() {
        for variant in [InductiveVariant::Im, InductiveVariant::Imd] {
            let ocpn = discover_oc_petri_net(&orders(), &OcpnOptions::new(variant)).unwrap();
            assert_eq!(
                ocpn.activities.iter().collect::<Vec<_>>(),
                ["pick", "place", "ship"]
            );
            assert_eq!(
                ocpn.nets.keys().collect::<Vec<_>>(),
                ["customer", "item", "order"]
            );
            let order = &ocpn.nets["order"];
            assert_eq!(order.tree, tree("->( 'place', 'ship' )"), "{variant:?}");
            assert_eq!(order.net, order.tree.to_petri_net());
            assert_eq!(
                ocpn.nets["item"].tree,
                tree("->( 'place', 'pick', 'ship' )"),
                "{variant:?}"
            );
            let ids = |ot: &str| ocpn.nets[ot].object_ids.iter().cloned().collect::<Vec<_>>();
            assert_eq!(ids("item"), ["i1", "i2", "i3"]);
            assert_eq!(ids("order"), ["o1", "o2"]);
            assert!(ids("customer").is_empty());
            // A type without relations gets the tree of an empty log.
            assert_eq!(ocpn.nets["customer"].tree, tree("tau"), "{variant:?}");
        }
    }

    #[test]
    fn double_arcs_follow_the_threshold() {
        let ocpn = discover_oc_petri_net(&orders(), &OcpnOptions::default()).unwrap();
        // Item: place and ship hold two items in one of their two events, so
        // half their events hold one item; every pick holds one.
        let item = &ocpn.nets["item"].double_arcs;
        assert_eq!(
            item.iter()
                .map(|(a, &d)| (a.as_str(), d))
                .collect::<Vec<_>>(),
            [("pick", false), ("place", true), ("ship", true)]
        );
        // Order: each place and ship holds one order; pick holds none.
        let order = &ocpn.nets["order"].double_arcs;
        assert_eq!(
            order
                .iter()
                .map(|(a, &d)| (a.as_str(), d))
                .collect::<Vec<_>>(),
            [("place", false), ("ship", false)]
        );
        assert!(ocpn.nets["customer"].double_arcs.is_empty());

        let ocpn = discover_oc_petri_net(
            &orders(),
            &OcpnOptions::default().with_double_arc_threshold(0.5),
        )
        .unwrap();
        assert!(!ocpn.nets["item"].double_arcs["place"]);
    }

    #[test]
    fn flattening_orders_by_timestamp_then_event() {
        let mut log = ocel(
            &[
                ("e1", "b", &["o1"]),
                ("e2", "a", &["o1"]),
                ("e3", "c", &["o1", "o1"]),
            ],
            &[("o1", "t")],
        );
        // e2 happens first; e1 and e3 tie, so event order breaks the tie.
        log.events[1].timestamp = log.events[0].timestamp - TimeDelta::seconds(1);
        log.events[2].timestamp = log.events[0].timestamp;
        let ocpn = discover_oc_petri_net(&log, &OcpnOptions::default()).unwrap();
        assert_eq!(ocpn.nets["t"].tree, tree("->( 'a', 'b', 'c' )"));
    }

    #[test]
    fn checks_relations_and_noise() {
        let mut log = orders();
        log.relations.push(EventObject {
            event: "e9".into(),
            object: "o1".into(),
            qualifier: None,
        });
        assert!(discover_oc_petri_net(&log, &OcpnOptions::default()).is_err());

        let imf = OcpnOptions::new(InductiveVariant::Imf {
            noise_threshold: 1.5,
        });
        assert!(matches!(
            discover_oc_petri_net(&orders(), &imf),
            Err(crate::Error::NoiseThreshold(_))
        ));
    }
}
