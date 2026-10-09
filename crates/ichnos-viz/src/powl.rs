//! POWL models, ported from pm4py's `visualization/powl/variants/basic.py`
//! and the SVG step of `visualization/powl/visualizer.py`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ichnos_model::Powl;

use crate::dot::{Attrs, Dot};
use crate::style::py_float;
use crate::write::{VizError, format_of, render};

/// The icons pm4py's POWL drawings use, by file name: the choice and loop
/// operators and the tags of skippable and repeating activities.
pub const POWL_ICONS: [(&str, &str); 5] = [
    ("loop.svg", include_str!("../icons/loop.svg")),
    ("xor.svg", include_str!("../icons/xor.svg")),
    ("skip-tag.svg", include_str!("../icons/skip-tag.svg")),
    ("loop-tag.svg", include_str!("../icons/loop-tag.svg")),
    (
        "skip-loop-tag.svg",
        include_str!("../icons/skip-loop-tag.svg"),
    ),
];

/// Writes the [`POWL_ICONS`] into `dir`, which must exist, so that a saved
/// `.dot` file can find them.
pub fn write_powl_icons(dir: impl AsRef<Path>) -> std::io::Result<()> {
    for (name, svg) in POWL_ICONS {
        std::fs::write(dir.as_ref().join(name), svg)?;
    }
    Ok(())
}

/// Options for [`powl_dot`], with the defaults of pm4py's `save_vis_powl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowlDotOptions {
    /// The background colour. Nested blocks get darker shades of it.
    /// Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `TB`.
    pub rankdir: String,
    /// Draw a choice or loop between an activity and a silent step as one
    /// tagged activity ([`Powl::simplify_using_frequent_transitions`]).
    /// Default `true`.
    pub frequency_tags: bool,
    /// The directory the icon paths point into. `None` writes bare file
    /// names, which Graphviz looks up in its working directory.
    pub icon_dir: Option<PathBuf>,
}

impl Default for PowlDotOptions {
    fn default() -> Self {
        PowlDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "TB".to_owned(),
            frequency_tags: true,
            icon_dir: None,
        }
    }
}

/// The DOT text of a POWL model, as pm4py's `save_vis_powl` draws it.
///
/// Activities are boxes and silent steps small black squares. A partial
/// order is a cluster with an edge for each pair of its transitive
/// reduction. A choice or loop is a cluster around an operator icon with an
/// edge to each child; the loop's redo edge is dashed. Each level of
/// nesting darkens the fill colour by 2%, up to 30%. Tagged activities
/// carry an icon in their top right corner. Icons are image files (see
/// [`POWL_ICONS`] and [`PowlDotOptions::icon_dir`]).
///
/// pm4py turns a choice or loop between two silent steps into an activity
/// labelled `None`; ichnos keeps and draws it. A partial order whose order
/// is cyclic draws its order as it is.
pub fn powl_dot(powl: &Powl, options: &PowlDotOptions) -> String {
    let simplified;
    let powl = if options.frequency_tags {
        simplified = powl.simplify_using_frequent_transitions();
        &simplified
    } else {
        powl
    };
    let mut dot = Dot::new(true, false, "powl", vec![]);
    let s = |v: &str| Some(v.to_owned());
    dot.defaults(
        "node",
        vec![("shape", s("ellipse")), ("fixedsize", s("false"))],
    );
    dot.set(vec![("nodesep", s("1"))]);
    dot.set(vec![("ranksep", s("1"))]);
    dot.set(vec![("compound", s("true"))]);
    dot.set(vec![("overlap", s("scale"))]);
    dot.set(vec![("splines", s("true"))]);
    dot.set(vec![("rankdir", Some(options.rankdir.clone()))]);
    dot.set(vec![("style", s("filled"))]);
    dot.set(vec![("fillcolor", Some(options.bgcolor.clone()))]);
    let mut drawer = Drawer { options, next: 0 };
    drawer.draw(&mut dot, powl, 0);
    dot.finish()
}

/// What a drawn element is: an edge to it goes to `base`, the first node
/// inside it, and clipped at `cluster` when it is one.
struct Drawn {
    base: Option<String>,
    cluster: Option<String>,
}

struct Drawer<'a> {
    options: &'a PowlDotOptions,
    next: usize,
}

