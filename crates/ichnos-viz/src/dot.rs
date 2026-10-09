//! A DOT writer that lays out statements as the Python `graphviz` package
//! does, which pm4py uses to build its graphs.

use std::fmt::Write as _;

/// An attribute list. `None` values are left out, as `graphviz` does.
pub(crate) type Attrs<'a> = Vec<(&'a str, Option<String>)>;

/// A DOT graph built statement by statement.
#[derive(Debug)]
pub(crate) struct Dot {
    directed: bool,
    head: String,
    body: Vec<String>,
    indent: usize,
}

impl Dot {
    /// Starts a graph. `graph_attrs` become the `graph [...]` statement that
    /// `graphviz` writes first, sorted by name.
    pub(crate) fn new(directed: bool, strict: bool, name: &str, graph_attrs: Attrs<'_>) -> Self {
        let mut head = String::new();
        if strict {
            head.push_str("strict ");
        }
        head.push_str(if directed { "digraph " } else { "graph " });
        if !name.is_empty() {
            head.push_str(&quote(name));
            head.push(' ');
        }
        head.push_str("{\n");
        let mut dot = Dot {
            directed,
            head,
            body: Vec::new(),
            indent: 1,
        };
        let list = attr_list(None, graph_attrs);
        if !list.is_empty() {
            dot.line(format!("graph [{list}]"));
        }
        dot
    }

    fn line(&mut self, text: String) {
        self.body
            .push(format!("{}{text}\n", "\t".repeat(self.indent)));
    }

    /// Sets graph attributes on one line (`graphviz`'s `attr()` with no
    /// keyword).
    pub(crate) fn set(&mut self, attrs: Attrs<'_>) {
        let list = attr_list(None, attrs);
        if !list.is_empty() {
            self.line(list);
        }
    }

    /// Sets default attributes for `kind`: `node`, `edge` or `graph`.
    pub(crate) fn defaults(&mut self, kind: &str, attrs: Attrs<'_>) {
        let list = attr_list(None, attrs);
        self.line(format!("{kind} [{list}]"));
    }

    /// Adds a node.
    pub(crate) fn node(&mut self, id: &str, label: Option<&str>, attrs: Attrs<'_>) {
        let list = attr_list(label, attrs);
        if list.is_empty() {
            self.line(quote(id));
        } else {
            self.line(format!("{} [{list}]", quote(id)));
        }
    }

    /// Adds an edge. The label goes first; the other attributes are sorted.
    pub(crate) fn edge(&mut self, tail: &str, head: &str, label: Option<&str>, attrs: Attrs<'_>) {
        let op = if self.directed { "->" } else { "--" };
        let list = attr_list(label, attrs);
        if list.is_empty() {
            self.line(format!("{} {op} {}", quote(tail), quote(head)));
        } else {
            self.line(format!("{} {op} {} [{list}]", quote(tail), quote(head)));
        }
    }

    /// Adds a subgraph with the statements `build` writes.
    pub(crate) fn subgraph(&mut self, name: &str, build: impl FnOnce(&mut Dot)) {
        self.line(format!("subgraph {} {{", quote(name)));
        self.indent += 1;
        build(self);
        self.indent -= 1;
        self.line("}".to_owned());
    }

    /// The DOT text.
    pub(crate) fn finish(self) -> String {
        let mut out = self.head;
        for line in self.body {
            out.push_str(&line);
        }
        out.push_str("}\n");
        out
    }
}

/// `label=...` first, then the other attributes sorted by name.
fn attr_list(label: Option<&str>, mut attrs: Attrs<'_>) -> String {
    let mut out = String::new();
    if let Some(label) = label {
        let _ = write!(out, "label={}", quote(label));
    }
    attrs.sort_by(|a, b| a.0.cmp(b.0));
    for (key, value) in attrs {
        if let Some(value) = value {
            if !out.is_empty() {
                out.push(' ');
            }
            let _ = write!(out, "{}={}", quote(key), quote(&value));
        }
    }
    out
}

/// Quotes a DOT identifier when needed, as `graphviz.quoting.quote` does.
///
/// An HTML string (`<...>`), a plain ASCII name or a number that is not a
/// keyword stays as it is. Anything else is quoted, with each `"` that is
/// not already escaped escaped.
pub(crate) fn quote(id: &str) -> String {
    if id.starts_with('<') && id.ends_with('>') {
        return id.to_owned();
    }
    let keyword = ["node", "edge", "graph", "digraph", "subgraph", "strict"]
        .iter()
        .any(|k| id.eq_ignore_ascii_case(k));
    if (is_name(id) || is_numeral(id)) && !keyword {
        return id.to_owned();
    }
    let mut out = String::with_capacity(id.len() + 2);
    out.push('"');
    let mut backslashes = 0;
    for c in id.chars() {
        if c == '"' && backslashes % 2 == 0 {
            out.push('\\');
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
        out.push(c);
    }
    out.push('"');
    out
}

fn is_name(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_numeral(id: &str) -> bool {
    let digits = id.strip_prefix('-').unwrap_or(id);
    let (int, frac) = match digits.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (digits, None),
    };
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    match frac {
        None => !int.is_empty() && all_digits(int),
        Some(frac) => all_digits(int) && all_digits(frac) && !(int.is_empty() && frac.is_empty()),
    }
}

/// The title label pm4py's Graphviz visualizers set: the title in an HTML
/// font twice the base size.
pub(crate) fn title_label(title: &str, font_size: u32) -> String {
    format!(
        "<<FONT POINT-SIZE=\"{}\">{title}</FONT>>",
        2 * u64::from(font_size)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_like_the_graphviz_package() {
        assert_eq!(quote("spam"), "spam");
        assert_eq!(quote("spam spam"), "\"spam spam\"");
        assert_eq!(quote("-4.2"), "-4.2");
        assert_eq!(quote(".42"), ".42");
        assert_eq!(quote("4."), "4.");
        assert_eq!(quote("."), "\".\"");
        assert_eq!(quote(""), "\"\"");
        assert_eq!(quote("Node"), "\"Node\"");
        assert_eq!(quote("<<b>spam</b>>"), "<<b>spam</b>>");
        assert_eq!(quote("\""), "\"\\\"\"");
        assert_eq!(quote("\\\""), "\"\\\"\"");
        assert_eq!(quote("\\\\\""), "\"\\\\\\\"\"");
        assert_eq!(quote("#FFFFFF"), "\"#FFFFFF\"");
        assert_eq!(quote("café"), "\"café\"");
    }

    #[test]
    fn writes_statements_like_the_graphviz_package() {
        let mut dot = Dot::new(
            true,
            false,
            "",
            vec![
                ("rankdir", Some("LR".into())),
                ("bgcolor", Some("white".into())),
            ],
        );
        dot.defaults("node", vec![("shape", Some("box".into()))]);
        dot.node("a", Some("A b"), vec![("z", Some("1".into())), ("c", None)]);
        dot.subgraph("cluster_x", |c| {
            c.set(vec![("label", Some("pool".into()))]);
            c.node("b", None, vec![]);
        });
        dot.edge("a", "b", Some(""), vec![]);
        assert_eq!(
            dot.finish(),
            "digraph {\n\tgraph [bgcolor=white rankdir=LR]\n\tnode [shape=box]\n\
             \ta [label=\"A b\" z=1]\n\tsubgraph cluster_x {\n\t\tlabel=pool\n\t\tb\n\t}\n\
             \ta -> b [label=\"\"]\n}\n"
        );
    }
}
