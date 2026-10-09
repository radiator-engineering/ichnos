//! Seeded causal-partition discovery from pm4py's
//! `algo.discovery.genetic.variants.classic`.
mod matrix;
mod operators;
use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog, Position};
use ichnos_model::{AcceptingPetriNet, Label, Marking, PetriNet};
pub use matrix::GeneticMatrix;
use operators::Individual;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeMap;

/// Genetic search options. Fractions must be finite and in `[0,1]`.
#[derive(Debug, Clone)]
pub struct GeneticOptions {
    /// Population size, at least two (default 500).
    pub population_size: usize,
    /// Fraction retained unchanged (default 0.01, round ties to even).
    pub elitism_rate: f64,
    /// Probability of binding crossover (default 1).
    pub crossover_rate: f64,
    /// Per-activity probability of repartitioning each input/output (default 0.01).
    pub mutation_rate: f64,
    /// Generation budget, including the initial population (default 100).
    /// Zero and one both evaluate only the initial population.
    pub generations: usize,
    /// Tournament sample size (default 5), clamped to population size minus one.
    pub elitism_min_sample: usize,
    /// ChaCha8 seed (default zero); Python's random sequence is not reproduced.
    pub seed: u64,
}

impl Default for GeneticOptions {
    fn default() -> Self {
        Self {
            population_size: 500,
            elitism_rate: 0.01,
            crossover_rate: 1.0,
            mutation_rate: 0.01,
            generations: 100,
            elitism_min_sample: 5,
            seed: 0,
        }
    }
}

/// Selected model and reproducible search diagnostics.
#[derive(Debug, Clone)]
pub struct GeneticResult {
    /// Petri net using sophisticated and silent-transition binding conversion.
    pub model: AcceptingPetriNet,
    /// Selected input/output partitions.
    pub matrix: GeneticMatrix,
    /// 0.4 times average trace fitness plus 0.6 times fitting-trace fraction.
    pub fitness: f64,
    /// Best fitness after each evaluated population, including the initial one.
    pub history: Vec<f64>,
}

fn fitness(log: &EventLog, keys: &EventKeys, model: &AcceptingPetriNet) -> Result<f64> {
    let metrics = ichnos_conformance::token_replay::fitness_token_based_replay(
        log,
        &model.net,
        &model.initial_marking,
        &model.final_marking,
        keys,
    )
    .map_err(Error::GeneticReplay)?;
    Ok(0.4 * metrics.average_trace_fitness + 0.6 * metrics.percentage_of_fitting_traces / 100.0)
}

/// Evaluate pm4py's genetic fitness formula for a supplied causal matrix.
///
/// Uses the existing token replay, caller activity keys, and the same stable
/// per-trace timestamp sorting as `discover_genetic`. Every event must have
/// a date timestamp. pm4py's tournament
/// replays input order, so unsorted logs may yield different scores.
pub fn genetic_matrix_fitness(
    log: &EventLog,
    keys: &EventKeys,
    matrix: &GeneticMatrix,
) -> Result<f64> {
    fitness(&prepare_log(log, keys)?, keys, &matrix.to_petri_net()?)
}

// Share preparation so public discovery and matrix fitness replay the same order.
fn prepare_log(log: &EventLog, keys: &EventKeys) -> Result<EventLog> {
    let mut prepared = log.clone();
    for (ti, trace) in prepared.traces.iter_mut().enumerate() {
        let mut order = Vec::new();
        for (ei, event) in trace.events.iter().enumerate() {
            let value =
                event
                    .get(&keys.timestamp)
                    .ok_or_else(|| ichnos_core::Error::MissingAttribute {
                        key: keys.timestamp.clone(),
                        position: Position::Event {
                            trace: ti,
                            event: ei,
                        },
                    })?;
            let date = value
                .as_date()
                .ok_or_else(|| ichnos_core::Error::AttributeType {
                    key: keys.timestamp.clone(),
                    position: Position::Event {
                        trace: ti,
                        event: ei,
                    },
                    expected: "date",
                    found: value.type_name(),
                })?;
            order.push((date, ei));
        }
        order.sort_by_key(|&(date, ei)| (date, ei));
        trace.events = order
            .iter()
            .map(|&(_, ei)| trace.events[ei].clone())
            .collect();
    }
    Ok(prepared)
}

