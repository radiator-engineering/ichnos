//! Frequency directly-follows graphs in pm4py's line-based `.dfg` format.

use crate::{Result, model_xml::invalid};
use ichnos_model::{Dfg, Label};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

/// Options for reading a DFG.
#[derive(Debug, Clone)]
pub struct DfgReadOptions {
    /// Maximum number of declared activities.
    pub max_activities: usize,
    /// Maximum number of frequency records (starts, ends and edges).
    pub max_records: usize,
}
impl Default for DfgReadOptions {
    fn default() -> Self {
        Self {
            max_activities: 1_000_000,
            max_records: 10_000_000,
        }
    }
}

/// Options for writing a DFG.
#[derive(Debug, Clone, Default)]
pub struct DfgWriteOptions {
    /// If explicit start/end maps are empty, infer the missing maps from edges,
    /// with frequency one, as pm4py's low-level exporter does.
    pub infer_boundaries: bool,
}

fn line(lines: &mut impl Iterator<Item = std::io::Result<String>>) -> Result<String> {
    lines
        .next()
        .ok_or_else(|| invalid("DFG", "unexpected end of file"))?
        .map_err(Into::into)
}
fn number<T: std::str::FromStr>(text: &str) -> Result<T> {
    text.trim()
        .parse()
        .map_err(|_| invalid("DFG", format!("invalid nonnegative integer {text:?}")))
}
fn activity(activities: &[Label], text: &str) -> Result<Label> {
    let index: usize = number(text)?;
    activities
        .get(index)
        .cloned()
        .ok_or_else(|| invalid("DFG", format!("activity index {index} is out of range")))
}

/// Reads a frequency DFG with explicit start/end counts.
pub fn read_dfg(path: impl AsRef<Path>, options: &DfgReadOptions) -> Result<Dfg> {
    read_dfg_from_reader(BufReader::new(File::open(path)?), options)
}

/// Reads UTF-8 DFG records from a buffered stream. Duplicate frequency records
/// use the last declared count, matching pm4py's importer.
pub fn read_dfg_from_reader(input: impl BufRead, options: &DfgReadOptions) -> Result<Dfg> {
    let mut lines = input.lines();
    let count: usize = number(&line(&mut lines)?)?;
    if count > options.max_activities {
        return Err(invalid("DFG", "activity limit exceeded"));
    }
    let mut activities = Vec::with_capacity(count);
    let mut names = BTreeSet::new();
    for _ in 0..count {
        let name = line(&mut lines)?.trim().to_owned();
        if !names.insert(name.clone()) {
            return Err(invalid("DFG", "duplicate activity declaration"));
        }
        activities.push(Label::from(name));
    }
    let mut dfg = Dfg::new();
    let mut records = 0;
    for map in [&mut dfg.start_activities, &mut dfg.end_activities] {
        let count: usize = number(&line(&mut lines)?)?;
        if count > options.max_records.saturating_sub(records) {
            return Err(invalid("DFG", "record limit exceeded"));
        }
        records += count;
        for _ in 0..count {
            let record = line(&mut lines)?;
            let (index, count) = record
                .trim()
                .split_once('x')
                .ok_or_else(|| invalid("DFG", "expected activity-index x count"))?;
            map.insert(activity(&activities, index)?, number(count)?);
        }
    }
    for record in lines {
        let record = record?;
        records += 1;
        if records > options.max_records {
            return Err(invalid("DFG", "record limit exceeded"));
        }
        let (edge, count) = record
            .trim()
            .split_once('x')
            .ok_or_else(|| invalid("DFG", "expected source > target x count"))?;
        let (source, target) = edge
            .split_once('>')
            .ok_or_else(|| invalid("DFG", "edge has no target"))?;
        dfg.graph.insert(
            (
                activity(&activities, source)?,
                activity(&activities, target)?,
            ),
            number(count)?,
        );
    }
    Ok(dfg)
}

/// Writes deterministic activity indexes and all explicit boundary frequencies.
pub fn write_dfg(dfg: &Dfg, path: impl AsRef<Path>, options: &DfgWriteOptions) -> Result<()> {
    write_dfg_to_writer(dfg, BufWriter::new(File::create(path)?), options)
}

/// Writes a frequency DFG to a stream, including boundary-only activities and
/// empty graphs. Labels with leading/trailing whitespace or newlines error.
pub fn write_dfg_to_writer(
    dfg: &Dfg,
    mut output: impl Write,
    options: &DfgWriteOptions,
) -> Result<()> {
    let activities = dfg.vertices();
    if activities
        .iter()
        .any(|a| a.as_str().trim() != a.as_str() || a.as_str().contains(['\n', '\r']))
    {
        return Err(invalid(
            "DFG",
            "activity cannot round-trip in a line-based format",
        ));
    }
    let indexes: BTreeMap<_, _> = activities.iter().enumerate().map(|(i, a)| (a, i)).collect();
    writeln!(output, "{}", activities.len())?;
    for activity in &activities {
        writeln!(output, "{activity}")?;
    }
    let inferred_start = if options.infer_boundaries && dfg.start_activities.is_empty() {
        dfg.infer_start_activities()
            .into_iter()
            .map(|a| (a, 1))
            .collect()
    } else {
        dfg.start_activities.clone()
    };
    let inferred_end = if options.infer_boundaries && dfg.end_activities.is_empty() {
        dfg.infer_end_activities()
            .into_iter()
            .map(|a| (a, 1))
            .collect()
    } else {
        dfg.end_activities.clone()
    };
    for map in [&inferred_start, &inferred_end] {
        writeln!(output, "{}", map.len())?;
        for (activity, count) in map {
            writeln!(output, "{}x{count}", indexes[activity])?;
        }
    }
    for ((a, b), count) in &dfg.graph {
        writeln!(output, "{}>{}x{count}", indexes[a], indexes[b])?;
    }
    output.flush()?;
    Ok(())
}
