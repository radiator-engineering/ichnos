use super::PnmlDocument;
use crate::{
    Result,
    model_xml::{self as xml, invalid},
};
use ichnos_model::{
    Marking, PetriNet,
    petri::{ArcEnds, ArcKind},
};
use quick_xml::{
    Writer,
    events::{BytesText, Event},
};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

/// Options for PNML export.
#[derive(Debug, Clone)]
pub struct PnmlWriteOptions {
    /// Indent the XML.
    pub indent: bool,
    /// Include alternative final markings (default true).
    /// pm4py merges all exported alternatives into one marking on import.
    /// Disable this to export only the primary final marking for pm4py.
    pub include_alternative_final_markings: bool,
}
impl Default for PnmlWriteOptions {
    fn default() -> Self {
        Self {
            indent: true,
            include_alternative_final_markings: true,
        }
    }
}

fn marking_valid(net: &PetriNet, marking: &Marking) -> Result<()> {
    if marking.iter().any(|(place, _)| !net.contains_place(place)) {
        return Err(invalid("PNML", "marking refers to a removed/foreign place"));
    }
    Ok(())
}

fn validate(document: &PnmlDocument) -> Result<BTreeSet<String>> {
    let net = &document.model.net;
    let mut names = BTreeSet::new();
    for name in net
        .places()
        .map(|(_, p)| &p.name)
        .chain(net.transitions().map(|(_, t)| &t.name))
    {
        if name.is_empty() || !names.insert(name.clone()) {
            return Err(invalid(
                "PNML",
                "node names must be nonempty and unique across places/transitions",
            ));
        }
    }
    marking_valid(net, &document.model.initial_marking)?;
    marking_valid(net, &document.model.final_marking)?;
    for marking in &document.additional_final_markings {
        marking_valid(net, marking)?;
    }
    if document.place_names.keys().any(|&p| !net.contains_place(p))
        || document
            .transition_names
            .keys()
            .chain(document.stochastic.keys())
            .chain(document.transition_data.keys())
            .any(|&t| !net.contains_transition(t))
    {
        return Err(invalid("PNML", "metadata refers to a removed/foreign node"));
    }
    for info in document.stochastic.values() {
        if info.distribution_type.is_empty()
            || info.weight.is_some_and(|w| !w.is_finite() || w < 0.0)
        {
            return Err(invalid("PNML", "invalid stochastic type/weight"));
        }
        if info.properties.keys().any(|k| {
            matches!(
                k.as_str(),
                "distributionType" | "distributionParameters" | "priority" | "weight" | "invisible"
            )
        }) {
            return Err(invalid(
                "PNML",
                "reserved stochastic property in extension map",
            ));
        }
    }
    Ok(names)
}

fn property(writer: &mut Writer<impl Write>, key: &str, text: &str) -> Result<()> {
    xml::start(writer, "property", &[("key", key)], false)?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    xml::end(writer, "property")
}

fn marking(writer: &mut Writer<impl Write>, net: &PetriNet, marking: &Marking) -> Result<()> {
    xml::start(writer, "marking", &[], false)?;
    for (place, tokens) in marking.iter() {
        xml::start(writer, "place", &[("idref", &net.place(place).name)], false)?;
        xml::text(writer, "text", &tokens.to_string())?;
        xml::end(writer, "place")?;
    }
    xml::end(writer, "marking")
}

/// Writes a PNML document, preserving additional markings and tool metadata.
pub fn write_pnml(
    document: &PnmlDocument,
    path: impl AsRef<Path>,
    options: &PnmlWriteOptions,
) -> Result<()> {
    validate(document)?;
    write_pnml_to_writer(document, BufWriter::new(File::create(path)?), options)
}

