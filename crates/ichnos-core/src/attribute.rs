//! Typed attribute values and the ordered [`Attributes`] map.

use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, Timelike, Utc};

/// A typed attribute value, one variant per XES attribute type.
///
/// Strings and keys are `Arc<str>`, so readers can share one allocation
/// between all events that carry the same value. Use the `as_*` accessors to
/// read values. They look through [`AttributeValue::Meta`], so a value that
/// carries XES meta-attributes reads the same as a plain one.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    /// XES `string`.
    String(Arc<str>),
    /// XES `int`.
    Int(i64),
    /// XES `float`.
    Float(f64),
    /// XES `boolean`.
    Bool(bool),
    /// XES `date`. The offset is kept so XES output can repeat it; ordering and
    /// equality compare the instant.
    Date(DateTime<FixedOffset>),
    /// XES `id`.
    Id(Arc<str>),
    /// XES `list`: an ordered sequence of child attributes. Keys may repeat.
    List(Vec<(Arc<str>, AttributeValue)>),
    /// XES 2.0 `container`: a map of child attributes.
    Container(Attributes),
    /// A value with XES meta-attributes (child attributes of a scalar or list).
    Meta(Box<MetaValue>),
}

/// A value together with its XES meta-attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct MetaValue {
    /// The value itself. Never another [`AttributeValue::Meta`].
    pub value: AttributeValue,
    /// The meta-attributes nested under the value.
    pub meta: Attributes,
}

impl AttributeValue {
    /// Builds a string value.
    pub fn string(value: impl Into<Arc<str>>) -> Self {
        Self::String(value.into())
    }

    /// Builds an ID value.
    pub fn id(value: impl Into<Arc<str>>) -> Self {
        Self::Id(value.into())
    }

    /// Attaches meta-attributes. Adds to existing ones if the value has some.
    /// An empty `meta` returns the value unchanged.
    pub fn with_meta(self, meta: Attributes) -> Self {
        if meta.is_empty() {
            return self;
        }
        match self {
            Self::Meta(mut boxed) => {
                boxed.meta.extend(meta);
                Self::Meta(boxed)
            }
            value => Self::Meta(Box::new(MetaValue { value, meta })),
        }
    }

    /// The value without its meta-attributes.
    pub fn plain(&self) -> &AttributeValue {
        match self {
            Self::Meta(boxed) => &boxed.value,
            value => value,
        }
    }

    /// The meta-attributes, if the value has any.
    pub fn meta(&self) -> Option<&Attributes> {
        match self {
            Self::Meta(boxed) => Some(&boxed.meta),
            _ => None,
        }
    }

