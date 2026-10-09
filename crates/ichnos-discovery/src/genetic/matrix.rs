use super::operators::{Individual, boundaries, flat};
use crate::{Error, Result};
use ichnos_model::{AcceptingPetriNet, Label, Marking, PetriNet};
use std::collections::{BTreeMap, BTreeSet};

/// Causal matrix: each binding is an alternative set of simultaneous neighbors.
/// Binding sets for an activity must be nonempty and disjoint, and input/output
/// edge unions must agree. Activity order controls workflow boundary selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneticMatrix {
    /// Unique visible labels in discovery order.
    pub activities: Vec<Label>,
    /// Input partitions per activity. Missing keys mean no inputs.
    pub inputs: BTreeMap<Label, Vec<BTreeSet<Label>>>,
    /// Output partitions per activity. Missing keys mean no outputs.
    pub outputs: BTreeMap<Label, Vec<BTreeSet<Label>>>,
}

impl GeneticMatrix {
    pub(super) fn from_individual(labels: &[Label], individual: &Individual) -> Self {
        let convert = |partitions: &[Vec<BTreeSet<usize>>]| {
            labels
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    (
                        label.clone(),
                        partitions[i]
                            .iter()
                            .map(|set| set.iter().map(|&j| labels[j].clone()).collect())
                            .collect(),
                    )
                })
                .collect()
        };
        Self {
            activities: labels.to_vec(),
            inputs: convert(&individual.inputs),
            outputs: convert(&individual.outputs),
        }
    }

    fn individual(&self) -> Result<Individual> {
        let indices: BTreeMap<_, _> = self
            .activities
            .iter()
            .enumerate()
            .map(|(i, label)| (label, i))
            .collect();
        if indices.len() != self.activities.len() {
            return Err(Error::InvalidOption("genetic activities must be unique"));
        }
        let convert =
            |map: &BTreeMap<Label, Vec<BTreeSet<Label>>>| -> Result<Vec<Vec<BTreeSet<usize>>>> {
                if map.keys().any(|label| !indices.contains_key(label)) {
                    return Err(Error::InvalidOption("unknown genetic activity"));
                }
                self.activities
                    .iter()
                    .map(|label| {
                        let mut seen = BTreeSet::new();
                        let mut partitions = Vec::new();
                        for set in map.get(label).into_iter().flatten() {
                            if set.is_empty() {
                                return Err(Error::InvalidOption("empty genetic binding"));
                            }
                            let mut binding = BTreeSet::new();
                            for neighbor in set {
                                let &index = indices
                                    .get(neighbor)
                                    .ok_or(Error::InvalidOption("unknown genetic neighbor"))?;
                                if !seen.insert(index) {
                                    return Err(Error::InvalidOption(
                                        "overlapping genetic bindings",
                                    ));
                                }
                                binding.insert(index);
                            }
                            partitions.push(binding);
                        }
                        Ok(partitions)
                    })
                    .collect()
            };
        let individual = Individual {
            inputs: convert(&self.inputs)?,
            outputs: convert(&self.outputs)?,
        };
        for i in 0..self.activities.len() {
            for j in 0..self.activities.len() {
                if flat(&individual.outputs[i]).contains(&j)
                    != flat(&individual.inputs[j]).contains(&i)
                {
                    return Err(Error::InvalidOption("genetic input/output edges disagree"));
                }
            }
        }
        Ok(individual)
    }

    /// Convert with the classic sophisticated mapping, falling back to silent
    /// transitions for adjacent non-simple bindings. No reduction is applied.
    /// An empty matrix returns a silent workflow shell.
    pub fn to_petri_net(&self) -> Result<AcceptingPetriNet> {
        let individual = self.individual()?;
        let n = self.activities.len();
        let mut net = PetriNet::new("genetic");
        let source = net.add_place("i");
        let sink = net.add_place("o");
        let transitions: Vec<_> = self
            .activities
            .iter()
            .enumerate()
            .map(|(i, label)| net.add_transition(format!("t{i}"), Some(label.clone())))
            .collect();
        if n == 0 {
            let tau = net.add_transition("skip", None::<Label>);
            net.add_input_arc(source, tau)
                .expect("place and transition were created in this net");
            net.add_output_arc(tau, sink)
                .expect("place and transition were created in this net");
        }
        let input_sets: BTreeSet<_> = individual.inputs.iter().flatten().cloned().collect();
        let output_sets: BTreeSet<_> = individual.outputs.iter().flatten().cloned().collect();
        let mut simple = Vec::new();
        for inputs in &input_sets {
            for outputs in &output_sets {
                if inputs
                    .iter()
                    .all(|&i| individual.outputs[i].contains(outputs))
                    && outputs
                        .iter()
                        .all(|&o| individual.inputs[o].contains(inputs))
                {
                    simple.push((inputs.clone(), outputs.clone()));
                }
            }
        }
        let mut other = Vec::new();
        for i in 0..n {
            for binding in &individual.inputs[i] {
                other.push((binding.clone(), BTreeSet::from([i])));
            }
            for binding in &individual.outputs[i] {
                other.push((BTreeSet::from([i]), binding.clone()));
            }
        }
        other.retain(|(inputs, outputs)| {
            !simple
                .iter()
                .any(|(a, b)| inputs.is_subset(a) && outputs.is_subset(b))
        });
        while let Some(index) = simple.iter().position(|(inputs, outputs)| {
            other
                .iter()
                .any(|(a, b)| !inputs.is_disjoint(a) || !outputs.is_disjoint(b))
        }) {
            other.push(simple.remove(index));
        }
        let (sources, sinks) = boundaries(&individual);
        for i in sources {
            net.add_input_arc(source, transitions[i])
                .expect("place and transition were created in this net");
        }
        for i in sinks {
            net.add_output_arc(transitions[i], sink)
                .expect("place and transition were created in this net");
        }
        for (i, (inputs, outputs)) in simple.iter().enumerate() {
            let place = net.add_place(format!("p{i}"));
            for &t in inputs {
                net.add_output_arc(transitions[t], place)
                    .expect("place and transition were created in this net");
            }
            for &t in outputs {
                net.add_input_arc(place, transitions[t])
                    .expect("place and transition were created in this net");
            }
        }
        let producers: BTreeSet<_> = other.iter().flat_map(|(a, _)| a.iter().copied()).collect();
        let consumers: BTreeSet<_> = other.iter().flat_map(|(_, b)| b.iter().copied()).collect();
        let mut silent = BTreeMap::new();
        for t in producers {
            for (i, binding) in individual.outputs[t].iter().enumerate() {
                let place = net.add_place(format!("out-{t}-{i}"));
                net.add_output_arc(transitions[t], place)
                    .expect("place and transition were created in this net");
                for &u in binding {
                    let tau = *silent.entry((t, u)).or_insert_with(|| {
                        net.add_transition(format!("tau-{t}-{u}"), None::<Label>)
                    });
                    net.add_input_arc(place, tau)
                        .expect("place and transition were created in this net");
                }
            }
        }
        for t in consumers {
            for (i, binding) in individual.inputs[t].iter().enumerate() {
                let place = net.add_place(format!("in-{t}-{i}"));
                net.add_input_arc(place, transitions[t])
                    .expect("place and transition were created in this net");
                for &u in binding {
                    let tau = *silent.entry((u, t)).or_insert_with(|| {
                        net.add_transition(format!("tau-{u}-{t}"), None::<Label>)
                    });
                    net.add_output_arc(tau, place)
                        .expect("place and transition were created in this net");
                }
            }
        }
        Ok(AcceptingPetriNet::new(
            net,
            Marking::from([(source, 1)]),
            Marking::from([(sink, 1)]),
        ))
    }
}
