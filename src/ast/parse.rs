use std::fmt;

use crate::{
    ast::{
        Ast,
        lex::{LexError, Lexer, Token, TokenKind},
        parse::ParseError::{EmptySnippet, UnexpectedToken},
        utils::parse_number,
    },
    interner::Interner,
};

struct Parser<'a> {
    lex: Lexer<'a>,
    peek_token: Option<Token<'a>>,

    data: Ast,

    stack: Vec<u32>,

    interner: &'a Interner<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError<'a> {
    Lex(LexError),
    UnexpectedToken(&'a str),
    EmptySnippet,
    InvalidNumber(&'a str),
    NumberOverflow(&'a str),
}

impl<'a> From<LexError> for ParseError<'a> {
    #[inline(always)]
    fn from(err: LexError) -> Self {
        Self::Lex(err)
    }
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex(err) => write!(f, "{err}"),
            Self::UnexpectedToken(d) => write!(f, "unexpected token '{}'", d),
            Self::EmptySnippet => write!(f, "empty snippet"),
            Self::InvalidNumber(d) => write!(f, "invalid number '{}'", d),
            Self::NumberOverflow(d) => write!(f, "number overflow '{}'", d),
        }
    }
}

pub(crate) fn parse_snippet<'a>(
    name: &str,
    data: &'a [u8],
    interner: &'a Interner<'a>,
) -> Result<(Ast, u32), ParseError<'a>> {
    let mut p = Parser {
        lex: Lexer { data: data, pos: 0 },
        peek_token: None,
        data: Ast::new(1024),
        stack: Vec::with_capacity(32),
        interner,
    };

    let root_idx = p.parse_expr(0)?;

    let tok = p.peek()?;

    if tok.kind != TokenKind::Eof {
        return Err(UnexpectedToken(tok.data));
    }

    if p.data.len() == 0 {
        return Err(EmptySnippet);
    }

    Ok((p.data, root_idx))
}

impl<'a> Parser<'a> {
    fn next_token(&mut self) -> Result<Token<'a>, LexError> {
        if let Some(tok) = self.peek_token.take() {
            return Ok(tok);
        }
        let t = self.lex.next()?;
        Ok(t)
    }

    fn peek(&mut self) -> Result<Token<'a>, LexError> {
        if let Some(t) = self.peek_token {
            return Ok(t);
        }
        let t = self.lex.next()?;
        self.peek_token = Some(t);
        return Ok(t);
    }

    fn parse_expr(&mut self, prec: usize) -> Result<u32, ParseError<'a>> {
        let tok = self.next_token()?;

        let mut lhs: u32 = 0;

        match tok.kind {
            TokenKind::Number => {
                let num = parse_number(tok.data)?;
                lhs = self.data.emit_number(num);
            }
            _ => return Err(UnexpectedToken(tok.data)),
        }

        Ok(lhs)
    }
}

#[cfg(test)]
mod tests {

    use crate::ast::Tag;

    use super::*;

    #[test]
    fn test_parse_number() {
        let interner = Interner::new();

        let snippet = r#"
            420_13.37
        "#;

        let (ast, root_idx) = parse_snippet("test.jsonnet", snippet.as_bytes(), &interner).unwrap();

        let bp = ast.base();

        assert_eq!(ast.len(), 3);
        assert_eq!(Ast::read_tag(bp, root_idx), Tag::Number);
        assert_eq!(Ast::read_f64(bp, root_idx + 1), 420_13.37);
    }
}
