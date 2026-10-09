//! Soundness of workflow nets, ported from pm4py's
//! `algo/analysis/woflan` as `pm4py.check_soundness` runs it.

use std::collections::{BTreeSet, HashMap};

use super::AnalysisError;
use super::lp::LinearProgram;
use super::workflow::{is_strongly_connected, short_circuit};
use crate::petri::{AcceptingPetriNet, PetriNet, PlaceId, TransitionId};

/// The verdict of [`AcceptingPetriNet::check_soundness`] and the facts
/// behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundnessReport {
    /// Whether the net is a sound workflow net.
    pub sound: bool,
    /// woflan's diagnostic messages, in the order of its steps, worded as
    /// pm4py words them.
    pub messages: Vec<String>,
    /// Places not covered by any S-component, sorted by name. Empty when
    /// the check stopped before it computed the S-components.
    pub uncovered_places: Vec<PlaceId>,
    /// Names of the visible transitions that never fire in the
    /// short-circuited net, sorted. They include
    /// `short_circuited_transition` when the sink is never reached. Empty
    /// when the check stopped before it looked for them.
    pub dead_tasks: Vec<String>,
}

/// `(place index, weight)` pairs.
type Weights = Vec<(usize, i64)>;

/// Places and transitions of a net sorted by name, as woflan indexes them.
struct Indexed<'a> {
    net: &'a PetriNet,
    places: Vec<PlaceId>,
    transitions: Vec<TransitionId>,
    place_index: HashMap<PlaceId, usize>,
}

impl<'a> Indexed<'a> {
    fn new(net: &'a PetriNet) -> Self {
        let mut places: Vec<PlaceId> = net.place_ids().collect();
        places.sort_by(|&a, &b| net.place(a).name.cmp(&net.place(b).name));
        let mut transitions: Vec<TransitionId> = net.transition_ids().collect();
        transitions.sort_by(|&a, &b| net.transition(a).name.cmp(&net.transition(b).name));
        let place_index = places.iter().enumerate().map(|(i, &p)| (p, i)).collect();
        Self {
            net,
            places,
            transitions,
            place_index,
        }
    }

    /// `(place index, weight)` of each input and output arc of `t`. Arc
    /// kinds are ignored, as in woflan.
    fn arcs(&self, t: TransitionId) -> (Weights, Weights) {
        let side = |arcs: &[crate::petri::ArcId]| {
            arcs.iter()
                .map(|&a| {
                    let arc = self.net.arc(a);
                    (self.place_index[&arc.place()], i64::from(arc.weight))
                })
                .collect()
        };
        let tr = self.net.transition(t);
        (side(tr.in_arcs()), side(tr.out_arcs()))
    }

    /// The incidence matrix, places by transitions.
    fn incidence(&self) -> Vec<Vec<i64>> {
        let mut c = vec![vec![0; self.transitions.len()]; self.places.len()];
        for (j, &t) in self.transitions.iter().enumerate() {
            let (inputs, outputs) = self.arcs(t);
            for (p, w) in inputs {
                c[p][j] -= w;
            }
            for (p, w) in outputs {
                c[p][j] += w;
            }
        }
        c
    }
}

/// pm4py's `rref` on the integer matrix `a`, in place. Returns the pivot
/// columns.
///
/// pm4py's matrix has numpy dtype `int64`, so each division by the pivot is
/// truncated toward zero when it is stored back. The result is then not
/// always a row echelon form of the real matrix; this keeps pm4py's
/// arithmetic. Pivoting takes the first largest absolute value, as
/// `np.argmax` does.
fn rref(a: &mut [Vec<i64>]) -> Vec<usize> {
    let m = a.len();
    let n = a.first().map_or(0, Vec::len);
    let (mut i, mut j) = (0, 0);
    let mut pivots = Vec::new();
    while i < m && j < n {
        let mut k = i;
        for r in i + 1..m {
            if a[r][j].abs() > a[k][j].abs() {
                k = r;
            }
        }
        if a[k][j] == 0 {
            for row in &mut a[i..m] {
                row[j] = 0;
            }
            j += 1;
            continue;
        }
        pivots.push(j);
        if i != k {
            a.swap(i, k);
        }
        let p = a[i][j];
        for x in &mut a[i][j..n] {
            *x = (*x as f64 / p as f64) as i64;
        }
        let pivot_row = a[i].clone();
        for (r, row) in a.iter_mut().enumerate() {
            if r != i {
                let f = row[j];
                for c in j..n {
                    row[c] -= f * pivot_row[c];
                }
            }
        }
        i += 1;
        j += 1;
    }
    pivots
}

