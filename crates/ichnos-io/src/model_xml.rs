use crate::{Error, Result};
use quick_xml::{
    Reader, Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use std::{
    collections::BTreeMap,
    io::{BufRead, Write},
};

pub(crate) fn invalid(format: &'static str, detail: impl Into<String>) -> Error {
    Error::ModelFormat {
        format,
        detail: detail.into(),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Element {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub text: String,
    pub children: Vec<Element>,
}

impl Element {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }
    pub fn required(&self, key: &str, format: &'static str) -> Result<&str> {
        self.attr(key)
            .ok_or_else(|| invalid(format, format!("<{}> lacks {key}", self.name)))
    }
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.name == name)
    }
    pub fn child_text(&self, name: &str) -> Option<&str> {
        self.child(name)
            .map(|child| child.child("text").unwrap_or(child).text.as_str())
    }
}

pub(crate) fn read(
    input: impl BufRead,
    format: &'static str,
    max_depth: usize,
    max_nodes: usize,
) -> Result<Element> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().expand_empty_elements = true;
    let mut buffer = Vec::new();
    let mut stack: Vec<Element> = Vec::new();
    let mut root = None;
    let mut nodes = 0;
    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(tag) => {
                nodes += 1;
                if stack.len() >= max_depth || nodes > max_nodes {
                    return Err(invalid(format, "XML size/depth limit exceeded"));
                }
                if stack.is_empty() && root.is_some() {
                    return Err(invalid(format, "multiple XML roots"));
                }
                let attrs = tag
                    .attributes()
                    .map(|attr| {
                        let attr = attr.map_err(|e| invalid(format, e.to_string()))?;
                        Ok((
                            attr.key.as_ref().to_owned(),
                            attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)?
                                .into_owned(),
                        ))
                    })
                    .collect::<Result<_>>()?;
                stack.push(Element {
                    name: tag.local_name().as_ref().to_owned(),
                    attrs,
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            Event::End(_) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| invalid(format, "unexpected closing tag"))?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(element);
                } else {
                    root = Some(element);
                }
            }
            Event::Text(text) => {
                if let Some(element) = stack.last_mut() {
                    element.text.push_str(text.as_ref());
                } else if !text.as_ref().trim().is_empty() {
                    return Err(invalid(format, "text outside root"));
                }
            }
            Event::CData(text) => {
                let element = stack
                    .last_mut()
                    .ok_or_else(|| invalid(format, "CDATA outside root"))?;
                element.text.push_str(text.as_ref());
            }
            Event::GeneralRef(reference) => {
                let encoded = format!("&{};", reference.as_ref());
                let decoded = quick_xml::escape::unescape(&encoded)
                    .map_err(|e| invalid(format, e.to_string()))?;
                stack
                    .last_mut()
                    .ok_or_else(|| invalid(format, "reference outside root"))?
                    .text
                    .push_str(&decoded);
            }
            Event::DocType(_) => return Err(invalid(format, "DOCTYPE is not supported")),
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if !stack.is_empty() {
        return Err(invalid(format, "incomplete XML document"));
    }
    root.ok_or_else(|| invalid(format, "missing XML root"))
}

pub(crate) fn writer(output: impl Write, indent: bool) -> Result<Writer<impl Write>> {
    let mut writer = if indent {
        Writer::new_with_indent(output, b' ', 2)
    } else {
        Writer::new(output)
    };
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    Ok(writer)
}

pub(crate) fn start(
    writer: &mut Writer<impl Write>,
    tag: &str,
    attrs: &[(&str, &str)],
    empty: bool,
) -> Result<()> {
    let mut element = BytesStart::new(tag);
    for &(key, value) in attrs {
        element.push_attribute((key, value));
    }
    writer.write_event(if empty {
        Event::Empty(element)
    } else {
        Event::Start(element)
    })?;
    Ok(())
}

pub(crate) fn end(writer: &mut Writer<impl Write>, tag: &str) -> Result<()> {
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

pub(crate) fn text(writer: &mut Writer<impl Write>, tag: &str, value: &str) -> Result<()> {
    start(writer, tag, &[], false)?;
    writer.write_event(Event::Text(BytesText::new(value)))?;
    end(writer, tag)
}

pub(crate) fn wrapped_text(writer: &mut Writer<impl Write>, tag: &str, value: &str) -> Result<()> {
    start(writer, tag, &[], false)?;
    text(writer, "text", value)?;
    end(writer, tag)
}
