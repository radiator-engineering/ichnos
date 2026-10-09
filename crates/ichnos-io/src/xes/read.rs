use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::Arc,
};

use flate2::bufread::MultiGzDecoder;
use ichnos_core::{
    AttributeValue, Attributes, Classifier, Event, EventLog, Extension, SortOrder, Trace,
    chrono::{DateTime, NaiveDateTime},
};
use quick_xml::{
    Reader,
    events::{BytesStart, Event as XmlEvent},
};

use crate::{Error, Result};

/// Options for reading XES. File order is preserved by default.
#[derive(Debug, Clone)]
pub struct XesReadOptions {
    /// Sort traces and events by this timestamp key when present.
    pub sort_timestamp: Option<String>,
    /// Maximum XML element nesting, including log, trace and event.
    pub max_depth: usize,
}

impl Default for XesReadOptions {
    fn default() -> Self {
        Self {
            sort_timestamp: None,
            max_depth: 128,
        }
    }
}

enum Frame {
    Log,
    Trace(Trace),
    Event(Event),
    Global(String, Attributes),
    Attribute(Arc<str>, AttributeValue, Attributes, bool),
    Values(Vec<(Arc<str>, AttributeValue)>),
    Declaration,
    Ignored,
}

fn intern(pool: &mut HashSet<Arc<str>>, text: &str) -> Arc<str> {
    if let Some(value) = pool.get(text) {
        return value.clone();
    }
    let value: Arc<str> = text.into();
    pool.insert(value.clone());
    value
}

fn required<'a>(attrs: &'a [(String, String)], key: &str) -> Result<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| Error::Xes(format!("missing XML attribute {key}")))
}

fn classifier_keys(text: &str) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    let mut key = String::new();
    let mut quote = None;
    for c in text.chars() {
        if quote == Some(c) {
            quote = None;
        } else if quote.is_some() {
            key.push(c);
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c.is_whitespace() {
            if !key.is_empty() {
                keys.push(std::mem::take(&mut key));
            }
        } else {
            key.push(c);
        }
    }
    if quote.is_some() {
        return Err(Error::Xes("unclosed classifier key quote".into()));
    }
    if !key.is_empty() {
        keys.push(key);
    }
    Ok(keys)
}