impl Drawer<'_> {
    fn icon(&self, name: &str) -> String {
        match &self.options.icon_dir {
            Some(dir) => dir.join(name).display().to_string(),
            None => name.to_owned(),
        }
    }

    /// pm4py's `repr_powl`.
    fn draw(&mut self, dot: &mut Dot, powl: &Powl, level: u32) -> Drawn {
        let id = format!("n{}", self.next);
        self.next += 1;
        let color = darken_color(&self.options.bgcolor, (0.02 * f64::from(level)).min(0.3));
        let s = |v: &str| Some(v.to_owned());
        let box_attrs = |extra: Attrs<'static>| -> Attrs<'static> {
            let mut attrs = vec![
                ("shape", s("box")),
                ("width", s("1.5")),
                ("fontsize", s("18")),
                ("style", s("filled")),
                ("fillcolor", Some(color.clone())),
            ];
            attrs.extend(extra);
            attrs
        };
        let transition = |id: &String| Drawn {
            base: Some(id.clone()),
            cluster: None,
        };
        match powl {
            Powl::Frequent(ft) => {
                let tag = match (ft.skippable, ft.selfloop) {
                    (true, true) => Some("skip-loop-tag.svg"),
                    (true, false) => Some("skip-tag.svg"),
                    (false, true) => Some("loop-tag.svg"),
                    (false, false) => None,
                };
                match tag {
                    Some(tag) => dot.node(
                        &id,
                        Some(&format!("\n{}", ft.activity)),
                        box_attrs(vec![("imagepos", s("tr")), ("image", Some(self.icon(tag)))]),
                    ),
                    None => dot.node(&id, Some(&ft.activity), box_attrs(vec![])),
                }
                transition(&id)
            }
            Powl::Silent => {
                dot.node(
                    &id,
                    Some(""),
                    vec![
                        ("style", s("filled")),
                        ("fillcolor", s("black")),
                        ("shape", s("square")),
                        ("width", s("0.3")),
                        ("height", s("0.3")),
                        ("fixedsize", s("true")),
                    ],
                );
                transition(&id)
            }
            Powl::Activity(label) => {
                dot.node(&id, Some(label), box_attrs(vec![]));
                transition(&id)
            }
            Powl::PartialOrder(po) => {
                let cluster = format!("cluster_{id}");
                let order = po.order();
                let reduction = order
                    .transitive_reduction()
                    .unwrap_or_else(|_| order.clone());
                let mut children = Vec::new();
                dot.subgraph(&cluster, |block| {
                    block_attrs(block, &color);
                    for child in po.children() {
                        children.push(self.draw(block, child, level + 1));
                    }
                    for (i, a) in children.iter().enumerate() {
                        for (j, b) in children.iter().enumerate() {
                            if reduction.is_edge(i, j) {
                                order_edge(block, a, b);
                            }
                        }
                    }
                });
                Drawn {
                    base: children.first().and_then(|c| c.base.clone()),
                    cluster: Some(cluster),
                }
            }
            Powl::Xor(_) | Powl::Loop(_) => {
                let cluster = format!("cluster_{id}");
                let looped = matches!(powl, Powl::Loop(_));
                let icon = self.icon(if looped { "loop.svg" } else { "xor.svg" });
                dot.subgraph(&cluster, |block| {
                    block_attrs(block, &color);
                    block.node(
                        &id,
                        Some(""),
                        vec![
                            ("image", Some(icon)),
                            ("fontsize", s("18")),
                            ("width", s("0.4")),
                            ("height", s("0.4")),
                            ("fixedsize", s("true")),
                        ],
                    );
                    for (i, child) in powl.children().iter().enumerate() {
                        let drawn = self.draw(block, child, level + 1);
                        let style = if looped && i == 1 { "dashed" } else { "" };
                        operator_edge(block, &id, &drawn, style);
                    }
                });
                Drawn {
                    base: Some(id),
                    cluster: Some(cluster),
                }
            }
        }
    }
}

fn block_attrs(block: &mut Dot, color: &str) {
    block.set(vec![("margin", Some("20,20".to_owned()))]);
    block.set(vec![("style", Some("filled".to_owned()))]);
    block.set(vec![("fillcolor", Some(color.to_owned()))]);
}

