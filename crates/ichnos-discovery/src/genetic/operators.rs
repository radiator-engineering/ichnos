use rand::{Rng, RngExt, seq::SliceRandom};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Individual {
    pub inputs: Vec<Vec<BTreeSet<usize>>>,
    pub outputs: Vec<Vec<BTreeSet<usize>>>,
}

pub(super) fn flat(partitions: &[BTreeSet<usize>]) -> BTreeSet<usize> {
    partitions.iter().flatten().copied().collect()
}

fn roots(partitions: &[Vec<BTreeSet<usize>>], order: &[usize]) -> Vec<usize> {
    let mut graphs: Vec<Vec<usize>> = Vec::new();
    for &t in order {
        let i = if let Some(i) = graphs.iter().position(|graph| graph.contains(&t)) {
            i
        } else {
            graphs.push(vec![t]);
            graphs.len() - 1
        };
        let successors: Vec<_> = partitions[t].iter().flatten().copied().collect();
        graphs[i].extend(&successors);
        // Keep the selected graph last while merging, so removing predecessors
        // cannot invalidate its index. Source equality is list equality.
        let mut graph = graphs.remove(i);
        let mut insertion = i;
        for next in successors {
            if let Some(j) = graphs
                .iter()
                .position(|other| other[0] == next && *other != graph)
            {
                graph.extend(graphs.remove(j));
                if j < insertion {
                    insertion -= 1;
                }
            }
        }
        graphs.insert(insertion, graph);
    }
    graphs.iter().map(|graph| graph[0]).collect()
}

pub(super) fn boundaries(individual: &Individual) -> (Vec<usize>, Vec<usize>) {
    boundaries_for(
        individual,
        &(0..individual.inputs.len()).collect::<Vec<_>>(),
    )
}

fn boundaries_for(individual: &Individual, order: &[usize]) -> (Vec<usize>, Vec<usize>) {
    (
        roots(&individual.outputs, order),
        roots(&individual.inputs, order),
    )
}

fn partition(mut pool: Vec<usize>, rng: &mut impl Rng) -> Vec<BTreeSet<usize>> {
    let mut result = Vec::new();
    while !pool.is_empty() {
        let count = rng.random_range(1..=pool.len());
        pool.shuffle(rng);
        result.push(pool.drain(..count).collect());
        pool.sort_unstable();
    }
    result
}

fn connect(individual: &mut Individual, from: usize, to: usize) {
    individual.outputs[from].push(BTreeSet::from([to]));
    individual.inputs[to].push(BTreeSet::from([from]));
}

fn weighted_connect(
    individual: &mut Individual,
    from: &[usize],
    to: &[usize],
    counts: &[Vec<u64>],
    rng: &mut impl Rng,
) {
    let pairs: Vec<_> = from
        .iter()
        .flat_map(|&a| to.iter().map(move |&b| (a, b)))
        .collect();
    let total: f64 = pairs.iter().map(|&(a, b)| counts[a][b] as f64).sum();
    let index = if total == 0.0 {
        rng.random_range(0..pairs.len())
    } else {
        let mut draw = rng.random::<f64>() * total;
        let mut selected = pairs.len() - 1;
        for (i, &(a, b)) in pairs.iter().enumerate() {
            draw -= counts[a][b] as f64;
            if draw < 0.0 {
                selected = i;
                break;
            }
        }
        selected
    };
    let (a, b) = pairs[index];
    connect(individual, a, b);
}

fn repair(individual: &mut Individual, counts: &[Vec<u64>], rng: &mut impl Rng) {
    let mut left: BTreeSet<_> = (0..counts.len()).collect();
    let mut components = Vec::new();
    while let Some(first) = left.pop_first() {
        let mut component = BTreeSet::from([first]);
        let mut queue = vec![first];
        while let Some(t) = queue.pop() {
            for next in flat(&individual.inputs[t]).union(&flat(&individual.outputs[t])) {
                if left.remove(next) {
                    component.insert(*next);
                    queue.push(*next);
                }
            }
        }
        components.push(component.into_iter().collect::<Vec<_>>());
    }
    while components.len() > 1 {
        components.shuffle(rng);
        let (sources, sinks) = boundaries_for(individual, &components[1]);
        for t in sources {
            weighted_connect(individual, &components[0], &[t], counts, rng);
        }
        for t in sinks {
            weighted_connect(individual, &[t], &components[0], counts, rng);
        }
        let merging = components.remove(0);
        components[0].extend(merging);
        components[0].sort_unstable();
    }
}

