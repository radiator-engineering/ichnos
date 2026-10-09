//! Saving DOT text, as pm4py's `visualization/common/save.py` does.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use ichnos_model::{Bpmn, Dfg, HeuristicsNet, Marking, PetriNet, ProcessTree};

use crate::{
    BpmnDotOptions, DfgDotOptions, HeuristicsNetDotOptions, PerformanceDfgDotOptions,
    PetriNetDotOptions, ProcessTreeDotOptions, bpmn_dot, dfg_dot, heuristics_net_dot,
    performance_dfg_dot, petri_net_dot, process_tree_dot,
};
use ichnos_discovery::PerformanceDfg;

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
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|f| {
            !f.is_empty()
                && f.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_+-".contains(&b))
        })
        .ok_or_else(|| VizError::UnknownFormat(path.display().to_string()))?;
    if format == "dot" || format == "gv" {
        std::fs::write(path, dot)?;
        return Ok(());
    }
    let mut child = match Command::new(program)
        .arg(format!("-T{format}"))
        .arg("-o")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(VizError::DotNotFound(format));
        }
        Err(e) => return Err(e.into()),
    };
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(dot.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(VizError::Dot {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ichnos-viz-write-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    #[test]
    fn a_missing_program_is_a_typed_error() {
        let err = write_with("digraph {}\n", &scratch("g.svg"), "ichnos-no-such-dot").unwrap_err();
        assert!(
            matches!(err, VizError::DotNotFound(ref f) if f == "svg"),
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
