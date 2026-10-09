//! OCEL 2.0 bundles (pm4py's `read_ocel2_bundle` and `write_ocel2_bundle`).
//!
//! A bundle is a directory, or a ZIP archive whose name ends in `.ocel.zip`,
//! that holds `ocel-meta.json` and one table per event type, two per object
//! type (its objects and their attribute changes) and two relation tables.
//! The tables are all CSV or all Parquet, and `ocel-meta.json` names them
//! and types their attribute columns. Each table's path follows from its
//! type's name: the name's UTF-8 bytes, with each byte other than ASCII
//! letters, digits, `.`, `_` and `-` written as `%XX`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use ichnos_core::arrow::array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int64Array, RecordBatch, StringArray,
    TimestampMicrosecondArray,
};
use ichnos_core::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::{EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use super::csv::write_record;
use super::csv2::{decimal, parse_time, records};
use super::lower_name;
use super::write::{Columns, Kind, isoformat, missing};
use crate::error::{Error, Result};

/// How a bundle stores its tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BundleStorage {
    /// RFC 4180 CSV files.
    Csv,
    /// Parquet files, pm4py's default.
    #[default]
    Parquet,
}

impl BundleStorage {
    fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Parquet => "parquet",
        }
    }
}

const META: &str = "ocel-meta.json";

fn error(detail: impl Into<String>) -> Error {
    Error::Ocel(detail.into())
}

/// pm4py's `_percent_encode`: a type name as a file name.
fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The type of an attribute column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prim {
    String,
    Time,
    Integer,
    Float,
    Boolean,
}

impl Prim {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "string" => Self::String,
            "time" => Self::Time,
            "integer" => Self::Integer,
            "float" => Self::Float,
            "boolean" => Self::Boolean,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Time => "time",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
        }
    }

    fn arrow(self) -> DataType {
        match self {
            Self::String => DataType::Utf8,
            Self::Time => DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            Self::Integer => DataType::Int64,
            Self::Float => DataType::Float64,
            Self::Boolean => DataType::Boolean,
        }
    }
}