pub(super) fn initial(counts: &[Vec<u64>], rng: &mut impl Rng) -> Individual {
    let n = counts.len();
    let mut result = Individual {
        inputs: vec![Vec::new(); n],
        outputs: vec![Vec::new(); n],
    };
    for (a, row) in counts.iter().enumerate() {
        let total: f64 = row.iter().map(|&count| count as f64).sum();
        for (b, &count) in row.iter().enumerate() {
            if rng.random::<f64>()
                < if total > 0.0 {
                    count as f64 / total
                } else {
                    0.0
                }
            {
                connect(&mut result, a, b);
            }
        }
    }
    repair(&mut result, counts, rng);
    for partitions in result.inputs.iter_mut().chain(&mut result.outputs) {
        *partitions = partition(flat(partitions).into_iter().collect(), rng);
    }
    result
}

fn tournament(
    candidates: &mut [usize],
    sample: usize,
    scored: &[(Individual, f64)],
    rng: &mut impl Rng,
) -> usize {
    candidates.shuffle(rng);
    let mut best = candidates[0];
    for &i in candidates.iter().take(sample) {
        if scored[i].1 > scored[best].1 {
            best = i;
        }
    }
    best
}

pub(super) fn parents(
    scored: &[(Individual, f64)],
    sample: usize,
    rng: &mut impl Rng,
) -> (usize, usize) {
    let a = tournament(
        &mut (0..scored.len()).collect::<Vec<_>>(),
        sample,
        scored,
        rng,
    );
    let mut candidates: Vec<_> = (0..scored.len())
        .filter(|&i| scored[i].0 != scored[a].0)
        .collect();
    if candidates.is_empty() {
        candidates = (0..scored.len()).filter(|&i| i != a).collect();
    }
    let b = tournament(&mut candidates, sample, scored, rng);
    (a, b)
}

fn synchronize(
    partitions: &mut [Vec<BTreeSet<usize>>],
    t: usize,
    old: &BTreeSet<usize>,
    new: &BTreeSet<usize>,
) {
    for &neighbor in new.difference(old) {
        if !flat(&partitions[neighbor]).contains(&t) {
            partitions[neighbor].push(BTreeSet::from([t]));
        }
    }
    for &neighbor in old.difference(new) {
        for binding in &mut partitions[neighbor] {
            binding.remove(&t);
        }
        partitions[neighbor].retain(|binding| !binding.is_empty());
    }
}

fn swap(
    first: &mut [Vec<BTreeSet<usize>>],
    second: &mut [Vec<BTreeSet<usize>>],
    reverse_first: &mut [Vec<BTreeSet<usize>>],
    reverse_second: &mut [Vec<BTreeSet<usize>>],
    t: usize,
    rng: &mut impl Rng,
) {
    if first[t].is_empty() || second[t].is_empty() {
        return;
    }
    let point = rng.random_range(0..first[t].len().min(second[t].len()));
    let old_a = flat(&first[t]);
    let old_b = flat(&second[t]);
    let merge = |prefix: &[BTreeSet<usize>], tail: &[BTreeSet<usize>]| {
        let used = flat(prefix);
        let mut result = prefix.to_vec();
        result.extend(
            tail.iter()
                .map(|binding| binding.difference(&used).copied().collect::<BTreeSet<_>>())
                .filter(|binding| !binding.is_empty()),
        );
        result
    };
    let new_a = merge(&first[t][..point], &second[t][point..]);
    let new_b = merge(&second[t][..point], &first[t][point..]);
    first[t] = new_a;
    second[t] = new_b;
    synchronize(reverse_first, t, &old_a, &flat(&first[t]));
    synchronize(reverse_second, t, &old_b, &flat(&second[t]));
}

pub(super) fn crossover(a: &Individual, b: &Individual, rng: &mut impl Rng) -> [Individual; 2] {
    let mut first = a.clone();
    let mut second = b.clone();
    let t = rng.random_range(0..a.inputs.len());
    swap(
        &mut first.inputs,
        &mut second.inputs,
        &mut first.outputs,
        &mut second.outputs,
        t,
        rng,
    );
    swap(
        &mut first.outputs,
        &mut second.outputs,
        &mut first.inputs,
        &mut second.inputs,
        t,
        rng,
    );
    [first, second]
}