/// pm4py's `add_operator_edge`. An empty partial order has no node to point
/// at, so it gets no edge; pm4py fails on it.
fn operator_edge(dot: &mut Dot, from: &str, child: &Drawn, style: &str) {
    let Some(base) = &child.base else { return };
    let mut attrs = vec![
        ("dir", Some("none".to_owned())),
        ("style", Some(style.to_owned())),
    ];
    if let Some(cluster) = &child.cluster {
        attrs.push(("lhead", Some(cluster.clone())));
        attrs.push(("minlen", Some("2".to_owned())));
    }
    dot.edge(from, base, None, attrs);
}

/// pm4py's `add_order_edge`.
fn order_edge(dot: &mut Dot, a: &Drawn, b: &Drawn) {
    let (Some(tail), Some(head)) = (&a.base, &b.base) else {
        return;
    };
    let mut attrs = vec![
        ("dir", Some("forward".to_owned())),
        ("color", Some("black".to_owned())),
        ("style", Some(String::new())),
        ("ltail", a.cluster.clone()),
        ("lhead", b.cluster.clone()),
    ];
    if a.cluster.is_some() || b.cluster.is_some() {
        attrs.push(("minlen", Some("2".to_owned())));
    }
    dot.edge(tail, head, None, attrs);
}

/// pm4py's `darken_color`: scales each RGB channel by `1 - amount` and
/// writes the colour as `#rrggbb`, as matplotlib's `to_rgb` and `to_hex`
/// do.
///
/// Hex colours (`#rgb`, `#rrggbb`, `#rrggbbaa`), `white` and `black` are
/// read. Other colour names are returned unchanged.
fn darken_color(color: &str, amount: f64) -> String {
    let Some(rgb) = parse_color(color) else {
        return color.to_owned();
    };
    let mut out = String::from("#");
    for channel in rgb {
        let value = channel * (1.0 - amount);
        out.push_str(&format!("{:02x}", (value * 255.0).round_ties_even() as u8));
    }
    out
}

fn parse_color(color: &str) -> Option<[f64; 3]> {
    match color.to_ascii_lowercase().as_str() {
        "white" | "w" => return Some([1.0; 3]),
        "black" | "k" => return Some([0.0; 3]),
        _ => {}
    }
    let hex = color.strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let digits: Vec<u8> = match hex.len() {
        3 | 4 => hex.bytes().take(3).flat_map(|b| [b, b]).collect(),
        6 | 8 => hex.bytes().take(6).collect(),
        _ => return None,
    };
    let channel = |i: usize| {
        let pair = std::str::from_utf8(&digits[2 * i..2 * i + 2]).expect("ASCII hex digits");
        f64::from(u8::from_str_radix(pair, 16).expect("hex digits")) / 255.0
    };
    Some([channel(0), channel(1), channel(2)])
}

/// Saves a POWL drawing ([`powl_dot`]) to `path`, in the format its
/// extension names. pm4py's `save_vis_powl`.
///
/// `.dot` and `.gv` get the DOT text, with icon paths from
/// [`PowlDotOptions::icon_dir`]. Other formats run Graphviz `dot` with the
/// icons in a temporary directory, so they need `dot` on the `PATH`
/// ([`VizError::DotNotFound`]). In `.svg` output the icons are inlined, as
/// pm4py does, so the file stands alone.
pub fn write_powl(
    powl: &Powl,
    options: &PowlDotOptions,
    path: impl AsRef<Path>,
) -> Result<(), VizError> {
    write_powl_with(powl, options, path.as_ref(), "dot")
}

fn write_powl_with(
    powl: &Powl,
    options: &PowlDotOptions,
    path: &Path,
    program: &str,
) -> Result<(), VizError> {
    let format = format_of(path)?;
    if format == "dot" || format == "gv" {
        std::fs::write(path, powl_dot(powl, options))?;
        return Ok(());
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ichnos-viz-powl-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir)?;
    let rendered = write_powl_icons(&dir)
        .map_err(VizError::from)
        .and_then(|()| {
            let options = PowlDotOptions {
                icon_dir: Some(dir.clone()),
                ..options.clone()
            };
            render(&powl_dot(powl, &options), &format, program, &[])
        });
    let _ = std::fs::remove_dir_all(&dir);
    let mut output = rendered?;
    if format == "svg" {
        output = inline_icons(&String::from_utf8_lossy(&output)).into_bytes();
    }
    std::fs::write(path, output)?;
    Ok(())
}

