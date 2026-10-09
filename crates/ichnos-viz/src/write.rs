//! Saving DOT text, as pm4py's `visualization/common/save.py` does.

use std::io::{ErrorKind, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};

use ichnos_conformance::alignments::{LogAlignment, TraceAlignment};
use ichnos_discovery::PerformanceDfg;
use ichnos_discovery::prefix_tree::PrefixTree;
use ichnos_model::{
    Bpmn, Dfg, Footprints, HeuristicsNet, Marking, PetriNet, ProcessTree, TransitionSystem,
};

use crate::{
    AlignmentsDotOptions, BpmnDotOptions, DfgDotOptions, FootprintsDotOptions,
    HeuristicsNetDotOptions, PerformanceDfgDotOptions, PetriNetDotOptions, PrefixTreeDotOptions,
    ProcessTreeDotOptions, TransitionSystemDotOptions, VariantAlignment, alignment_table_dot,
    alignments_dot, bpmn_dot, dfg_dot, footprints_comparison_dot, footprints_dot,
    heuristics_net_dot, performance_dfg_dot, petri_net_dot, prefix_tree_dot, process_tree_dot,
    transition_system_dot,
};

/// Errors from saving a graph.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VizError {
    /// The path has no extension, or one that is not a Graphviz format
    /// name.
    #[error(
        "cannot tell the output format from {0:?}; give the file an extension such as .dot or .svg"
    )]
    UnknownFormat(String),
    /// The format needs Graphviz, and `dot` is not on the `PATH`.
    #[error("writing .{0} needs the Graphviz `dot` program, which is not on the PATH")]
    DotNotFound(String),
    /// `dot` failed.
    #[error("Graphviz `dot` failed ({status}): {stderr}")]
    Dot {
        /// The exit status.
        status: String,
        /// What `dot` wrote to standard error.
        stderr: String,
    },
    /// Graphviz drew a POWL model without its icons. It needs an SVG image
    /// loader, such as the rsvg plugin, to put SVG icons into PNG or PDF
    /// output; without one it warns and still exits with success.
    #[error("Graphviz drew the POWL model without its icons: {0}")]
    IconsNotLoaded(String),
    /// Reading or writing a file failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Saves DOT text to `path`, in the format its extension names.
///
/// `.dot` and `.gv` get the text itself. Any other extension, such as
/// `.svg`, `.png` or `.pdf`, is passed to Graphviz as `dot -T<ext>`, so it
/// needs `dot` on the `PATH` and fails with [`VizError::DotNotFound`]
/// without it.
pub fn write_dot(dot: &str, path: impl AsRef<Path>) -> Result<(), VizError> {
    write_with(dot, path.as_ref(), "dot")
}

fn write_with(dot: &str, path: &Path, program: &str) -> Result<(), VizError> {
    let format = format_of(path)?;
    if format == "dot" || format == "gv" {
        std::fs::write(path, dot)?;
        return Ok(());
    }
    std::fs::write(path, render(dot, &format, program)?.stdout)?;
    Ok(())
}

/// The format a path's extension names: lowercase ASCII letters, digits and
/// `_+-`.
pub(crate) fn format_of(path: &Path) -> Result<String, VizError> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|f| {
            !f.is_empty()
                && f.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_+-".contains(&b))
        })
        .ok_or_else(|| VizError::UnknownFormat(path.display().to_string()))
}

/// What Graphviz wrote for one run that succeeded.
pub(crate) struct Rendered {
    /// The rendered file.
    pub(crate) stdout: Vec<u8>,
    /// Warnings.
    pub(crate) stderr: String,
}

/// Renders DOT text with `program -T<format>` and returns what it writes.
///
/// The DOT text goes to `program` from a separate thread, so a program that
/// stops reading early, as `dot` does on a syntax error, still gets its exit
/// status and standard error read.
pub(crate) fn render(dot: &str, format: &str, program: &str) -> Result<Rendered, VizError> {
    let mut child = match Command::new(program)
        .arg(format!("-T{format}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(VizError::DotNotFound(format.to_owned()));
        }
        Err(e) => return Err(e.into()),
    };
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let text = dot.as_bytes().to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&text));
    let output = child.wait_with_output()?;
    let written = writer.join().expect("writing stdin does not panic");
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.status.success() {
        return Err(VizError::Dot {
            status: output.status.to_string(),
            stderr,
        });
    }
    written?;
    Ok(Rendered {
        stdout: output.stdout,
        stderr,
    })
}

/// Saves a Petri net drawing ([`petri_net_dot`]); see [`write_dot`] for the
/// formats. pm4py's `save_vis_petri_net`.
pub fn write_petri_net(
    net: &PetriNet,
    initial: &Marking,
    fin: &Marking,
    options: &PetriNetDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&petri_net_dot(net, initial, fin, options), path)
}

