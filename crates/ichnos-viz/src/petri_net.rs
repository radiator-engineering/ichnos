//! Petri nets, ported from pm4py's `visualization/petri_net/common/visualize.py`.

use std::cmp::Reverse;
use std::collections::{BTreeMap, VecDeque};

use ichnos_model::petri::{ArcEnds, ArcId, ArcKind};
use ichnos_model::{Marking, PetriNet, PlaceId, TransitionId};

use crate::dot::{Dot, title_label};

/// pm4py's `DEFAULT_START_SYMBOL_GRAPHS`, an HTML filled circle.
pub(crate) const START_SYMBOL: &str = "<&#9679;>";
/// pm4py's `DEFAULT_END_SYMBOL_GRAPHS`, an HTML filled square.
pub(crate) const END_SYMBOL: &str = "<&#9632;>";

/// How to draw one place, transition or arc, overriding the defaults
/// (pm4py's `decorations`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decoration {
    /// The label. An empty label counts as none.
    pub label: Option<String>,
    /// The fill colour of a place or transition, or the colour of an arc.
    pub color: Option<String>,
    /// The pen width of an arc.
    pub penwidth: Option<String>,
}

/// Decorations by element (pm4py's `decorations` dictionary).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PetriNetDecorations {
    /// Place decorations.
    pub places: BTreeMap<PlaceId, Decoration>,
    /// Transition decorations.
    pub transitions: BTreeMap<TransitionId, Decoration>,
    /// Arc decorations.
    pub arcs: BTreeMap<ArcId, Decoration>,
}

/// Options for [`petri_net_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetriNetDotOptions {
    /// The background colour and the fill colour of places and visible
    /// transitions. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// The font size. Default 12.
    pub font_size: u32,
    /// Label places and transitions with their names (pm4py's `debug`).
    pub debug: bool,
    /// Labels, colours and pen widths that override the defaults.
    pub decorations: PetriNetDecorations,
    /// Data-net guards by transition, each drawn as a dotted box linked to
    /// its transition. pm4py reads them from the transitions' properties;
    /// `ichnos_io::read_pnml` keeps them in `PnmlDocument::transition_data`.
    pub guards: BTreeMap<TransitionId, String>,
}

impl Default for PetriNetDotOptions {
    fn default() -> Self {
        PetriNetDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            font_size: 12,
            debug: false,
            decorations: PetriNetDecorations::default(),
            guards: BTreeMap::new(),
        }
    }
}

