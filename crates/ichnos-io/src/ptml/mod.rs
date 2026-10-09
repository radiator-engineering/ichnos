//! Process trees in the ProM PTML XML format.

use crate::{
    Result,
    model_xml::{self as xml, invalid},
};
use ichnos_model::{Operator, ProcessTree};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

/// Limits for reading PTML XML and expanding its referenced tree nodes.
#[derive(Debug, Clone)]
pub struct PtmlReadOptions {
    /// Maximum XML and expanded tree nesting.
    pub max_depth: usize,
    /// Maximum XML elements and expanded tree nodes.
    pub max_nodes: usize,
}
impl Default for PtmlReadOptions {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 1_000_000,
        }
    }
}

/// Options for writing PTML.
#[derive(Debug, Clone)]
pub struct PtmlWriteOptions {
    /// Indent the XML.
    pub indent: bool,
    /// Emit a silent third child for loops for ProM compatibility.
    pub prom_loop_exit: bool,
    /// Maximum tree nesting.
    pub max_depth: usize,
    /// Maximum exported nodes, including compatibility exits.
    pub max_nodes: usize,
}
impl Default for PtmlWriteOptions {
    fn default() -> Self {
        Self {
            indent: true,
            prom_loop_exit: true,
            max_depth: 128,
            max_nodes: 1_000_000,
        }
    }
}

#[derive(PartialEq, Eq)]
struct Node {
    operator: Option<Operator>,
    label: Option<String>,
    children: Vec<String>,
}

struct Expansion<'a> {
    nodes: &'a BTreeMap<String, Node>,
    options: &'a PtmlReadOptions,
    active: BTreeSet<String>,
    used: BTreeSet<String>,
    count: usize,
}

impl Expansion<'_> {
    fn build(&mut self, id: &str, depth: usize) -> Result<ProcessTree> {
        self.count += 1;
        if depth >= self.options.max_depth || self.count > self.options.max_nodes {
            return Err(invalid("PTML", "expanded tree size/depth limit exceeded"));
        }
        if !self.active.insert(id.into()) {
            return Err(invalid("PTML", format!("cycle at {id:?}")));
        }
        self.used.insert(id.into());
        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| invalid("PTML", format!("unknown node {id:?}")))?;
        let tree = if let Some(operator) = node.operator {
            if node.children.is_empty() {
                return Err(ichnos_model::Error::from(
                    ichnos_model::process_tree::TreeError::EmptyOperator(operator),
                )
                .into());
            }
            let mut children = node
                .children
                .iter()
                .map(|child| self.build(child, depth + 1))
                .collect::<Result<Vec<_>>>()?;
            if operator == Operator::Loop {
                if children.len() != 2 && children.len() != 3 {
                    return Err(ichnos_model::Error::from(
                        ichnos_model::process_tree::TreeError::LoopArity(children.len()),
                    )
                    .into());
                }
                if children.len() == 3 {
                    let exit = children.pop().expect("three children");
                    let loop_tree = ProcessTree::node(operator, children);
                    if exit.is_tau() {
                        loop_tree
                    } else {
                        self.count += 1;
                        if self.count > self.options.max_nodes {
                            return Err(invalid("PTML", "expanded tree size limit exceeded"));
                        }
                        ProcessTree::sequence([loop_tree, exit])
                    }
                } else {
                    ProcessTree::node(operator, children)
                }
            } else {
                ProcessTree::node(operator, children)
            }
        } else {
            if !node.children.is_empty() {
                return Err(invalid("PTML", "leaf has children"));
            }
            node.label.as_ref().map_or(ProcessTree::Tau, |label| {
                ProcessTree::activity(label.clone())
            })
        };
        self.active.remove(id);
        Ok(tree)
    }
}

/// Reads a PTML file into a core process tree.
pub fn read_ptml(path: impl AsRef<Path>, options: &PtmlReadOptions) -> Result<ProcessTree> {
    read_ptml_from_reader(BufReader::new(File::open(path)?), options)
}

