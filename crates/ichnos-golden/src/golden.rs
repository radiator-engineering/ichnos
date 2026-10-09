use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::GoldenError;
use crate::paths::{golden_dir, golden_path, workspace_root};

/// One loaded golden file: what pm4py produced for one case.
#[derive(Debug, Clone)]
pub struct Golden {
    /// The area, for example `log`.
    pub area: String,
    /// The case id within the area.
    pub case: String,
    /// The file it was loaded from.
    pub path: PathBuf,
    /// The `meta` object: pm4py version and commit, functions, fixtures, parameters.
    pub meta: Value,
    /// The `expected` value.
    pub expected: Value,
}

/// The typed form of a golden file's `meta` object.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Meta {
    /// The area.
    pub area: String,
    /// The case id.
    pub case: String,
    /// pm4py's version, for example `2.7.23.8`.
    pub pm4py_version: String,
    /// The full pm4py commit hash.
    pub pm4py_commit: String,
    /// The pm4py functions the case called.
    pub functions: Vec<String>,
    /// Role (for example `log` or `model`) to fixture path, relative to the workspace root.
    pub fixtures: BTreeMap<String, String>,
    /// Role to the loader call used for that fixture.
    pub loaders: BTreeMap<String, Value>,
    /// The case's parameters.
    pub params: Value,
}

impl Golden {
    /// The typed `meta` object.
    ///
    /// # Panics
    ///
    /// Panics if `meta` lacks a field of [`Meta`].
    #[track_caller]
    pub fn meta(&self) -> Meta {
        self.part_as("meta", &self.meta)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    /// Deserialize `expected` into the caller's type.
    ///
    /// # Panics
    ///
    /// Panics with the file path and the serde error if the shape does not match.
    #[track_caller]
    pub fn expected_as<T: DeserializeOwned>(&self) -> T {
        self.try_expected_as().unwrap_or_else(|err| panic!("{err}"))
    }

    /// Deserialize `expected` into the caller's type, returning an error on mismatch.
    pub fn try_expected_as<T: DeserializeOwned>(&self) -> Result<T, GoldenError> {
        self.part_as("expected", &self.expected)
    }

    /// The part of `expected` at a JSON pointer, such as `/activities/decide`.
    ///
    /// # Panics
    ///
    /// Panics if nothing is at `pointer`.
    #[track_caller]
    pub fn expected_at(&self, pointer: &str) -> &Value {
        self.expected
            .pointer(pointer)
            .unwrap_or_else(|| panic!("{}: no value at `expected{pointer}`", self.path.display()))
    }

    /// The absolute path of the fixture with the given role (`log` for single-fixture cases).
    ///
    /// # Panics
    ///
    /// Panics if the case has no fixture with that role.
    #[track_caller]
    pub fn fixture(&self, role: &str) -> PathBuf {
        let meta = self.meta();
        let rel = meta.fixtures.get(role).unwrap_or_else(|| {
            panic!(
                "{}: no fixture with role `{role}` (roles: {:?})",
                self.path.display(),
                meta.fixtures.keys().collect::<Vec<_>>()
            )
        });
        workspace_root().join(rel)
    }

    fn part_as<T: DeserializeOwned>(
        &self,
        part: &'static str,
        value: &Value,
    ) -> Result<T, GoldenError> {
        T::deserialize(value).map_err(|source| GoldenError::Shape {
            path: self.path.clone(),
            part,
            source,
        })
    }
}

/// Load the golden file for `area`/`case`.
///
/// # Panics
///
/// Panics with the path and the cause if the file is missing or malformed.
#[track_caller]
pub fn golden(area: &str, case: &str) -> Golden {
    try_golden(area, case).unwrap_or_else(|err| panic!("{err}"))
}

/// Load the golden file for `area`/`case`, returning an error if it is missing or malformed.
pub fn try_golden(area: &str, case: &str) -> Result<Golden, GoldenError> {
    let path = golden_path(area, case);
    let text = std::fs::read_to_string(&path).map_err(|source| GoldenError::Io {
        path: path.clone(),
        source,
    })?;
    let mut doc: Value = serde_json::from_str(&text).map_err(|source| GoldenError::Json {
        path: path.clone(),
        source,
    })?;
    let mut take = |field: &'static str| {
        doc.get_mut(field)
            .map(Value::take)
            .ok_or_else(|| GoldenError::MissingField {
                path: path.clone(),
                field,
            })
    };
    let meta = take("meta")?;
    let expected = take("expected")?;
    Ok(Golden {
        area: area.to_owned(),
        case: case.to_owned(),
        path,
        meta,
        expected,
    })
}

/// The areas that have golden files, sorted.
pub fn areas() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(golden_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    out.sort();
    out
}

/// The case ids in `area`, sorted. Empty if the area has no golden files.
pub fn cases(area: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(golden_dir().join(area))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.strip_suffix(".json").map(str::to_owned)
        })
        .collect();
    out.sort();
    out
}