/// The DOT text of an accepting Petri net, as pm4py's `save_vis_petri_net`
/// draws it with the `wo_decoration` variant.
///
/// Transitions are boxes; silent ones are black. Places are circles. A place
/// of the initial marking shows a dot (one token) or its token count; a
/// place of the final marking only shows a square. Inhibitor arcs end in a
/// dot and reset arcs in a `vee`. When any arc has a weight other than 1,
/// every arc shows its weight. Guards ([`PetriNetDotOptions::guards`]) are
/// dotted boxes. Elements are written in the order of their distance from
/// the initial marking, which guides the Graphviz layout.
pub fn petri_net_dot(
    net: &PetriNet,
    initial: &Marking,
    fin: &Marking,
    options: &PetriNetDotOptions,
) -> String {
    let font_size = options.font_size.to_string();
    let bgcolor = options.bgcolor.as_str();
    let deco = &options.decorations;
    let mut dot = Dot::new(
        true,
        false,
        &net.name,
        vec![
            ("bgcolor", Some(bgcolor.to_owned())),
            ("rankdir", Some(options.rankdir.clone())),
        ],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, options.font_size))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }
    let (transitions, places, arcs) = sort_petri_net(net, initial, fin);
    let place_id = |p: PlaceId| format!("p{}", p.index());
    let transition_id = |t: TransitionId| format!("t{}", t.index());

    let s = |v: &str| Some(v.to_owned());
    dot.defaults("node", vec![("shape", s("box"))]);
    for t in transitions {
        let tr = net.transition(t);
        let d = deco.transitions.get(&t);
        let mut label = d.and_then(|d| d.label.clone()).unwrap_or_default();
        let mut fillcolor = d.and_then(|d| d.color.clone());
        let mut fontcolor = "black";
        if let Some(l) = &tr.label
            && label.is_empty()
        {
            label = l.to_string();
        }
        if options.debug {
            label.clone_from(&tr.name);
        }
        if fillcolor.is_none() {
            if tr.label.is_none() {
                fillcolor = Some("black".to_owned());
                if !label.is_empty() {
                    fontcolor = "white";
                }
            } else {
                fillcolor = Some(bgcolor.to_owned());
            }
        }
        dot.node(
            &transition_id(t),
            Some(&label),
            vec![
                ("style", Some("filled".to_owned())),
                ("fillcolor", fillcolor),
                ("border", Some("1".to_owned())),
                ("fontsize", Some(font_size.clone())),
                ("fontcolor", Some(fontcolor.to_owned())),
            ],
        );
        if let Some(guard) = options.guards.get(&t) {
            let guard_id = format!("{}guard", transition_id(t));
            dot.node(&guard_id, Some(guard), vec![("style", s("dotted"))]);
            dot.edge(
                &guard_id,
                &transition_id(t),
                None,
                vec![("arrowhead", s("none")), ("style", s("dotted"))],
            );
        }
    }

    for p in places {
        let d = deco.places.get(&p);
        let deco_label = d.and_then(|d| d.label.clone());
        let label = deco_label.clone().unwrap_or_default();
        let fillcolor = d
            .and_then(|d| d.color.clone())
            .unwrap_or_else(|| bgcolor.to_owned());
        let id = place_id(p);
        let tokens = initial.get(p);
        if tokens > 0 {
            if tokens == 1 {
                dot.node(
                    &id,
                    Some(START_SYMBOL),
                    vec![
                        ("fontsize", s("34")),
                        ("fixedsize", s("true")),
                        ("shape", s("circle")),
                        ("width", s("0.75")),
                        ("style", s("filled")),
                        ("fillcolor", Some(fillcolor)),
                    ],
                );
            } else {
                let count = tokens.to_string();
                let mut attrs = vec![
                    ("fontsize", s("34")),
                    ("style", s("filled")),
                    ("fillcolor", Some(fillcolor)),
                ];
                if count.len() >= 3 {
                    attrs.push(("shape", s("ellipse")));
                } else {
                    attrs.extend([
                        ("fixedsize", s("true")),
                        ("shape", s("circle")),
                        ("width", s("0.75")),
                    ]);
                }
                dot.node(&id, Some(&count), attrs);
            }
        } else if fin.get(p) > 0 {
            dot.node(
                &id,
                Some(END_SYMBOL),
                vec![
                    ("fontsize", s("32")),
                    ("shape", s("doublecircle")),
                    ("fixedsize", s("true")),
                    ("width", s("0.75")),
                    ("style", s("filled")),
                    ("fillcolor", Some(fillcolor)),
                ],
            );
        } else if options.debug {
            dot.node(
                &id,
                Some(&net.place(p).name),
                vec![
                    ("fontsize", Some(font_size.clone())),
                    ("shape", s("ellipse")),
                ],
            );
        } else if deco_label.is_some() {
            dot.node(
                &id,
                Some(&label),
                vec![
                    ("style", s("filled")),
                    ("fillcolor", Some(fillcolor)),
                    ("fontsize", Some(font_size.clone())),
                    ("shape", s("ellipse")),
                ],
            );
        } else {
            dot.node(
                &id,
                Some(&label),
                vec![
                    ("shape", s("circle")),
                    ("fixedsize", s("true")),
                    ("width", s("0.75")),
                    ("style", s("filled")),
                    ("fillcolor", Some(fillcolor)),
                ],
            );
        }
    }

    let weights_visible = net.arcs().any(|(_, a)| a.weight != 1);
    for a in arcs {
        let arc = net.arc(a);
        let d = deco.arcs.get(&a);
        let mut label = d.and_then(|d| d.label.clone()).unwrap_or_default();
        if label.is_empty() && weights_visible {
            label = arc.weight.to_string();
        }
        let color = d.and_then(|d| d.color.clone());
        let arrowhead = match arc.kind {
            ArcKind::Reset => "vee",
            ArcKind::Inhibitor => "dot",
            _ => "normal",
        };
        let (tail, head) = match arc.ends {
            ArcEnds::PlaceToTransition(p, t) => (place_id(p), transition_id(t)),
            ArcEnds::TransitionToPlace(t, p) => (transition_id(t), place_id(p)),
        };
        dot.edge(
            &tail,
            &head,
            Some(&label),
            vec![
                ("penwidth", d.and_then(|d| d.penwidth.clone())),
                ("color", color.clone()),
                ("fontsize", Some(font_size.clone())),
                ("arrowhead", s(arrowhead)),
                ("fontcolor", color),
            ],
        );
    }
    dot.set(vec![("overlap", s("false"))]);
    dot.finish()
}

