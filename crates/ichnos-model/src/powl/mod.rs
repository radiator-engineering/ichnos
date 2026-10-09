//! Partially ordered workflow language (POWL) models, ported from pm4py's
//! `objects/powl/`.
//!
//! A [`Powl`] model is a silent step, an activity, a choice ([`Powl::Xor`])
//! or loop ([`Powl::Loop`]) over sub-models, or a [`StrictPartialOrder`]
//! over sub-models. Unordered children of a partial order run concurrently.
//!
//! Children of a partial order are identified by their position. pm4py
//! identifies activities by object identity and operator and partial-order
//! nodes by structure, so in pm4py two equal operator children of one
//! partial order share their order pairs.

mod parse;
mod relation;

pub use parse::PowlParseError;
pub use relation::BinaryRelation;

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use crate::{Label, Operator};

/// Structural problems in a POWL model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PowlError {
    /// A choice has fewer than two children.
    #[error("a choice needs at least two children; this one has {0}")]
    ChoiceArity(usize),
    /// A loop does not have exactly two children.
    #[error("a loop needs two children (do, redo); this one has {0}")]
    LoopArity(usize),
    /// A partial order relates a child to itself.
    #[error("the order relates child {0} to itself")]
    Reflexive(usize),
    /// A partial order is not transitive.
    #[error("the order has {0} -> {1} -> {2} but not {0} -> {2}")]
    NotTransitive(usize, usize, usize),
    /// A partial order has a cycle, so no process tree can express it.
    #[error("the order has a cycle")]
    Cyclic,
    /// A process tree operator POWL cannot express.
    #[error("POWL cannot express the {0} operator")]
    UnsupportedOperator(Operator),
}

/// An activity with frequency annotations (pm4py's `FrequentTransition`).
///
/// [`Powl::simplify_using_frequent_transitions`] creates these from a choice
/// between an activity and a silent step (skippable) and from a loop between
/// an activity and a silent step (self-loop).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrequentTransition {
    /// The activity.
    pub activity: Label,
    /// The activity may be skipped.
    pub skippable: bool,
    /// The activity may repeat.
    pub selfloop: bool,
}

impl FrequentTransition {
    /// Creates a frequent transition.
    pub fn new(activity: impl Into<Label>, skippable: bool, selfloop: bool) -> Self {
        Self {
            activity: activity.into(),
            skippable,
            selfloop,
        }
    }

    /// The label pm4py gives the transition: the activity, followed by
    /// `\n[1,1]` when it is skippable and `\n[1,-]` when it repeats.
    ///
    /// pm4py always shows a minimum of 1, also for a skippable activity.
    pub fn label(&self) -> Label {
        if self.selfloop {
            Label::new(format!("{}\n[1,-]", self.activity))
        } else if self.skippable {
            Label::new(format!("{}\n[1,1]", self.activity))
        } else {
            self.activity.clone()
        }
    }
}

/// A partial order over sub-models (pm4py's `StrictPartialOrder`).
///
/// The order is a [`BinaryRelation`] over the positions of the children.
/// `i -> j` says child `i` completes before child `j` starts. A valid order
/// is irreflexive and transitive; [`Powl::validate`] checks this.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StrictPartialOrder {
    children: Vec<Powl>,
    order: BinaryRelation,
}

impl StrictPartialOrder {
    /// Creates a partial order over `children` with no pairs, so all of them
    /// run concurrently.
    pub fn new(children: impl IntoIterator<Item = Powl>) -> Self {
        let children: Vec<Powl> = children.into_iter().collect();
        let order = BinaryRelation::new(children.len());
        Self { children, order }
    }

    /// Creates a total order: each child before all later ones (pm4py's
    /// `Sequence`).
    pub fn sequence(children: impl IntoIterator<Item = Powl>) -> Self {
        let mut po = Self::new(children);
        let n = po.children.len();
        for i in 0..n {
            for j in i + 1..n {
                po.order.add_edge(i, j);
            }
        }
        po
    }

    /// The children.
    pub fn children(&self) -> &[Powl] {
        &self.children
    }

    /// The children, mutable. The slice keeps its length, so the order stays
    /// in step with it.
    pub fn children_mut(&mut self) -> &mut [Powl] {
        &mut self.children
    }

    /// The order over the children's positions.
    pub fn order(&self) -> &BinaryRelation {
        &self.order
    }

    /// The order, mutable.
    pub fn order_mut(&mut self) -> &mut BinaryRelation {
        &mut self.order
    }

    /// Adds a child with no order pairs and returns its position.
    pub fn add_child(&mut self, child: Powl) -> usize {
        self.children.push(child);
        self.order.push_node()
    }