/// A JSON value with object entries in file order. A repeated key keeps
/// its first position and its last value, as in a Python `dict`.
#[derive(Debug, Clone, PartialEq)]
enum J {
    Null,
    Bool(bool),
    /// A number as Python's `str` writes it.
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The value as Python's `str` writes what `json.loads` gives.
    fn python(&self, repr: bool) -> String {
        match self {
            J::Null => "None".into(),
            J::Bool(true) => "True".into(),
            J::Bool(false) => "False".into(),
            J::Num(n) => n.clone(),
            J::Str(s) if repr => python_repr(s),
            J::Str(s) => s.clone(),
            J::Arr(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|i| i.python(true))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            J::Obj(entries) => format!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", python_repr(k), v.python(true)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// Python's `repr` of a string, for the printable characters JSON
/// metadata holds.
fn python_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

impl<'de> Deserialize<'de> for J {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = J;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON")
            }
            fn visit_unit<E>(self) -> std::result::Result<J, E> {
                Ok(J::Null)
            }
            fn visit_bool<E>(self, b: bool) -> std::result::Result<J, E> {
                Ok(J::Bool(b))
            }
            fn visit_i64<E>(self, i: i64) -> std::result::Result<J, E> {
                Ok(J::Num(i.to_string()))
            }
            fn visit_u64<E>(self, i: u64) -> std::result::Result<J, E> {
                Ok(J::Num(i.to_string()))
            }
            fn visit_f64<E>(self, f: f64) -> std::result::Result<J, E> {
                Ok(J::Num(AttributeValue::Float(f).to_string()))
            }
            fn visit_str<E>(self, s: &str) -> std::result::Result<J, E> {
                Ok(J::Str(s.to_string()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<J, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(J::Arr(items))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<J, A::Error> {
                let mut entries: Vec<(String, J)> = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, J>()? {
                    match entries.iter_mut().find(|(key, _)| *key == k) {
                        Some(entry) => entry.1 = v,
                        None => entries.push((k, v)),
                    }
                }
                Ok(J::Obj(entries))
            }
        }
        d.deserialize_any(V)
    }
}

/// An attribute column declared in `ocel-meta.json`.
#[derive(Debug, Clone)]
struct Attr {
    name: Arc<str>,
    prim: Prim,
}

/// The tables of one event or object type.
struct TypeTables {
    name: Arc<str>,
    file: String,
    changes: String,
    attributes: Vec<Attr>,
}

/// `ocel-meta.json` after pm4py's checks.
struct Meta {
    storage: BundleStorage,
    events: Vec<TypeTables>,
    objects: Vec<TypeTables>,
    e2o: String,
    o2o: String,
    declared: Vec<String>,
}

/// pm4py's `_validate_relative_path`.
fn relative_path(path: Option<&J>, label: &str) -> Result<String> {
    let path = match path.and_then(J::str) {
        Some(p) if !p.is_empty() && !p.contains('\\') => p,
        _ => {
            return Err(error(format!(
                "{label} must be a non-empty POSIX relative path."
            )));
        }
    };
    check_parts(path, label)?;
    Ok(path.to_string())
}

/// The root and component checks of pm4py's `_validate_relative_path`.
/// Python's `PurePosixPath` drops empty and `.` components, so only `..`
/// and a leading `/` fail.
fn check_parts(path: &str, label: &str) -> Result<()> {
    if path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err(error(format!(
            "{label} must contain no root, '.' or '..' component."
        )));
    }
    Ok(())
}

/// pm4py's `_attribute_descriptors`.
fn attribute_descriptors(descriptor: &J, label: &str, reserved: &[&str]) -> Result<Vec<Attr>> {
    let Some(J::Arr(attributes)) = descriptor.get("attributes") else {
        return Err(error(format!("{label} attributes must be an array.")));
    };
    let mut out: Vec<Attr> = Vec::new();
    for attribute in attributes {
        if !matches!(attribute, J::Obj(_)) {
            return Err(error(format!(
                "{label} attribute declarations must be objects."
            )));
        }
        let name = match attribute.get("name").and_then(J::str) {
            Some(n) if !n.is_empty() => n,
            _ => {
                return Err(error(format!(
                    "{label} attribute names must be non-empty strings."
                )));
            }
        };
        if out.iter().any(|a| &*a.name == name) {
            return Err(error(format!(
                "Duplicate attribute '{name}' in {label} metadata."
            )));
        }
        if reserved.contains(&name) {
            return Err(error(format!(
                "Attribute '{name}' collides with a fixed {label} column."
            )));
        }
        let declared = attribute.get("type");
        let Some(prim) = declared.and_then(J::str).and_then(Prim::parse) else {
            return Err(error(format!(
                "Attribute '{name}' in {label} has unsupported type '{}'.",
                declared.map_or("None".into(), |t| t.python(false))
            )));
        };
        out.push(Attr {
            name: name.into(),
            prim,
        });
    }
    Ok(out)
}

/// pm4py's `_validate_metadata`.
fn validate_metadata(meta: &J) -> Result<Meta> {
    if !matches!(meta, J::Obj(_)) {
        return Err(error("ocel-meta.json must contain a JSON object."));
    }
    if meta.get("ocelVersion").and_then(J::str) != Some("2.0") {
        return Err(error("OCEL bundle ocelVersion must be '2.0'."));
    }
    if meta.get("bundleFormatVersion").and_then(J::str) != Some("1.0") {
        return Err(error("OCEL bundle bundleFormatVersion must be '1.0'."));
    }
    let storage = match meta.get("storageFormat").and_then(J::str) {
        Some("csv") => BundleStorage::Csv,
        Some("parquet") => BundleStorage::Parquet,
        _ => {
            return Err(error(
                "OCEL bundle storageFormat must be 'csv' or 'parquet'.",
            ));
        }
    };
    let (Some(J::Obj(event_types)), Some(J::Obj(object_types))) =
        (meta.get("eventTypes"), meta.get("objectTypes"))
    else {
        return Err(error(
            "OCEL bundle eventTypes and objectTypes must be objects.",
        ));
    };
    let Some(relations @ J::Obj(_)) = meta.get("relations") else {
        return Err(error("OCEL bundle relations must be an object."));
    };
    let extension = storage.extension();
    let mut declared = Vec::new();
    let mut events = Vec::new();
    for (name, descriptor) in event_types {
        if !matches!(descriptor, J::Obj(_)) {
            return Err(error(
                "Event type metadata entries must map strings to objects.",
            ));
        }
        let expected = format!("events/event_{}.{extension}", percent_encode(name));
        let path = relative_path(descriptor.get("file"), "Event table path")?;
        if path != expected {
            return Err(error(format!(
                "Event table for '{name}' must be '{expected}'."
            )));
        }
        let attributes = attribute_descriptors(
            descriptor,
            &format!("event type '{name}'"),
            &["ocel_id", "ocel_time"],
        )?;
        declared.push(path.clone());
        events.push(TypeTables {
            name: name.as_str().into(),
            file: path,
            changes: String::new(),
            attributes,
        });
    }
    let mut objects = Vec::new();
    for (name, descriptor) in object_types {
        if !matches!(descriptor, J::Obj(_)) {
            return Err(error(
                "Object type metadata entries must map strings to objects.",
            ));
        }
        let encoded = percent_encode(name);
        let expected = format!("objects/object_{encoded}.{extension}");
        let changes_expected = format!("object_changes/object_changes_{encoded}.{extension}");
        let path = relative_path(descriptor.get("file"), "Object table path")?;
        let changes = relative_path(descriptor.get("changesFile"), "Object-change table path")?;
        if path != expected || changes != changes_expected {
            return Err(error(format!(
                "Object tables for '{name}' must use the deterministic bundle paths."
            )));
        }
        let attributes = attribute_descriptors(
            descriptor,
            &format!("object type '{name}'"),
            &["ocel_id", "ocel_time", "ocel_changed_field"],
        )?;
        declared.push(path.clone());
        declared.push(changes.clone());
        objects.push(TypeTables {
            name: name.as_str().into(),
            file: path,
            changes,
            attributes,
        });
    }
    let e2o = relative_path(relations.get("e2o"), "E2O table path")?;
    let o2o = relative_path(relations.get("o2o"), "O2O table path")?;
    if e2o != format!("relations/e2o.{extension}") || o2o != format!("relations/o2o.{extension}") {
        return Err(error(
            "Relation tables must use the deterministic bundle paths.",
        ));
    }
    declared.push(e2o.clone());
    declared.push(o2o.clone());
    if declared.iter().collect::<HashSet<_>>().len() != declared.len() {
        return Err(error(
            "OCEL bundle metadata declares the same table path more than once.",
        ));
    }
    Ok(Meta {
        storage,
        events,
        objects,
        e2o,
        o2o,
        declared,
    })
}

/// Where the bundle's files are.
enum Source {
    Directory { root: PathBuf, real: PathBuf },
    Archive(zip::ZipArchive<File>),
}

impl Source {
    fn open(path: &Path) -> Result<Self> {
        if path.is_dir() {
            let real = fs::canonicalize(path)?;
            return Ok(Source::Directory {
                root: path.to_path_buf(),
                real,
            });
        }
        if !lower_name(path).ends_with(".ocel.zip") {
            return Err(error(
                "Bundled OCEL archives use the '.ocel.zip' extension.",
            ));
        }
        Ok(Source::Archive(zip::ZipArchive::new(File::open(path)?)?))
    }

    fn meta(&mut self) -> Result<Vec<u8>> {
        match self {
            Source::Directory { root, .. } => Ok(fs::read(root.join(META))?),
            Source::Archive(archive) => {
                let mut entry = match archive.by_name(META) {
                    Ok(entry) => entry,
                    Err(zip::result::ZipError::FileNotFound) => {
                        return Err(error("OCEL bundle is missing root ocel-meta.json."));
                    }
                    Err(e) => return Err(e.into()),
                };
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                Ok(data)
            }
        }
    }

    /// pm4py's `_container_entries`: the paths of the files, relative to
    /// the root.
    fn entries(&mut self, path: &Path) -> Result<HashSet<String>> {
        let mut entries = HashSet::new();
        match self {
            Source::Directory { root, real } => walk(real, root, root, &mut entries)?,
            Source::Archive(archive) => {
                if central_directory_entries(path)?.is_some_and(|n| n != archive.len() as u64) {
                    return Err(error(
                        "OCEL bundle archives cannot contain duplicate entry names.",
                    ));
                }
                for i in 0..archive.len() {
                    let entry = archive.by_index_raw(i)?;
                    let name = entry.name()?.into_owned();
                    let normalized = name.strip_suffix('/').unwrap_or(&name);
                    if !normalized.is_empty() {
                        relative_path(Some(&J::Str(normalized.to_string())), "Archive entry name")?;
                        if !name.ends_with('/') {
                            entries.insert(name);
                        }
                    }
                }
            }
        }
        Ok(entries)
    }

    /// pm4py's `_table_bytes`.
    fn read(&mut self, table: &str) -> Result<Vec<u8>> {
        match self {
            Source::Directory { root, real } => {
                let full = fs::canonicalize(table.split('/').fold(root.clone(), |p, c| p.join(c)))?;
                if !full.starts_with(&*real) {
                    return Err(error("OCEL bundle table path escapes the container root."));
                }
                Ok(fs::read(full)?)
            }
            Source::Archive(archive) => {
                let mut data = Vec::new();
                archive.by_name(table)?.read_to_end(&mut data)?;
                Ok(data)
            }
        }
    }
}

/// Python's `os.walk` over `dir`, which does not enter linked directories,
/// adding each file's path relative to `base`. A file whose real path is
/// outside `real` is an error.
fn walk(real: &Path, base: &Path, dir: &Path, entries: &mut HashSet<String>) -> Result<()> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in listing {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        let is_dir = if kind.is_symlink() {
            fs::metadata(&path).is_ok_and(|m| m.is_dir())
        } else {
            kind.is_dir()
        };
        if is_dir {
            if !kind.is_symlink() {
                walk(real, base, &path, entries)?;
            }
            continue;
        }
        let target = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !target.starts_with(real) {
            return Err(error("OCEL bundle table path escapes the container root."));
        }
        let relative = path.strip_prefix(base).unwrap_or(&path);
        let parts: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        entries.insert(parts.join("/"));
    }
    Ok(())
}

/// The entry count that the archive's end of central directory record
/// states. The `zip` crate keeps one entry per name, so a smaller count
/// from it means a name repeats. `None` when the record is not found or
/// the count is in a ZIP64 record; such an archive is not checked.
fn central_directory_entries(path: &Path) -> Result<Option<u64>> {
    let mut file = File::open(path)?;
    let size = file.seek(SeekFrom::End(0))?;
    let tail = size.min(22 + 65_535);
    file.seek(SeekFrom::Start(size - tail))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let Some(at) = data.windows(4).rposition(|w| w == [0x50, 0x4b, 0x05, 0x06]) else {
        return Ok(None);
    };
    let Some(count) = data.get(at + 10..at + 12) else {
        return Ok(None);
    };
    let count = u16::from_le_bytes([count[0], count[1]]);
    Ok((count != u16::MAX).then_some(u64::from(count)))
}

/// pm4py's `_validate_container`.
fn validate_container(entries: &HashSet<String>, meta: &Meta) -> Result<()> {
    let mut missing: Vec<&str> = std::iter::once(META)
        .chain(meta.declared.iter().map(String::as_str))
        .filter(|p| !entries.contains(*p))
        .collect();
    if !missing.is_empty() {
        missing.sort();
        missing.dedup();
        return Err(error(format!(
            "OCEL bundle is missing declared tables: {}.",
            missing.join(", ")
        )));
    }
    let opposite = match meta.storage {
        BundleStorage::Csv => ".parquet",
        BundleStorage::Parquet => ".csv",
    };
    if entries.iter().any(|p| p.to_lowercase().ends_with(opposite)) {
        return Err(error("OCEL bundle mixes CSV and Parquet table files."));
    }
    Ok(())
}

/// One table in metadata column order: per column its values, `None` where
/// pandas holds a missing value.
type Columns2 = Vec<Vec<Option<AttributeValue>>>;

/// pm4py's `_TIMEZONE_RE`: the text ends in `Z` or a `±hh:mm` or `±hhmm`
/// offset.
fn has_zone(text: &str) -> bool {
    if text.ends_with('Z') {
        return true;
    }
    let b = text.as_bytes();
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    let sign = |i: usize| b.get(i).is_some_and(|c| matches!(c, b'+' | b'-'));
    let n = b.len();
    (n >= 6
        && sign(n - 6)
        && digit(n - 5)
        && digit(n - 4)
        && b[n - 3] == b':'
        && digit(n - 2)
        && digit(n - 1))
        || (n >= 5 && sign(n - 5) && digit(n - 4) && digit(n - 3) && digit(n - 2) && digit(n - 1))
}

/// pm4py's `_INTEGER_RE`.
fn integer_text(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// pm4py's `_FLOAT_RE`.
fn float_text(text: &str) -> bool {
    let text = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (text, None),
    };
    if let Some(e) = exponent {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e.is_empty() || !e.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (mantissa, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    digits(whole)
        && fraction.is_none_or(digits)
        && (!whole.is_empty() || fraction.is_some_and(|f| !f.is_empty()))
}

/// pm4py's `_parse_csv_value`.
fn csv_value(text: &str, prim: Prim, label: &str) -> Result<Option<AttributeValue>> {
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(match prim {
        Prim::String => AttributeValue::String(text.into()),
        Prim::Integer => {
            let parsed = integer_text(text)
                .then(|| text.parse::<i64>().ok())
                .flatten()
                .ok_or_else(|| error(format!("{label} is not a signed decimal integer.")))?;
            AttributeValue::Int(parsed)
        }
        Prim::Float => {
            if !float_text(text) {
                return Err(error(format!(
                    "{label} is not a decimal floating-point number."
                )));
            }
            let parsed: f64 = text
                .parse()
                .map_err(|_| error(format!("{label} is not a decimal floating-point number.")))?;
            if !parsed.is_finite() {
                return Err(error(format!(
                    "{label} is not a finite floating-point number."
                )));
            }
            AttributeValue::Float(parsed)
        }
        Prim::Boolean => match text {
            "true" => AttributeValue::Bool(true),
            "false" => AttributeValue::Bool(false),
            _ => return Err(error(format!("{label} must be 'true' or 'false'."))),
        },
        Prim::Time => {
            if !has_zone(text) {
                return Err(error(format!(
                    "{label} must be an ISO 8601 timestamp with timezone information."
                )));
            }
            let time = parse_time(text)
                .ok_or_else(|| error(format!("{label} is not a valid ISO 8601 timestamp.")))?;
            AttributeValue::Date(time)
        }
    }))
}

/// A table's fixed columns: name, type, and whether it may hold an empty
/// value.
type Fixed = [(&'static str, Prim, bool)];

/// pm4py's `_read_table` for a CSV table.
fn read_csv_table(
    data: Vec<u8>,
    path: &str,
    fixed: &Fixed,
    attributes: &[Attr],
) -> Result<Columns2> {
    let invalid = || error(format!("Invalid UTF-8/RFC 4180 CSV table '{path}'."));
    let text = String::from_utf8(data).map_err(|_| invalid())?;
    let rows = records(&text).map_err(|_| invalid())?;
    let Some((header, body)) = rows.split_first() else {
        return Err(error(format!(
            "CSV table '{path}' must contain a header row."
        )));
    };
    if header.iter().collect::<HashSet<_>>().len() != header.len() {
        return Err(error(format!("CSV table '{path}' has duplicate columns.")));
    }
    for (i, row) in body.iter().enumerate() {
        if row.len() != header.len() {
            return Err(error(format!(
                "CSV table '{path}' row {} has the wrong number of fields.",
                i + 2
            )));
        }
    }
    let expected: Vec<(&str, Prim, bool)> = fixed
        .iter()
        .map(|&(n, p, _)| (n, p, true))
        .chain(attributes.iter().map(|a| (&*a.name, a.prim, false)))
        .collect();
    let names: HashSet<&str> = expected.iter().map(|e| e.0).collect();
    if header.iter().map(String::as_str).collect::<HashSet<_>>() != names {
        return Err(error(format!(
            "CSV table '{path}' columns do not match its metadata."
        )));
    }
    let mut columns = Vec::new();
    for (name, prim, is_fixed) in expected {
        let index = header.iter().position(|h| h == name).expect("checked");
        let label = format!("{path}.{name}");
        let values = body
            .iter()
            .map(|row| {
                let text = &row[index];
                if is_fixed && prim == Prim::String {
                    Ok(Some(AttributeValue::String(text.as_str().into())))
                } else {
                    csv_value(text, prim, &label)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        columns.push(values);
    }
    Ok(columns)
}

/// pm4py's `_read_table` for a Parquet table. pyarrow's `to_pandas` turns
/// an integer column with a null into floats, and a float NaN is a missing
/// value.
fn read_parquet_table(
    data: Vec<u8>,
    path: &str,
    fixed: &Fixed,
    attributes: &[Attr],
) -> Result<Columns2> {
    let builder = ParquetRecordBatchReaderBuilder::try_new(bytes::Bytes::from(data))?;
    let schema = builder.schema().clone();
    let expected: Vec<(&str, Prim, bool)> = fixed
        .iter()
        .map(|&(n, p, _)| (n, p, true))
        .chain(attributes.iter().map(|a| (&*a.name, a.prim, false)))
        .collect();
    let names: HashSet<&str> = expected.iter().map(|e| e.0).collect();
    let present: HashSet<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    if present != names {
        return Err(error(format!(
            "Parquet table '{path}' columns do not match its metadata."
        )));
    }
    for &(name, prim, is_fixed) in &expected {
        let field = schema.field_with_name(name)?;
        if *field.data_type() != prim.arrow() {
            return Err(error(format!(
                "Parquet column '{path}.{name}' has type {}; expected {}.",
                arrow_name(field.data_type()),
                arrow_name(&prim.arrow())
            )));
        }
        if is_fixed && field.is_nullable() {
            return Err(error(format!(
                "Parquet fixed column '{path}.{name}' must be required."
            )));
        }
        if !is_fixed && !field.is_nullable() {
            return Err(error(format!(
                "Parquet attribute column '{path}.{name}' must be optional."
            )));
        }
    }
    let batches = builder
        .build()?
        .collect::<std::result::Result<Vec<RecordBatch>, _>>()?;
    let mut columns = Vec::new();
    for &(name, prim, _) in &expected {
        let mut values: Vec<Option<AttributeValue>> = Vec::new();
        let mut nulls = false;
        for batch in &batches {
            let array = batch.column_by_name(name).expect("checked");
            nulls |= array.null_count() > 0;
            for i in 0..array.len() {
                values.push(if array.is_null(i) {
                    None
                } else {
                    Some(arrow_value(array, i, prim))
                });
            }
        }
        if prim == Prim::Integer && nulls {
            for v in values.iter_mut().flatten() {
                if let AttributeValue::Int(i) = v {
                    *v = AttributeValue::Float(*i as f64);
                }
            }
        }
        values.retain_mut(|_| true);
        for v in &mut values {
            if v.as_ref().is_some_and(missing) {
                *v = None;
            }
        }
        columns.push(values);
    }
    Ok(columns)
}

/// pyarrow's name for an Arrow type.
fn arrow_name(t: &DataType) -> String {
    let unit = |u: &TimeUnit| match u {
        TimeUnit::Second => "s",
        TimeUnit::Millisecond => "ms",
        TimeUnit::Microsecond => "us",
        TimeUnit::Nanosecond => "ns",
    };
    match t {
        DataType::Null => "null".into(),
        DataType::Boolean => "bool".into(),
        DataType::Int8 => "int8".into(),
        DataType::Int16 => "int16".into(),
        DataType::Int32 => "int32".into(),
        DataType::Int64 => "int64".into(),
        DataType::UInt8 => "uint8".into(),
        DataType::UInt16 => "uint16".into(),
        DataType::UInt32 => "uint32".into(),
        DataType::UInt64 => "uint64".into(),
        DataType::Float16 => "halffloat".into(),
        DataType::Float32 => "float".into(),
        DataType::Float64 => "double".into(),
        DataType::Utf8 => "string".into(),
        DataType::LargeUtf8 => "large_string".into(),
        DataType::Utf8View => "string_view".into(),
        DataType::Binary => "binary".into(),
        DataType::LargeBinary => "large_binary".into(),
        DataType::Date32 => "date32[day]".into(),
        DataType::Date64 => "date64[ms]".into(),
        DataType::Timestamp(u, None) => format!("timestamp[{}]", unit(u)),
        DataType::Timestamp(u, Some(tz)) => format!("timestamp[{}, tz={tz}]", unit(u)),
        other => other.to_string().to_lowercase(),
    }
}

fn arrow_value(array: &ArrayRef, i: usize, prim: Prim) -> AttributeValue {
    let any = array.as_any();
    match prim {
        Prim::String => AttributeValue::String(
            any.downcast_ref::<StringArray>()
                .expect("utf8")
                .value(i)
                .into(),
        ),
        Prim::Integer => {
            AttributeValue::Int(any.downcast_ref::<Int64Array>().expect("int64").value(i))
        }
        Prim::Float => AttributeValue::Float(
            any.downcast_ref::<Float64Array>()
                .expect("float64")
                .value(i),
        ),
        Prim::Boolean => {
            AttributeValue::Bool(any.downcast_ref::<BooleanArray>().expect("bool").value(i))
        }
        Prim::Time => {
            let micros = any
                .downcast_ref::<TimestampMicrosecondArray>()
                .expect("timestamp")
                .value(i);
            AttributeValue::Date(utc(Utc
                .timestamp_micros(micros)
                .single()
                .expect("in range")))
        }
    }
}

fn utc(d: DateTime<Utc>) -> DateTime<FixedOffset> {
    d.fixed_offset()
}

/// pm4py's `_read_table`, with its check that fixed columns hold no missing
/// or empty value.
fn read_table(
    source: &mut Source,
    storage: BundleStorage,
    path: &str,
    fixed: &Fixed,
    attributes: &[Attr],
) -> Result<Columns2> {
    let data = source.read(path)?;
    let columns = match storage {
        BundleStorage::Csv => read_csv_table(data, path, fixed, attributes)?,
        BundleStorage::Parquet => read_parquet_table(data, path, fixed, attributes)?,
    };
    for (i, &(name, _, empty_allowed)) in fixed.iter().enumerate() {
        if empty_allowed {
            continue;
        }
        let bad = columns[i].iter().any(|v| match v {
            None => true,
            Some(AttributeValue::String(s)) => s.is_empty(),
            Some(_) => false,
        });
        if bad {
            return Err(error(format!(
                "Fixed column '{path}.{name}' cannot contain missing values."
            )));
        }
    }
    Ok(columns)
}

fn text_at(columns: &Columns2, column: usize, row: usize) -> Arc<str> {
    match &columns[column][row] {
        Some(AttributeValue::String(s)) => s.clone(),
        _ => unreachable!("fixed string column"),
    }
}

fn time_at(columns: &Columns2, column: usize, row: usize) -> DateTime<FixedOffset> {
    match &columns[column][row] {
        Some(AttributeValue::Date(d)) => *d,
        _ => unreachable!("fixed time column"),
    }
}

fn row_count(columns: &Columns2) -> usize {
    columns.first().map_or(0, Vec::len)
}

const EVENT_FIXED: &Fixed = &[
    ("ocel_id", Prim::String, false),
    ("ocel_time", Prim::Time, false),
];
const OBJECT_FIXED: &Fixed = &[("ocel_id", Prim::String, false)];
const CHANGE_FIXED: &Fixed = &[
    ("ocel_id", Prim::String, false),
    ("ocel_time", Prim::Time, false),
    ("ocel_changed_field", Prim::String, false),
];
const E2O_FIXED: &Fixed = &[
    ("ocel_event_id", Prim::String, false),
    ("ocel_object_id", Prim::String, false),
    ("ocel_qualifier", Prim::String, true),
];
const O2O_FIXED: &Fixed = &[
    ("ocel_source_id", Prim::String, false),
    ("ocel_target_id", Prim::String, false),
    ("ocel_qualifier", Prim::String, true),
];

/// Reads an OCEL 2.0 bundle (pm4py's `read_ocel2_bundle`): a directory, or
/// a ZIP archive whose name ends in `.ocel.zip` in any case.
///
/// The reader makes pm4py's checks, and fails where pm4py does:
///
/// - `ocel-meta.json` declares OCEL version `2.0`, bundle format `1.0`,
///   CSV or Parquet storage, and each table at the path that follows from
///   its type's name. Attribute names are unique, are not fixed column
///   names, and have the type `string`, `time`, `integer`, `float` or
///   `boolean`.
/// - Every declared table is there, and no file has the other storage's
///   extension. Archive entry names are unique and relative, without `..`.
///   A file of a directory may not resolve to a path outside it.
/// - A CSV table is UTF-8 text as Python's `csv` module reads it in strict
///   mode, with unique column names. A Parquet table has the declared
///   Arrow types: `string`, `int64`, `float64`, `bool` and microsecond
///   UTC timestamps. Its fixed columns are required and its attribute
///   columns optional.
/// - Each table has exactly its fixed and declared columns, in any order.
///   Ids and times are never missing or empty. A CSV time names its offset.
/// - Event ids and object ids are unique. An object change sets exactly
///   its declared field, of an object of its type, not at the Unix epoch.
///   Changes and relations do not repeat and refer to known events and
///   objects.
///
/// The events, relations and changes are then sorted by time, keeping
/// table order for ties, and [`Ocel::make_consistent`] runs. Events and
/// objects without relations stay. Every relation has a qualifier, empty
/// when the table holds none.
pub fn read_ocel2_bundle(path: impl AsRef<Path>) -> Result<Ocel> {
    let path = path.as_ref();
    let mut source = Source::open(path)?;
    let meta: J = serde_json::from_slice(&source.meta()?)?;
    let meta = validate_metadata(&meta)?;
    let entries = source.entries(path)?;
    validate_container(&entries, &meta)?;
    let storage = meta.storage;

    let mut ocel = Ocel::new();
    let mut event_time: HashMap<Arc<str>, DateTime<FixedOffset>> = HashMap::new();
    for t in &meta.events {
        let table = read_table(&mut source, storage, &t.file, EVENT_FIXED, &t.attributes)?;
        for row in 0..row_count(&table) {
            let id = text_at(&table, 0, row);
            let timestamp = time_at(&table, 1, row);
            if event_time.insert(id.clone(), timestamp).is_some() {
                return Err(error(format!(
                    "Event id '{id}' occurs more than once in the bundle."
                )));
            }
            ocel.events.push(OcelEvent {
                id,
                activity: t.name.clone(),
                timestamp,
                attributes: row_attributes(&table, EVENT_FIXED.len(), &t.attributes, row),
            });
        }
    }

    let mut object_type: HashMap<Arc<str>, Arc<str>> = HashMap::new();
    for t in &meta.objects {
        let table = read_table(&mut source, storage, &t.file, OBJECT_FIXED, &t.attributes)?;
        for row in 0..row_count(&table) {
            let id = text_at(&table, 0, row);
            if object_type.insert(id.clone(), t.name.clone()).is_some() {
                return Err(error(format!(
                    "Object id '{id}' occurs more than once in the bundle."
                )));
            }
            ocel.objects.push(OcelObject {
                id,
                object_type: t.name.clone(),
                attributes: row_attributes(&table, OBJECT_FIXED.len(), &t.attributes, row),
            });
        }
        let table = read_table(
            &mut source,
            storage,
            &t.changes,
            CHANGE_FIXED,
            &t.attributes,
        )?;
        for row in 0..row_count(&table) {
            let object = text_at(&table, 0, row);
            let timestamp = time_at(&table, 1, row);
            let field = text_at(&table, 2, row);
            let Some(index) = t.attributes.iter().position(|a| a.name == field) else {
                return Err(error(format!(
                    "Object-change field '{field}' is not declared."
                )));
            };
            if timestamp.timestamp_nanos_opt() == Some(0) {
                return Err(error(
                    "Object-change tables cannot contain assignments at the UNIX epoch; \
                     initial values belong in the object table.",
                ));
            }
            let value_at = |i: usize| table[CHANGE_FIXED.len() + i][row].clone();
            let Some(value) = value_at(index) else {
                return Err(error(format!(
                    "Object-change row for '{object}' has no changed value."
                )));
            };
            if (0..t.attributes.len()).any(|i| i != index && value_at(i).is_some()) {
                return Err(error(
                    "An object-change row must assign exactly one attribute.",
                ));
            }
            ocel.object_changes.push(ObjectChange {
                object,
                object_type: t.name.clone(),
                timestamp,
                field,
                value: Some(value),
            });
        }
    }
    let mut keys = HashSet::new();
    for c in &ocel.object_changes {
        if !keys.insert((&c.object, c.timestamp, &c.field)) {
            return Err(error(
                "Object-change tables contains duplicate rows for its OCEL set key.",
            ));
        }
    }
    for c in &ocel.object_changes {
        if object_type.get(&c.object) != Some(&c.object_type) {
            return Err(error(format!(
                "Object change references an unknown or differently typed object '{}'.",
                c.object
            )));
        }
    }

    let table = read_table(&mut source, storage, &meta.e2o, E2O_FIXED, &[])?;
    let mut keys = HashSet::new();
    for row in 0..row_count(&table) {
        let key = (
            text_at(&table, 0, row),
            text_at(&table, 1, row),
            text_at(&table, 2, row),
        );
        if !keys.insert(key.clone()) {
            return Err(error(
                "E2O table contains duplicate rows for its OCEL set key.",
            ));
        }
        ocel.relations.push(EventObject {
            event: key.0,
            object: key.1,
            qualifier: Some(key.2),
        });
    }
    if ocel
        .relations
        .iter()
        .any(|r| !event_time.contains_key(&r.event))
    {
        return Err(error("E2O table references an unknown event."));
    }
    if ocel
        .relations
        .iter()
        .any(|r| !object_type.contains_key(&r.object))
    {
        return Err(error("E2O table references an unknown object."));
    }

    let table = read_table(&mut source, storage, &meta.o2o, O2O_FIXED, &[])?;
    let mut keys = HashSet::new();
    for row in 0..row_count(&table) {
        let key = (
            text_at(&table, 0, row),
            text_at(&table, 1, row),
            text_at(&table, 2, row),
        );
        if !keys.insert(key.clone()) {
            return Err(error(
                "O2O table contains duplicate rows for its OCEL set key.",
            ));
        }
        ocel.o2o.push(ObjectObject {
            source: key.0,
            target: key.1,
            qualifier: Some(key.2),
        });
    }
    if ocel
        .o2o
        .iter()
        .any(|r| !object_type.contains_key(&r.source))
    {
        return Err(error("O2O table references an unknown source object."));
    }
    if ocel
        .o2o
        .iter()
        .any(|r| !object_type.contains_key(&r.target))
    {
        return Err(error("O2O table references an unknown target object."));
    }

    ocel.events.sort_by_key(|e| e.timestamp);
    ocel.relations.sort_by_key(|r| event_time[&r.event]);
    ocel.object_changes.sort_by_key(|c| c.timestamp);
    ocel.make_consistent();
    Ok(ocel)
}

/// The non-missing attribute values of one row, in metadata order.
fn row_attributes(table: &Columns2, skip: usize, attributes: &[Attr], row: usize) -> Attributes {
    let mut out = Attributes::default();
    for (i, a) in attributes.iter().enumerate() {
        if let Some(v) = &table[skip + i][row] {
            out.insert(a.name.clone(), v.clone());
        }
    }
    out
}

/// A value as Python holds it in pm4py's data frame, as far as the bundle
/// writer looks at it.
#[derive(Debug, Clone, PartialEq)]
enum Py {
    Str(Arc<str>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Time(DateTime<FixedOffset>),
    /// A list or container, which the writer cannot write.
    Other,
}

impl Py {
    /// The value in a column of this kind: a float column holds integers as
    /// floats. A missing value is `None`.
    fn of(value: &AttributeValue, kind: Kind) -> Option<Py> {
        if missing(value) {
            return None;
        }
        Some(match (kind, value.plain()) {
            (Kind::Float, AttributeValue::Int(i)) => Py::Float(*i as f64),
            (_, AttributeValue::String(s) | AttributeValue::Id(s)) => Py::Str(s.clone()),
            (_, AttributeValue::Int(i)) => Py::Int(*i),
            (_, AttributeValue::Float(f)) => Py::Float(*f),
            (_, AttributeValue::Bool(b)) => Py::Bool(*b),
            (_, AttributeValue::Date(d)) => Py::Time(*d),
            _ => Py::Other,
        })
    }

    /// pm4py's `normalize_value`: a time becomes its `isoformat` text.
    fn normalized(&self) -> Py {
        match self {
            Py::Time(d) => Py::Str(isoformat(d).into()),
            other => other.clone(),
        }
    }

    fn text(&self) -> String {
        match self {
            Py::Str(s) => s.to_string(),
            Py::Int(i) => i.to_string(),
            Py::Float(f) => AttributeValue::Float(*f).to_string(),
            Py::Bool(true) => "True".into(),
            Py::Bool(false) => "False".into(),
            Py::Time(d) => isoformat(d),
            Py::Other => String::new(),
        }
    }

    fn number(&self) -> Option<String> {
        match self {
            Py::Int(_) | Py::Float(_) => Some(self.text()),
            _ => None,
        }
    }
}

/// pm4py's `_values_equal`: numbers by decimal value, others by type and
/// value.
fn values_equal(a: &Py, b: &Py) -> bool {
    match (a.number(), b.number()) {
        (Some(x), Some(y)) => decimal(&x) == decimal(&y),
        _ => a == b,
    }
}

/// pm4py's `_primitive_type_from_values`.
fn primitive_type<'a>(values: impl Iterator<Item = &'a Py> + Clone) -> Prim {
    let mut values = values.peekable();
    if values.peek().is_none() {
        return Prim::String;
    }
    let all = |f: fn(&Py) -> bool| values.clone().all(f);
    if all(|v| matches!(v, Py::Bool(_))) {
        Prim::Boolean
    } else if all(|v| matches!(v, Py::Int(_))) {
        Prim::Integer
    } else if all(|v| match v {
        Py::Int(_) => true,
        Py::Float(f) => f.is_finite(),
        _ => false,
    }) {
        Prim::Float
    } else if all(|v| matches!(v, Py::Time(_))) {
        Prim::Time
    } else {
        Prim::String
    }
}

/// A coerced cell (pm4py's `_coerce_value`).
#[derive(Debug, Clone)]
enum Cell {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Time(DateTime<Utc>),
}

fn coerce(value: Option<&Py>, prim: Prim, label: &str, optional: bool) -> Result<Option<Cell>> {
    let Some(value) = value else {
        if optional {
            return Ok(None);
        }
        return Err(error(format!("{label} cannot be missing.")));
    };
    Ok(Some(match (prim, value.normalized()) {
        (_, Py::Other) if prim == Prim::String => {
            return Err(error(format!("{label} must be a scalar string value.")));
        }
        (Prim::String, v) => Cell::Str(v.text()),
        (Prim::Integer, Py::Int(i)) => Cell::Int(i),
        (Prim::Integer, _) => return Err(error(format!("{label} must be an integer."))),
        (Prim::Float, Py::Int(i)) => Cell::Float(i as f64),
        (Prim::Float, Py::Float(f)) if f.is_finite() => Cell::Float(f),
        (Prim::Float, Py::Float(_)) => return Err(error(format!("{label} must be finite."))),
        (Prim::Float, _) => {
            return Err(error(format!("{label} must be a floating-point number.")));
        }
        (Prim::Boolean, Py::Bool(b)) => Cell::Bool(b),
        (Prim::Boolean, _) => return Err(error(format!("{label} must be boolean."))),
        (Prim::Time, _) => match value {
            Py::Time(d) => Cell::Time(d.with_timezone(&Utc)),
            _ => return Err(error(format!("{label} is not a timestamp."))),
        },
    }))
}

/// One table to write: its fixed columns, its attribute columns and its
/// rows of values in that column order.
struct TableSpec {
    fixed: &'static Fixed,
    attributes: Vec<(Arc<str>, Prim)>,
    rows: Vec<Vec<Option<Py>>>,
}

impl TableSpec {
    /// pm4py's `_prepare_table`: each value coerced to its column's type.
    fn cells(&self) -> Result<Vec<Vec<Option<Cell>>>> {
        let columns: Vec<(&str, Prim, bool, bool)> = self
            .fixed
            .iter()
            .map(|&(n, p, empty)| (n, p, false, empty))
            .chain(self.attributes.iter().map(|(n, p)| (&**n, *p, true, true)))
            .collect();
        let mut cells: Vec<Vec<Option<Cell>>> = vec![Vec::new(); self.rows.len()];
        for (c, &(name, prim, optional, empty_allowed)) in columns.iter().enumerate() {
            for (r, row) in self.rows.iter().enumerate() {
                let cell = coerce(row[c].as_ref(), prim, name, optional)?;
                if !optional
                    && !empty_allowed
                    && matches!(&cell, Some(Cell::Str(s)) if s.is_empty())
                {
                    return Err(error(format!(
                        "Fixed column '{name}' cannot contain empty strings."
                    )));
                }
                cells[r].push(cell);
            }
        }
        Ok(cells)
    }

    fn names(&self) -> Vec<&str> {
        self.fixed
            .iter()
            .map(|f| f.0)
            .chain(self.attributes.iter().map(|(n, _)| &**n))
            .collect()
    }

    fn prims(&self) -> Vec<Prim> {
        self.fixed
            .iter()
            .map(|f| f.1)
            .chain(self.attributes.iter().map(|(_, p)| *p))
            .collect()
    }

    fn csv(&self) -> Result<Vec<u8>> {
        let cells = self.cells()?;
        let mut out = Vec::new();
        let header: Vec<String> = self.names().into_iter().map(String::from).collect();
        write_record(&mut out, &header, "\r\n")?;
        for row in cells {
            let fields: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    None => String::new(),
                    Some(Cell::Str(s)) => s.clone(),
                    Some(Cell::Int(i)) => i.to_string(),
                    Some(Cell::Float(f)) => AttributeValue::Float(*f).to_string(),
                    Some(Cell::Bool(b)) => b.to_string(),
                    Some(Cell::Time(d)) => isoformat(&d.fixed_offset()),
                })
                .collect();
            write_record(&mut out, &fields, "\r\n")?;
        }
        Ok(out)
    }

    fn parquet(&self) -> Result<Vec<u8>> {
        let cells = self.cells()?;
        let names = self.names();
        let prims = self.prims();
        let fields: Vec<Field> = names
            .iter()
            .zip(&prims)
            .enumerate()
            .map(|(i, (n, p))| Field::new(*n, p.arrow(), i >= self.fixed.len()))
            .collect();
        let schema = Arc::new(Schema::new(fields));
        let column = |c: usize| cells.iter().map(move |row| row[c].as_ref());
        let arrays: Vec<ArrayRef> = prims
            .iter()
            .enumerate()
            .map(|(c, prim)| -> ArrayRef {
                match prim {
                    Prim::String => Arc::new(StringArray::from_iter(column(c).map(|v| match v {
                        Some(Cell::Str(s)) => Some(s.clone()),
                        _ => None,
                    }))),
                    Prim::Integer => Arc::new(Int64Array::from_iter(column(c).map(|v| match v {
                        Some(Cell::Int(i)) => Some(*i),
                        _ => None,
                    }))),
                    Prim::Float => Arc::new(Float64Array::from_iter(column(c).map(|v| match v {
                        Some(Cell::Float(f)) => Some(*f),
                        _ => None,
                    }))),
                    Prim::Boolean => {
                        Arc::new(BooleanArray::from_iter(column(c).map(|v| match v {
                            Some(Cell::Bool(b)) => Some(*b),
                            _ => None,
                        })))
                    }
                    Prim::Time => Arc::new(
                        TimestampMicrosecondArray::from_iter(column(c).map(|v| match v {
                            Some(Cell::Time(d)) => Some(d.timestamp_micros()),
                            _ => None,
                        }))
                        .with_timezone("UTC"),
                    ),
                }
            })
            .collect();
        let batch = RecordBatch::try_new(schema.clone(), arrays)?;
        let properties = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();
        let mut out = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut out, schema, Some(properties))?;
        writer.write(&batch)?;
        writer.close()?;
        Ok(out)
    }
}

/// `json.dumps(indent=2, ensure_ascii=False)` of the metadata.
#[derive(Debug, Clone)]
enum MetaJson {
    Str(String),
    Arr(Vec<MetaJson>),
    Obj(Vec<(String, MetaJson)>),
}

impl MetaJson {
    fn write(&self, level: usize, out: &mut String) {
        let indent = |level: usize, out: &mut String| {
            out.push('\n');
            out.push_str(&"  ".repeat(level));
        };
        match self {
            MetaJson::Str(s) => json_string(s, out),
            MetaJson::Arr(items) if items.is_empty() => out.push_str("[]"),
            MetaJson::Obj(items) if items.is_empty() => out.push_str("{}"),
            MetaJson::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(level + 1, out);
                    item.write(level + 1, out);
                }
                indent(level, out);
                out.push(']');
            }
            MetaJson::Obj(items) => {
                out.push('{');
                for (i, (key, item)) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(level + 1, out);
                    json_string(key, out);
                    out.push_str(": ");
                    item.write(level + 1, out);
                }
                indent(level, out);
                out.push('}');
            }
        }
    }
}

/// A JSON string as Python writes it with `ensure_ascii=False`.
fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn attributes_json(attributes: &[(Arc<str>, Prim)]) -> MetaJson {
    MetaJson::Arr(
        attributes
            .iter()
            .map(|(n, p)| {
                MetaJson::Obj(vec![
                    ("name".into(), MetaJson::Str(n.to_string())),
                    ("type".into(), MetaJson::Str(p.name().into())),
                ])
            })
            .collect(),
    )
}

/// pm4py's `_normalized_string` for an id, type or field name.
fn non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        return Err(error(format!("{label} cannot be empty.")));
    }
    Ok(())
}