    /// The XES type name of the plain value: `string`, `int`, `float`,
    /// `boolean`, `date`, `id`, `list` or `container`.
    pub fn type_name(&self) -> &'static str {
        match self.plain() {
            Self::String(_) => "string",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Bool(_) => "boolean",
            Self::Date(_) => "date",
            Self::Id(_) => "id",
            Self::List(_) => "list",
            Self::Container(_) => "container",
            Self::Meta(_) => unreachable!("plain() strips Meta"),
        }
    }

    /// The text of a string or ID value.
    pub fn as_str(&self) -> Option<&str> {
        match self.plain() {
            Self::String(s) | Self::Id(s) => Some(s),
            _ => None,
        }
    }

    /// An int value.
    pub fn as_i64(&self) -> Option<i64> {
        match self.plain() {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// A float value, or an int value widened to `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self.plain() {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// A boolean value.
    pub fn as_bool(&self) -> Option<bool> {
        match self.plain() {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// A date value.
    pub fn as_date(&self) -> Option<DateTime<FixedOffset>> {
        match self.plain() {
            Self::Date(v) => Some(*v),
            _ => None,
        }
    }

    /// The children of a list value.
    pub fn as_list(&self) -> Option<&[(Arc<str>, AttributeValue)]> {
        match self.plain() {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// The children of a container value.
    pub fn as_container(&self) -> Option<&Attributes> {
        match self.plain() {
            Self::Container(children) => Some(children),
            _ => None,
        }
    }
}

/// Formats the plain value the way Python's `str()` formats the pm4py value,
/// so joined classifiers and stringified activities match pm4py: `True`,
/// `1.0`, `2020-01-01 10:00:00+00:00`.
impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.plain() {
            Self::String(s) | Self::Id(s) => f.write_str(s),
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => fmt_python_float(*v, f),
            Self::Bool(true) => f.write_str("True"),
            Self::Bool(false) => f.write_str("False"),
            Self::Date(d) => fmt_python_datetime(d, f),
            Self::List(items) => {
                f.write_str("[")?;
                for (i, (key, value)) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "({key}, {value})")?;
                }
                f.write_str("]")
            }
            Self::Container(children) => {
                f.write_str("{")?;
                for (i, (key, value)) in children.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                f.write_str("}")
            }
            Self::Meta(_) => unreachable!("plain() strips Meta"),
        }
    }
}

fn fmt_python_float(v: f64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if v.is_nan() {
        f.write_str("nan")
    } else if v.is_infinite() {
        f.write_str(if v > 0.0 { "inf" } else { "-inf" })
    } else {
        // Python's repr: the shortest digits that round-trip, in exponent form
        // when the decimal exponent is below -4 or at least 16.
        let sci = format!("{v:e}");
        let (mantissa, exp) = sci.split_once('e').expect("`{:e}` has an exponent");
        let exp: i32 = exp.parse().expect("`{:e}` exponent is an integer");
        if (-4..16).contains(&exp) {
            if v.fract() == 0.0 {
                write!(f, "{v:.1}")
            } else {
                write!(f, "{v}")
            }
        } else {
            let sign = if exp < 0 { '-' } else { '+' };
            write!(f, "{mantissa}e{sign}{:02}", exp.abs())
        }
    }
}

fn fmt_python_datetime(d: &DateTime<FixedOffset>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", d.format("%Y-%m-%d %H:%M:%S"))?;
    let micros = d.nanosecond() % 1_000_000_000 / 1_000;
    if micros != 0 {
        write!(f, ".{micros:06}")?;
    }
    write!(f, "{}", d.format("%:z"))
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<Arc<str>> for AttributeValue {
    fn from(value: Arc<str>) -> Self {
        Self::String(value)
    }
}

impl From<i64> for AttributeValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<i32> for AttributeValue {
    fn from(value: i32) -> Self {
        Self::Int(value.into())
    }
}

impl From<u32> for AttributeValue {
    fn from(value: u32) -> Self {
        Self::Int(value.into())
    }
}

impl From<f64> for AttributeValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for AttributeValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<DateTime<FixedOffset>> for AttributeValue {
    fn from(value: DateTime<FixedOffset>) -> Self {
        Self::Date(value)
    }
}

impl From<DateTime<Utc>> for AttributeValue {
    fn from(value: DateTime<Utc>) -> Self {
        Self::Date(value.fixed_offset())
    }
}

impl From<Attributes> for AttributeValue {
    fn from(value: Attributes) -> Self {
        Self::Container(value)
    }
}

/// An attribute map that keeps insertion order, so XES output repeats the
/// input order.
///
/// Events usually carry a handful of attributes, so the map is a vector of
/// pairs with linear lookup: one allocation per event and no hashing.
/// Equality ignores order, as pm4py's does.
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    entries: Vec<(Arc<str>, AttributeValue)>,
}

impl Attributes {
    /// An empty map.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// An empty map with room for `capacity` attributes.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// The number of attributes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn position(&self, key: &str) -> Option<usize> {
        self.entries.iter().position(|(k, _)| &**k == key)
    }

    /// The value for `key`.
    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.position(key).map(|i| &self.entries[i].1)
    }

    /// A mutable reference to the value for `key`.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut AttributeValue> {
        self.position(key).map(|i| &mut self.entries[i].1)
    }

    /// Whether the map holds `key`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.position(key).is_some()
    }

    /// Sets `key` to `value`. An existing key keeps its position and the old
    /// value is returned; a new key goes to the end.
    pub fn insert<K, V>(&mut self, key: K, value: V) -> Option<AttributeValue>
    where
        K: AsRef<str> + Into<Arc<str>>,
        V: Into<AttributeValue>,
    {
        let value = value.into();
        match self.position(key.as_ref()) {
            Some(i) => Some(std::mem::replace(&mut self.entries[i].1, value)),
            None => {
                self.entries.push((key.into(), value));
                None
            }
        }
    }

    /// Removes `key`, keeping the order of the rest.
    pub fn remove(&mut self, key: &str) -> Option<AttributeValue> {
        self.position(key).map(|i| self.entries.remove(i).1)
    }

    /// Keeps only the attributes for which `keep` returns true.
    pub fn retain(&mut self, mut keep: impl FnMut(&str, &AttributeValue) -> bool) {
        self.entries.retain(|(k, v)| keep(k, v));
    }

    /// Removes every attribute.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Iterates over `(key, value)` pairs in insertion order.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.entries.iter(),
        }
    }

    /// Iterates over the keys in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &Arc<str>> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Iterates over the values in insertion order.
    pub fn values(&self) -> impl Iterator<Item = &AttributeValue> {
        self.entries.iter().map(|(_, v)| v)
    }
}

impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<K, V> FromIterator<(K, V)> for Attributes
where
    K: AsRef<str> + Into<Arc<str>>,
    V: Into<AttributeValue>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut attributes = Self::new();
        attributes.extend(iter);
        attributes
    }
}

impl<K, V> Extend<(K, V)> for Attributes
where
    K: AsRef<str> + Into<Arc<str>>,
    V: Into<AttributeValue>,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl IntoIterator for Attributes {
    type Item = (Arc<str>, AttributeValue);
    type IntoIter = std::vec::IntoIter<(Arc<str>, AttributeValue)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a Arc<str>, &'a AttributeValue);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the pairs of an [`Attributes`] map.
#[derive(Debug, Clone)]
pub struct Iter<'a> {
    inner: std::slice::Iter<'a, (Arc<str>, AttributeValue)>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Arc<str>, &'a AttributeValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Iter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn insert_keeps_order_and_replaces_in_place() {
        let mut a = Attributes::new();
        a.insert("b", 1);
        a.insert("a", "x");
        assert_eq!(a.insert("b", 2.5), Some(AttributeValue::Int(1)));
        let keys: Vec<&str> = a.keys().map(|k| &**k).collect();
        assert_eq!(keys, ["b", "a"]);
        assert_eq!(a.get("b"), Some(&AttributeValue::Float(2.5)));
        assert_eq!(a.remove("b"), Some(AttributeValue::Float(2.5)));
        assert!(!a.contains_key("b"));
    }

    #[test]
    fn equality_ignores_order() {
        let a: Attributes = [("x", 1), ("y", 2)].into_iter().collect();
        let b: Attributes = [("y", 2), ("x", 1)].into_iter().collect();
        assert_eq!(a, b);
        let c: Attributes = [("y", 2), ("x", 3)].into_iter().collect();
        assert_ne!(a, c);
    }

    #[test]
    fn typed_accessors() {
        let date = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2020, 1, 1, 10, 0, 0)
            .unwrap();
        assert_eq!(AttributeValue::from("a").as_str(), Some("a"));
        assert_eq!(AttributeValue::id("id-1").as_str(), Some("id-1"));
        assert_eq!(AttributeValue::id("id-1").type_name(), "id");
        assert_eq!(AttributeValue::from(3).as_i64(), Some(3));
        assert_eq!(AttributeValue::from(3).as_f64(), Some(3.0));
        assert_eq!(AttributeValue::from(true).as_bool(), Some(true));
        assert_eq!(AttributeValue::from(date).as_date(), Some(date));
        assert_eq!(AttributeValue::from(3).as_str(), None);
    }

    #[test]
    fn meta_values_read_like_plain_ones() {
        let meta: Attributes = [("unit", "EUR")].into_iter().collect();
        let v = AttributeValue::from(12.5).with_meta(meta.clone());
        assert_eq!(v.as_f64(), Some(12.5));
        assert_eq!(v.type_name(), "float");
        assert_eq!(v.meta(), Some(&meta));
        assert_eq!(v.to_string(), "12.5");
        assert_eq!(
            AttributeValue::from(1).with_meta(Attributes::new()),
            AttributeValue::Int(1)
        );
    }

    #[test]
    fn display_matches_python_str() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let d = utc.with_ymd_and_hms(2020, 1, 1, 10, 0, 0).unwrap();
        assert_eq!(
            AttributeValue::from(d).to_string(),
            "2020-01-01 10:00:00+00:00"
        );
        let d = d + chrono::Duration::microseconds(1500);
        assert_eq!(
            AttributeValue::from(d).to_string(),
            "2020-01-01 10:00:00.001500+00:00"
        );
        assert_eq!(AttributeValue::from(1.0).to_string(), "1.0");
        assert_eq!(AttributeValue::from(0.1).to_string(), "0.1");
        assert_eq!(AttributeValue::from(f64::NAN).to_string(), "nan");
        assert_eq!(AttributeValue::from(1e16).to_string(), "1e+16");
        assert_eq!(AttributeValue::from(1e15).to_string(), "1000000000000000.0");
        assert_eq!(AttributeValue::from(1e-5).to_string(), "1e-05");
        assert_eq!(AttributeValue::from(1e-4).to_string(), "0.0001");
        assert_eq!(
            AttributeValue::from(1.2345678901234568e17).to_string(),
            "1.2345678901234568e+17"
        );
        assert_eq!(AttributeValue::from(-2.5e-300).to_string(), "-2.5e-300");
        assert_eq!(AttributeValue::from(false).to_string(), "False");
        assert_eq!(AttributeValue::from(-4).to_string(), "-4");
    }

    #[test]
    fn value_stays_small() {
        assert!(std::mem::size_of::<AttributeValue>() <= 32);
    }
}
