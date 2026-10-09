use std::collections::BTreeMap;
use std::fmt;

use super::{PetriNet, PlaceId};

/// A marking: the number of tokens in each place.
///
/// Places with zero tokens are never stored, so two markings are equal
/// exactly when every place holds the same count. Iteration is in place-id
/// order, which keeps every algorithm built on markings deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Marking(BTreeMap<PlaceId, u32>);

impl Marking {
    /// Creates the empty marking.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the token count of a place (zero if absent).
    pub fn get(&self, place: PlaceId) -> u32 {
        self.0.get(&place).copied().unwrap_or(0)
    }

    /// Sets the token count of a place. Setting zero removes the place.
    pub fn set(&mut self, place: PlaceId, tokens: u32) {
        if tokens == 0 {
            self.0.remove(&place);
        } else {
            self.0.insert(place, tokens);
        }
    }

    /// Adds tokens to a place.
    pub fn add(&mut self, place: PlaceId, tokens: u32) {
        if tokens > 0 {
            *self.0.entry(place).or_insert(0) += tokens;
        }
    }

    /// Removes up to `tokens` tokens from a place, stopping at zero.
    pub fn remove(&mut self, place: PlaceId, tokens: u32) {
        let left = self.get(place).saturating_sub(tokens);
        self.set(place, left);
    }

    /// Returns `true` if no place holds a token.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of places that hold at least one token.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Total number of tokens.
    pub fn total_tokens(&self) -> u64 {
        self.0.values().map(|&n| u64::from(n)).sum()
    }

    /// Iterates over `(place, tokens)` pairs with a non-zero count.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (PlaceId, u32)> + '_ {
        self.0.iter().map(|(&p, &n)| (p, n))
    }

    /// Returns `true` if every place holds at most as many tokens here as in
    /// `other` (pm4py's `Marking.__le__`).
    pub fn is_covered_by(&self, other: &Marking) -> bool {
        self.iter().all(|(p, n)| other.get(p) >= n)
    }

    /// Formats the marking with place names, as pm4py's `repr` does:
    /// `['p1:1', 'p2:2']`, sorted by place name.
    pub fn display<'a>(&'a self, net: &'a PetriNet) -> impl fmt::Display + 'a {
        DisplayMarking { marking: self, net }
    }
}

struct DisplayMarking<'a> {
    marking: &'a Marking,
    net: &'a PetriNet,
}

impl fmt::Display for DisplayMarking<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut items: Vec<(&str, u32)> = self
            .marking
            .iter()
            .map(|(p, n)| (self.net.place(p).name.as_str(), n))
            .collect();
        items.sort();
        f.write_str("[")?;
        for (i, (name, n)) in items.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "'{name}:{n}'")?;
        }
        f.write_str("]")
    }
}

impl FromIterator<(PlaceId, u32)> for Marking {
    fn from_iter<I: IntoIterator<Item = (PlaceId, u32)>>(iter: I) -> Self {
        let mut m = Marking::new();
        for (p, n) in iter {
            m.add(p, n);
        }
        m
    }
}

impl<const N: usize> From<[(PlaceId, u32); N]> for Marking {
    fn from(items: [(PlaceId, u32); N]) -> Self {
        items.into_iter().collect()
    }
}

impl std::ops::Add<&Marking> for &Marking {
    type Output = Marking;

    fn add(self, rhs: &Marking) -> Marking {
        let mut m = self.clone();
        for (p, n) in rhs.iter() {
            Marking::add(&mut m, p, n);
        }
        m
    }
}
