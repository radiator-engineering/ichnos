use super::{PnmlDocument, PnmlVariable, StochasticInfo, TransitionData};
use crate::{
    Result,
    model_xml::{self as xml, Element, invalid},
};
use ichnos_model::{
    AcceptingPetriNet, Marking, PetriNet,
    petri::{ArcEnds, ArcKind},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

/// Options for PNML import.
#[derive(Debug, Clone)]
pub struct PnmlReadOptions {
    /// Infer a marking with one token per sink when no final marking is declared.
    pub guess_final_marking: bool,
    /// Maximum XML nesting.
    pub max_depth: usize,
    /// Maximum XML element count.
    pub max_nodes: usize,
}

impl Default for PnmlReadOptions {
    fn default() -> Self {
        Self {
            guess_final_marking: false,
            max_depth: 128,
            max_nodes: 1_000_000,
        }
    }
}

fn nodes<'a>(element: &'a Element, output: &mut Vec<&'a Element>) {
    for child in &element.children {
        if child.name == "page" {
            nodes(child, output);
        } else if matches!(child.name.as_str(), "place" | "transition" | "arc") {
            output.push(child);
        }
    }
}

fn tokens(text: &str) -> Result<u32> {
    text.trim()
        .parse()
        .map_err(|_| invalid("PNML", format!("invalid token/arc count {text:?}")))
}

/// Reads a PNML file, retaining accepting markings and stochastic information.
pub fn read_pnml(path: impl AsRef<Path>, options: &PnmlReadOptions) -> Result<PnmlDocument> {
    read_pnml_from_reader(BufReader::new(File::open(path)?), options)
}