/// A basis of the place invariants (the null space of the transposed
/// incidence matrix), as pm4py's `compute_place_invariants` builds it.
fn place_invariants(ix: &Indexed<'_>) -> Vec<Vec<f64>> {
    let c = ix.incidence();
    let n = ix.places.len();
    let mut a: Vec<Vec<i64>> = (0..ix.transitions.len())
        .map(|t| (0..n).map(|p| c[p][t]).collect())
        .collect();
    let pivots = rref(&mut a);
    (0..n)
        .filter(|f| !pivots.contains(f))
        .map(|free| {
            let mut v = vec![0.0; n];
            v[free] = 1.0;
            for (row, &col) in pivots.iter().enumerate() {
                v[col] -= a[row][free] as f64;
            }
            v
        })
        .collect()
}

/// pm4py's `transform_basis` with `style="uniform"`: turns the basis into
/// invariants with entries 0 or 1 where it can, one linear program per
/// vector that has another entry.
fn uniform_invariants(basis: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, AnalysisError> {
    let mut modified: Vec<Vec<f64>> = basis
        .into_iter()
        .map(|v| {
            if v.iter().all(|&x| x <= 0.0) {
                v.into_iter().map(|x| -x).collect()
            } else {
                v
            }
        })
        .collect();
    let to_modify: Vec<Vec<f64>> = modified
        .iter()
        .filter(|v| v.iter().any(|&x| !(0.0..=1.0).contains(&x)))
        .cloned()
        .collect();
    for vector in to_modify {
        let at = modified
            .iter()
            .position(|v| *v == vector)
            .expect("vector to modify is in the basis");
        modified.remove(at);
        let b = modified.len();
        let n = vector.len();
        let width = b + 1 + n;
        let (y, z) = (b, |i: usize| b + 1 + i);
        let mut lp =
            LinearProgram::new((0..width).map(|k| if k < b { 1.0 } else { 0.0 }).collect());
        let unit = |k: usize, value: f64| {
            let mut row = vec![0.0; width];
            row[k] = value;
            row
        };
        lp.a_ub.push(unit(y, -1.0));
        lp.b_ub.push(-1.0);
        for (i, &vi) in vector.iter().enumerate() {
            let mut row = unit(y, vi);
            for (j, base) in modified.iter().enumerate() {
                row[j] = base[i];
            }
            row[z(i)] = -1.0;
            lp.a_eq.push(row);
            lp.b_eq.push(0.0);
        }
        for i in 0..n {
            lp.a_ub.push(unit(z(i), 1.0));
            lp.b_ub.push(1.0);
            lp.a_ub.push(unit(z(i), -1.0));
            lp.b_ub.push(0.0);
        }
        if let Some(points) = lp.solve()? {
            modified.push((0..n).map(|i| points[z(i)].round_ties_even()).collect());
        }
    }
    Ok(modified)
}

/// pm4py's `compute_s_components`: the places with a positive entry in an
/// invariant, with their neighbouring transitions, when every such
/// transition has exactly one input and one output place among them.
fn s_components(ix: &Indexed<'_>, invariants: &[Vec<f64>]) -> Vec<BTreeSet<PlaceId>> {
    let net = ix.net;
    let mut out = Vec::new();
    for inv in invariants {
        let places: BTreeSet<PlaceId> = inv
            .iter()
            .enumerate()
            .filter(|&(_, &x)| x > 0.0)
            .map(|(i, _)| ix.places[i])
            .collect();
        if places.is_empty() {
            continue;
        }
        let transitions: BTreeSet<TransitionId> = places
            .iter()
            .flat_map(|&p| net.place_preset(p).chain(net.place_postset(p)))
            .collect();
        let one = |side: BTreeSet<PlaceId>| side.intersection(&places).count() == 1;
        if transitions
            .iter()
            .all(|&t| one(net.preset(t).collect()) && one(net.postset(t).collect()))
        {
            out.push(places);
        }
    }
    out
}

/// The reachable markings of a net under woflan's semantics, with the
/// transitions that fire and whether the graph is strongly connected.
struct StateSpace {
    fired: BTreeSet<TransitionId>,
    strongly_connected: bool,
}

const MAX_MARKINGS: usize = 1_000_000;

fn state_space(ix: &Indexed<'_>, initial: Vec<i64>) -> Result<StateSpace, AnalysisError> {
    let rules: Vec<(TransitionId, Weights, Weights)> = ix
        .transitions
        .iter()
        .map(|&t| {
            let (inputs, outputs) = ix.arcs(t);
            let mut need: Weights = Vec::new();
            let mut delta: Weights = Vec::new();
            for &(p, w) in &inputs {
                match need.iter_mut().find(|(q, _)| *q == p) {
                    Some(e) => e.1 += w,
                    None => need.push((p, w)),
                }
            }
            for (p, w) in inputs
                .iter()
                .map(|&(p, w)| (p, -w))
                .chain(outputs.iter().copied())
            {
                match delta.iter_mut().find(|(q, _)| *q == p) {
                    Some(e) => e.1 += w,
                    None => delta.push((p, w)),
                }
            }
            (t, need, delta)
        })
        .collect();
    let mut index: HashMap<Vec<i64>, usize> = HashMap::from([(initial.clone(), 0)]);
    let mut markings = vec![initial];
    let mut succ: Vec<Vec<usize>> = vec![Vec::new()];
    let mut fired = BTreeSet::new();
    let mut stack = vec![0];
    while let Some(i) = stack.pop() {
        for (t, need, delta) in &rules {
            let m = &markings[i];
            if need.iter().any(|&(p, w)| m[p] < w) {
                continue;
            }
            let mut next = m.clone();
            for &(p, d) in delta {
                next[p] += d;
            }
            fired.insert(*t);
            let j = match index.get(&next) {
                Some(&j) => j,
                None => {
                    if markings.len() >= MAX_MARKINGS {
                        return Err(AnalysisError::TooManyMarkings(MAX_MARKINGS));
                    }
                    let j = markings.len();
                    index.insert(next.clone(), j);
                    markings.push(next);
                    succ.push(Vec::new());
                    stack.push(j);
                    j
                }
            };
            succ[i].push(j);
        }
    }
    let n = markings.len();
    let mut pred = vec![Vec::new(); n];
    for (i, s) in succ.iter().enumerate() {
        for &j in s {
            pred[j].push(i);
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
    Ok(StateSpace {
        fired,
        strongly_connected: reaches_all(&succ) && reaches_all(&pred),
    })
}

fn list(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(", "))
}

impl AcceptingPetriNet {
    /// Checks whether the net is a sound workflow net with pm4py's woflan,
    /// as `pm4py.check_soundness` runs it: it stops at the first sign of
    /// unsoundness.
    ///
    /// The steps:
    ///
    /// 1. Both markings mark exactly one place.
    /// 2. The net is a workflow net.
    /// 3. Every place is covered by an S-component. woflan computes a
    ///    basis of place invariants, turns it into 0/1 invariants with one
    ///    linear program per vector, and keeps those that are S-components.
    /// 4. No visible transition is dead in the short-circuited net.
    /// 5. The reachability graph of the short-circuited net is strongly
    ///    connected, so every transition is live.
    ///
    /// Markings count one token per marked place, and arc kinds are
    /// ignored, as in woflan. For step 4, woflan builds a minimal
    /// coverability graph; a net that passes step 3 is bounded, and that
    /// graph has the same transitions as the reachability graph used here.
    /// woflan's further diagnostics (not well-handled pairs, weighted
    /// invariants, unbounded and deadlocking sequences) only run when it
    /// does not stop early, so they are not ported.
    ///
    /// The linear programs are solved with `microlp`; pm4py uses scipy or
    /// PuLP. Their optimal points are rounded as pm4py rounds them; another
    /// solver can return another optimal point, and with it another
    /// invariant.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::LinearProgram`] when the solver fails, and
    /// [`AnalysisError::TooManyMarkings`] when the short-circuited net has
    /// more than a million reachable markings.
    pub fn check_soundness(&self) -> Result<SoundnessReport, AnalysisError> {
        let mut report = SoundnessReport {
            sound: false,
            messages: Vec::new(),
            uncovered_places: Vec::new(),
            dead_tasks: Vec::new(),
        };
        let say = |r: &mut SoundnessReport, m: &str| r.messages.push(m.to_owned());
        if self.initial_marking.len() != 1 || self.final_marking.len() != 1 {
            say(
                &mut report,
                "There is more than one initial or final marking.",
            );
            return Ok(report);
        }
        say(&mut report, "Input is ok.");
        let sc = match short_circuit(&self.net) {
            Ok(sc) => sc,
            Err(e) => {
                say(&mut report, e.message());
                return Ok(report);
            }
        };
        if !is_strongly_connected(&sc) {
            say(&mut report, "Petri Net is a not a worflow net.");
            return Ok(report);
        }
        say(&mut report, "Petri Net is a workflow net.");

        let ix = Indexed::new(&sc);
        let invariants = uniform_invariants(place_invariants(&ix))?;
        let covered: BTreeSet<PlaceId> = s_components(&ix, &invariants)
            .into_iter()
            .flatten()
            .collect();
        report.uncovered_places = ix
            .places
            .iter()
            .copied()
            .filter(|p| !covered.contains(p))
            .collect();
        if !report.uncovered_places.is_empty() {
            let names = report
                .uncovered_places
                .iter()
                .map(|&p| sc.place(p).name.clone());
            let message = format!(
                "The following places are not covered by an s-component: {}.",
                list(names)
            );
            report.messages.push(message);
            return Ok(report);
        }
        say(&mut report, "Every place is covered by s-components.");

        let mut initial = vec![0; ix.places.len()];
        for (p, _) in self.initial_marking.iter() {
            initial[ix.place_index[&p]] = 1;
        }
        let space = state_space(&ix, initial)?;
        let dead: Vec<TransitionId> = ix
            .transitions
            .iter()
            .copied()
            .filter(|&t| sc.transition(t).label.is_some() && !space.fired.contains(&t))
            .collect();
        report.dead_tasks = dead
            .iter()
            .map(|&t| sc.transition(t).name.clone())
            .collect();
        if !dead.is_empty() {
            let names = dead.iter().map(|&t| {
                let tr = sc.transition(t);
                format!(
                    "({}, '{}')",
                    tr.name,
                    tr.label.as_deref().unwrap_or_default()
                )
            });
            let message = format!("The following tasks are dead: {}", list(names));
            report.messages.push(message);
            return Ok(report);
        }
        say(&mut report, "There are no dead tasks.");
        if space.strongly_connected {
            say(&mut report, "All tasks are live.");
            report.sound = true;
        }
        Ok(report)
    }

    /// pm4py's POWL-first `analysis.check_is_sound` shortcut.
    ///
    /// A net that converts to POWL returns `true`, even when Woflan finds it
    /// unsound. The `analysis/net-and-split-xor-join` and `analysis/net-murata3`
    /// goldens are examples. Conversion ignores the accepting markings.
    /// If POWL conversion fails, Woflan decides.
    ///
    /// Use [`AcceptingPetriNet::check_soundness`] for the Woflan soundness verdict.
    ///
    /// # Errors
    ///
    /// As [`AcceptingPetriNet::check_soundness`].
    pub fn is_sound(&self) -> Result<bool, AnalysisError> {
        if self.net.to_powl().is_ok() {
            return Ok(true);
        }
        Ok(self.check_soundness()?.sound)
    }
}

#[cfg(test)]
mod tests {
    use super::rref;

    #[test]
    fn rref_truncates_like_numpy_int64() {
        let mut a = vec![vec![0, 2, 4], vec![1, 1, 1]];
        assert_eq!(rref(&mut a), vec![0, 1]);
        assert_eq!(a, vec![vec![1, 0, -1], vec![0, 1, 2]]);
        // 3 / 2 is stored as 1.
        let mut b = vec![vec![2, 3]];
        assert_eq!(rref(&mut b), vec![0]);
        assert_eq!(b, vec![vec![1, 1]]);
    }
}