fn start(
    element: &BytesStart<'_>,
    stack: &[Frame],
    log: &mut EventLog,
    pool: &mut HashSet<Arc<str>>,
    seen: &mut bool,
) -> Result<Frame> {
    if matches!(stack.last(), Some(Frame::Ignored)) {
        return Ok(Frame::Ignored);
    }
    let attrs = element
        .attributes()
        .map(|a| {
            let a = a.map_err(|e| Error::Xes(e.to_string()))?;
            Ok((
                a.key.as_ref().to_owned(),
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)?
                    .into_owned(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let parent = stack.last();
    let name = element.local_name();
    let tag = name.as_ref();
    let invalid = || Error::Xes(format!("unexpected <{tag}>"));
    match tag {
        "log" if parent.is_none() && !*seen => {
            *seen = true;
            Ok(Frame::Log)
        }
        "trace" if matches!(parent, Some(Frame::Log)) => Ok(Frame::Trace(Trace::new())),
        "event" if matches!(parent, Some(Frame::Trace(_))) => Ok(Frame::Event(Event::new())),
        "global" if matches!(parent, Some(Frame::Log)) => {
            let scope = required(&attrs, "scope")?;
            if scope != "trace" && scope != "event" {
                return Err(invalid());
            }
            Ok(Frame::Global(scope.into(), Attributes::new()))
        }
        "extension" if matches!(parent, Some(Frame::Log)) => {
            log.extensions.push(Extension {
                name: required(&attrs, "name")?.into(),
                prefix: required(&attrs, "prefix")?.into(),
                uri: required(&attrs, "uri")?.into(),
            });
            Ok(Frame::Declaration)
        }
        "classifier" if matches!(parent, Some(Frame::Log)) => {
            log.classifiers.push(Classifier {
                name: required(&attrs, "name")?.into(),
                keys: classifier_keys(required(&attrs, "keys")?)?,
            });
            Ok(Frame::Declaration)
        }
        "values"
            if matches!(
                parent,
                Some(Frame::Attribute(_, AttributeValue::List(_), _, _))
            ) =>
        {
            Ok(Frame::Values(Vec::new()))
        }
        "string" | "id" | "int" | "float" | "boolean" | "date" | "list" | "container" => {
            if !matches!(
                parent,
                Some(
                    Frame::Log
                        | Frame::Trace(_)
                        | Frame::Event(_)
                        | Frame::Global(..)
                        | Frame::Attribute(..)
                        | Frame::Values(_)
                )
            ) {
                return Err(invalid());
            }
            let Some((_, key)) = attrs.iter().find(|(k, _)| k == "key") else {
                // pm4py stores these under None. The core model has text keys.
                return Ok(Frame::Ignored);
            };
            let key = intern(pool, key);
            let text = if tag == "list" || tag == "container" {
                ""
            } else {
                match attrs.iter().find(|(k, _)| k == "value") {
                    Some((_, value)) => value,
                    // EventLog has no null value: omit pm4py None attributes.
                    None => return Ok(Frame::Ignored),
                }
            };
            let bad = || Error::Xes(format!("invalid {tag} value for {key}: {text}"));
            let value = match tag {
                "string" => AttributeValue::String(intern(pool, text)),
                "id" => AttributeValue::Id(intern(pool, text)),
                "int" => AttributeValue::Int(text.parse().map_err(|_| bad())?),
                "float" => AttributeValue::Float(text.parse().map_err(|_| bad())?),
                "boolean" => AttributeValue::Bool(text.eq_ignore_ascii_case("true")),
                "date" => AttributeValue::Date(
                    DateTime::parse_from_rfc3339(text)
                        .or_else(|_| {
                            NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
                                .map(|d| d.and_utc().fixed_offset())
                        })
                        .map_err(|_| bad())?,
                ),
                "list" => AttributeValue::List(Vec::new()),
                "container" => AttributeValue::Container(Attributes::new()),
                _ => unreachable!(),
            };
            Ok(Frame::Attribute(key, value, Attributes::new(), false))
        }
        "log" | "trace" | "event" | "values" => Err(invalid()),
        _ => Ok(Frame::Ignored),
    }
}

fn finish(frame: Frame, stack: &mut [Frame], log: &mut EventLog) -> Result<()> {
    match frame {
        Frame::Trace(trace) => log.traces.push(trace),
        Frame::Event(event) => {
            if let Some(Frame::Trace(trace)) = stack.last_mut() {
                trace.events.push(event);
            }
        }
        Frame::Global(scope, attrs) => {
            if scope == "trace" {
                log.globals.trace = attrs;
            } else {
                log.globals.event = attrs;
            }
        }
        Frame::Values(values) => {
            if let Some(Frame::Attribute(_, AttributeValue::List(items), meta, has_values)) =
                stack.last_mut()
            {
                if !*has_values {
                    meta.extend(std::mem::take(items));
                }
                *has_values = true;
                items.extend(values);
            }
        }
        Frame::Attribute(key, value, meta, _) => {
            let value = value.with_meta(meta);
            match stack.last_mut() {
                Some(Frame::Log) => {
                    log.attributes.insert(key, value);
                }
                Some(Frame::Trace(trace)) => {
                    trace.attributes.insert(key, value);
                }
                Some(Frame::Event(event)) => {
                    event.attributes.insert(key, value);
                }
                Some(Frame::Global(_, attrs)) => {
                    attrs.insert(key, value);
                }
                Some(Frame::Values(items)) => items.push((key, value)),
                Some(Frame::Attribute(_, AttributeValue::List(items), _, false)) => {
                    items.push((key, value))
                }
                Some(Frame::Attribute(_, AttributeValue::Container(children), _, _)) => {
                    children.insert(key, value);
                }
                Some(Frame::Attribute(_, _, meta, _)) => {
                    meta.insert(key, value);
                }
                _ => return Err(Error::Xes("attribute outside log".into())),
            }
        }
        Frame::Log | Frame::Declaration | Frame::Ignored => {}
    }
    Ok(())
}

/// Reads a plain `.xes` or gzip-compressed `.xes.gz` file.
pub fn read_xes(path: impl AsRef<Path>, options: &XesReadOptions) -> Result<EventLog> {
    let path = path.as_ref();
    let input = BufReader::new(File::open(path)?);
    if path.extension().is_some_and(|e| e == "gz") {
        read_xes_from_reader(BufReader::new(MultiGzDecoder::new(input)), options)
    } else {
        read_xes_from_reader(input, options)
    }
}

/// Reads uncompressed XES from a buffered stream without constructing an XML DOM.
pub fn read_xes_from_reader(input: impl BufRead, options: &XesReadOptions) -> Result<EventLog> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().expand_empty_elements = true;
    let mut buffer = Vec::new();
    let mut stack = Vec::new();
    let mut log = EventLog::default();
    let mut pool = HashSet::new();
    let mut seen = false;
    loop {
        match reader.read_event_into(&mut buffer)? {
            XmlEvent::Start(element) => {
                if stack.len() >= options.max_depth {
                    return Err(Error::Xes("maximum element depth exceeded".into()));
                }
                let frame = start(&element, &stack, &mut log, &mut pool, &mut seen)?;
                stack.push(frame);
            }
            XmlEvent::End(_) => {
                let frame = stack
                    .pop()
                    .ok_or_else(|| Error::Xes("unexpected closing tag".into()))?;
                finish(frame, &mut stack, &mut log)?;
            }
            XmlEvent::Eof => break,
            XmlEvent::DocType(_) => return Err(Error::Xes("DOCTYPE is not supported".into())),
            _ => {}
        }
        buffer.clear();
    }
    if !seen || !stack.is_empty() {
        return Err(Error::Xes("missing or incomplete log".into()));
    }
    if let Some(key) = &options.sort_timestamp {
        log.sort_by_timestamp(key, SortOrder::Ascending)?;
    }
    Ok(log)
}
