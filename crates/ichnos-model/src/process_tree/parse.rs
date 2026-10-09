//! Parser for pm4py's process tree string syntax
//! (`process_tree/utils/generic.py::parse`).

use super::{Operator, ProcessTree, TreeError};

/// Errors from [`ProcessTree::parse`].
///
/// The grammar, with whitespace allowed between tokens:
///
/// ```text
/// tree     := operator '(' tree (',' tree)* ')' | "'" label "'" | 'tau' | 'τ'
/// operator := '->' | 'X' | '+' | '*' | 'O' | '<>'
/// ```
///
/// A label runs to the next single quote; there is no escape syntax, as in
/// pm4py.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// The input ended in the middle of a tree.
    #[error("unexpected end of input")]
    UnexpectedEnd,
    /// A character that cannot start or continue a tree.
    #[error("unexpected {found:?} at byte {offset}; expected {expected}")]
    Unexpected {
        /// Byte offset in the input.
        offset: usize,
        /// The character found.
        found: char,
        /// What the parser expected there.
        expected: &'static str,
    },
    /// A quoted label has no closing quote.
    #[error("unterminated label starting at byte {0}")]
    UnterminatedLabel(usize),
    /// Text left over after a complete tree.
    #[error("trailing input at byte {0}")]
    TrailingInput(usize),
    /// The tree parsed but breaks an arity rule.
    #[error(transparent)]
    Invalid(#[from] TreeError),
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

    fn unexpected(&self, expected: &'static str) -> ParseError {
        match self.peek() {
            Some(found) => ParseError::Unexpected {
                offset: self.pos,
                found,
                expected,
            },
            None => ParseError::UnexpectedEnd,
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

    fn tree(&mut self) -> Result<ProcessTree, ParseError> {
        self.skip_ws();
        // Same order of checks as pm4py.
        const OPERATORS: [(&str, Operator); 6] = [
            ("*", Operator::Loop),
            ("+", Operator::Parallel),
            ("X", Operator::Xor),
            ("O", Operator::Or),
            ("->", Operator::Sequence),
            ("<>", Operator::Interleaving),
        ];
        if let Some(&(sym, op)) = OPERATORS
            .iter()
            .find(|(sym, _)| self.rest().starts_with(sym))
        {
            self.pos += sym.len();
            self.skip_ws();
            if !self.eat("(") {
                return Err(self.unexpected("'('"));
            }
            let mut children = vec![self.tree()?];
            loop {
                self.skip_ws();
                if self.eat(")") {
                    break;
                }
                if !self.eat(",") {
                    return Err(self.unexpected("',' or ')'"));
                }
                children.push(self.tree()?);
            }
            return Ok(ProcessTree::Node(op, children));
        }
        if self.eat("'") {
            let start = self.pos - 1;
            let Some(end) = self.rest().find('\'') else {
                return Err(ParseError::UnterminatedLabel(start));
            };
            let label = &self.rest()[..end];
            let tree = ProcessTree::activity(label);
            self.pos += end + 1;
            return Ok(tree);
        }
        if self.eat("tau") || self.eat("τ") {
            return Ok(ProcessTree::Tau);
        }
        Err(self.unexpected("an operator, a quoted label or tau"))
    }
}

pub(super) fn parse(src: &str) -> Result<ProcessTree, ParseError> {
    let mut p = Parser { src, pos: 0 };
    let tree = p.tree()?;
    p.skip_ws();
    if p.pos < src.len() {
        return Err(ParseError::TrailingInput(p.pos));
    }
    tree.validate()?;
    Ok(tree)
}