pub(super) fn mutate(individual: &mut Individual, rate: f64, rng: &mut impl Rng) {
    for bindings in individual.inputs.iter_mut().chain(&mut individual.outputs) {
        if rng.random::<f64>() < rate {
            *bindings = partition(flat(bindings).into_iter().collect(), rng);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use serde_json::{Value, json};
    fn parse(value: &Value) -> Individual {
        let convert = |v: &Value| {
            v.as_array()
                .unwrap()
                .iter()
                .map(|bindings| {
                    bindings
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|set| {
                            set.as_array()
                                .unwrap()
                                .iter()
                                .map(|v| v.as_u64().unwrap() as usize)
                                .collect()
                        })
                        .collect()
                })
                .collect()
        };
        Individual {
            inputs: convert(&value["inputs"]),
            outputs: convert(&value["outputs"]),
        }
    }

    fn serial(individual: &Individual) -> Value {
        json!({"inputs":individual.inputs,"outputs":individual.outputs})
    }

    fn invariant(individual: &Individual) {
        let n = individual.inputs.len();
        assert_eq!(n, individual.outputs.len());
        for partitions in individual.inputs.iter().chain(&individual.outputs) {
            assert!(partitions.iter().all(|set| !set.is_empty()));
            assert_eq!(
                flat(partitions).len(),
                partitions.iter().map(BTreeSet::len).sum::<usize>()
            );
        }
        for a in 0..n {
            for b in 0..n {
                assert_eq!(
                    flat(&individual.outputs[a]).contains(&b),
                    flat(&individual.inputs[b]).contains(&a)
                );
            }
        }
    }

    #[test]
    fn crossover_matches_pm4py_operator_choices() {
        check_crossover(include_str!(
            "../../../../fixtures/golden/discovery/genetic-operators.json"
        ));
        check_crossover(include_str!(
            "../../../../fixtures/golden/discovery/genetic-operators-prefix.json"
        ));
    }

    fn check_crossover(source: &str) {
        let golden: Value = serde_json::from_str(source).unwrap();
        let expected = &golden["expected"];
        let a = parse(&expected["parents"][0]);
        let b = parse(&expected["parents"][1]);
        let mut observed = BTreeSet::new();
        for seed in 0..512 {
            let children = crossover(&a, &b, &mut ChaCha8Rng::seed_from_u64(seed));
            let actual = json!([serial(&children[0]), serial(&children[1])]);
            assert!(
                expected["crossovers"].as_array().unwrap().contains(&actual),
                "{actual}"
            );
            observed.insert(actual.to_string());
            for child in &children {
                invariant(child);
            }
        }
        assert_eq!(
            observed.len(),
            expected["crossovers"]
                .as_array()
                .unwrap()
                .iter()
                .map(Value::to_string)
                .collect::<BTreeSet<_>>()
                .len()
        );
    }

    #[test]
    fn repair_mutation_and_identical_parent_fallback() {
        for counts in [
            vec![vec![0; 4]; 4],
            vec![
                vec![0, 8, 1, 0],
                vec![0, 0, 0, 2],
                vec![0, 0, 0, 1],
                vec![0, 0, 0, 0],
            ],
        ] {
            for seed in 0..32 {
                let mut rng = ChaCha8Rng::seed_from_u64(seed);
                let mut individual = initial(&counts, &mut rng);
                invariant(&individual);
                let before = individual.clone();
                mutate(&mut individual, 0.0, &mut rng);
                assert_eq!(before, individual);
                mutate(&mut individual, 1.0, &mut rng);
                invariant(&individual);
                for i in 0..4 {
                    assert_eq!(flat(&before.inputs[i]), flat(&individual.inputs[i]));
                    assert_eq!(flat(&before.outputs[i]), flat(&individual.outputs[i]));
                }
                let population = vec![(individual.clone(), 0.5); 4];
                let (a, b) = parents(&population, 3, &mut rng);
                assert_ne!(a, b);
                let (sources, sinks) = boundaries(&individual);
                assert!(!sources.is_empty());
                assert!(!sinks.is_empty());
                // Initial repair connects the undirected graph, including zero-weight components.
                let mut seen = BTreeSet::from([0]);
                loop {
                    let old = seen.clone();
                    for &i in &old {
                        seen.extend(flat(&individual.inputs[i]));
                        seen.extend(flat(&individual.outputs[i]));
                    }
                    if old == seen {
                        break;
                    }
                }
                assert_eq!(seen.len(), 4);
            }
        }
    }
}
