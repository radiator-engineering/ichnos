//! The net as token replay sees it, and the searches pm4py prepares once per
//! net: shortest paths of silent transitions between places
//! (`petri_utils.get_places_shortest_path_by_hidden`) and S-components
//! (`petri_utils.get_s_components_from_petri`).
//!
//! pm4py walks Python sets of places, transitions and arcs here, and their
//! order follows object addresses. Where that order can change a result,
//! this port takes nodes in name order.

use std::cmp::Reverse;
use std::collections::{BTreeSet, HashMap};

use ichnos_model::{Marking, PetriNet, PlaceId, TransitionId};
use rustc_hash::{FxHashMap, FxHashSet};

/// pm4py's `MAX_REC_DEPTH`: the depth limit of the shortest-path search.
const MAX_PATH_DEPTH: usize = 50;
/// pm4py's `max_rec_depth` default for S-components.
const MAX_S_COMPONENT_DEPTH: usize = 6;

/// Token counts indexed by place index. Zero means the place is not in the
/// marking.
pub(crate) type Tokens = Vec<u32>;

/// A Petri net prepared for token replay.
///
/// Arcs of every kind count as ordinary arcs, as in pm4py's classic
/// semantics, which token replay uses.
#[derive(Debug, Clone)]
pub(crate) struct ReplayNet {
    /// Input arcs per transition index: place index and weight.
    pub pre: Vec<Vec<(usize, u32)>>,
    /// Output arcs per transition index.
    pub post: Vec<Vec<(usize, u32)>>,
    /// Tokens consumed and produced by each transition.
    pub consumed: Vec<u64>,
    pub produced: Vec<u64>,
    pub silent: Vec<bool>,
    /// Place ids by index (`None` for holes in the arena).
    pub place_ids: Vec<Option<PlaceId>>,
    /// Live place indices in name order.
    pub places_by_name: Vec<usize>,
    /// The position of each place index in `places_by_name`.
    pub place_rank: Vec<usize>,
    /// Live transitions in name order.
    pub transitions_by_name: Vec<TransitionId>,
    /// For each source place index, the silent transitions on a shortest
    /// path to each target place index.
    pub paths: Vec<FxHashMap<usize, Vec<TransitionId>>>,
}

impl ReplayNet {
    pub fn new(net: &PetriNet) -> Self {
        let tb = net.transition_index_bound();
        let pb = net.place_index_bound();
        let mut pre = vec![Vec::new(); tb];
        let mut post = vec![Vec::new(); tb];
        let mut silent = vec![false; tb];
        for (t, tr) in net.transitions() {
            silent[t.index()] = tr.is_silent();
            pre[t.index()] = tr
                .in_arcs()
                .iter()
                .map(|&a| {
                    let arc = net.arc(a);
                    (arc.place().index(), arc.weight)
                })
                .collect();
            post[t.index()] = tr
                .out_arcs()
                .iter()
                .map(|&a| {
                    let arc = net.arc(a);
                    (arc.place().index(), arc.weight)
                })
                .collect();
        }
        let sum = |arcs: &Vec<(usize, u32)>| arcs.iter().map(|&(_, w)| u64::from(w)).sum();
        let consumed = pre.iter().map(sum).collect();
        let produced = post.iter().map(sum).collect();
        let mut place_ids = vec![None; pb];
        for p in net.place_ids() {
            place_ids[p.index()] = Some(p);
        }
        let mut places_by_name: Vec<usize> = net.place_ids().map(|p| p.index()).collect();
        places_by_name.sort_by(|&a, &b| {
            let name = |i: usize| net.place(place_ids[i].expect("live place")).name.as_str();
            name(a).cmp(name(b))
        });
        let mut place_rank = vec![0usize; pb];
        for (r, &p) in places_by_name.iter().enumerate() {
            place_rank[p] = r;
        }
        let mut transitions_by_name: Vec<TransitionId> = net.transition_ids().collect();
        transitions_by_name.sort_by(|&a, &b| net.transition(a).name.cmp(&net.transition(b).name));
        let mut this = Self {
            pre,
            post,
            consumed,
            produced,
            silent,
            place_ids,
            places_by_name,
            place_rank,
            transitions_by_name,
            paths: Vec::new(),
        };
        this.paths = this.shortest_hidden_paths(net);
        this
    }

    /// Dense token counts of `m`.
    pub fn tokens(&self, m: &Marking) -> Tokens {
        let mut tokens = vec![0; self.place_ids.len()];
        for (p, n) in m.iter() {
            tokens[p.index()] = n;
        }
        tokens
    }

