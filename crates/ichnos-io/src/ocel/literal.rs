//! Python list literals, as pm4py's OCEL 1.0 CSV reader reads the object
//! columns (`extended_table.safe_parse_list`: `ast.literal_eval` on a cell
//! that starts with `[`).
//!
//! `literal_eval` raises `SyntaxError` for text that is not Python and
//! `ValueError` for Python that is not a literal, and pm4py reads no objects
//! from such a cell. Any other literal is kept, and pm4py fails later unless
//! each item is a string, a byte string or `None`. So the parser below needs
//! to tell three outcomes apart: not a literal, a list of usable items, and
//! a list holding something else.

/// One item of a list literal.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Item {
    /// A string. `None` when it holds a lone surrogate, which a Rust string
    /// cannot hold.
    Str(Option<String>),
    /// A byte string.
    Bytes(Vec<u8>),
    /// `None`.
    None,
    /// A string with a `\N{name}` escape, which ichnos does not resolve.
    NamedEscape,
    /// Any other literal: a number, a boolean, a tuple, a list, a dict, a set
    /// or `...`.
    Other,
}

/// Parses `text` as a Python list literal. `None` when `literal_eval` raises
/// `SyntaxError` or `ValueError`.
pub(super) fn parse_list(text: &str) -> Option<Vec<Item>> {
    if text.contains('\0') {
        return None;
    }
    // Python reads `\r\n` and `\r` in source as `\n`.
    let source: Vec<char> = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .collect();
    let mut p = Parser {
        s: source,
        pos: 0,
        depth: 0,
    };
    if !p.eat('[') {
        return None;
    }
    let items = p.sequence(']')?;
    p.skip();
    (p.pos == p.s.len()).then_some(items.into_iter().map(Expr::into_item).collect())
}

/// What `literal_eval` sees in one expression.
enum Expr {
    /// A literal that is not a number.
    Value(Item),
    /// A number constant; `true` for an imaginary one.
    Number(bool),
    /// A number constant under unary `+` or `-`; `true` for an imaginary one.
    Signed(bool),
    /// A real number plus or minus an imaginary one.
    Complex,
}

impl Expr {
    fn into_item(self) -> Item {
        match self {
            Expr::Value(item) => item,
            _ => Item::Other,
        }
    }
}

/// Deeper nesting than CPython's parser accepts.
const MAX_DEPTH: usize = 200;