/// pm4py's `_attribute_columns`: the attribute names, without names that
/// start with `ocel:`.
fn attribute_columns(columns: &Columns) -> Result<Vec<Arc<str>>> {
    let mut out = Vec::new();
    for name in &columns.names {
        if name.starts_with("ocel:") {
            continue;
        }
        if name.is_empty() {
            return Err(error(
                "OCEL bundle attribute names must be non-empty strings.",
            ));
        }
        out.push(name.clone());
    }
    Ok(out)
}

/// The files of a bundle, in the order pm4py writes them: the metadata,
/// the event tables by type name, each object type's object and change
/// tables by type name, then the relation tables.
fn bundle_files(ocel: &Ocel, storage: BundleStorage) -> Result<Vec<(String, Vec<u8>)>> {
    for e in &ocel.events {
        non_empty(&e.id, "event id")?;
    }
    for o in &ocel.objects {
        non_empty(&o.id, "object id")?;
    }
    let mut seen = HashSet::new();
    if !ocel.events.iter().all(|e| seen.insert(&*e.id)) {
        return Err(error("OCEL bundle event identifiers must be unique."));
    }
    let mut seen = HashSet::new();
    if !ocel.objects.iter().all(|o| seen.insert(&*o.id)) {
        return Err(error("OCEL bundle object identifiers must be unique."));
    }
    for e in &ocel.events {
        non_empty(&e.activity, "event type")?;
    }
    for o in &ocel.objects {
        non_empty(&o.object_type, "object type")?;
    }
    let object_index: HashMap<&str, usize> = ocel
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| (&*o.id, i))
        .collect();

    let events = Columns::new(ocel.events.iter().map(|e| &e.attributes));
    let objects = Columns::new(ocel.objects.iter().map(|o| &o.attributes));
    let change_rows: Vec<Attributes> = ocel
        .object_changes
        .iter()
        .map(|c| {
            let mut row = Attributes::default();
            if let Some(v) = &c.value {
                row.insert(c.field.clone(), v.clone());
            }
            row
        })
        .collect();
    let changes = Columns::new(change_rows.iter());
    let event_columns = attribute_columns(&events)?;
    let mut object_columns = attribute_columns(&objects)?;
    let change_columns = attribute_columns(&changes)?;
    if event_columns
        .iter()
        .any(|c| matches!(&**c, "ocel_id" | "ocel_time"))
    {
        return Err(error(
            "Event attribute name collides with a fixed bundle column.",
        ));
    }
    if object_columns
        .iter()
        .chain(&change_columns)
        .any(|c| matches!(&**c, "ocel_id" | "ocel_time" | "ocel_changed_field"))
    {
        return Err(error(
            "Object attribute name collides with a fixed bundle column.",
        ));
    }

    // The objects table's values, with the Unix-epoch changes folded in.
    let mut object_values: Vec<HashMap<Arc<str>, Py>> = ocel
        .objects
        .iter()
        .map(|o| {
            o.attributes
                .iter()
                .filter_map(|(k, v)| Py::of(v, objects.kind(k)).map(|v| (k.clone(), v)))
                .collect()
        })
        .collect();
    let mut kept: Vec<(usize, DateTime<FixedOffset>)> = Vec::new();
    let mut change_keys = HashSet::new();
    for (i, c) in ocel.object_changes.iter().enumerate() {
        non_empty(&c.object, "object-change id")?;
        let Some(&o) = object_index.get(&*c.object) else {
            return Err(error(format!(
                "Object change references unknown object '{}'.",
                c.object
            )));
        };
        if c.object_type != ocel.objects[o].object_type {
            return Err(error(format!(
                "Object change type does not match object '{}'.",
                c.object
            )));
        }
        non_empty(&c.field, "changed field")?;
        let Some(value) = c
            .value
            .as_ref()
            .and_then(|v| Py::of(v, changes.kind(&c.field)))
        else {
            return Err(error(format!(
                "Object change for '{}' has no value for '{}'.",
                c.object, c.field
            )));
        };
        let time = c.timestamp.with_timezone(&Utc);
        let value = value.normalized();
        if c.timestamp.timestamp_nanos_opt() == Some(0) {
            // pm4py keeps a value at the epoch in the objects table only.
            let new_column = !objects.names.contains(&c.field);
            if new_column && !object_columns.contains(&c.field) {
                object_columns.push(c.field.clone());
            }
            let value = match (new_column, objects.kind(&c.field), &value) {
                (false, Kind::Float, Py::Int(i)) => Py::Float(*i as f64),
                _ => value,
            };
            if let Some(existing) = object_values[o].get(&c.field)
                && !values_equal(&existing.normalized(), &value)
            {
                return Err(error(format!(
                    "Object '{}' has conflicting time-0 values for attribute '{}'.",
                    c.object, c.field
                )));
            }
            object_values[o].insert(c.field.clone(), value);
            continue;
        }
        if !change_keys.insert((&*c.object, time, &*c.field)) {
            return Err(error(
                "Duplicate object attribute assignment cannot be exported.",
            ));
        }
        kept.push((i, c.timestamp));
    }
    // pandas rebuilds the changes table from the kept rows and infers its
    // column types again.
    let kept_rows: Vec<Attributes> = kept
        .iter()
        .map(|&(i, _)| {
            let c = &ocel.object_changes[i];
            let mut row = Attributes::default();
            if let Some(v) = &c.value {
                let v = match (changes.kind(&c.field), v.plain()) {
                    (Kind::Float, AttributeValue::Int(i)) => AttributeValue::Float(*i as f64),
                    _ => v.clone(),
                };
                row.insert(c.field.clone(), v);
            }
            row
        })
        .collect();
    let kept_kinds = Columns::new(kept_rows.iter());
    let kept_value = |k: usize| -> Py {
        let c = &ocel.object_changes[kept[k].0];
        let v = kept_rows[k].get(&c.field).expect("kept");
        Py::of(v, kept_kinds.kind(&c.field)).expect("present")
    };

    let ext = storage.extension();
    let mut specs: Vec<(String, TableSpec)> = Vec::new();
    let mut event_meta = Vec::new();
    let mut types: Vec<&Arc<str>> = ocel.events.iter().map(|e| &e.activity).collect();
    types.sort();
    types.dedup();
    for name in types {
        let selected: Vec<_> = ocel.events.iter().filter(|e| e.activity == *name).collect();
        let values = |e: &OcelEvent, column: &str| {
            e.attributes
                .get(column)
                .and_then(|v| Py::of(v, events.kind(column)))
        };
        let attributes: Vec<(Arc<str>, Prim)> = event_columns
            .iter()
            .filter_map(|column| {
                let present: Vec<Py> = selected.iter().filter_map(|e| values(e, column)).collect();
                (!present.is_empty()).then(|| (column.clone(), primitive_type(present.iter())))
            })
            .collect();
        let rows = selected
            .iter()
            .map(|e| {
                let mut row = vec![Some(Py::Str(e.id.clone())), Some(Py::Time(e.timestamp))];
                row.extend(attributes.iter().map(|(column, _)| values(e, column)));
                row
            })
            .collect();
        let path = format!("events/event_{}.{ext}", percent_encode(name));
        event_meta.push((
            name.to_string(),
            MetaJson::Obj(vec![
                ("file".into(), MetaJson::Str(path.clone())),
                ("attributes".into(), attributes_json(&attributes)),
            ]),
        ));
        specs.push((
            path,
            TableSpec {
                fixed: EVENT_FIXED,
                attributes,
                rows,
            },
        ));
    }

    let mut object_meta = Vec::new();
    let mut types: Vec<&Arc<str>> = ocel.objects.iter().map(|o| &o.object_type).collect();
    types.sort();
    types.dedup();
    for name in types {
        let selected: Vec<usize> = (0..ocel.objects.len())
            .filter(|&i| ocel.objects[i].object_type == *name)
            .collect();
        let type_changes: Vec<usize> = (0..kept.len())
            .filter(|&k| ocel.object_changes[kept[k].0].object_type == *name)
            .collect();
        let mut columns: Vec<Arc<str>> = object_columns
            .iter()
            .filter(|c| selected.iter().any(|&i| object_values[i].contains_key(*c)))
            .cloned()
            .collect();
        for &k in &type_changes {
            let field = &ocel.object_changes[kept[k].0].field;
            if !columns.contains(field) {
                columns.push(field.clone());
            }
        }
        let attributes: Vec<(Arc<str>, Prim)> = columns
            .iter()
            .map(|column| {
                let mut values: Vec<Py> = selected
                    .iter()
                    .filter_map(|&i| object_values[i].get(column).cloned())
                    .collect();
                values.extend(
                    type_changes
                        .iter()
                        .filter(|&&k| ocel.object_changes[kept[k].0].field == *column)
                        .map(|&k| kept_value(k)),
                );
                (column.clone(), primitive_type(values.iter()))
            })
            .collect();
        let encoded = percent_encode(name);
        let path = format!("objects/object_{encoded}.{ext}");
        let changes_path = format!("object_changes/object_changes_{encoded}.{ext}");
        let rows = selected
            .iter()
            .map(|&i| {
                let mut row = vec![Some(Py::Str(ocel.objects[i].id.clone()))];
                row.extend(columns.iter().map(|c| object_values[i].get(c).cloned()));
                row
            })
            .collect();
        let change_rows = type_changes
            .iter()
            .map(|&k| {
                let c = &ocel.object_changes[kept[k].0];
                let mut row = vec![
                    Some(Py::Str(c.object.clone())),
                    Some(Py::Time(c.timestamp)),
                    Some(Py::Str(c.field.clone())),
                ];
                row.extend(
                    columns
                        .iter()
                        .map(|column| (*column == c.field).then(|| kept_value(k))),
                );
                row
            })
            .collect();
        object_meta.push((
            name.to_string(),
            MetaJson::Obj(vec![
                ("file".into(), MetaJson::Str(path.clone())),
                ("changesFile".into(), MetaJson::Str(changes_path.clone())),
                ("attributes".into(), attributes_json(&attributes)),
            ]),
        ));
        specs.push((
            path,
            TableSpec {
                fixed: OBJECT_FIXED,
                attributes: attributes.clone(),
                rows,
            },
        ));
        specs.push((
            changes_path,
            TableSpec {
                fixed: CHANGE_FIXED,
                attributes,
                rows: change_rows,
            },
        ));
    }

    let event_ids: HashSet<&str> = ocel.events.iter().map(|e| &*e.id).collect();
    let mut keys = HashSet::new();
    let mut rows = Vec::new();
    for r in &ocel.relations {
        non_empty(&r.event, "E2O event id")?;
        non_empty(&r.object, "E2O object id")?;
        let qualifier: Arc<str> = r.qualifier.clone().unwrap_or_else(|| "".into());
        if !event_ids.contains(&*r.event) || !object_index.contains_key(&*r.object) {
            return Err(error("E2O relation references an unknown event or object."));
        }
        if !keys.insert((&r.event, &r.object, qualifier.clone())) {
            return Err(error("Duplicate E2O relation cannot be exported."));
        }
        rows.push(vec![
            Some(Py::Str(r.event.clone())),
            Some(Py::Str(r.object.clone())),
            Some(Py::Str(qualifier)),
        ]);
    }
    let e2o_path = format!("relations/e2o.{ext}");
    specs.push((
        e2o_path.clone(),
        TableSpec {
            fixed: E2O_FIXED,
            attributes: Vec::new(),
            rows,
        },
    ));
    let mut keys = HashSet::new();
    let mut rows = Vec::new();
    for r in &ocel.o2o {
        non_empty(&r.source, "O2O source id")?;
        non_empty(&r.target, "O2O target id")?;
        let qualifier: Arc<str> = r.qualifier.clone().unwrap_or_else(|| "".into());
        if !object_index.contains_key(&*r.source) || !object_index.contains_key(&*r.target) {
            return Err(error("O2O relation references an unknown object."));
        }
        if !keys.insert((&r.source, &r.target, qualifier.clone())) {
            return Err(error("Duplicate O2O relation cannot be exported."));
        }
        rows.push(vec![
            Some(Py::Str(r.source.clone())),
            Some(Py::Str(r.target.clone())),
            Some(Py::Str(qualifier)),
        ]);
    }
    let o2o_path = format!("relations/o2o.{ext}");
    specs.push((
        o2o_path.clone(),
        TableSpec {
            fixed: O2O_FIXED,
            attributes: Vec::new(),
            rows,
        },
    ));

    let meta = MetaJson::Obj(vec![
        ("ocelVersion".into(), MetaJson::Str("2.0".into())),
        ("bundleFormatVersion".into(), MetaJson::Str("1.0".into())),
        ("storageFormat".into(), MetaJson::Str(ext.into())),
        ("eventTypes".into(), MetaJson::Obj(event_meta)),
        ("objectTypes".into(), MetaJson::Obj(object_meta)),
        (
            "relations".into(),
            MetaJson::Obj(vec![
                ("e2o".into(), MetaJson::Str(e2o_path)),
                ("o2o".into(), MetaJson::Str(o2o_path)),
            ]),
        ),
    ]);
    let mut text = String::new();
    meta.write(0, &mut text);
    let mut files = vec![(META.to_string(), text.into_bytes())];
    for (path, spec) in specs {
        let data = match storage {
            BundleStorage::Csv => spec.csv()?,
            BundleStorage::Parquet => spec.parquet()?,
        };
        files.push((path, data));
    }
    Ok(files)
}