    /// The marking with these token counts.
    pub fn marking(&self, tokens: &[u32]) -> Marking {
        tokens
            .iter()
            .enumerate()
            .filter(|&(_, &n)| n > 0)
            .map(|(i, &n)| (self.place_ids[i].expect("tokens only on live places"), n))
            .collect()
    }

    pub fn is_enabled(&self, t: TransitionId, m: &[u32]) -> bool {
        self.pre[t.index()].iter().all(|&(p, w)| m[p] >= w)
    }

    /// Fires `t`, which must be enabled.
    pub fn fire(&self, t: TransitionId, m: &mut [u32]) {
        for &(p, w) in &self.pre[t.index()] {
            m[p] -= w;
        }
        for &(p, w) in &self.post[t.index()] {
            m[p] += w;
        }
    }

    /// Transitions enabled in `m`, in name order.
    pub fn enabled_by_name<'a>(&'a self, m: &'a [u32]) -> impl Iterator<Item = TransitionId> + 'a {
        self.transitions_by_name
            .iter()
            .copied()
            .filter(move |&t| self.is_enabled(t, m))
    }

    /// Marked places in name order.
    pub fn marked_by_name<'a>(&'a self, m: &'a [u32]) -> impl Iterator<Item = usize> + 'a {
        self.places_by_name
            .iter()
            .copied()
            .filter(move |&p| m[p] > 0)
    }

    /// The hidden paths from each place in `sources` to each place in
    /// `targets`, shortest first (pm4py's `get_hidden_transitions_to_enable`
    /// and `get_req_transitions_for_final_marking`). Both lists are in name
    /// order, and the sort is stable.
    pub fn paths_between(&self, sources: &[usize], targets: &[usize]) -> Vec<&[TransitionId]> {
        let mut out: Vec<&[TransitionId]> = Vec::new();
        for &p1 in sources {
            for &p2 in targets {
                if let Some(path) = self.paths[p1].get(&p2) {
                    out.push(path);
                }
            }
        }
        out.sort_by_key(|p| p.len());
        out
    }

    /// pm4py's `get_places_shortest_path_by_hidden`: a depth-first search
    /// from every place through silent transitions that keeps, per target
    /// place, the first shortest path it finds.
    fn shortest_hidden_paths(&self, net: &PetriNet) -> Vec<FxHashMap<usize, Vec<TransitionId>>> {
        let rank = |t: TransitionId| net.transition(t).name.as_str();
        let place_rank = &self.place_rank;
        // Silent transitions after each place, and places after each
        // transition, in name order.
        let mut silent_after: Vec<Vec<TransitionId>> = vec![Vec::new(); self.place_ids.len()];
        for t in net.transition_ids() {
            if self.silent[t.index()] {
                for &(p, _) in &self.pre[t.index()] {
                    silent_after[p].push(t);
                }
            }
        }
        for ts in &mut silent_after {
            ts.sort_by(|&a, &b| rank(a).cmp(rank(b)));
        }
        let places_after: Vec<Vec<usize>> = self
            .post
            .iter()
            .map(|arcs| {
                let mut ps: Vec<usize> = arcs.iter().map(|&(p, _)| p).collect();
                ps.sort_by_key(|&p| place_rank[p]);
                ps
            })
            .collect();

        struct Search<'a> {
            silent_after: &'a [Vec<TransitionId>],
            places_after: &'a [Vec<usize>],
            found: FxHashMap<usize, Vec<TransitionId>>,
            path: Vec<TransitionId>,
        }
        impl Search<'_> {
            fn visit(&mut self, place: usize, depth: usize) {
                if depth > MAX_PATH_DEPTH {
                    return;
                }
                for &t in &self.silent_after[place] {
                    for &p2 in &self.places_after[t.index()] {
                        let shorter = self
                            .found
                            .get(&p2)
                            .is_none_or(|known| self.path.len() + 1 < known.len());
                        if shorter {
                            self.path.push(t);
                            self.found.insert(p2, self.path.clone());
                            self.visit(p2, depth + 1);
                            self.path.pop();
                        }
                    }
                }
            }
        }

        let mut paths = vec![FxHashMap::default(); self.place_ids.len()];
        for &p in &self.places_by_name {
            let mut search = Search {
                silent_after: &silent_after,
                places_after: &places_after,
                found: FxHashMap::default(),
                path: Vec::new(),
            };
            search.visit(p, 0);
            paths[p] = search.found;
        }
        paths
    }

    /// pm4py's `get_s_components_from_petri`, as sets of place indices.
    ///
    /// pm4py deep-copies the places it has collected each time a
    /// transition has several output places. Its sets then hold copies and
    /// originals of one place as different objects. To follow it, each
    /// place carries the generation of the copy it belongs to.
    pub fn s_components(&self, net: &PetriNet, im: &Marking, fm: &Marking) -> Vec<BTreeSet<usize>> {
        let mut found = Vec::new();
        if im.len() > 1 || fm.len() > 1 {
            return found;
        }
        let Some((source, _)) = im.iter().next() else {
            return found;
        };
        let mut transitions_after: Vec<Vec<TransitionId>> = vec![Vec::new(); self.place_ids.len()];
        for &t in &self.transitions_by_name {
            for &(p, _) in &self.pre[t.index()] {
                if !transitions_after[p].contains(&t) {
                    transitions_after[p].push(t);
                }
            }
        }
        let out_degree_place: Vec<usize> = (0..self.place_ids.len())
            .map(|p| {
                self.place_ids[p]
                    .map(|id| net.place(id).out_arcs().len())
                    .unwrap_or(0)
            })
            .collect();
        for ts in &mut transitions_after {
            // Stable: name order among transitions with as many output arcs.
            ts.sort_by_key(|t| self.post[t.index()].len());
        }
        let mut search = SComponents {
            net: self,
            transitions_after,
            out_degree_place,
            next_generation: 1,
            found: &mut found,
        };
        search.visit(
            0,
            vec![Occurrence {
                place: source.index(),
                generation: 0,
            }],
            Vec::new(),
        );
        found
    }
}

