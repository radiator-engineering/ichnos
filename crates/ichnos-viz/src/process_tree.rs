//! Process trees, ported from pm4py's
//! `visualization/process_tree/variants/wo_decoration.py`.

use ichnos_model::{Operator, ProcessTree};

use crate::dot::{Dot, title_label};

/// Options for [`process_tree_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTreeDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// The font size. Default 15.
    pub font_size: u32,
    /// Order the children of choice and parallel nodes as pm4py's
    /// `tree_sort` does (pm4py's `enable_deepcopy`). Default `true`.
    pub sort: bool,
}

impl Default for ProcessTreeDotOptions {
    fn default() -> Self {
        ProcessTreeDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            font_size: 15,
            sort: true,
        }
    }
}

/// The DOT text of a process tree, as pm4py's `save_vis_process_tree` draws
/// it.
///
/// The graph is undirected. Operators show pm4py's ProM names (`seq`,
/// `xor`, `and`, `xor loop`, `or`, `interleaving`), activities their
/// label, and silent leaves are black points.
pub fn process_tree_dot(tree: &ProcessTree, options: &ProcessTreeDotOptions) -> String {
    let mut dot = Dot::new(
        false,
        false,
        "pt",
        vec![
            ("bgcolor", Some(options.bgcolor.clone())),
            ("rankdir", Some(options.rankdir.clone())),
        ],
    );
    dot.defaults(
        "node",
        vec![
            ("shape", Some("ellipse".to_owned())),
            ("fixedsize", Some("false".to_owned())),
        ],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, options.font_size))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }
    let sorted;
    let tree = if options.sort {
        sorted = tree_sort(tree).0;
        &sorted
    } else {
        tree
    };
    let mut next = 0;
    draw(
        &mut dot,
        tree,
        None,
        &options.font_size.to_string(),
        &mut next,
    );
    dot.set(vec![("overlap", Some("false".to_owned()))]);
    dot.set(vec![("splines", Some("false".to_owned()))]);
    dot.finish()
}

/// pm4py's `operators_mapping`.
fn operator_name(op: Operator) -> &'static str {
    match op {
        Operator::Sequence => "seq",
        Operator::Xor => "xor",
        Operator::Parallel => "and",
        Operator::Loop => "xor loop",
        Operator::Or => "or",
        Operator::Interleaving => "interleaving",
    }
}

/// pm4py's `repr_tree_2`: the node, its subtrees, then the edge from its
/// parent.
fn draw(
    dot: &mut Dot,
    tree: &ProcessTree,
    parent: Option<&str>,
    font_size: &str,
    next: &mut usize,
) {
    let id = format!("n{next}");
    *next += 1;
    let s = |v: &str| Some(v.to_owned());
    match tree {
        ProcessTree::Tau => dot.node(
            &id,
            Some("tau"),
            vec![
                ("style", s("filled")),
                ("fillcolor", s("black")),
                ("shape", s("point")),
                ("width", s("0.075")),
                ("fontsize", s(font_size)),
            ],
        ),
        ProcessTree::Activity(label) => dot.node(
            &id,
            Some(label),
            vec![
                ("color", s("black")),
                ("fontcolor", s("black")),
                ("fontsize", s(font_size)),
            ],
        ),
        ProcessTree::Node(op, children) => {
            dot.node(
                &id,
                Some(operator_name(*op)),
                vec![
                    ("color", s("black")),
                    ("fontcolor", s("black")),
                    ("fontsize", s(font_size)),
                ],
            );
            for child in children {
                draw(dot, child, Some(&id), font_size, next);
            }
        }
    }
    if let Some(parent) = parent {
        dot.edge(parent, &id, None, vec![("dirType", s("none"))]);
    }
}

/// The sum of the MD5 values of a subtree's labels, a Python integer that
/// can exceed 128 bits: `(carry, low)`.
type HashSum = (u64, u128);

fn add(a: HashSum, b: HashSum) -> HashSum {
    let (low, carry) = a.1.overflowing_add(b.1);
    (a.0 + b.0 + u64::from(carry), low)
}

/// pm4py's `tree_sort`: orders the children of choice and parallel nodes by
/// the sum of the MD5 values of their labels, keeping ties in place.
fn tree_sort(tree: &ProcessTree) -> (ProcessTree, HashSum) {
    match tree {
        ProcessTree::Tau => (ProcessTree::Tau, (0, 0)),
        ProcessTree::Activity(label) => (
            tree.clone(),
            (0, u128::from_be_bytes(md5(label.as_bytes()))),
        ),
        ProcessTree::Node(op, children) => {
            let mut sorted: Vec<(ProcessTree, HashSum)> = children.iter().map(tree_sort).collect();
            let sum = sorted.iter().fold((0, 0), |acc, (_, h)| add(acc, *h));
            if matches!(op, Operator::Parallel | Operator::Xor) {
                sorted.sort_by_key(|(_, h)| *h);
            }
            (
                ProcessTree::Node(*op, sorted.into_iter().map(|(t, _)| t).collect()),
                sum,
            )
        }
    }
}

/// The MD5 digest (RFC 1321).
pub(crate) fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut state: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&((input.len() as u64).wrapping_mul(8)).to_le_bytes());
    for chunk in message.chunks_exact(64) {
        let m: [u32; 16] = std::array::from_fn(|i| {
            u32::from_le_bytes(chunk[4 * i..4 * i + 4].try_into().expect("four bytes"))
        });
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let rotated = a
                .wrapping_add(f)
                .wrapping_add(K[i])
                .wrapping_add(m[g])
                .rotate_left(S[i]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rotated);
        }
        for (s, v) in state.iter_mut().zip([a, b, c, d]) {
            *s = s.wrapping_add(v);
        }
    }
    let mut out = [0; 16];
    for (i, s) in state.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: [u8; 16]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn md5_matches_the_rfc_examples() {
        assert_eq!(hex(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(md5(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            )),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn choices_are_sorted_by_label_hash() {
        // md5("c") < md5("b"), so `c` goes first under a choice but not
        // under a sequence.
        assert!(md5(b"c") < md5(b"b"));
        let tree = ProcessTree::parse("X( 'b', 'c' )").expect("tree");
        assert_eq!(tree_sort(&tree).0.to_string(), "X( 'c', 'b' )");
        let tree = ProcessTree::parse("->( 'b', 'c' )").expect("tree");
        assert_eq!(tree_sort(&tree).0.to_string(), "->( 'b', 'c' )");
    }

    #[test]
    fn hash_sums_carry_past_128_bits() {
        assert_eq!(add((0, u128::MAX), (0, 2)), (1, 1));
    }
}