/// Writes deterministic UTF-8 PNML to a stream. Convert an AcceptingPetriNet
/// with `PnmlDocument::from` when no extra PNML metadata is needed.
pub fn write_pnml_to_writer(
    document: &PnmlDocument,
    output: impl Write,
    options: &PnmlWriteOptions,
) -> Result<()> {
    let mut used_ids = validate(document)?;
    let net = &document.model.net;
    let mut writer = xml::writer(output, options.indent)?;
    xml::start(&mut writer, "pnml", &[], false)?;
    xml::start(
        &mut writer,
        "net",
        &[
            ("id", &net.name),
            (
                "type",
                "http://www.pnml.org/version-2009/grammar/pnmlcoremodel",
            ),
        ],
        false,
    )?;
    xml::wrapped_text(&mut writer, "name", &net.name)?;
    xml::start(&mut writer, "page", &[("id", "page0")], false)?;
    for (id, place) in net.places() {
        xml::start(&mut writer, "place", &[("id", &place.name)], false)?;
        if let Some(name) = document.place_names.get(&id) {
            xml::wrapped_text(&mut writer, "name", name)?;
        }
        let tokens = document.model.initial_marking.get(id);
        if tokens > 0 {
            xml::wrapped_text(&mut writer, "initialMarking", &tokens.to_string())?;
        }
        xml::end(&mut writer, "place")?;
    }
    for (id, transition) in net.transitions() {
        let mut attrs = vec![("id", transition.name.as_str())];
        let data = document.transition_data.get(&id);
        if let Some(guard) = data.and_then(|data| data.guard.as_deref()) {
            attrs.push(("guard", guard));
        }
        xml::start(&mut writer, "transition", &attrs, false)?;
        if let Some(name) = transition
            .label
            .as_ref()
            .map(|label| label.as_str())
            .or_else(|| document.transition_names.get(&id).map(String::as_str))
        {
            xml::wrapped_text(&mut writer, "name", name)?;
        }
        if transition.is_silent() {
            xml::start(
                &mut writer,
                "toolspecific",
                &[
                    ("tool", "ProM"),
                    ("version", "6.4"),
                    ("activity", "$invisible$"),
                ],
                true,
            )?;
        }
        if let Some(info) = document.stochastic.get(&id) {
            xml::start(
                &mut writer,
                "toolspecific",
                &[("tool", "StochasticPetriNet"), ("version", "0.2")],
                false,
            )?;
            property(&mut writer, "distributionType", &info.distribution_type)?;
            if let Some(parameters) = &info.distribution_parameters {
                property(&mut writer, "distributionParameters", parameters)?;
            }
            if let Some(priority) = info.priority {
                property(&mut writer, "priority", &priority.to_string())?;
            }
            if let Some(weight) = info.weight {
                property(&mut writer, "weight", &weight.to_string())?;
            }
            property(
                &mut writer,
                "invisible",
                if transition.is_silent() {
                    "true"
                } else {
                    "false"
                },
            )?;
            for (key, value) in &info.properties {
                property(&mut writer, key, value)?;
            }
            xml::end(&mut writer, "toolspecific")?;
        }
        if let Some(data) = data {
            for name in &data.read_variables {
                xml::text(&mut writer, "readVariable", name)?;
            }
            for name in &data.write_variables {
                xml::text(&mut writer, "writeVariable", name)?;
            }
        }
        xml::end(&mut writer, "transition")?;
    }
    for (id, arc) in net.arcs() {
        let (source, target) = match arc.ends {
            ArcEnds::PlaceToTransition(p, t) => (&net.place(p).name, &net.transition(t).name),
            ArcEnds::TransitionToPlace(t, p) => (&net.transition(t).name, &net.place(p).name),
        };
        let mut arc_id = format!("arc_{}", id.index());
        while !used_ids.insert(arc_id.clone()) {
            arc_id.push('_');
        }
        xml::start(
            &mut writer,
            "arc",
            &[("id", &arc_id), ("source", source), ("target", target)],
            false,
        )?;
        if arc.weight != 1 {
            xml::wrapped_text(&mut writer, "inscription", &arc.weight.to_string())?;
        }
        if arc.kind != ArcKind::Normal {
            xml::wrapped_text(
                &mut writer,
                "arctype",
                match arc.kind {
                    ArcKind::Inhibitor => "inhibitor",
                    ArcKind::Reset => "reset",
                    ArcKind::Normal => unreachable!(),
                },
            )?;
        }
        xml::end(&mut writer, "arc")?;
    }
    xml::end(&mut writer, "page")?;
    // An explicit empty marking must stay empty instead of being guessed on import.
    xml::start(&mut writer, "finalmarkings", &[], false)?;
    marking(&mut writer, net, &document.model.final_marking)?;
    if options.include_alternative_final_markings {
        for alternative in &document.additional_final_markings {
            marking(&mut writer, net, alternative)?;
        }
    }
    xml::end(&mut writer, "finalmarkings")?;
    if !document.variables.is_empty() {
        xml::start(&mut writer, "variables", &[], false)?;
        for variable in &document.variables {
            xml::start(
                &mut writer,
                "variable",
                &[("type", &variable.type_name)],
                false,
            )?;
            xml::text(&mut writer, "name", &variable.name)?;
            xml::end(&mut writer, "variable")?;
        }
        xml::end(&mut writer, "variables")?;
    }
    xml::end(&mut writer, "net")?;
    xml::end(&mut writer, "pnml")?;
    writer.get_mut().flush()?;
    Ok(())
}