    /// Orders child `source` before child `target`.
    ///
    /// # Panics
    ///
    /// When a position is out of range.
    pub fn add_edge(&mut self, source: usize, target: usize) {
        self.order.add_edge(source, target);
    }

    /// Returns `true` when child `source` is ordered before child `target`.
    ///
    /// # Panics
    ///
    /// When a position is out of range.
    pub fn is_edge(&self, source: usize, target: usize) -> bool {
        self.order.is_edge(source, target)
    }

    /// Splits the partial order into its children and its order.
    pub fn into_parts(self) -> (Vec<Powl>, BinaryRelation) {
        (self.children, self.order)
    }

    /// Returns `true` when child `i` is ordered with some other child.
    fn connected(&self, i: usize) -> bool {
        (0..self.children.len()).any(|j| self.is_edge(i, j) || self.is_edge(j, i))
    }
}

/// A POWL model.
///
/// [`Powl::Xor`] should have at least two children, as pm4py requires;
/// [`Powl::validate`] reports one with fewer, and the parser and
/// [`ProcessTree::to_powl`](crate::ProcessTree::to_powl) never build one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Powl {
    /// A silent step (pm4py's `SilentTransition`).
    Silent,
    /// A visible activity (pm4py's `Transition`).
    Activity(Label),
    /// An activity with frequency annotations.
    Frequent(FrequentTransition),
    /// Exactly one child runs.
    Xor(Vec<Powl>),
    /// The do part runs, then optionally the redo part followed by the do
    /// part again, any number of times.
    Loop(Box<[Powl; 2]>),
    /// The children run as their partial order allows.
    PartialOrder(StrictPartialOrder),
}

impl From<StrictPartialOrder> for Powl {
    fn from(po: StrictPartialOrder) -> Self {
        Powl::PartialOrder(po)
    }
}

impl From<FrequentTransition> for Powl {
    fn from(t: FrequentTransition) -> Self {
        Powl::Frequent(t)
    }
}

impl Powl {
    /// Creates an activity.
    pub fn activity(label: impl Into<Label>) -> Self {
        Powl::Activity(label.into())
    }

    /// Creates a choice.
    pub fn xor(children: impl IntoIterator<Item = Powl>) -> Self {
        Powl::Xor(children.into_iter().collect())
    }

    /// Creates a loop with a do and a redo part.
    pub fn looped(do_part: Powl, redo: Powl) -> Self {
        Powl::Loop(Box::new([do_part, redo]))
    }

    /// Creates a partial order with no pairs.
    pub fn concurrent(children: impl IntoIterator<Item = Powl>) -> Self {
        Powl::PartialOrder(StrictPartialOrder::new(children))
    }

    /// Creates a total order over `children`.
    pub fn sequence(children: impl IntoIterator<Item = Powl>) -> Self {
        Powl::PartialOrder(StrictPartialOrder::sequence(children))
    }

    /// Parses pm4py's POWL string syntax (pm4py's
    /// `parse_powl_model_string`). See [`PowlParseError`] for the grammar.
    pub fn parse(s: &str) -> Result<Self, PowlParseError> {
        parse::parse(s)
    }

    /// The label pm4py gives this node: the activity of an activity, the
    /// annotated label of a frequent transition, and `None` otherwise.
    pub fn label(&self) -> Option<Label> {
        match self {
            Powl::Activity(l) => Some(l.clone()),
            Powl::Frequent(t) => Some(t.label()),
            _ => None,
        }
    }

    /// Returns `true` for a silent step.
    pub fn is_silent(&self) -> bool {
        matches!(self, Powl::Silent)
    }

    /// Returns `true` for a silent step, an activity or a frequent
    /// transition (pm4py's `Transition` class).
    pub fn is_transition(&self) -> bool {
        matches!(self, Powl::Silent | Powl::Activity(_) | Powl::Frequent(_))
    }

    /// The children of an operator or partial order; empty for a leaf.
    pub fn children(&self) -> &[Powl] {
        match self {
            Powl::Xor(c) => c,
            Powl::Loop(c) => &c[..],
            Powl::PartialOrder(po) => po.children(),
            _ => &[],
        }
    }

    /// Checks the model: every choice has at least two children and every
    /// partial order is irreflexive and transitive (pm4py's
    /// `validate_partial_orders`, plus the arity check pm4py makes when it
    /// builds a choice).
    pub fn validate(&self) -> Result<(), PowlError> {
        match self {
            Powl::Xor(c) if c.len() < 2 => return Err(PowlError::ChoiceArity(c.len())),
            Powl::PartialOrder(po) => {
                if let Some(i) = (0..po.children.len()).find(|&i| po.is_edge(i, i)) {
                    return Err(PowlError::Reflexive(i));
                }
                if let Some((i, j, k)) = po.order.transitivity_gap() {
                    return Err(PowlError::NotTransitive(i, j, k));
                }
            }
            _ => {}
        }
        self.children().iter().try_for_each(Powl::validate)
    }