struct Parser {
    s: Vec<char>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.pos).copied()
    }

    fn at(&self, offset: usize) -> Option<char> {
        self.s.get(self.pos + offset).copied()
    }

    /// Skips spaces, newlines, comments and backslash continuations.
    fn skip(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\x0c' | '\n' => self.pos += 1,
                '#' => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.pos += 1;
                    }
                }
                '\\' if self.at(1) == Some('\n') => self.pos += 2,
                _ => break,
            }
        }
    }

    /// Skips space, then consumes `c` if it comes next.
    fn eat(&mut self, c: char) -> bool {
        self.skip();
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Comma-separated expressions up to `close`, with an optional trailing
    /// comma. The opening bracket is already consumed.
    fn sequence(&mut self, close: char) -> Option<Vec<Expr>> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut items = Vec::new();
        loop {
            if self.eat(close) {
                break;
            }
            items.push(self.expr()?);
            if self.eat(close) {
                break;
            }
            if !self.eat(',') {
                return None;
            }
        }
        self.depth -= 1;
        Some(items)
    }

    /// Unary terms joined by binary `+` and `-`. `literal_eval` accepts a
    /// binary operation only as a real number plus or minus an imaginary
    /// one.
    fn expr(&mut self) -> Option<Expr> {
        let mut left = self.unary()?;
        loop {
            self.skip();
            if !matches!(self.peek(), Some('+' | '-')) {
                return Some(left);
            }
            self.pos += 1;
            let right = self.unary()?;
            left = match (left, right) {
                (Expr::Number(false) | Expr::Signed(false), Expr::Number(true)) => Expr::Complex,
                _ => return None,
            };
        }
    }

    /// `+` or `-` before a number, or an atom.
    fn unary(&mut self) -> Option<Expr> {
        self.skip();
        if matches!(self.peek(), Some('+' | '-')) {
            self.pos += 1;
            return match self.unary()? {
                Expr::Number(imaginary) => Some(Expr::Signed(imaginary)),
                _ => None,
            };
        }
        self.atom()
    }

    fn atom(&mut self) -> Option<Expr> {
        self.skip();
        let c = self.peek()?;
        match c {
            '[' => {
                self.pos += 1;
                self.sequence(']')?;
                Some(Expr::Value(Item::Other))
            }
            '(' => {
                self.pos += 1;
                if self.eat(')') {
                    return Some(Expr::Value(Item::Other));
                }
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return None;
                }
                let inner = self.expr()?;
                self.depth -= 1;
                if self.eat(')') {
                    // Parentheses only group.
                    return Some(inner);
                }
                if !self.eat(',') {
                    return None;
                }
                self.sequence(')')?;
                Some(Expr::Value(Item::Other))
            }
            '{' => {
                self.pos += 1;
                self.braces()?;
                Some(Expr::Value(Item::Other))
            }
            '\'' | '"' => self.strings(),
            '.' if self.at(1) == Some('.') && self.at(2) == Some('.') => {
                self.pos += 3;
                Some(Expr::Value(Item::Other))
            }
            '0'..='9' => self.number(),
            '.' if self.at(1).is_some_and(|c| c.is_ascii_digit()) => self.number(),
            c if c == '_' || c.is_alphabetic() => {
                let start = self.pos;
                while self.peek().is_some_and(|c| c == '_' || c.is_alphanumeric()) {
                    self.pos += 1;
                }
                let name: String = self.s[start..self.pos].iter().collect();
                if matches!(self.peek(), Some('\'' | '"')) && prefix(&name).is_some() {
                    self.pos = start;
                    return self.strings();
                }
                match name.as_str() {
                    "True" | "False" => Some(Expr::Value(Item::Other)),
                    "None" => Some(Expr::Value(Item::None)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// A dict or set display after `{`.
    fn braces(&mut self) -> Option<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        if self.eat('}') {
            self.depth -= 1;
            return Some(());
        }
        self.expr()?;
        let dict = self.eat(':');
        if dict {
            self.expr()?;
        }
        loop {
            if self.eat('}') {
                break;
            }
            if !self.eat(',') {
                return None;
            }
            if self.eat('}') {
                break;
            }
            self.expr()?;
            if dict {
                if !self.eat(':') {
                    return None;
                }
                self.expr()?;
            }
        }
        self.depth -= 1;
        Some(())
    }

    /// Adjacent string literals, which Python joins. Joining a string and a
    /// byte string is a syntax error, and an f-string is not a literal.
    fn strings(&mut self) -> Option<Expr> {
        let mut text: Vec<u32> = Vec::new();
        let mut bytes = None;
        let mut named = false;
        loop {
            self.skip();
            let start = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                self.pos += 1;
            }
            let name: String = self.s[start..self.pos].iter().collect();
            if !matches!(self.peek(), Some('\'' | '"')) {
                self.pos = start;
                break;
            }
            let p = prefix(&name)?;
            if p.formatted || bytes.is_some_and(|b| b != p.bytes) {
                return None;
            }
            bytes = Some(p.bytes);
            named |= self.string(p, &mut text)?;
        }
        if bytes? {
            let bytes = text
                .into_iter()
                .map(|u| u8::try_from(u).ok())
                .collect::<Option<Vec<u8>>>()?;
            return Some(Expr::Value(Item::Bytes(bytes)));
        }
        if named {
            return Some(Expr::Value(Item::NamedEscape));
        }
        let text: Option<String> = text.into_iter().map(char::from_u32).collect();
        Some(Expr::Value(Item::Str(text)))
    }

    /// One quoted literal, appended to `out` as code points (or byte
    /// values). Returns whether it holds a `\N{name}` escape.
    fn string(&mut self, p: Prefix, out: &mut Vec<u32>) -> Option<bool> {
        let quote = self.peek()?;
        let triple = self.at(1) == Some(quote) && self.at(2) == Some(quote);
        self.pos += if triple { 3 } else { 1 };
        let mut named = false;
        loop {
            let c = self.peek()?;
            if c == quote && (!triple || (self.at(1) == Some(quote) && self.at(2) == Some(quote))) {
                self.pos += if triple { 3 } else { 1 };
                return Some(named);
            }
            if c == '\n' && !triple {
                return None;
            }
            if p.bytes && !c.is_ascii() {
                return None;
            }
            self.pos += 1;
            if c != '\\' {
                out.push(c as u32);
                continue;
            }
            let d = self.peek()?;
            self.pos += 1;
            if p.raw {
                out.extend([u32::from('\\'), d as u32]);
                continue;
            }
            match d {
                '\n' => {}
                '\\' | '\'' | '"' => out.push(d as u32),
                'a' => out.push(0x07),
                'b' => out.push(0x08),
                'f' => out.push(0x0c),
                'n' => out.push(0x0a),
                'r' => out.push(0x0d),
                't' => out.push(0x09),
                'v' => out.push(0x0b),
                '0'..='7' => {
                    let mut value = d.to_digit(8)?;
                    for _ in 0..2 {
                        match self.peek().and_then(|c| c.to_digit(8)) {
                            Some(v) => {
                                value = value * 8 + v;
                                self.pos += 1;
                            }
                            None => break,
                        }
                    }
                    out.push(value);
                }
                'x' => out.push(self.hex(2)?),
                'u' if !p.bytes => out.push(self.hex(4)?),
                'U' if !p.bytes => {
                    let value = self.hex(8)?;
                    if value > 0x10_ffff {
                        return None;
                    }
                    out.push(value);
                }
                'N' if !p.bytes => {
                    if !self.eat_raw('{') {
                        return None;
                    }
                    let start = self.pos;
                    while self
                        .peek()
                        .is_some_and(|c| c != '}' && c != quote && c != '\n')
                    {
                        self.pos += 1;
                    }
                    if self.pos == start || !self.eat_raw('}') {
                        return None;
                    }
                    named = true;
                }
                other => out.extend([u32::from('\\'), other as u32]),
            }
        }
    }

    fn eat_raw(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Exactly `n` hex digits.
    fn hex(&mut self, n: usize) -> Option<u32> {
        let mut value = 0;
        for _ in 0..n {
            value = value * 16 + self.peek()?.to_digit(16)?;
            self.pos += 1;
        }
        Some(value)
    }

    /// A number token: an integer, a float or an imaginary number.
    fn number(&mut self) -> Option<Expr> {
        let radix = match (self.peek(), self.at(1)) {
            (Some('0'), Some('x' | 'X')) => Some(16),
            (Some('0'), Some('o' | 'O')) => Some(8),
            (Some('0'), Some('b' | 'B')) => Some(2),
            _ => None,
        };
        let imaginary = if let Some(radix) = radix {
            self.pos += 2;
            // `0x_1f` is valid: an underscore may follow the prefix.
            self.eat_raw('_');
            if self.digits(radix) == 0 {
                return None;
            }
            false
        } else {
            let int = self.pos;
            let whole = self.digits(10);
            let leading_zero = whole > 1 && self.s[int] == '0';
            let all_zero = self.s[int..self.pos].iter().all(|&c| c == '0' || c == '_');
            let mut float = false;
            if self.peek() == Some('.') {
                self.pos += 1;
                float = true;
                if self.peek().is_some_and(|c| c.is_ascii_digit()) && self.digits(10) == 0 {
                    return None;
                }
            }
            if matches!(self.peek(), Some('e' | 'E')) {
                self.pos += 1;
                float = true;
                if matches!(self.peek(), Some('+' | '-')) {
                    self.pos += 1;
                }
                if self.digits(10) == 0 {
                    return None;
                }
            }
            let imaginary = self.eat_raw('j') || self.eat_raw('J');
            if leading_zero && !all_zero && !float && !imaginary {
                return None;
            }
            imaginary
        };
        if self.peek().is_some_and(|c| c == '_' || c.is_alphanumeric()) {
            return None;
        }
        Some(Expr::Number(imaginary))
    }

    /// Digits in `radix` with single underscores between them. Returns the
    /// digit count, or 0 when an underscore is misplaced.
    fn digits(&mut self, radix: u32) -> usize {
        let mut count = 0;
        loop {
            match self.peek() {
                Some(c) if c.is_digit(radix) => {
                    count += 1;
                    self.pos += 1;
                }
                Some('_') if count > 0 && self.at(1).is_some_and(|c| c.is_digit(radix)) => {
                    self.pos += 1;
                }
                _ => return count,
            }
        }
    }
}

/// A string prefix.
#[derive(Clone, Copy)]
struct Prefix {
    raw: bool,
    bytes: bool,
    formatted: bool,
}

fn prefix(name: &str) -> Option<Prefix> {
    let lower = name.to_ascii_lowercase();
    let (raw, bytes, formatted) = match lower.as_str() {
        "" | "u" => (false, false, false),
        "r" => (true, false, false),
        "b" => (false, true, false),
        "br" | "rb" => (true, true, false),
        "f" => (false, false, true),
        "fr" | "rf" => (true, false, true),
        _ => return None,
    };
    Some(Prefix {
        raw,
        bytes,
        formatted,
    })
}

#[cfg(test)]
mod tests {
    use super::{Item, parse_list};

    fn strings(text: &str) -> Option<Vec<String>> {
        parse_list(text).map(|items| {
            items
                .into_iter()
                .map(|i| match i {
                    Item::Str(Some(s)) => s,
                    Item::Bytes(b) => format!("b{b:?}"),
                    other => format!("{other:?}"),
                })
                .collect()
        })
    }

    #[test]
    fn follows_literal_eval() {
        let ok = |text: &str, want: &[&str]| {
            let want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
            assert_eq!(strings(text), Some(want), "{text:?}");
        };
        ok("[]", &[]);
        ok("[ 'a' , \"b\" ]", &["a", "b"]);
        ok("['a',]", &["a"]);
        ok("[u'c' \"d\", ('e')]", &["cd", "e"]);
        assert_eq!(strings("[u'c' \"d\" ('e')]"), None);
        ok("['a\\tb', '\\x41\\u00e9', 'q\\'s']", &["a\tb", "Aé", "q's"]);
        ok("[r'\\n', R'\\'']", &["\\n", "\\'"]);
        ok("['\\q', '\\101', '\\\n']", &["\\q", "A", ""]);
        ok("['''t''', \"\"\"u\nv\"\"\"]", &["t", "u\nv"]);
        ok("['a'] # note", &["a"]);
        ok("[\n 'd',  \\\n'e']", &["d", "e"]);
        ok("['x'] ", &["x"]);
        ok("[b'\\x41', None]", &["b[65]", "None"]);
        ok(
            "[1, -2.5, 1_000, 0x1F, 1e3, 1+2j, -0b1, 01.5, 00]",
            &["Other"; 9],
        );
        ok(
            "[True, ..., (), ('a',), {}, {1}, {1: 2}, [['a']]]",
            &["Other"; 8],
        );
        ok("['\\N{BULLET}']", &["NamedEscape"]);
        ok("['\\ud800']", &["Str(None)"]);
        for bad in [
            "['a' 1]",
            "[",
            "['a'] + ['b']",
            "[1+2]",
            "[--1]",
            "[-'a']",
            "[f'a']",
            "['a' b'b']",
            "[x]",
            "[01]",
            "[1__0]",
            "[1abc]",
            "['\\x4']",
            "['a\nb']",
            "['a'][0]",
            "[b'é']",
            "['a' for a in b]",
            "[{**a}]",
            "[1j+1]",
        ] {
            assert_eq!(strings(bad), None, "{bad:?}");
        }
    }
}
