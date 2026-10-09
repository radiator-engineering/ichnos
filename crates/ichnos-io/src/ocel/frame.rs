//! Tables as pandas' C parser reads them for pm4py's OCEL 1.0 CSV reader:
//! `read_csv(dtype=str, index_col=False)` with the default options.
//!
//! - A field is quoted when it starts with `"`. Inside, `""` is a quote and
//!   line breaks are kept. Text after the closing quote joins the field, and
//!   a quote inside an unquoted field is kept as it is.
//! - `\n`, `\r\n` and `\r` end a record. Empty lines and lines of spaces and
//!   tabs are skipped. A byte order mark at the start is dropped.
//! - The first record names the columns. An empty name becomes
//!   `Unnamed: {i}`, and a repeated name gets `.1`, `.2` and so on.
//! - A short record is padded with missing values. When the first data
//!   record is longer than the header, its extra fields are dropped, and so
//!   are those of later records up to its length; a record longer than that
//!   is an error.
//! - pandas' default missing-value tokens, such as an empty field, `NA` or
//!   `null`, become missing values, quoted or not.

use std::collections::HashMap;
use std::io::Read;

use crate::csv::is_na;
use crate::error::{Error, Result};

/// A table of text values.
pub(super) struct Frame {
    /// The column names.
    pub(super) columns: Vec<String>,
    /// The records, each as long as `columns`. A missing value is `None`.
    pub(super) rows: Vec<Vec<Option<String>>>,
}

impl Frame {
    /// The index of the column named `name`.
    pub(super) fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

fn error(detail: impl Into<String>) -> Error {
    Error::Ocel(detail.into())
}

/// Reads a table.
pub(super) fn read(mut input: impl Read) -> Result<Frame> {
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes)?;
    let text = String::from_utf8(bytes).map_err(|_| error("the CSV file is not UTF-8"))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut records = tokenize(text)?.into_iter();
    let header = records
        .next()
        .ok_or_else(|| error("the CSV file has no columns"))?;
    let columns = names(header);
    let mut limit = None;
    let mut rows = Vec::new();
    for (i, record) in records.enumerate() {
        let limit = *limit.get_or_insert(record.len().max(columns.len()));
        if record.len() > limit {
            return Err(error(format!(
                "CSV record {} has {} fields; expected {limit}",
                i + 2,
                record.len()
            )));
        }
        let mut row: Vec<Option<String>> = record
            .into_iter()
            .take(columns.len())
            .map(|field| (!is_na(&field)).then_some(field))
            .collect();
        row.resize(columns.len(), None);
        rows.push(row);
    }
    Ok(Frame { columns, rows })
}

/// pandas' column names: `Unnamed: {i}` for an empty name, then a suffix
/// for each repeat (`dedup_names`).
fn names(header: Vec<String>) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    header
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let mut name = if name.is_empty() {
                format!("Unnamed: {i}")
            } else {
                name
            };
            let mut count = counts.get(&name).copied().unwrap_or(0);
            while count > 0 {
                counts.insert(name.clone(), count + 1);
                name = format!("{name}.{count}");
                count = counts.get(&name).copied().unwrap_or(0);
            }
            counts.insert(name.clone(), count + 1);
            name
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    StartRecord,
    StartField,
    InField,
    InQuoted,
    QuoteInQuoted,
}

/// Splits the text into records of fields.
fn tokenize(text: &str) -> Result<Vec<Vec<String>>> {
    let s = text.as_bytes();
    let mut records = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field: Vec<u8> = Vec::new();
    let mut state = State::StartRecord;
    let mut i = 0;
    // Fields split at ASCII bytes, so each one is valid UTF-8.
    let take = |field: &mut Vec<u8>| String::from_utf8(std::mem::take(field)).expect("UTF-8");
    while i < s.len() {
        let c = s[i];
        let newline = c == b'\n' || c == b'\r';
        // The length of the line break at `i`.
        let eol = if c == b'\r' && s.get(i + 1) == Some(&b'\n') {
            2
        } else {
            1
        };
        match state {
            State::StartRecord => {
                if newline {
                    i += eol;
                    continue;
                }
                if c == b' ' || c == b'\t' {
                    let end = s[i..]
                        .iter()
                        .position(|&b| b != b' ' && b != b'\t')
                        .map_or(s.len(), |p| i + p);
                    if end == s.len() || s[end] == b'\n' || s[end] == b'\r' {
                        i = end;
                        continue;
                    }
                }
                state = State::StartField;
                continue;
            }
            State::StartField if c == b'"' => state = State::InQuoted,
            State::StartField | State::InField | State::QuoteInQuoted if c == b',' => {
                record.push(take(&mut field));
                state = State::StartField;
            }
            State::StartField | State::InField | State::QuoteInQuoted if newline => {
                record.push(take(&mut field));
                records.push(std::mem::take(&mut record));
                state = State::StartRecord;
                i += eol;
                continue;
            }
            State::StartField | State::InField => {
                field.push(c);
                state = State::InField;
            }
            State::InQuoted if c == b'"' => state = State::QuoteInQuoted,
            State::InQuoted => field.push(c),
            State::QuoteInQuoted if c == b'"' => {
                field.push(c);
                state = State::InQuoted;
            }
            State::QuoteInQuoted => {
                field.push(c);
                state = State::InField;
            }
        }
        i += 1;
    }
    match state {
        State::StartRecord => {}
        State::InQuoted => return Err(error("the CSV file ends inside a quoted field")),
        _ => {
            record.push(take(&mut field));
            records.push(record);
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::{names, tokenize};

    #[test]
    fn splits_like_pandas() {
        let t = |text: &str| tokenize(text).unwrap();
        assert_eq!(t("a,b\n1,2"), [["a", "b"], ["1", "2"]]);
        assert_eq!(
            t("a\r\n\r\n \t\n\"x\ny\"\rb\"c\n"),
            [["a"], ["x\ny"], ["b\"c"]]
        );
        assert_eq!(t("\"a\"x,\"q\"\"\",\n"), [["ax", "q\"", ""]]);
        assert!(tokenize("a,\"b\n").is_err());
        assert_eq!(t(" a\n"), [[" a"]]);
    }

    #[test]
    fn names_follow_pandas() {
        let n = |names_: &[&str]| names(names_.iter().map(|s| s.to_string()).collect());
        assert_eq!(
            n(&["x", "x", "", "x.1", "x"]),
            ["x", "x.1", "Unnamed: 2", "x.1.1", "x.2"]
        );
    }
}