/// Saves a frequency DFG drawing ([`dfg_dot`]); see [`write_dot`] for the
/// formats. pm4py's `save_vis_dfg`.
pub fn write_dfg(
    dfg: &Dfg,
    options: &DfgDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&dfg_dot(dfg, options), path)
}

/// Saves a performance DFG drawing ([`performance_dfg_dot`]); see
/// [`write_dot`] for the formats. pm4py's `save_vis_performance_dfg`.
pub fn write_performance_dfg(
    dfg: &PerformanceDfg,
    options: &PerformanceDfgDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&performance_dfg_dot(dfg, options), path)
}

/// Saves a process tree drawing ([`process_tree_dot`]); see [`write_dot`]
/// for the formats. pm4py's `save_vis_process_tree`.
pub fn write_process_tree(
    tree: &ProcessTree,
    options: &ProcessTreeDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&process_tree_dot(tree, options), path)
}

/// Saves a BPMN drawing ([`bpmn_dot`]); see [`write_dot`] for the formats.
/// pm4py's `save_vis_bpmn`.
pub fn write_bpmn(
    bpmn: &Bpmn,
    options: &BpmnDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&bpmn_dot(bpmn, options), path)
}

/// Saves a heuristics net drawing ([`heuristics_net_dot`]); see
/// [`write_dot`] for the formats. pm4py's `save_vis_heuristics_net`.
pub fn write_heuristics_net(
    net: &HeuristicsNet,
    options: &HeuristicsNetDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&heuristics_net_dot(net, options), path)
}

/// Saves a transition system drawing ([`transition_system_dot`]); see
/// [`write_dot`] for the formats. pm4py's `save_vis_transition_system`.
pub fn write_transition_system(
    ts: &TransitionSystem,
    options: &TransitionSystemDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&transition_system_dot(ts, options), path)
}

/// Saves a prefix tree drawing ([`prefix_tree_dot`]); see [`write_dot`] for
/// the formats. pm4py's `save_vis_prefix_tree`.
pub fn write_prefix_tree(
    tree: &PrefixTree,
    options: &PrefixTreeDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&prefix_tree_dot(tree, options), path)
}

/// Saves a footprint table ([`footprints_dot`]); see [`write_dot`] for the
/// formats. pm4py's `save_vis_footprints` with one footprint.
pub fn write_footprints(
    fp: &Footprints,
    options: &FootprintsDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&footprints_dot(fp, options), path)
}

/// Saves a footprint comparison ([`footprints_comparison_dot`]); see
/// [`write_dot`] for the formats. pm4py's `save_vis_footprints` with two
/// footprints.
pub fn write_footprints_comparison(
    first: &Footprints,
    second: &Footprints,
    options: &FootprintsDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&footprints_comparison_dot(first, second, options), path)
}

/// Saves an alignment table ([`alignments_dot`]); see [`write_dot`] for the
/// formats. pm4py's `save_vis_alignments`.
pub fn write_alignments(
    alignments: &LogAlignment<TraceAlignment>,
    net: &PetriNet,
    options: &AlignmentsDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&alignments_dot(alignments, net, options), path)
}

/// Saves an alignment table from its rows ([`alignment_table_dot`]); see
/// [`write_dot`] for the formats.
pub fn write_alignment_table(
    rows: &[VariantAlignment],
    options: &AlignmentsDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_dot(&alignment_table_dot(rows, options), path)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A path in the temporary directory. Nothing is written there: the
    /// tests below fail before creating a file.
    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ichnos-viz-write-{}-{name}", std::process::id()))
    }

    #[test]
    fn a_missing_program_is_a_typed_error() {
        let path = scratch("g.svg");
        let err = write_with("digraph {}\n", &path, "ichnos-no-such-dot").unwrap_err();
        assert!(!path.exists());
        assert!(
            matches!(err, VizError::DotNotFound(ref f) if f == "svg"),
            "{err}"
        );
    }

    /// An executable shell script in the temporary directory that stands in
    /// for `dot`.
    #[cfg(unix)]
    pub(crate) fn fake_program(name: &str, script: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let path = scratch(name);
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn a_program_that_stops_reading_keeps_its_error() {
        // Far more than a pipe buffer, which the program never reads.
        let dot = format!("digraph {{ {} }}\n", "a -> ; ".repeat(200_000));
        let program = fake_program(
            "syntax-error",
            "echo 'Error: syntax error in line 1' >&2; exit 1",
        );
        let err = render(&dot, "svg", program.to_str().unwrap())
            .err()
            .unwrap();
        let _ = std::fs::remove_file(&program);
        assert!(
            matches!(err, VizError::Dot { ref stderr, .. } if stderr.contains("syntax error")),
            "{err}"
        );
    }

    #[test]
    fn the_extension_must_name_a_format() {
        for name in ["graph", "graph.", "graph.s v"] {
            let err = write_dot("digraph {}\n", scratch(name)).unwrap_err();
            assert!(matches!(err, VizError::UnknownFormat(_)), "{name}: {err}");
        }
    }
}
