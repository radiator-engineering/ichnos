//! Parser for pm4py's POWL string syntax (`powl/parser.py`).

use std::ops::Range;

use super::{Powl, PowlError, StrictPartialOrder};

/// Errors from [`Powl::parse`].
///
/// The grammar, with whitespace allowed between tokens:
///
/// ```text
/// model := po | xor | loop | 'tau' | "'" label "'" | bare
/// po    := 'PO' ['='] '(' 'nodes' '=' '{' [model (',' model)*] '}' ','
///          'order' '=' '{' [pair (',' pair)*] '}' ')'
/// xor   := 'X' '(' model (',' model)+ ')'
/// loop  := '*' '(' model ',' model ')'
/// pair  := node '-->' node
/// ```
///
/// As in pm4py, newlines, carriage returns and tabs are removed before
/// parsing, and byte offsets count in the text without them. A bare label
/// runs to the next `,`, `(`, `)`, `{` or `}` and is trimmed. A quoted label
/// runs to the next single quote. A `node` in a pair is the text of a child
/// of that partial order, exactly as written in its `nodes` list.
///
/// pm4py reads any text that starts with `X`, `*` or `tau` as an operator
/// or a silent step; here `Xray` is an activity. pm4py also accepts any text
/// between two node names as a pair, and ignores a pair it cannot read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PowlParseError {
    /// The input ended in the middle of a model.
    #[error("unexpected end of input")]
    UnexpectedEnd,
    /// A character that cannot start or continue a model.
    #[error("unexpected {found:?} at byte {offset}; expected {expected}")]
    Unexpected {
        /// Byte offset in the input without newlines and tabs.
        offset: usize,
        /// The character found.
        found: char,
        /// What the parser expected there.
        expected: &'static str,
    },
    /// A quoted label has no closing quote.
    #[error("unterminated label starting at byte {0}")]
    UnterminatedLabel(usize),
    /// Text left over after a complete model.
    #[error("trailing input at byte {0}")]
    TrailingInput(usize),
    /// An order pair that is not two node names joined by `-->`.
    #[error("cannot read order pair {0:?}")]
    UnknownPair(String),
    /// An order pair names a node that occurs twice in the `nodes` list.
    #[error("order pair names {0:?}, which occurs twice in the partial order")]
    AmbiguousNode(String),
    /// The model parsed but breaks an arity rule.
    #[error(transparent)]
    Invalid(#[from] PowlError),
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn skip_ws(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.src.len() - trimmed.len();
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn unexpected(&self, expected: &'static str) -> PowlParseError {
        match self.peek() {
            Some(found) => PowlParseError::Unexpected {
                offset: self.pos,
                found,
                expected,
            },
            None => PowlParseError::UnexpectedEnd,
        }
    }

    fn eat(&mut self, token: &str) -> bool {
        if self.rest().starts_with(token) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &'static str) -> Result<(), PowlParseError> {
        self.skip_ws();
        if self.eat(token) {
            Ok(())
        } else {
            Err(self.unexpected(token))
        }
    }

    /// Returns `true` when `keyword` is followed, after whitespace, by one
    /// of `next`.
    fn at_keyword(&self, keyword: &str, next: &[char]) -> bool {
        self.rest()
            .strip_prefix(keyword)
            .and_then(|r| r.trim_start().chars().next())
            .is_some_and(|c| next.contains(&c))
    }

    /// Parses one model and returns it with the span of text it was
    /// written as.
    fn model(&mut self) -> Result<(Powl, Range<usize>), PowlParseError> {
        self.skip_ws();
        let start = self.pos;
        let model = if self.at_keyword("PO", &['=', '(']) {
            self.pos += 2;
            self.skip_ws();
            self.eat("=");
            self.partial_order()?
        } else if self.at_keyword("X", &['(']) {
            self.pos += 1;
            let children = self.operands()?;
            if children.len() < 2 {
                return Err(PowlError::ChoiceArity(children.len()).into());
            }
            Powl::Xor(children)
        } else if self.at_keyword("*", &['(']) {
            self.pos += 1;
            let children = self.operands()?;
            let n = children.len();
            let Ok([d, r]) = <[Powl; 2]>::try_from(children) else {
                return Err(PowlError::LoopArity(n).into());
            };
            Powl::looped(d, r)
        } else if self.eat("'") {
            let Some(end) = self.rest().find('\'') else {
                return Err(PowlParseError::UnterminatedLabel(start));
            };
            let label = &self.rest()[..end];
            let model = Powl::activity(label);
            self.pos += end + 1;
            model
        } else {
            let len = self
                .rest()
                .find([',', '(', ')', '{', '}'])
                .unwrap_or(self.rest().len());
            let text = self.rest()[..len].trim_end();
            if text.is_empty() {
                return Err(self.unexpected("a model"));
            }
            let model = if text == "tau" {
                Powl::Silent
            } else {
                Powl::activity(text)
            };
            self.pos += text.len();
            model
        };
        Ok((model, start..self.pos))
    }

    /// `'(' model (',' model)* ')'`.
    fn operands(&mut self) -> Result<Vec<Powl>, PowlParseError> {
        self.expect("(")?;
        let mut children = vec![self.model()?.0];
        loop {
            self.skip_ws();
            if self.eat(")") {
                break;
            }
            if !self.eat(",") {
                return Err(self.unexpected("',' or ')'"));
            }
            children.push(self.model()?.0);
        }
        Ok(children)
    }

    /// The part of a partial order after `PO=`.
    fn partial_order(&mut self) -> Result<Powl, PowlParseError> {
        self.expect("(")?;
        self.expect("nodes")?;
        self.expect("=")?;
        self.expect("{")?;
        let mut children = Vec::new();
        let mut texts: Vec<String> = Vec::new();
        self.skip_ws();
        if !self.eat("}") {
            loop {
                let (child, span) = self.model()?;
                texts.push(self.src[span].to_owned());
                children.push(child);
                self.skip_ws();
                if self.eat("}") {
                    break;
                }
                if !self.eat(",") {
                    return Err(self.unexpected("',' or '}'"));
                }
            }
        }
        self.expect(",")?;
        self.expect("order")?;
        self.expect("=")?;
        self.expect("{")?;
        let mut po = StrictPartialOrder::new(children);
        for pair in self.pairs()? {
            let (i, j) = resolve_pair(&pair, &texts)?;
            po.add_edge(i, j);
        }
        self.expect(")")?;
        Ok(Powl::PartialOrder(po))
    }

    /// The comma-separated pairs up to the closing `}`, which it consumes.
    fn pairs(&mut self) -> Result<Vec<String>, PowlParseError> {
        let mut pairs = Vec::new();
        let mut depth = 0usize;
        let mut quoted = false;
        let mut piece_start = self.pos;
        for (offset, c) in self.rest().char_indices() {
            let at = self.pos + offset;
            match c {
                '\'' => quoted = !quoted,
                _ if quoted => {}
                '(' | '{' => depth += 1,
                ')' if depth > 0 => depth -= 1,
                '}' if depth > 0 => depth -= 1,
                ',' | '}' if depth == 0 => {
                    let piece = self.src[piece_start..at].trim();
                    if !piece.is_empty() {
                        pairs.push(piece.to_owned());
                    }
                    piece_start = at + 1;
                    if c == '}' {
                        self.pos = at + 1;
                        return Ok(pairs);
                    }
                }
                ')' => {
                    self.pos = at;
                    return Err(self.unexpected("',' or '}'"));
                }
                _ => {}
            }
        }
        self.pos = self.src.len();
        Err(PowlParseError::UnexpectedEnd)
    }
}

/// Finds the `-->` in `pair` that splits it into two node texts.
fn resolve_pair(pair: &str, texts: &[String]) -> Result<(usize, usize), PowlParseError> {
    let find = |text: &str| -> Result<Option<usize>, PowlParseError> {
        let mut hits = texts.iter().enumerate().filter(|(_, t)| *t == text);
        match (hits.next(), hits.next()) {
            (Some(_), Some(_)) => Err(PowlParseError::AmbiguousNode(text.to_owned())),
            (Some((i, _)), None) => Ok(Some(i)),
            _ => Ok(None),
        }
    };
    for (k, _) in pair.match_indices("-->") {
        let (left, right) = (pair[..k].trim(), pair[k + 3..].trim());
        if let (Some(i), Some(j)) = (find(left)?, find(right)?) {
            return Ok((i, j));
        }
    }
    Err(PowlParseError::UnknownPair(pair.to_owned()))
}

pub(super) fn parse(src: &str) -> Result<Powl, PowlParseError> {
    let cleaned: String = src
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | '\t'))
        .collect();
    let mut p = Parser {
        src: &cleaned,
        pos: 0,
    };
    let (model, _) = p.model()?;
    p.skip_ws();
    if p.pos < cleaned.len() {
        return Err(PowlParseError::TrailingInput(p.pos));
    }
    Ok(model)
}
