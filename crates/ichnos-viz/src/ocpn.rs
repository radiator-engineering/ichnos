//! Object-centric Petri nets, ported from pm4py's
//! `visualization/ocel/ocpn/variants/wo_decoration.py`.

use std::collections::BTreeMap;

use ichnos_model::petri::ArcEnds;
use ichnos_model::{Marking, PetriNet, PlaceId, TransitionId};

use crate::dot::{Dot, title_label};
use crate::ocdfg::color_of;

/// Token counts of one place after token-based replay (pm4py's
/// `tbr_results` place diagnostics).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlaceDiagnostics {
    /// Produced tokens.
    pub produced: u64,
    /// Missing tokens.
    pub missing: u64,
    /// Consumed tokens.
    pub consumed: u64,
    /// Remaining tokens.
    pub remaining: u64,
}

/// Token-based replay results for the net of one object type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OcpnDiagnostics {
    /// Token counts by place.
    pub places: BTreeMap<PlaceId, PlaceDiagnostics>,
    /// How often each transition fired.
    pub transitions: BTreeMap<TransitionId, u64>,
}

/// The accepting Petri net of one object type in an object-centric Petri
/// net.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTypeNet {
    /// The net.
    pub net: PetriNet,
    /// The initial marking.
    pub initial: Marking,
    /// The final marking.
    pub fin: Marking,
    /// Activities whose events can hold several objects of this type
    /// (pm4py's `double_arcs_on_activity`). Their arcs are drawn thick.
    pub double_arcs: BTreeMap<String, bool>,
    /// Replay results, when the net was discovered with them.
    pub diagnostics: Option<OcpnDiagnostics>,
}

/// An object-centric Petri net: the parts of pm4py's `discover_oc_petri_net`
/// dictionary that its drawing reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcPetriNet {
    /// The activities, each drawn once and shared by the nets.
    pub activities: Vec<String>,
    /// The net of each object type.
    pub nets: BTreeMap<String, ObjectTypeNet>,
}

/// Options for [`ocpn_dot`], with pm4py's defaults.
#[derive(Debug, Clone)]
pub struct OcpnDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// Colours by object type. Other types get
    /// [`object_type_color`](crate::object_type_color).
    pub object_type_colors: BTreeMap<String, String>,
}

impl Default for OcpnDotOptions {
    fn default() -> Self {
        OcpnDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            object_type_colors: BTreeMap::new(),
        }
    }
}

/// The DOT text of an object-centric Petri net, as pm4py's
/// `save_vis_ocpn` draws it.
///
/// Each activity is one box, shared by the nets of all object types. Places
/// and silent transitions are filled with their object type's colour, and
/// so are the arcs; arcs of activities with several objects per event are
/// thick. Initial places show the object type's name; final places
/// underline it. With replay results, other places show their token counts
/// and arcs show how often their transition fired.
///
/// pm4py fails when a visible transition's label is not among the
/// activities; this draws a box for it instead.
pub fn ocpn_dot(ocpn: &OcPetriNet, options: &OcpnDotOptions) -> String {
    let s = |v: &str| Some(v.to_owned());
    let mut dot = Dot::new(true, false, "ocpn", vec![("bgcolor", s(&options.bgcolor))]);
    dot.defaults(
        "node",
        vec![("shape", s("ellipse")), ("fixedsize", s("false"))],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, 10))),
            ("labelloc", s("top")),
        ]);
    }

    let mut activities: BTreeMap<String, String> = BTreeMap::new();
    let mut activity_node = |dot: &mut Dot, name: &str| -> String {
        if let Some(id) = activities.get(name) {
            return id.clone();
        }
        let id = format!("a{}", activities.len());
        dot.node(&id, Some(name), vec![("shape", s("box"))]);
        activities.insert(name.to_owned(), id.clone());
        id
    };
    for act in &ocpn.activities {
        activity_node(&mut dot, act);
    }

    for (k, (ot, otn)) in ocpn.nets.iter().enumerate() {
        let color = color_of(&options.object_type_colors, ot);
        let empty = OcpnDiagnostics::default();
        let diag = otn.diagnostics.as_ref().unwrap_or(&empty);
        let place_id = |p: PlaceId| format!("o{k}p{}", p.index());

        for (p, _) in otn.net.places() {
            let mut label = " ".to_owned();
            let mut shape = "circle";
            let mut fontcolor = None;
            let mut fillcolor = Some(color.clone());
            if otn.initial.get(p) > 0 {
                label.clone_from(ot);
                shape = "ellipse";
            } else if otn.fin.get(p) > 0 {
                label.clone_from(ot);
                shape = "underline";
                fontcolor = Some(color.clone());
                fillcolor = None;
            }
            if let Some(d) = diag.places.get(&p)
                && shape == "circle"
            {
                shape = "ellipse";
                label = format!(
                    "p={} m={}\nc={} r={}",
                    d.produced, d.missing, d.consumed, d.remaining
                );
            }
            dot.node(
                &place_id(p),
                Some(&label),
                vec![
                    ("shape", s(shape)),
                    ("style", fillcolor.as_ref().map(|_| "filled".to_owned())),
                    ("fillcolor", fillcolor),
                    ("fontcolor", fontcolor),
                ],
            );
        }

        let mut transitions: BTreeMap<TransitionId, String> = BTreeMap::new();
        for (t, tr) in otn.net.transitions() {
            let id = match &tr.label {
                Some(l) => activity_node(&mut dot, l.as_ref()),
                None => {
                    let id = format!("o{k}t{}", t.index());
                    dot.node(
                        &id,
                        Some(" "),
                        vec![
                            ("shape", s("box")),
                            ("style", s("filled")),
                            ("fillcolor", Some(color.clone())),
                        ],
                    );
                    id
                }
            };
            transitions.insert(t, id);
        }

        for (_, arc) in otn.net.arcs() {
            let t = arc.transition();
            let double = otn.net.transition(t).label.as_ref().is_some_and(|l| {
                otn.double_arcs
                    .get(l.as_ref() as &str)
                    .copied()
                    .unwrap_or(false)
            });
            let label = diag
                .transitions
                .get(&t)
                .map_or_else(|| " ".to_owned(), u64::to_string);
            let (tail, head) = match arc.ends {
                ArcEnds::PlaceToTransition(p, t) => (place_id(p), transitions[&t].clone()),
                ArcEnds::TransitionToPlace(t, p) => (transitions[&t].clone(), place_id(p)),
            };
            dot.edge(
                &tail,
                &head,
                Some(&label),
                vec![
                    ("color", Some(color.clone())),
                    ("penwidth", s(if double { "4.0" } else { "1.0" })),
                ],
            );
        }
    }
    dot.defaults("graph", vec![("nodesep", s("0.1")), ("ranksep", s("0.2"))]);
    dot.set(vec![("rankdir", s(&options.rankdir))]);
    dot.finish()
}