/// A place object in pm4py's S-component search: the place and the copy of
/// the net it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Occurrence {
    place: usize,
    generation: u32,
}

struct SComponents<'a> {
    net: &'a ReplayNet,
    transitions_after: Vec<Vec<TransitionId>>,
    out_degree_place: Vec<usize>,
    next_generation: u32,
    found: &'a mut Vec<BTreeSet<usize>>,
}

impl SComponents<'_> {
    fn visit(&mut self, depth: usize, mut current: Vec<Occurrence>, mut visited: Vec<Occurrence>) {
        let mut changed = true;
        while changed && depth < MAX_S_COMPONENT_DEPTH {
            changed = false;
            let mut seen = FxHashSet::default();
            let mut to_visit: Vec<Occurrence> = current[visited.len().min(current.len())..]
                .iter()
                .copied()
                .filter(|o| seen.insert(*o))
                .collect();
            to_visit.sort_by_key(|o| Reverse(self.out_degree_place[o.place]));
            for place in to_visit {
                visited.push(place);
                let visited_places: FxHashSet<usize> = visited.iter().map(|o| o.place).collect();
                for ti in 0..self.transitions_after[place.place].len() {
                    let t = self.transitions_after[place.place][ti];
                    let mut targets: Vec<usize> = self.net.post[t.index()]
                        .iter()
                        .map(|&(p, _)| p)
                        .filter(|p| !visited_places.contains(p))
                        .collect();
                    targets.sort_by_key(|&p| self.net.place_rank[p]);
                    targets.dedup();
                    if targets.is_empty() {
                        continue;
                    }
                    changed = true;
                    let occurrence = |p| Occurrence {
                        place: p,
                        generation: place.generation,
                    };
                    if let [only] = targets[..] {
                        current.push(occurrence(only));
                    } else {
                        for p in targets {
                            let (mut c, v) = self.deep_copy(&current, &visited);
                            c.push(occurrence(p));
                            self.visit(depth + 1, c, v);
                        }
                    }
                }
            }
        }
        let component: BTreeSet<usize> = current.iter().map(|o| o.place).collect();
        if !self.found.contains(&component) {
            self.found.push(component);
        }
    }

    /// pm4py's `deepcopy([curr_s_comp, visited_places])`: every net copy
    /// referenced becomes a new copy.
    fn deep_copy(
        &mut self,
        current: &[Occurrence],
        visited: &[Occurrence],
    ) -> (Vec<Occurrence>, Vec<Occurrence>) {
        let mut map: HashMap<u32, u32> = HashMap::new();
        let mut copy = |o: &Occurrence| {
            let generation = *map.entry(o.generation).or_insert_with(|| {
                self.next_generation += 1;
                self.next_generation
            });
            Occurrence {
                place: o.place,
                generation,
            }
        };
        let c = current.iter().map(&mut copy).collect();
        let v = visited.iter().map(&mut copy).collect();
        (c, v)
    }
}