/// Reads PNML from a buffered UTF-8 XML stream. Node and arc declarations
/// may appear in any order, including inside nested pages.
pub fn read_pnml_from_reader(
    input: impl BufRead,
    options: &PnmlReadOptions,
) -> Result<PnmlDocument> {
    let root = xml::read(input, "PNML", options.max_depth, options.max_nodes)?;
    if root.name != "pnml" {
        return Err(invalid("PNML", "expected <pnml>"));
    }
    let nets: Vec<_> = root.children.iter().filter(|e| e.name == "net").collect();
    if nets.len() != 1 {
        return Err(invalid("PNML", "expected exactly one net"));
    }
    let net_xml = nets[0];
    let mut all_nodes = Vec::new();
    nodes(net_xml, &mut all_nodes);
    let mut ids = BTreeSet::new();
    for node in &all_nodes {
        let id = node.required("id", "PNML")?;
        if !ids.insert(id) {
            return Err(invalid("PNML", format!("duplicate node/arc id {id:?}")));
        }
    }
    let mut document = PnmlDocument::default();
    let mut net = PetriNet::new(net_xml.attr("id").unwrap_or("net"));
    let mut places = BTreeMap::new();
    let mut transitions = BTreeMap::new();
    let mut initial = Marking::new();
    for node in &all_nodes {
        if node.name != "place" {
            continue;
        }
        let id = node.required("id", "PNML")?;
        let place = net.add_place(id);
        places.insert(id, place);
        if let Some(name) = node.child_text("name") {
            document.place_names.insert(place, name.into());
        }
        if let Some(text) = node.child_text("initialMarking") {
            initial.set(place, tokens(text)?);
        }
    }
    for node in &all_nodes {
        if node.name != "transition" {
            continue;
        }
        let id = node.required("id", "PNML")?;
        let name = node.child_text("name").unwrap_or(id);
        let mut silent = false;
        let mut stochastic = None;
        for tool in node
            .children
            .iter()
            .filter(|child| child.name == "toolspecific")
        {
            let tool_name = tool.attr("tool").unwrap_or("");
            if tool_name.contains("ProM")
                && tool
                    .attr("activity")
                    .is_some_and(|a| a.contains("invisible"))
            {
                silent = true;
            }
            if tool_name.contains("StochasticPetriNet") {
                if stochastic.is_some() {
                    return Err(invalid("PNML", "duplicate stochastic declaration"));
                }
                let mut info = StochasticInfo::default();
                for property in &tool.children {
                    if property.name != "property" {
                        continue;
                    }
                    let key = property.required("key", "PNML")?;
                    let text = property.text.as_str();
                    match key {
                        "distributionType" => info.distribution_type = text.into(),
                        "distributionParameters" => {
                            info.distribution_parameters = Some(text.into())
                        }
                        "priority" => {
                            info.priority = Some(
                                text.trim()
                                    .parse()
                                    .map_err(|_| invalid("PNML", "invalid stochastic priority"))?,
                            )
                        }
                        "weight" => {
                            let weight: f64 = text
                                .trim()
                                .parse()
                                .map_err(|_| invalid("PNML", "invalid stochastic weight"))?;
                            if !weight.is_finite() || weight < 0.0 {
                                return Err(invalid(
                                    "PNML",
                                    "stochastic weight must be finite and nonnegative",
                                ));
                            }
                            info.weight = Some(weight);
                        }
                        "invisible" => silent |= text.trim().eq_ignore_ascii_case("true"),
                        _ => {
                            info.properties.insert(key.into(), text.into());
                        }
                    }
                }
                if info.distribution_type.is_empty() {
                    return Err(invalid("PNML", "stochastic distribution has no type"));
                }
                stochastic = Some(info);
            }
        }
        let transition = net.add_transition(id, if silent { None } else { Some(name) });
        transitions.insert(id, transition);
        if node.child("name").is_some() {
            document.transition_names.insert(transition, name.into());
        }
        if let Some(info) = stochastic {
            document.stochastic.insert(transition, info);
        }
        let mut data = TransitionData {
            guard: node.attr("guard").map(str::to_owned),
            ..Default::default()
        };
        for child in &node.children {
            match child.name.as_str() {
                "readVariable" => data.read_variables.push(child.text.clone()),
                "writeVariable" => data.write_variables.push(child.text.clone()),
                _ => {}
            }
        }
        if data != TransitionData::default() {
            document.transition_data.insert(transition, data);
        }
    }
    for node in &all_nodes {
        if node.name != "arc" {
            continue;
        }
        let source = node.required("source", "PNML")?;
        let target = node.required("target", "PNML")?;
        let weight = node
            .child_text("inscription")
            .map(tokens)
            .transpose()?
            .unwrap_or(1);
        let kind = match node
            .child_text("arctype")
            .or_else(|| node.child("type").and_then(|e| e.attr("value")))
            .unwrap_or("normal")
            .trim()
        {
            "normal" => ArcKind::Normal,
            "inhibitor" => ArcKind::Inhibitor,
            "reset" => ArcKind::Reset,
            other => return Err(invalid("PNML", format!("unsupported arc kind {other:?}"))),
        };
        let ends = if let (Some(&p), Some(&t)) = (places.get(source), transitions.get(target)) {
            ArcEnds::PlaceToTransition(p, t)
        } else if let (Some(&t), Some(&p)) = (transitions.get(source), places.get(target)) {
            if kind != ArcKind::Normal {
                return Err(invalid(
                    "PNML",
                    "special arcs must run from place to transition",
                ));
            }
            ArcEnds::TransitionToPlace(t, p)
        } else {
            return Err(invalid(
                "PNML",
                format!("invalid arc endpoints {source:?} -> {target:?}"),
            ));
        };
        net.add_arc(ends, weight, kind)
            .map_err(ichnos_model::Error::from)?;
    }
    let mut finals = Vec::new();
    if let Some(finals_xml) = net_xml.child("finalmarkings") {
        for marking_xml in &finals_xml.children {
            if marking_xml.name != "marking" {
                return Err(invalid("PNML", "expected <marking> in finalmarkings"));
            }
            let mut marking = Marking::new();
            let mut seen = BTreeSet::new();
            for place_xml in &marking_xml.children {
                if place_xml.name != "place" {
                    return Err(invalid("PNML", "expected final marking place"));
                }
                let id = place_xml.required("idref", "PNML")?;
                if !seen.insert(id) {
                    return Err(invalid("PNML", "duplicate final marking place"));
                }
                let place = *places.get(id).ok_or_else(|| {
                    invalid("PNML", format!("unknown final marking place {id:?}"))
                })?;
                let text = place_xml
                    .child_text("text")
                    .ok_or_else(|| invalid("PNML", "final marking place has no text"))?;
                marking.set(place, tokens(text)?);
            }
            finals.push(marking);
        }
    }
    let final_marking = if finals.is_empty() {
        if net_xml.child("finalmarkings").is_none() && options.guess_final_marking {
            net.discover_final_marking()
        } else {
            Marking::new()
        }
    } else {
        finals.remove(0)
    };
    document.additional_final_markings = finals;
    if let Some(variables) = net_xml.child("variables") {
        for variable in &variables.children {
            document.variables.push(PnmlVariable {
                name: variable
                    .child_text("name")
                    .ok_or_else(|| invalid("PNML", "variable has no name"))?
                    .into(),
                type_name: variable.required("type", "PNML")?.into(),
            });
        }
    }
    document.model = AcceptingPetriNet::new(net, initial, final_marking);
    Ok(document)
}