    /// Renames activities whose name is a key of `labels` (pm4py's
    /// `powl.utils.label_replacing.apply`). A frequent transition is renamed
    /// by its activity.
    pub fn replace_labels(&mut self, labels: &BTreeMap<String, String>) {
        match self {
            Powl::Activity(l) => {
                if let Some(new) = labels.get(l.as_str()) {
                    *l = Label::new(new.clone());
                }
            }
            Powl::Frequent(t) => {
                if let Some(new) = labels.get(t.activity.as_str()) {
                    t.activity = Label::new(new.clone());
                }
            }
            Powl::Silent => {}
            Powl::Xor(c) => c.iter_mut().for_each(|c| c.replace_labels(labels)),
            Powl::Loop(c) => c.iter_mut().for_each(|c| c.replace_labels(labels)),
            Powl::PartialOrder(po) => po
                .children
                .iter_mut()
                .for_each(|c| c.replace_labels(labels)),
        }
    }

    /// pm4py's `simplify`.
    ///
    /// - A choice between a silent step and a loop whose do or redo part is
    ///   silent becomes a loop with a silent do part.
    /// - A choice inside a choice is merged into it.
    /// - A partial order inside a partial order is merged into it when it is
    ///   not ordered with any sibling, or when it has one start and one end
    ///   child; the outer pairs then attach to those.
    ///
    /// The merged order is closed under transitivity, so the result stays a
    /// strict partial order. pm4py leaves out the pairs that transitivity
    /// implies.
    pub fn simplify(&self) -> Powl {
        match self {
            Powl::Xor(c) => {
                if let [a, b] = c.as_slice()
                    && let Some(merged) = merge_skip_loop(a, b).or_else(|| merge_skip_loop(b, a))
                {
                    return merged;
                }
                let mut children = Vec::new();
                for child in c {
                    match child.simplify() {
                        Powl::Xor(grand) => children.extend(grand.iter().map(Powl::simplify)),
                        s => children.push(s),
                    }
                }
                Powl::Xor(children)
            }
            Powl::Loop(c) => Powl::looped(c[0].simplify(), c[1].simplify()),
            Powl::PartialOrder(po) => Powl::PartialOrder(simplify_partial_order(po)),
            leaf => leaf.clone(),
        }
    }

    /// pm4py's `simplify_using_frequent_transitions`.
    ///
    /// A choice between an activity `a` (or frequent transition) and a
    /// silent step becomes a skippable `a`. A loop with `a` as do part and a
    /// silent redo becomes a repeating `a`; with a silent do part and `a` as
    /// redo, a skippable, repeating `a`.
    ///
    /// A choice or loop between two silent steps stays as it is. pm4py turns
    /// it into a frequent transition with no activity, labelled
    /// `None\n[1,1]` or `None\n[1,-]`.
    pub fn simplify_using_frequent_transitions(&self) -> Powl {
        let visible = |p: &Powl| p.is_transition() && !p.is_silent();
        let frequent = |p: &Powl, skippable, selfloop| {
            let activity = p.label().expect("a visible transition has a label");
            Powl::Frequent(FrequentTransition::new(activity, skippable, selfloop))
        };
        match self {
            Powl::Xor(c) => match c.as_slice() {
                [a, b] if visible(a) && b.is_silent() => frequent(a, true, false),
                [a, b] if a.is_silent() && visible(b) => frequent(b, true, false),
                _ => Powl::Xor(
                    c.iter()
                        .map(Powl::simplify_using_frequent_transitions)
                        .collect(),
                ),
            },
            Powl::Loop(c) => match &**c {
                [a, b] if visible(a) && b.is_silent() => frequent(a, false, true),
                [a, b] if a.is_silent() && visible(b) => frequent(b, true, true),
                [a, b] => Powl::looped(
                    a.simplify_using_frequent_transitions(),
                    b.simplify_using_frequent_transitions(),
                ),
            },
            Powl::PartialOrder(po) => Powl::PartialOrder(StrictPartialOrder {
                children: po
                    .children
                    .iter()
                    .map(Powl::simplify_using_frequent_transitions)
                    .collect(),
                order: po.order.clone(),
            }),
            leaf => leaf.clone(),
        }
    }

    /// Writes the node as an operator child: pm4py quotes a labelled leaf
    /// there.
    fn fmt_operand(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.label() {
            Some(l) => write!(f, "'{l}'"),
            None => fmt::Display::fmt(self, f),
        }
    }
}