/// pm4py's `inline_images_and_svgs`: replaces each `<image>` of one of the
/// [`POWL_ICONS`] with the icon's SVG content, moved and scaled to the
/// image's box. Other images stay as they are.
fn inline_icons(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(start) = rest.find("<image") {
        out.push_str(&rest[..start]);
        let Some(len) = rest[start..].find('>') else {
            rest = &rest[start..];
            break;
        };
        let tag = &rest[start..=start + len];
        out.push_str(&inline_icon(tag).unwrap_or_else(|| tag.to_owned()));
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    out
}

fn inline_icon(tag: &str) -> Option<String> {
    let attr = |name: &str| {
        let key = format!(" {name}=\"");
        let start = tag.find(&key)? + key.len();
        let len = tag[start..].find('"')?;
        Some(&tag[start..start + len])
    };
    let href = attr("xlink:href")?;
    let name = Path::new(href).file_name()?.to_str()?;
    let icon = POWL_ICONS.iter().find(|(n, _)| *n == name)?.1;
    let number = |name: &str| attr(name)?.trim_end_matches("px").parse::<f64>().ok();
    let (width, height, x, y) = (
        number("width")?,
        number("height")?,
        number("x")?,
        number("y")?,
    );
    let (content, view_box) = icon_content(icon);
    let scale_x = width / view_box[2];
    let scale_y = height / view_box[3];
    Some(format!(
        "<g transform=\"translate({},{}) scale({},{})\">{content}</g>",
        py_float(x),
        py_float(y),
        py_float(scale_x),
        py_float(scale_y)
    ))
}

/// An icon's markup inside its `<svg>` element, and its `viewBox`.
fn icon_content(icon: &str) -> (&str, [f64; 4]) {
    let view_box = icon
        .split_once("viewBox=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(v, _)| v)
        .unwrap_or("0 0 1 1");
    let mut values = [0.0, 0.0, 1.0, 1.0];
    for (slot, v) in values.iter_mut().zip(view_box.split_whitespace()) {
        *slot = v.parse().unwrap_or(*slot);
    }
    let body = icon
        .find("<svg")
        .and_then(|start| {
            let open = start + icon[start..].find('>')? + 1;
            let close = open + icon[open..].find("</svg>")?;
            Some(&icon[open..close])
        })
        .unwrap_or(icon);
    (body, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darkening_matches_matplotlib() {
        // matplotlib: to_hex([x * (1 - 0.02) for x in to_rgb("#ddeeff")]).
        assert_eq!(darken_color("#ddeeff", 0.02), "#d9e9fa");
        assert_eq!(darken_color("#ddeeff", 0.04), "#d4e4f5");
        assert_eq!(darken_color("white", 0.0), "#ffffff");
        assert_eq!(darken_color("white", 0.3), "#b2b2b2");
        assert_eq!(darken_color("#FFF", 0.0), "#ffffff");
        assert_eq!(darken_color("lightblue", 0.1), "lightblue");
    }

    #[test]
    fn icons_are_inlined_into_svg() {
        let svg = "<svg><g><image xlink:href=\"/tmp/x/loop.svg\" width=\"29px\" \
                   height=\"29px\" preserveAspectRatio=\"xMinYMin meet\" x=\"10.5\" \
                   y=\"-40\"/></g><image xlink:href=\"other.png\" width=\"1px\"/></svg>";
        let out = inline_icons(svg);
        let scale = py_float(29.0 / 1696.0);
        assert!(
            out.starts_with(&format!(
                "<svg><g><g transform=\"translate(10.5,-40.0) scale({scale},{scale})\">"
            )),
            "{out}"
        );
        assert!(out.contains("<path d="), "{out}");
        assert!(!out.contains("loop.svg"), "{out}");
        assert!(out.ends_with("</g></g><image xlink:href=\"other.png\" width=\"1px\"/></svg>"));
    }

    #[test]
    fn rendering_without_dot_is_a_typed_error() {
        let path = std::env::temp_dir().join(format!("ichnos-viz-powl-{}.svg", std::process::id()));
        let err = write_powl_with(
            &Powl::activity("a"),
            &PowlDotOptions::default(),
            &path,
            "ichnos-no-such-dot",
        )
        .unwrap_err();
        assert!(
            matches!(err, VizError::DotNotFound(ref f) if f == "svg"),
            "{err}"
        );
    }
}
