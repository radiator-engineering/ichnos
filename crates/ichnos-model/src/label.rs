//! Activity labels shared by every model type.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

/// The name of an activity, as carried by a visible Petri net transition or a
/// process tree leaf.
///
/// `Label` is a thin owned string. A later release can add conversions to the
/// interned activity ids of `ichnos-core`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Label(String);

impl Label {
    /// Creates a label from anything that converts into a `String`.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the label and returns its text.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl Deref for Label {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Label {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for Label {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<Label> for String {
    fn from(l: Label) -> Self {
        l.0
    }
}

impl PartialEq<str> for Label {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Label {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}
