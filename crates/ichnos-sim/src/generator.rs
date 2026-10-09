//! Random PTAndLogGenerator-style tree growth with triangular activity counts.
use crate::SimulationError;
use ichnos_model::process_tree::{Operator, ProcessTree};
use rand::{Rng, RngExt};

/// Parameters of the default pm4py tree generator.
#[derive(Debug, Clone)]
pub struct GeneratorOptions {
    /// Triangular activity-count mode.
    pub mode: usize,
    /// Minimum activity count.
    pub min: usize,
    /// Maximum activity count.
    pub max: usize,
    /// Relative weights: sequence, XOR, parallel, loop, OR.
    pub operator_weights: [f64; 5],
    /// Probability of inserting tau instead of the second activity.
    pub silent: f64,
    /// Probability of selecting a leaf for label duplication.
    pub duplicate: f64,
    /// Resource bound on growth attempts.
    pub max_steps: usize,
}
impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            mode: 20,
            min: 10,
            max: 30,
            operator_weights: [0.25, 0.25, 0.25, 0.25, 0.],
            silent: 0.2,
            duplicate: 0.,
            max_steps: 100_000,
        }
    }
}
/// Generates one tree. Invoke repeatedly for a model population, with the same RNG.
pub fn generate_process_tree<R: Rng + ?Sized>(
    o: &GeneratorOptions,
    rng: &mut R,
) -> Result<ProcessTree, SimulationError> {
    if o.min == 0
        || o.min > o.mode
        || o.mode > o.max
        || o.max > 10_000
        || [o.silent, o.duplicate]
            .iter()
            .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
        || o.operator_weights.iter().any(|w| !w.is_finite() || *w < 0.)
        || !o.operator_weights.iter().sum::<f64>().is_finite()
        || o.operator_weights.iter().sum::<f64>() <= 0.
    {
        return Err(SimulationError::Options(
            "invalid generator range/probabilities",
        ));
    }
    let u = rng.random::<f64>();
    let width = (o.max - o.min) as f64;
    let c = (o.mode - o.min) as f64 / (width.max(1.));
    let draw = if width == 0. {
        o.min as f64
    } else if u < c {
        o.min as f64 + (u * width * (o.mode - o.min) as f64).sqrt()
    } else {
        o.max as f64 - ((1. - u) * width * (o.max - o.mode) as f64).sqrt()
    };
    let target = draw.round_ties_even() as usize;
    let mut tree = ProcessTree::Tau;
    let mut next = 0;
    let operators = [
        Operator::Sequence,
        Operator::Xor,
        Operator::Parallel,
        Operator::Loop,
        Operator::Or,
    ];
    for _ in 0..o.max_steps {
        let visible = tree.leaves().filter(|l| l.label().is_some()).count();
        if visible >= target {
            duplicate(&mut tree, o.duplicate, rng);
            return Ok(tree);
        }
        let leaves = paths(&tree);
        let chosen = &leaves[rng.random_range(0..leaves.len())];
        let mut draw = rng.random::<f64>() * o.operator_weights.iter().sum::<f64>();
        let mut op = Operator::Or;
        for (i, w) in o.operator_weights.iter().enumerate() {
            draw -= w;
            if draw < 0. {
                op = operators[i];
                break;
            }
        }
        let old = at(&tree, chosen).clone();
        let first = if old.is_tau() {
            let a = ProcessTree::activity(label(next));
            next += 1;
            a
        } else {
            old
        };
        let added = usize::from(at(&tree, chosen).is_tau());
        let second = if target - visible > added && rng.random::<f64>() >= o.silent {
            let a = ProcessTree::activity(label(next));
            next += 1;
            a
        } else {
            ProcessTree::Tau
        };
        let children = if rng.random::<bool>() {
            vec![first, second]
        } else {
            vec![second, first]
        };
        replace(&mut tree, chosen, ProcessTree::node(op, children));
    }
    Err(SimulationError::Limit("tree growth steps"))
}
fn label(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push((b'a' + (i % 26) as u8) as char);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.into_iter().rev().collect()
}
fn paths(tree: &ProcessTree) -> Vec<Vec<usize>> {
    fn walk(t: &ProcessTree, p: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if t.is_leaf() {
            out.push(p.clone());
        } else {
            for (i, c) in t.children().iter().enumerate() {
                p.push(i);
                walk(c, p, out);
                p.pop();
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, &mut Vec::new(), &mut out);
    out
}
fn at<'a>(t: &'a ProcessTree, path: &[usize]) -> &'a ProcessTree {
    path.iter().fold(t, |node, &i| &node.children()[i])
}
fn replace(t: &mut ProcessTree, path: &[usize], new: ProcessTree) {
    if path.is_empty() {
        *t = new;
    } else if let ProcessTree::Node(_, c) = t {
        if path.len() == 1 {
            c.remove(path[0]);
            c.push(new);
        } else {
            replace(&mut c[path[0]], &path[1..], new);
        }
    }
}
fn duplicate<R: Rng + ?Sized>(tree: &mut ProcessTree, prob: f64, rng: &mut R) {
    let leaves = paths(tree);
    if !leaves.iter().any(|path| path.len() >= 2) {
        return;
    }
    let selected: Vec<_> = leaves
        .iter()
        .filter(|_| rng.random::<f64>() < prob)
        .cloned()
        .collect();
    for source in &selected {
        let candidates: Vec<_> = leaves
            .iter()
            .filter(|p| {
                !selected.contains(p)
                    && p[..p.len().saturating_sub(1)] != source[..source.len() - 1]
            })
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let label = at(tree, source).clone();
        let target = candidates[rng.random_range(0..candidates.len())];
        assign(tree, target, label);
    }
}
fn assign(t: &mut ProcessTree, path: &[usize], new: ProcessTree) {
    if path.is_empty() {
        *t = new;
    } else if let ProcessTree::Node(_, children) = t {
        assign(&mut children[path[0]], &path[1..], new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    #[test]
    fn root_child_can_be_a_duplicate_source() {
        let tree = ProcessTree::sequence([
            ProcessTree::activity("a"),
            ProcessTree::sequence([ProcessTree::activity("b"), ProcessTree::activity("c")]),
        ]);
        let mut root_source_seen = false;
        for seed in 0..100 {
            let mut copy = tree.clone();
            duplicate(&mut copy, 0.5, &mut ChaCha8Rng::seed_from_u64(seed));
            root_source_seen |= copy
                .leaves()
                .filter(|leaf| leaf.label().is_some_and(|label| label.as_str() == "a"))
                .count()
                >= 2;
        }
        assert!(root_source_seen);
        let shallow =
            ProcessTree::sequence([ProcessTree::activity("a"), ProcessTree::activity("b")]);
        let mut copy = shallow.clone();
        duplicate(&mut copy, 0.5, &mut ChaCha8Rng::seed_from_u64(1));
        assert_eq!(copy, shallow);
    }
}