/// pm4py's `merge_with_children` inside `OperatorPOWL.simplify`.
fn merge_skip_loop(skip: &Powl, other: &Powl) -> Option<Powl> {
    let (Powl::Silent, Powl::Loop(c)) = (skip, other) else {
        return None;
    };
    let [d, r] = &**c;
    if d.is_silent() {
        Some(Powl::looped(d.simplify(), r.simplify()))
    } else if r.is_silent() {
        Some(Powl::looped(r.simplify(), d.simplify()))
    } else {
        None
    }
}

/// pm4py's `StrictPartialOrder.simplify`.
fn simplify_partial_order(po: &StrictPartialOrder) -> StrictPartialOrder {
    enum Part {
        Kept(Powl),
        /// A partial order merged into the parent, with its single start and
        /// end child when it is ordered with a sibling.
        Merged(StrictPartialOrder, Option<(usize, usize)>),
    }
    let parts: Vec<Part> = po
        .children
        .iter()
        .enumerate()
        .map(|(i, child)| match child.simplify() {
            Powl::PartialOrder(sub) if !po.connected(i) => Part::Merged(sub, None),
            Powl::PartialOrder(sub) => {
                match (
                    sub.order.start_nodes().as_slice(),
                    sub.order.end_nodes().as_slice(),
                ) {
                    (&[s], &[e]) => Part::Merged(sub, Some((s, e))),
                    _ => Part::Kept(Powl::PartialOrder(sub)),
                }
            }
            s => Part::Kept(s),
        })
        .collect();

    // Kept children come first, then the children of merged orders.
    let mut position = vec![0; parts.len()];
    let mut children = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        if let Part::Kept(p) = part {
            position[i] = children.len();
            children.push(p.clone());
        }
    }
    let mut inner = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        if let Part::Merged(sub, _) = part {
            position[i] = children.len();
            inner.extend(
                sub.order
                    .edges()
                    .map(|(a, b)| (position[i] + a, position[i] + b)),
            );
            children.extend(sub.children.iter().cloned());
        }
    }
    let mut res = StrictPartialOrder::new(children);
    let ends = |i: usize, pick: fn((usize, usize)) -> usize| match &parts[i] {
        Part::Kept(_) => position[i],
        Part::Merged(_, Some(se)) => position[i] + pick(*se),
        Part::Merged(_, None) => unreachable!("an unordered child has no pairs"),
    };
    for (i, j) in po.order.edges() {
        res.add_edge(ends(i, |(_, e)| e), ends(j, |(s, _)| s));
    }
    for (a, b) in inner {
        res.add_edge(a, b);
    }
    res.order.add_transitive_edges();
    res
}

/// Returns `true` when a label written bare would not read back as itself.
fn needs_quotes(label: &str) -> bool {
    label.is_empty()
        || label == "tau"
        || label.starts_with('\'')
        || label.trim() != label
        || label.contains(['\n', '\r', '\t', ',', '(', ')', '{', '}'])
        || label.contains("-->")
}

fn fmt_bare(label: &Label, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if needs_quotes(label) {
        write!(f, "'{label}'")
    } else {
        f.write_str(label)
    }
}

/// pm4py's string form: `PO=(nodes={a, X( 'b', 'c' )}, order={a-->X( 'b',
/// 'c' )})`.
///
/// Activities are bare inside a partial order and quoted inside an
/// operator, as in pm4py. An order pair names each child by its string
/// form. pm4py names a child without a label by its hash (`id_<n>`), which
/// its parser cannot read back. A bare label that would not read back as
/// itself is quoted.
impl fmt::Display for Powl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Powl::Silent => f.write_str("tau"),
            Powl::Activity(l) => fmt_bare(l, f),
            Powl::Frequent(t) => f.write_str(&t.label()),
            Powl::Xor(c) => write_operator(f, "X", c),
            Powl::Loop(c) => write_operator(f, "*", &c[..]),
            Powl::PartialOrder(po) => {
                let nodes: Vec<String> = po.children.iter().map(ToString::to_string).collect();
                write!(f, "PO=(nodes={{{}}}, order={{", nodes.join(", "))?;
                for (n, (i, j)) in po.order.edges().enumerate() {
                    if n > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}-->{}", nodes[i], nodes[j])?;
                }
                f.write_str("})")
            }
        }
    }
}

fn write_operator(f: &mut fmt::Formatter<'_>, symbol: &str, children: &[Powl]) -> fmt::Result {
    write!(f, "{symbol}( ")?;
    for (i, c) in children.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        c.fmt_operand(f)?;
    }
    f.write_str(" )")
}

impl FromStr for Powl {
    type Err = PowlParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Powl::parse(s)
    }
}

#[cfg(test)]
mod tests;
