use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use flate2::{Compression, write::GzEncoder};
use ichnos_core::{AttributeValue, Attributes, EventLog};
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, Event},
};

use crate::{Error, Result};

/// Options for writing XES.
#[derive(Debug, Clone)]
pub struct XesWriteOptions {
    /// Indent the XML for human readers.
    pub indent: bool,
    /// Maximum nested attribute depth.
    pub max_depth: usize,
}

impl Default for XesWriteOptions {
    fn default() -> Self {
        Self {
            indent: true,
            max_depth: 128,
        }
    }
}

fn element(
    writer: &mut Writer<impl Write>,
    name: &str,
    attrs: &[(&str, &str)],
    empty: bool,
) -> Result<()> {
    let mut tag = BytesStart::new(name);
    for &(key, value) in attrs {
        tag.push_attribute((key, value));
    }
    writer.write_event(if empty {
        Event::Empty(tag)
    } else {
        Event::Start(tag)
    })?;
    Ok(())
}

fn end(writer: &mut Writer<impl Write>, tag: &str) -> Result<()> {
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

fn attributes(
    writer: &mut Writer<impl Write>,
    attrs: &Attributes,
    depth: usize,
    options: &XesWriteOptions,
) -> Result<()> {
    for (key, value) in attrs {
        attribute(writer, key, value, depth, options)?;
    }
    Ok(())
}

fn attribute(
    writer: &mut Writer<impl Write>,
    key: &str,
    value: &AttributeValue,
    depth: usize,
    options: &XesWriteOptions,
) -> Result<()> {
    if depth >= options.max_depth {
        return Err(Error::Xes("maximum attribute depth exceeded".into()));
    }
    let plain = value.plain();
    let tag = plain.type_name();
    let text = match plain {
        AttributeValue::Date(d) => d.to_rfc3339(),
        AttributeValue::Bool(v) => v.to_string(),
        AttributeValue::List(_) | AttributeValue::Container(_) => String::new(),
        _ => plain.to_string(),
    };
    let complex = matches!(
        plain,
        AttributeValue::List(_) | AttributeValue::Container(_)
    ) || value.meta().is_some();
    let mut attrs = vec![("key", key)];
    if !matches!(
        plain,
        AttributeValue::List(_) | AttributeValue::Container(_)
    ) {
        attrs.push(("value", &text));
    }
    element(writer, tag, &attrs, !complex)?;
    if complex {
        if let Some(meta) = value.meta() {
            attributes(writer, meta, depth + 1, options)?;
        }
        match plain {
            AttributeValue::List(items) => {
                element(writer, "values", &[], false)?;
                for (key, value) in items {
                    attribute(writer, key, value, depth + 1, options)?;
                }
                end(writer, "values")?;
            }
            AttributeValue::Container(children) => {
                attributes(writer, children, depth + 1, options)?
            }
            _ => {}
        }
        end(writer, tag)?;
    }
    Ok(())
}

/// Writes a log to XES; a `.gz` extension selects gzip compression.
pub fn write_xes(log: &EventLog, path: impl AsRef<Path>, options: &XesWriteOptions) -> Result<()> {
    let path = path.as_ref();
    let mut output = BufWriter::new(File::create(path)?);
    if path.extension().is_some_and(|e| e == "gz") {
        let mut encoder = GzEncoder::new(&mut output, Compression::default());
        write_xes_to_writer(log, &mut encoder, options)?;
        encoder.finish()?;
    } else {
        write_xes_to_writer(log, &mut output, options)?;
    }
    output.flush()?;
    Ok(())
}

/// Writes uncompressed XES, retaining typed attributes and log metadata.
pub fn write_xes_to_writer(
    log: &EventLog,
    output: impl Write,
    options: &XesWriteOptions,
) -> Result<()> {
    let mut writer = if options.indent {
        Writer::new_with_indent(output, b' ', 2)
    } else {
        Writer::new(output)
    };
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    element(
        &mut writer,
        "log",
        &[
            ("xes.version", "2.0"),
            ("xes.features", "nested-attributes"),
            ("xmlns", "http://www.xes-standard.org/"),
        ],
        false,
    )?;
    for ext in &log.extensions {
        element(
            &mut writer,
            "extension",
            &[
                ("name", &ext.name),
                ("prefix", &ext.prefix),
                ("uri", &ext.uri),
            ],
            true,
        )?;
    }
    for (scope, attrs) in [("trace", &log.globals.trace), ("event", &log.globals.event)] {
        if !attrs.is_empty() {
            element(&mut writer, "global", &[("scope", scope)], false)?;
            attributes(&mut writer, attrs, 0, options)?;
            end(&mut writer, "global")?;
        }
    }
    for classifier in &log.classifiers {
        let keys = classifier
            .keys
            .iter()
            .map(|key| {
                if key.chars().any(char::is_whitespace) || key.contains(['\'', '"']) {
                    if key.contains('\'') {
                        if key.contains('"') {
                            return Err(Error::Xes(
                                "classifier key contains both quote styles".into(),
                            ));
                        }
                        Ok(format!("\"{key}\""))
                    } else {
                        Ok(format!("'{key}'"))
                    }
                } else {
                    Ok(key.clone())
                }
            })
            .collect::<Result<Vec<_>>>()?
            .join(" ");
        element(
            &mut writer,
            "classifier",
            &[("name", &classifier.name), ("keys", &keys)],
            true,
        )?;
    }
    attributes(&mut writer, &log.attributes, 0, options)?;
    for trace in &log.traces {
        element(&mut writer, "trace", &[], false)?;
        attributes(&mut writer, &trace.attributes, 0, options)?;
        for event in &trace.events {
            element(&mut writer, "event", &[], false)?;
            attributes(&mut writer, &event.attributes, 0, options)?;
            end(&mut writer, "event")?;
        }
        end(&mut writer, "trace")?;
    }
    end(&mut writer, "log")?;
    writer.get_mut().flush()?;
    Ok(())
}