/// Writes an OCEL 2.0 bundle (pm4py's `write_ocel2_bundle`). A path whose
/// name ends in `.ocel.zip`, in any case, becomes a ZIP archive; any other
/// name ending in `.zip` is an error; any other path becomes a directory,
/// from whose `events`, `objects`, `object_changes` and `relations`
/// folders the old `.csv` and `.parquet` files are first removed.
///
/// pm4py does not run its consistency step here, so the log is written as
/// it is, with events and objects that have no relation. The writer fails
/// where pm4py's does: on an empty or repeated event or object id, an
/// empty type, a relation to an unknown event or object, a repeated
/// relation, an object change of an unknown object, of another type, or
/// without a value, two changes of one attribute at one time, an attribute
/// named like a fixed column, and a list or container value.
///
/// Each event type's table has the attributes that hold a value for one of
/// its events, in first-appearance order, and each object type's tables
/// the attributes of its objects and then the fields its changes set. A
/// column's type follows from its values: `boolean` when all are booleans,
/// `integer` when all are integers, `float` when all are finite numbers,
/// `time` when all are times, else `string`. A change at the Unix epoch
/// sets the object's value in the objects table instead; a different value
/// there is an error. Times are written in UTC.
///
/// CSV tables are written as pm4py writes them byte for byte when the
/// attribute order agrees. Parquet tables hold the same schema and values
/// as pm4py's, Snappy-compressed; their bytes differ from pyarrow's.
pub fn write_ocel2_bundle(
    ocel: &Ocel,
    path: impl AsRef<Path>,
    storage: BundleStorage,
) -> Result<()> {
    let path = path.as_ref();
    let files = bundle_files(ocel, storage)?;
    let name = lower_name(path);
    if name.ends_with(".ocel.zip") {
        let mut archive = zip::ZipWriter::new(File::create(path)?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in &files {
            archive.start_file(name.as_str(), options)?;
            archive.write_all(data)?;
        }
        archive.finish()?;
    } else if name.ends_with(".zip") {
        return Err(error(
            "Bundled OCEL archives use the '.ocel.zip' extension.",
        ));
    } else {
        fs::create_dir_all(path)?;
        for folder in ["events", "objects", "object_changes", "relations"] {
            let Ok(listing) = fs::read_dir(path.join(folder)) else {
                continue;
            };
            for entry in listing {
                let entry = entry?;
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if file_name.ends_with(".csv") || file_name.ends_with(".parquet") {
                    fs::remove_file(entry.path())?;
                }
            }
        }
        for (name, data) in &files {
            let full = name.split('/').fold(path.to_path_buf(), |p, c| p.join(c));
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full, data)?;
        }
    }
    Ok(())
}