/// pm4py's `sort_petri_net`: transitions, places and arcs in the order of
/// their distance from the initial marking.
///
/// Ties go to the element farther from the final marking. pm4py's second
/// search, from the final marking, looks places up in its transition map
/// and so never leaves the final places: they are at distance 0 and every
/// other element is unreachable. That is kept here. Remaining ties keep id
/// order.
fn sort_petri_net(
    net: &PetriNet,
    initial: &Marking,
    fin: &Marking,
) -> (Vec<TransitionId>, Vec<PlaceId>, Vec<ArcId>) {
    const INF: u64 = u64::MAX;
    let mut place_to_transition: BTreeMap<PlaceId, Vec<TransitionId>> = BTreeMap::new();
    let mut transition_to_place: BTreeMap<TransitionId, Vec<PlaceId>> = BTreeMap::new();
    for (_, arc) in net.arcs() {
        match arc.ends {
            ArcEnds::PlaceToTransition(p, t) => place_to_transition.entry(p).or_default().push(t),
            ArcEnds::TransitionToPlace(t, p) => transition_to_place.entry(t).or_default().push(p),
        }
    }
    let mut place_dist: BTreeMap<PlaceId, u64> = net.place_ids().map(|p| (p, INF)).collect();
    let mut transition_dist: BTreeMap<TransitionId, u64> =
        net.transition_ids().map(|t| (t, INF)).collect();
    let mut queue: VecDeque<PlaceId> = VecDeque::new();
    for (p, _) in initial.iter() {
        place_dist.insert(p, 0);
        queue.push_back(p);
    }
    while let Some(current) = queue.pop_front() {
        let d = place_dist[&current];
        for &t in place_to_transition.get(&current).into_iter().flatten() {
            if transition_dist[&t] > d + 1 {
                transition_dist.insert(t, d + 1);
                for &p in transition_to_place.get(&t).into_iter().flatten() {
                    if place_dist[&p] > d + 2 {
                        place_dist.insert(p, d + 2);
                        queue.push_back(p);
                    }
                }
            }
        }
    }
    let place_final = |p: PlaceId| if fin.get(p) > 0 { 0 } else { INF };
    let place_key = |p: PlaceId| (place_dist[&p], Reverse(place_final(p)));
    let transition_key = |t: TransitionId| (transition_dist[&t], Reverse(INF));

    let mut places: Vec<PlaceId> = net.place_ids().collect();
    places.sort_by_key(|&p| place_key(p));
    let mut transitions: Vec<TransitionId> = net.transition_ids().collect();
    transitions.sort_by_key(|&t| transition_key(t));
    let mut arcs: Vec<ArcId> = net.arc_ids().collect();
    arcs.sort_by_key(|&a| match net.arc(a).ends {
        ArcEnds::PlaceToTransition(p, t) => (place_key(p), transition_key(t)),
        ArcEnds::TransitionToPlace(t, p) => (transition_key(t), place_key(p)),
    });
    (transitions, places, arcs)
}