/// Reads referenced PTML nodes, preserving edge declaration order. Three-child
/// loops become a two-child loop followed by a non-silent exit, when present.
pub fn read_ptml_from_reader(
    input: impl BufRead,
    options: &PtmlReadOptions,
) -> Result<ProcessTree> {
    let root = xml::read(input, "PTML", options.max_depth, options.max_nodes)?;
    if root.name != "ptml" || root.children.len() != 1 || root.children[0].name != "processTree" {
        return Err(invalid("PTML", "expected one <processTree> in <ptml>"));
    }
    let tree = &root.children[0];
    let root_id = tree.required("root", "PTML")?;
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    for element in &tree.children {
        if element.name == "parentsNode" {
            edges.push((
                element.required("sourceId", "PTML")?.to_owned(),
                element.required("targetId", "PTML")?.to_owned(),
            ));
            continue;
        }
        let operator = match element.name.as_str() {
            "sequence" => Some(Operator::Sequence),
            "xor" => Some(Operator::Xor),
            "and" => Some(Operator::Parallel),
            "xorLoop" => Some(Operator::Loop),
            "or" => Some(Operator::Or),
            "manualTask" | "automaticTask" => None,
            name => return Err(invalid("PTML", format!("unsupported node tag {name:?}"))),
        };
        let node = Node {
            operator,
            label: if element.name == "manualTask" {
                Some(element.required("name", "PTML")?.into())
            } else {
                None
            },
            children: Vec::new(),
        };
        let id = element.required("id", "PTML")?;
        if let Some(previous) = nodes.insert(id.to_owned(), node)
            && nodes[id] != previous
        {
            return Err(invalid(
                "PTML",
                format!("conflicting duplicate node {id:?}"),
            ));
        }
    }
    for (source, target) in edges {
        if !nodes.contains_key(&target) {
            return Err(invalid("PTML", format!("unknown edge target {target:?}")));
        }
        nodes
            .get_mut(&source)
            .ok_or_else(|| invalid("PTML", format!("unknown edge source {source:?}")))?
            .children
            .push(target);
    }
    let mut expansion = Expansion {
        nodes: &nodes,
        options,
        active: BTreeSet::new(),
        used: BTreeSet::new(),
        count: 0,
    };
    let tree = expansion.build(root_id, 0)?;
    if expansion.used.len() != nodes.len() {
        return Err(invalid("PTML", "unreachable node declarations"));
    }
    Ok(tree)
}

fn flat_nodes<'a>(
    tree: &'a ProcessTree,
    options: &PtmlWriteOptions,
) -> Result<Vec<(&'a ProcessTree, Option<usize>, usize)>> {
    static TAU: ProcessTree = ProcessTree::Tau;
    let mut nodes = vec![(tree, None, 0)];
    let mut index = 0;
    while index < nodes.len() {
        if nodes.len() > options.max_nodes {
            return Err(invalid("PTML", "exported tree size limit exceeded"));
        }
        let (node, _, depth) = nodes[index];
        if depth >= options.max_depth {
            return Err(invalid("PTML", "exported tree depth limit exceeded"));
        }
        if let Some(operator) = node.operator() {
            if operator == Operator::Interleaving {
                return Err(invalid("PTML", "PTML has no interleaving operator"));
            }
            if node.children().is_empty() {
                return Err(ichnos_model::Error::from(
                    ichnos_model::process_tree::TreeError::EmptyOperator(operator),
                )
                .into());
            }
            if operator == Operator::Loop && node.children().len() != 2 {
                return Err(ichnos_model::Error::from(
                    ichnos_model::process_tree::TreeError::LoopArity(node.children().len()),
                )
                .into());
            }
        }
        nodes.extend(
            node.children()
                .iter()
                .map(|child| (child, Some(index), depth + 1)),
        );
        if node.operator() == Some(Operator::Loop) && options.prom_loop_exit {
            nodes.push((&TAU, Some(index), depth + 1));
        }
        index += 1;
    }
    Ok(nodes)
}

/// Writes deterministic PTML node ids and ProM-compatible loops.
pub fn write_ptml(
    tree: &ProcessTree,
    path: impl AsRef<Path>,
    options: &PtmlWriteOptions,
) -> Result<()> {
    flat_nodes(tree, options)?;
    write_ptml_to_writer(tree, BufWriter::new(File::create(path)?), options)
}

/// Writes a process tree as UTF-8 PTML to a stream.
pub fn write_ptml_to_writer(
    tree: &ProcessTree,
    output: impl Write,
    options: &PtmlWriteOptions,
) -> Result<()> {
    let nodes = flat_nodes(tree, options)?;
    let mut writer = xml::writer(output, options.indent)?;
    xml::start(&mut writer, "ptml", &[], false)?;
    xml::start(
        &mut writer,
        "processTree",
        &[("id", "tree0"), ("name", "tree"), ("root", "n0")],
        false,
    )?;
    for (index, (node, _, _)) in nodes.iter().enumerate() {
        let tag = match node {
            ProcessTree::Tau => "automaticTask",
            ProcessTree::Activity(_) => "manualTask",
            ProcessTree::Node(op, _) => match op {
                Operator::Sequence => "sequence",
                Operator::Xor => "xor",
                Operator::Parallel => "and",
                Operator::Loop => "xorLoop",
                Operator::Or => "or",
                Operator::Interleaving => unreachable!(),
            },
        };
        let id = format!("n{index}");
        let name = node.label().map_or("", |label| label.as_str());
        xml::start(&mut writer, tag, &[("id", &id), ("name", name)], true)?;
    }
    for (index, (_, parent, _)) in nodes.iter().enumerate() {
        if let Some(parent) = parent {
            xml::start(
                &mut writer,
                "parentsNode",
                &[
                    ("id", &format!("e{index}")),
                    ("sourceId", &format!("n{parent}")),
                    ("targetId", &format!("n{index}")),
                ],
                true,
            )?;
        }
    }
    xml::end(&mut writer, "processTree")?;
    xml::end(&mut writer, "ptml")?;
    writer.get_mut().flush()?;
    Ok(())
}
