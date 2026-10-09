//! Binary-region ILP discovery, with sequence-prefix filtering and model reductions.
use crate::{Error, Result};
use good_lp::{
    Expression, ProblemVariables, ResolutionError, Solution, SolverModel, constraint, microlp,
    variable,
};
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{AcceptingPetriNet, Label, Marking, PetriNet};
use std::collections::{BTreeMap, BTreeSet};

/// An observed activity or a collision-free synthetic boundary in a causal relation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IlpActivity {
    /// Visible activity.
    Activity(Label),
    /// Synthetic start transition.
    Start,
    /// Synthetic end transition.
    End,
}
impl IlpActivity {
    fn sort_key(&self) -> (&str, u8) {
        match self {
            Self::Activity(label) => (label.as_str(), 0),
            Self::Start => ("▶", 1),
            Self::End => ("■", 1),
        }
    }
}
/// Options for the classic ILP region miner.
#[derive(Debug, Clone)]
pub struct IlpOptions {
    /// Sequence graph filtering fraction: 1 keeps every prefix; 0 keeps only
    /// maximum-frequency children. Must be finite and in `[0,1]`.
    pub alpha: f64,
    /// Optional causal pairs, including typed boundaries. None uses the alpha
    /// causal relation on the variant-deduplicated, boundary-augmented log.
    pub causal_relation: Option<BTreeSet<(IlpActivity, IlpActivity)>>,
}
impl Default for IlpOptions {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            causal_relation: None,
        }
    }
}
struct Problem {
    objective: Vec<f64>,
    inequalities: BTreeSet<Vec<i64>>,
    equalities: BTreeSet<Vec<i64>>,
}
impl Problem {
    fn solve_once(
        &self,
        source: usize,
        target: usize,
        fixed: &[bool],
        optimum: Option<f64>,
    ) -> Result<Option<Vec<bool>>> {
        let n = self.objective.len() / 2;
        let mut vars = ProblemVariables::new();
        let x = vars.add_vector(variable().binary(), 2 * n);
        let objective: Expression = x.iter().zip(&self.objective).map(|(&v, &c)| c * v).sum();
        let mut problem = vars.minimise(objective.clone()).using(microlp);
        if let Some(optimum) = optimum {
            problem = problem.with(constraint!(objective == optimum));
        }
        for (i, &value) in fixed.iter().enumerate() {
            problem = problem.with(constraint!(x[i] == if value { 1 } else { 0 }));
        }
        for row in &self.inequalities {
            let lhs: Expression = x.iter().zip(row).map(|(&v, &c)| c as f64 * v).sum();
            problem = problem.with(constraint!(lhs <= 0));
        }
        for row in &self.equalities {
            let lhs: Expression = x.iter().zip(row).map(|(&v, &c)| c as f64 * v).sum();
            problem = problem.with(constraint!(lhs == 0));
        }
        let sum: Expression = x.iter().copied().sum();
        problem = problem
            .with(constraint!(sum >= 1))
            .with(constraint!(x[source] == 1))
            .with(constraint!(x[n + target] == 1));
        let solution = match problem.solve() {
            Ok(solution) => solution,
            Err(ResolutionError::Infeasible) => return Ok(None),
            Err(e) => return Err(Error::IlpSolver(e.to_string())),
        };
        let values: Vec<_> = x.iter().map(|&v| solution.value(v)).collect();
        if values.iter().any(|v| {
            !v.is_finite() || (*v - v.round()).abs() > 1e-6 || *v < -1e-6 || *v > 1.0 + 1e-6
        }) {
            return Err(Error::IlpSolver("nonbinary region solution".into()));
        }
        Ok(Some(values.iter().map(|v| *v > 0.5).collect()))
    }
    fn solve(&self, source: usize, target: usize) -> Result<Option<Vec<bool>>> {
        let Some(mut solution) = self.solve_once(source, target, &[], None)? else {
            return Ok(None);
        };
        let optimum = self
            .objective
            .iter()
            .zip(&solution)
            .filter(|(_, value)| **value)
            .map(|(cost, _)| cost)
            .sum();
        let mut fixed = Vec::new();
        for i in 0..solution.len() {
            fixed.push(false);
            if solution[i] {
                if let Some(next) = self.solve_once(source, target, &fixed, Some(optimum))? {
                    solution = next;
                } else {
                    fixed[i] = true;
                }
            }
        }
        Ok(Some(fixed))
    }
}
/// Discover a Petri net by solving one binary region per causal pair, then
/// removing implicit places and reducing silent transitions with the model crate.
/// Prefix constraints retain first-appearance trace variants with multiplicity
/// used only for filtering. Input activity order is preserved; timestamps are
/// unnecessary. Empty input returns a silent workflow shell. Tied optimal
/// regions use the lexicographically smallest binary vector at the original
/// objective optimum; a differently configured pm4py LP backend can select
/// another optimal region and therefore a different net.
pub fn petri_net_ilp(
    log: &EventLog,
    keys: &EventKeys,
    options: &IlpOptions,
) -> Result<AcceptingPetriNet> {
    if !options.alpha.is_finite() || !(0.0..=1.0).contains(&options.alpha) {
        return Err(Error::InvalidOption(
            "ILP alpha must be finite and in `[0,1]`",
        ));
    }
    let sequences = log.activity_sequences(keys)?;
    let mut activities: Vec<_> = sequences
        .activities
        .iter()
        .map(|(_, label)| IlpActivity::Activity(Label::from(label)))
        .collect();
    activities.extend([IlpActivity::Start, IlpActivity::End]);
    activities.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    let index: BTreeMap<_, _> = activities
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, a)| (a, i))
        .collect();
    let n = activities.len();
    let start = index[&IlpActivity::Start];
    let end = index[&IlpActivity::End];
    let mut variants = Vec::<(Vec<usize>, u64)>::new();
    let mut seen = BTreeMap::<Vec<usize>, usize>::new();
    for trace in &sequences.traces {
        let mut trace: Vec<_> = trace
            .iter()
            .map(|&a| index[&IlpActivity::Activity(Label::from(sequences.activities.name(a)))])
            .collect();
        trace.insert(0, start);
        trace.push(end);
        if let Some(&i) = seen.get(&trace) {
            variants[i].1 += 1;
        } else {
            seen.insert(trace.clone(), variants.len());
            variants.push((trace, 1));
        }
    }
    // A trace-free input is interpreted as one empty trace for the workflow shell.
    if variants.is_empty() {
        variants.push((vec![start, end], 1));
    }
    let mut edges = BTreeSet::new();
    for (trace, _) in &variants {
        for edge in trace.windows(2) {
            edges.insert((edge[0], edge[1]));
        }
    }
    let causal = if let Some(causal) = &options.causal_relation {
        let mut pairs = BTreeSet::new();
        for (a, b) in causal {
            let (Some(&a), Some(&b)) = (index.get(a), index.get(b)) else {
                return Err(Error::InvalidOption(
                    "ILP causal relation references an unknown activity",
                ));
            };
            pairs.insert((a, b));
        }
        pairs
    } else {
        edges
            .iter()
            .copied()
            .filter(|&(a, b)| !edges.contains(&(b, a)))
            .collect()
    };
    let mut matrix = Vec::new();
    let mut graph = BTreeMap::<Vec<i64>, BTreeMap<Vec<i64>, u64>>::new();
    for (trace, count) in &variants {
        let mut prefix = vec![0i64; n];
        let mut rows = Vec::new();
        for &activity in trace {
            let previous = prefix.iter().map(|v| -v).collect::<Vec<_>>();
            prefix[activity] += 1;
            *graph
                .entry(previous.clone())
                .or_default()
                .entry(prefix.clone())
                .or_default() += count;
            rows.push((previous, prefix.clone()));
        }
        matrix.push(rows);
    }
    let mut base = Problem {
        objective: vec![0.0; 2 * n],
        inequalities: BTreeSet::new(),
        equalities: BTreeSet::new(),
    };
    for trace in &matrix {
        for (i, (previous, current)) in trace.iter().enumerate() {
            let children = &graph[previous];
            let largest = *children.values().max().unwrap();
            if (children[current] as f64) < (1.0 - options.alpha) * largest as f64 {
                break;
            }
            let row = previous.iter().chain(current).copied().collect();
            if i + 1 == trace.len() {
                base.equalities.insert(row);
            } else {
                base.inequalities.insert(row);
            }
            for (j, &count) in current.iter().enumerate() {
                base.objective[j] += count as f64;
                base.objective[n + j] -= count as f64;
            }
        }
    }
    let mut net = PetriNet::new("ilp");
    let source = net.add_place("source");
    let sink = net.add_place("sink");
    let transitions: Vec<_> = activities
        .iter()
        .enumerate()
        .map(|(i, a)| {
            net.add_transition(
                format!("t{i}"),
                match a {
                    IlpActivity::Activity(label) => Some(label.clone()),
                    _ => None,
                },
            )
        })
        .collect();
    net.add_input_arc(source, transitions[start]).unwrap();
    net.add_output_arc(transitions[end], sink).unwrap();
    let mut regions = BTreeSet::new();
    for (a, b) in causal {
        if let Some(solution) = base.solve(a, b)?
            && regions.insert(solution.clone())
            && solution[..n].iter().any(|v| *v)
            && solution[n..].iter().any(|v| *v)
        {
            let p = net.add_place(net.place_count().to_string());
            for i in 0..n {
                if solution[i] {
                    net.add_output_arc(transitions[i], p).unwrap();
                }
                if solution[n + i] {
                    net.add_input_arc(p, transitions[i]).unwrap();
                }
            }
        }
    }
    let mut initial = Marking::new();
    initial.set(source, 1);
    let mut final_marking = Marking::new();
    final_marking.set(sink, 1);
    let mut model = AcceptingPetriNet::new(net, initial, final_marking);
    model
        .reduce_implicit_places()
        .map_err(|e| Error::IlpSolver(e.to_string()))?;
    model.net.apply_simple_reduction();
    Ok(model)
}
