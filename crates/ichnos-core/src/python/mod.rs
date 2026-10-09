//! Python's text forms of values, for output that must match pm4py's.

mod nonprintable;

use std::fmt::Write;

/// Whether Python's `str.isprintable` accepts `c`. It rejects control and
/// format characters, surrogates, private-use and unassigned code points,
/// and every separator except the ASCII space.
pub fn is_printable(c: char) -> bool {
    let c = c as u32;
    nonprintable::RANGES
        .binary_search_by(|&(lo, hi)| {
            if hi < c {
                std::cmp::Ordering::Less
            } else if lo > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_err()
}

/// Python's `repr` of a string.
///
/// The quote is `'` unless the string holds `'` and no `"`. Backslash, the
/// quote, tab, newline and carriage return get their short escapes. Other
/// characters that [`is_printable`] rejects become `\xNN`, `\uNNNN` or
/// `\UNNNNNNNN`.
///
/// ```
/// use ichnos_core::python::repr;
///
/// assert_eq!(repr("it's"), "\"it's\"");
/// assert_eq!(repr("a\u{a0}b"), "'a\\xa0b'");
/// ```
pub fn repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_printable(c) => out.push(c),
            c => {
                let n = c as u32;
                // Writing to a String cannot fail.
                let _ = if n <= 0xff {
                    write!(out, "\\x{n:02x}")
                } else if n <= 0xffff {
                    write!(out, "\\u{n:04x}")
                } else {
                    write!(out, "\\U{n:08x}")
                };
            }
        }
    }
    out.push(quote);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_follows_python() {
        assert_eq!(repr("a b"), "'a b'");
        assert_eq!(repr("it's"), "\"it's\"");
        assert_eq!(repr("it's \"x\""), "'it\\'s \"x\"'");
        assert_eq!(repr("'\""), "'\\'\"'");
        assert_eq!(repr("a\\b\n\r\t"), "'a\\\\b\\n\\r\\t'");
        assert_eq!(repr("\0\x1b\x7f"), "'\\x00\\x1b\\x7f'");
        // Values checked against Python 3.12.
        assert_eq!(repr("a\u{a0}b\u{85}c "), "'a\\xa0b\\x85c '");
        assert_eq!(repr("soft\u{ad}hyphen"), "'soft\\xadhyphen'");
        assert_eq!(repr("\u{2028}\u{2029}\u{200b}"), "'\\u2028\\u2029\\u200b'");
        assert_eq!(repr("\u{3000}\u{e000}"), "'\\u3000\\ue000'");
        assert_eq!(repr("\u{e0001}\u{10ffff}"), "'\\U000e0001\\U0010ffff'");
        assert_eq!(repr("é ü 日本 🙂"), "'é ü 日本 🙂'");
    }

    #[test]
    fn printable_matches_python_edges() {
        assert!(is_printable(' '));
        assert!(is_printable('~'));
        assert!(!is_printable('\u{7f}'));
        assert!(!is_printable('\u{a0}'));
        assert!(is_printable('\u{a1}'));
        assert!(!is_printable('\u{378}'));
        assert!(is_printable('\u{37a}'));
    }
}
