//! Label matching and structural or behavioral comparison of process models.
use crate::{AcceptingPetriNet, Bpmn, Dfg, Footprints, Label, Operator, Powl, ProcessTree};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A typed alternative to heterogeneous positional model arguments.
#[derive(Debug, Clone)]
pub enum Model {
    /// An accepting Petri net, including both markings.
    Petri(AcceptingPetriNet),
    /// A process tree.
    Tree(ProcessTree),
    /// A POWL model.
    Powl(Powl),
    /// A BPMN diagram.
    Bpmn(Bpmn),
    /// A directly-follows graph.
    Dfg(Dfg),
}

/// Invalid matching options or failed model conversion.
#[derive(Debug, thiserror::Error)]
pub enum ComparisonError {
    /// Matching thresholds must be finite and between zero and one.
    #[error("threshold must be finite and between zero and one")]
    InvalidThreshold,
    /// Direct DFG relabeling is not defined by the model wrapper.
    #[error("convert a DFG to an accepting net before relabeling")]
    UnsupportedRelabeling,
    /// A conversion required by the comparison is unavailable.
    #[error("model conversion failed: {0}")]
    Conversion(String),
}

impl Model {
    /// Converts to an accepting Petri net without modifying the input.
    pub fn to_petri_net(&self) -> Result<AcceptingPetriNet, ComparisonError> {
        Ok(match self {
            Self::Petri(n) => n.clone(),
            Self::Tree(t) => {
                t.validate()
                    .map_err(|e| ComparisonError::Conversion(e.to_string()))?;
                t.to_petri_net()
            }
            Self::Powl(p) => p
                .to_petri_net()
                .map_err(|e| ComparisonError::Conversion(e.to_string()))?,
            Self::Bpmn(b) => b.to_petri_net(Default::default()).net,
            Self::Dfg(d) => d.to_petri_net(),
        })
    }
    /// Sorted, distinct visible transition labels after net conversion.
    pub fn activity_labels(&self) -> Result<BTreeSet<Label>, ComparisonError> {
        Ok(self
            .to_petri_net()?
            .net
            .transitions()
            .filter_map(|(_, t)| t.label.clone())
            .collect())
    }
    /// Returns a renamed copy. Silent steps and markings are retained.
    pub fn replace_activity_labels(
        &self,
        mapping: &BTreeMap<String, String>,
    ) -> Result<Self, ComparisonError> {
        let mut out = self.clone();
        match &mut out {
            Self::Petri(n) => {
                let ids: Vec<_> = n.net.transitions().map(|(id, _)| id).collect();
                for id in ids {
                    let t = n.net.transition_mut(id);
                    if let Some(label) = t.label.as_ref().and_then(|l| mapping.get(l.as_str())) {
                        t.label = Some(label.clone().into());
                    }
                }
            }
            Self::Tree(t) => rename_tree(t, mapping),
            Self::Powl(p) => p.replace_labels(mapping),
            Self::Bpmn(b) => b.replace_task_labels(mapping),
            Self::Dfg(_) => return Err(ComparisonError::UnsupportedRelabeling),
        }
        Ok(out)
    }
    /// Rename a copy of this model using a one-to-one match to the second model.
    pub fn map_labels_from_second_model(
        &self,
        target: &Model,
        minimum: f64,
    ) -> Result<Self, ComparisonError> {
        self.replace_activity_labels(&map_labels(
            &self.activity_labels()?,
            &target.activity_labels()?,
            minimum,
        )?)
    }
    /// Compare visible label sets after net conversion.
    pub fn label_sets_similarity(
        &self,
        target: &Model,
        minimum: f64,
    ) -> Result<f64, ComparisonError> {
        label_sets_similarity(
            &self.activity_labels()?,
            &target.activity_labels()?,
            minimum,
        )
    }
    /// Convert via POWL before measuring structural features.
    pub fn structural_tree(&self) -> Result<ProcessTree, ComparisonError> {
        let p = match self {
            Self::Powl(p) => structural_powl_input(p),
            Self::Tree(t) => t
                .to_powl()
                .map_err(|e| ComparisonError::Conversion(e.to_string()))?,
            _ => self
                .to_petri_net()?
                .net
                .to_powl()
                .map_err(|e| ComparisonError::Conversion(e.to_string()))?,
        };
        p.to_process_tree()
            .map_err(|e| ComparisonError::Conversion(e.to_string()))
    }
}
// The top-level source re-converts POWL through its ProcessTree base class.
// The PARTIALORDER operator falls through to an order with no edges.
// Keep this quirk local to the comparison dispatcher; Powl::to_process_tree
// continues to preserve its partial-order relations.
fn structural_powl_input(p: &Powl) -> Powl {
    match p {
        Powl::PartialOrder(po) => {
            crate::powl::StrictPartialOrder::new(po.children().iter().map(structural_powl_input))
                .into()
        }
        Powl::Frequent(t) => Powl::Activity(t.label()),
        Powl::Xor(children) => Powl::Xor(children.iter().map(structural_powl_input).collect()),
        Powl::Loop(children) => Powl::Loop(Box::new([
            structural_powl_input(&children[0]),
            structural_powl_input(&children[1]),
        ])),
        other => other.clone(),
    }
}
fn rename_tree(t: &mut ProcessTree, mapping: &BTreeMap<String, String>) {
    match t {
        ProcessTree::Activity(l) => {
            if let Some(new) = mapping.get(l.as_str()) {
                *l = new.clone().into();
            }
        }
        ProcessTree::Node(_, children) => {
            for child in children {
                rename_tree(child, mapping);
            }
        }
        ProcessTree::Tau => {}
    }
}