fn evaluate(
    population: Vec<Individual>,
    labels: &[Label],
    log: &EventLog,
    keys: &EventKeys,
) -> Result<Vec<(Individual, f64)>> {
    let mut scored = Vec::new();
    for individual in population {
        let matrix = GeneticMatrix::from_individual(labels, &individual);
        scored.push((
            individual,
            fitness(&prepare_log(log, keys)?, keys, &matrix.to_petri_net()?)?,
        ));
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    Ok(scored)
}

/// Discover a causal-partition model with seeded initial sampling, connectivity
/// repair, stable fitness tournaments, elitism, crossover and repartition mutation.
/// Events are stably timestamp-sorted within canonical traces. Case IDs are not
/// used to merge traces. Custom keys apply to both discovery and fitness.
/// Fitness replays this sorted copy; pm4py tournaments replay input order,
/// so logs out of time order can change scores and selected models.
/// Stagnation stopping follows pm4py's available history window;
/// it does not require a full half-budget window. Identical parents safely fall
/// back to different population indices rather than a Python sample-size error.
pub fn discover_genetic(
    log: &EventLog,
    keys: &EventKeys,
    options: &GeneticOptions,
) -> Result<GeneticResult> {
    if options.population_size < 2
        || options.elitism_min_sample == 0
        || [
            options.elitism_rate,
            options.crossover_rate,
            options.mutation_rate,
        ]
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(Error::InvalidOption(
            "genetic population must be at least two, sample positive, rates finite and in `[0,1]`",
        ));
    }
    let sequences = log.activity_sequences(keys)?;
    let labels: Vec<_> = sequences
        .activities
        .iter()
        .map(|(_, label)| Label::from(label))
        .collect();
    let n = labels.len();
    let prepared = prepare_log(log, keys)?;
    if n == 0 {
        let mut net = PetriNet::new("genetic-empty");
        let source = net.add_place("i");
        let sink = net.add_place("o");
        let tau = net.add_transition("skip", None::<Label>);
        net.add_input_arc(source, tau)
            .expect("place and transition were created in this net");
        net.add_output_arc(tau, sink)
            .expect("place and transition were created in this net");
        let model = AcceptingPetriNet::new(
            net,
            Marking::from([(source, 1)]),
            Marking::from([(sink, 1)]),
        );
        let score = fitness(&prepared, keys, &model)?;
        return Ok(GeneticResult {
            model,
            matrix: GeneticMatrix {
                activities: labels,
                inputs: BTreeMap::new(),
                outputs: BTreeMap::new(),
            },
            fitness: score,
            history: vec![score],
        });
    }
    let sorted = prepared.activity_sequences(keys)?;
    let index: BTreeMap<_, _> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| (label.as_str(), i))
        .collect();
    let mut counts = vec![vec![0u64; n]; n];
    for trace in &sorted.traces {
        for pair in trace.windows(2) {
            let a = index[sorted.activities.name(pair[0])];
            let b = index[sorted.activities.name(pair[1])];
            counts[a][b] += 1;
        }
    }
    let mut rng = ChaCha8Rng::seed_from_u64(options.seed);
    let mut initial = Vec::new();
    for _ in 0..options.population_size {
        initial.push(operators::initial(&counts, &mut rng));
    }
    let mut scored = evaluate(initial, &labels, &prepared, keys)?;
    let mut history = vec![scored[0].1];
    let mut stopping_history = Vec::new();
    for _ in 1..options.generations {
        let best = scored[0].1;
        let window = options.generations / 2;
        let lower = stopping_history.len().saturating_sub(window);
        if best == 1.0
            || (!stopping_history.is_empty()
                && stopping_history[lower..].iter().all(|&value| value == best))
        {
            break;
        }
        stopping_history.push(best);
        let elites =
            (options.elitism_rate * options.population_size as f64).round_ties_even() as usize;
        let mut next: Vec<_> = scored
            .iter()
            .take(elites)
            .map(|(individual, _)| individual.clone())
            .collect();
        while next.len() < options.population_size {
            let (a, b) = operators::parents(
                &scored,
                options.elitism_min_sample.min(options.population_size - 1),
                &mut rng,
            );
            let children = if rng.random::<f64>() < options.crossover_rate {
                operators::crossover(&scored[a].0, &scored[b].0, &mut rng)
            } else {
                [scored[a].0.clone(), scored[b].0.clone()]
            };
            for mut child in children {
                if next.len() == options.population_size {
                    break;
                }
                operators::mutate(&mut child, options.mutation_rate, &mut rng);
                next.push(child);
            }
        }
        scored = evaluate(next, &labels, &prepared, keys)?;
        history.push(scored[0].1);
    }
    let matrix = GeneticMatrix::from_individual(&labels, &scored[0].0);
    let model = matrix.to_petri_net()?;
    Ok(GeneticResult {
        model,
        matrix,
        fitness: scored[0].1,
        history,
    })
}

/// Discover only the selected Petri net; use [`discover_genetic`] for diagnostics.
pub fn petri_net_genetic(
    log: &EventLog,
    keys: &EventKeys,
    options: &GeneticOptions,
) -> Result<AcceptingPetriNet> {
    Ok(discover_genetic(log, keys, options)?.model)
}