/// Python SequenceMatcher's matching-block ratio, over Unicode characters.
/// Frequent characters in the second string are excluded as anchors at length 200.
pub fn label_similarity(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut indices: HashMap<char, Vec<usize>> = HashMap::new();
    for (i, c) in b.iter().enumerate() {
        indices.entry(*c).or_default().push(i);
    }
    if b.len() >= 200 {
        indices.retain(|_, v| v.len() <= b.len() / 100 + 1);
    }
    let mut ranges = vec![(0, a.len(), 0, b.len())];
    let mut matches = 0;
    while let Some((alo, ahi, blo, bhi)) = ranges.pop() {
        let (mut ai, mut bi, mut size) = (alo, blo, 0);
        let mut previous: HashMap<usize, usize> = HashMap::new();
        for (i, c) in a.iter().enumerate().take(ahi).skip(alo) {
            let mut current = HashMap::new();
            if let Some(js) = indices.get(c) {
                for &j in js {
                    if j < blo || j >= bhi {
                        continue;
                    }
                    let k = j
                        .checked_sub(1)
                        .and_then(|p| previous.get(&p))
                        .copied()
                        .unwrap_or(0)
                        + 1;
                    current.insert(j, k);
                    if k > size {
                        ai = i + 1 - k;
                        bi = j + 1 - k;
                        size = k;
                    }
                }
            }
            previous = current;
        }
        while ai > alo && bi > blo && a[ai - 1] == b[bi - 1] {
            ai -= 1;
            bi -= 1;
            size += 1;
        }
        while ai + size < ahi && bi + size < bhi && a[ai + size] == b[bi + size] {
            size += 1;
        }
        if size > 0 {
            matches += size;
            if alo < ai && blo < bi {
                ranges.push((alo, ai, blo, bi));
            }
            if ai + size < ahi && bi + size < bhi {
                ranges.push((ai + size, ahi, bi + size, bhi));
            }
        }
    }
    2.0 * matches as f64 / (a.len() + b.len()) as f64
}
fn threshold(t: f64) -> Result<(), ComparisonError> {
    if !t.is_finite() || !(0.0..=1.0).contains(&t) {
        Err(ComparisonError::InvalidThreshold)
    } else {
        Ok(())
    }
}
/// Greedy one-to-one fuzzy label matches, resolving ties lexically.
pub fn label_sets_similarity(
    a: &BTreeSet<Label>,
    b: &BTreeSet<Label>,
    minimum: f64,
) -> Result<f64, ComparisonError> {
    threshold(minimum)?;
    if a.is_empty() && b.is_empty() {
        return Ok(1.0);
    }
    let mut remaining = b.clone();
    let mut matches = 0;
    for x in a {
        let mut best = None;
        let mut score = 0.0;
        for y in &remaining {
            let s = label_similarity(x, y);
            if s > score {
                best = Some(y.clone());
                score = s;
            }
        }
        if score >= minimum
            && let Some(y) = best
        {
            remaining.remove(&y);
            matches += 1;
        }
    }
    Ok(2.0 * matches as f64 / (a.len() + b.len()) as f64)
}
/// Match source labels to unused target labels; leave unmatched labels intact.
pub fn map_labels(
    a: &BTreeSet<Label>,
    b: &BTreeSet<Label>,
    minimum: f64,
) -> Result<BTreeMap<String, String>, ComparisonError> {
    threshold(minimum)?;
    let mut used = BTreeSet::new();
    let mut mapping = BTreeMap::new();
    for x in a {
        let mut best = None;
        let mut score = 0.0;
        if b.contains(x) && !used.contains(x) {
            best = Some(x.clone());
            score = 1.0;
        } else {
            for y in b.difference(&used) {
                let s = label_similarity(x, y);
                if s > score {
                    best = Some(y.clone());
                    score = s;
                }
            }
        }
        let y = if score >= minimum { best } else { None };
        if let Some(y) = y {
            used.insert(y.clone());
            mapping.insert(x.to_string(), y.to_string());
        } else {
            mapping.insert(x.to_string(), x.to_string());
        }
    }
    Ok(mapping)
}
/// Jaccard similarity of sequence and parallel relations, separately typed.
/// Models without either relation have similarity zero, including self-comparison.
pub fn behavioral_similarity(a: &Footprints, b: &Footprints) -> f64 {
    let total = a.sequence.union(&b.sequence).count() + a.parallel.union(&b.parallel).count();
    if total == 0 {
        0.0
    } else {
        (a.sequence.intersection(&b.sequence).count()
            + a.parallel.intersection(&b.parallel).count()) as f64
            / total as f64
    }
}
/// The ten structural features used by process-tree comparison.
pub fn structural_features(t: &ProcessTree) -> [f64; 10] {
    fn visit(t: &ProcessTree, depth: usize, f: &mut [f64; 10], branches: &mut usize) {
        f[0] += 1.0;
        f[2] = f[2].max(depth as f64);
        if let ProcessTree::Node(op, children) = t {
            f[8] += 1.0;
            *branches += children.len();
            match op {
                Operator::Parallel => f[4] += 1.0,
                Operator::Xor => f[5] += 1.0,
                Operator::Or => f[6] += 1.0,
                Operator::Loop => f[7] += 1.0,
                _ => {}
            }
            for child in children {
                visit(child, depth + 1, f, branches);
            }
        }
    }
    let mut f = [0.0; 10];
    let mut branches = 0;
    visit(t, 1, &mut f, &mut branches);
    f[1] = f[0] - 1.0;
    f[3] = if f[8] > 0.0 {
        branches as f64 / f[8]
    } else {
        0.0
    };
    f[9] = 1.0;
    f
}
/// Pairwise-max normalized structural feature distance, converted to similarity.
pub fn structural_similarity(a: &Model, b: &Model) -> Result<f64, ComparisonError> {
    let a = structural_features(&a.structural_tree()?);
    let b = structural_features(&b.structural_tree()?);
    let distance: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            let max = x.max(y);
            if max == 0.0 {
                0.0
            } else {
                ((x - y) / max).powi(2)
            }
        })
        .sum();
    Ok(1.0 - distance.sqrt() / 10.0_f64.sqrt())
}
