use std::fmt;

use crate::ast::lex::LexError::{
    LeadingZeroInNumber, UnexpectedChar, UnterminatedBlockComment, UnterminatedString,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Eof,

    Number,
    String,
    Ident,

    BraceL,
    BraceR,
    BracketL,
    BracketR,
    Comma,
    Dollar,
    Dot,
    ParenL,
    ParenR,
    Semicolon,
    Colon,

    Operator,

    // Keywords
    Assert,
    Else,
    Error,
    False,
    For,
    Function,
    If,
    Import,
    ImportStr,
    ImportBin,
    In,
    Local,
    Null,
    SelfKw,
    Super,
    TailStrict,
    Then,
    True,
}

impl TokenKind {
    #[inline]
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "assert" => Some(Self::Assert),
            "else" => Some(Self::Else),
            "error" => Some(Self::Error),
            "false" => Some(Self::False),
            "for" => Some(Self::For),
            "function" => Some(Self::Function),
            "if" => Some(Self::If),
            "import" => Some(Self::Import),
            "importstr" => Some(Self::ImportStr),
            "importbin" => Some(Self::ImportBin),
            "in" => Some(Self::In),
            "local" => Some(Self::Local),
            "null" => Some(Self::Null),
            "self" => Some(Self::SelfKw),
            "super" => Some(Self::Super),
            "tailstrict" => Some(Self::TailStrict),
            "then" => Some(Self::Then),
            "true" => Some(Self::True),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    UnterminatedBlockComment,
    UnterminatedString,
    UnterminatedVerbatimString,
    UnterminatedBlockString,
    LeadingZeroInNumber,
    UnexpectedAt,
    UnexpectedChar(u8),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedBlockComment => write!(f, "unterminated block comment"),
            Self::UnterminatedString => write!(f, "unterminated string literal"),
            Self::UnterminatedVerbatimString => write!(f, "unterminated verbatim string literal"),
            Self::UnterminatedBlockString => write!(f, "unterminated block string"),
            Self::LeadingZeroInNumber => write!(f, "leading zero is not allowed in numbers"),
            Self::UnexpectedAt => write!(f, "unexpected '@'"),
            Self::UnexpectedChar(c) => write!(f, "unexpected character '{}'", *c as char),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub data: &'a str,
    pub pos: u32,
}

impl<'a> Token<'a> {
    #[inline(always)]
    pub const fn new(kind: TokenKind, data: &'a str, pos: u32) -> Self {
        Self { kind, data, pos }
    }
}

pub struct Lexer<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

// - `memchr::memchr(b'\n', slice)` → `strings.IndexByte`
// - `memchr::memrchr(b'\n', slice)` → `strings.LastIndexByte` (used in `lexBlockString`)
// - `memchr::memmem::find(slice, b"*/")` → `strings.Index` (used for `*/` block comments)

impl<'a> Lexer<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data: data.as_bytes(),
            pos: 0,
        }
    }

    #[inline(always)]
    fn slice(&self, start: usize, end: usize) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(&self.data[start..end]) }
    }

    pub fn next(&mut self) -> Result<Token<'a>, LexError> {
        self.consumeWhitespaceAndComments()?;

        if self.pos >= self.data.len() {
            return Ok(Token {
                kind: TokenKind::Eof,
                pos: self.pos as u32,
                data: "",
            });
        }

        let start = self.pos;
        let c = self.data[start];
        self.pos += 1;

        match c {
            b'{' => return Ok(Token::new(TokenKind::BraceL, "{", start as u32)),
            b'}' => return Ok(Token::new(TokenKind::BraceR, "}", start as u32)),
            b'[' => return Ok(Token::new(TokenKind::BracketL, "[", start as u32)),
            b']' => return Ok(Token::new(TokenKind::BracketR, "]", start as u32)),
            b',' => return Ok(Token::new(TokenKind::Comma, ",", start as u32)),
            b'.' => return Ok(Token::new(TokenKind::Dot, ".", start as u32)),
            b'(' => return Ok(Token::new(TokenKind::ParenL, "(", start as u32)),
            b')' => return Ok(Token::new(TokenKind::ParenR, ")", start as u32)),
            b';' => return Ok(Token::new(TokenKind::Semicolon, ";", start as u32)),
            b'$' => return Ok(Token::new(TokenKind::Dollar, "$", start as u32)),
            b'"' | b'\'' => return self.lex_string(c, start),
            b'@' => {
                if self.pos < self.data.len()
                    && (self.data[self.pos] == b'"' || self.data[self.pos] == b'\'')
                {
                    let quote_type = self.data[self.pos];
                    self.pos += 1;
                    return self.lex_verbatim_string(quote_type, start);
                }
                return Err(UnexpectedChar(c));
            }
            b'|' => {
                if self.pos + 1 < self.data.len()
                    && self.data[self.pos] == b'|'
                    && self.data[self.pos + 1] == b'|'
                {
                    self.pos += 2;
                    return self.lex_block_string(start);
                }
                // Otherwise, it's an operator starting with |
                self.pos -= 1;
                return self.lex_operator(start);
            }
            _ => {}
        }

        if is_digit(c) {
            self.pos -= 1;
            return self.lex_number(start);
        }

        if is_ident(c) {
            self.pos -= 1;
            return self.lex_ident(start);
        }

        if is_symbol(c) {
            self.pos -= 1;
            return self.lex_operator(start);
        }

        Err(UnexpectedChar(c))
    }

    fn consumeWhitespaceAndComments(&mut self) -> Result<(), LexError> {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.pos += 1;
                }
                b'#' => {
                    self.pos += 1;
                    match memchr::memchr(b'\n', &self.data[self.pos..]) {
                        Some(idx) => self.pos += idx + 1,
                        None => self.pos = self.data.len(),
                    }
                }
                b'/' if self.pos + 1 < self.data.len() => match self.data[self.pos + 1] {
                    b'/' => {
                        self.pos += 2;
                        match memchr::memchr(b'\n', &self.data[self.pos..]) {
                            Some(idx) => self.pos += idx + 1,
                            None => self.pos = self.data.len(),
                        };
                    }
                    b'*' => {
                        self.pos += 2;
                        match memchr::memmem::find(&self.data[self.pos..], b"*/") {
                            Some(idx) => self.pos += idx + 2,
                            None => return Err(UnterminatedBlockComment),
                        };
                    }
                    _ => break,
                },
                _ => break,
            }
        }
        Ok(())
    }

    fn lex_string(&mut self, quote_type: u8, start: usize) -> Result<Token<'a>, LexError> {
        loop {
            let Some(idx) = memchr::memchr(quote_type, &self.data[self.pos..]) else {
                return Err(UnterminatedString);
            };

            let abs_idx = self.pos + idx;

            let mut j = abs_idx - 1;
            while self.data[j] == b'\\' {
                j -= 1;
            }

            self.pos = abs_idx + 1;

            let backslashes = abs_idx - 1 - j;
            if backslashes % 2 == 0 {
                break;
            }
        }
        Ok(Token::new(
            TokenKind::String,
            self.slice(start + 1, self.pos - 1),
            start as u32,
        ))
    }

    fn lex_verbatim_string(&mut self, quote_type: u8, start: usize) -> Result<Token<'a>, LexError> {
        todo!("lex_verbatim_string");
        Ok(Token::new(TokenKind::String, "", start as u32))
    }

    fn lex_block_string(&mut self, start: usize) -> Result<Token<'a>, LexError> {
        todo!("lex_block_string");
        Ok(Token::new(TokenKind::String, "", start as u32))
    }

    fn lex_number(&mut self, start: usize) -> Result<Token<'a>, LexError> {
        if self.data[self.pos] == b'0'
            && self.pos + 1 < self.data.len()
            && self.data[self.pos + 1].is_ascii_digit()
        {
            return Err(LeadingZeroInNumber);
        }

        while self.pos < self.data.len() {
            let c = self.data[self.pos];

            if c.is_ascii_digit() || c == b'_' || c == b'.' || c == b'e' || c == b'E' {
                self.pos += 1;
            } else if c == b'+' || c == b'-' {
                if self.pos > start
                    && (self.data[self.pos - 1] == b'e' || self.data[self.pos - 1] == b'E')
                {
                    self.pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(Token::new(
            TokenKind::Number,
            self.slice(start, self.pos),
            start as u32,
        ))
    }

    fn lex_ident(&mut self, start: usize) -> Result<Token<'a>, LexError> {
        while self.pos < self.data.len() && is_ident(self.data[self.pos]) {
            self.pos += 1;
        }
        let data = self.slice(start, self.pos);
        let kind = TokenKind::from_keyword(data).unwrap_or(TokenKind::Ident);
        Ok(Token::new(kind, data, start as u32))
    }

    fn lex_operator(&mut self, start: usize) -> Result<Token<'a>, LexError> {
        while self.pos < self.data.len() && is_symbol(self.data[self.pos]) {
            self.pos += 1;
        }
        let data = self.slice(start, self.pos);
        let kind = if data == ":" {
            TokenKind::Colon
        } else {
            TokenKind::Operator
        };
        Ok(Token::new(kind, data, start as u32))
    }
}

const IDENT_TABLE: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < 256 {
        let c = i as u8;
        table[i] = (c >= b'a' && c <= b'z')
            || (c >= b'A' && c <= b'Z')
            || (c >= b'0' && c <= b'9')
            || c == b'_';
        i += 1;
    }
    table
};

const SYMBOL_TABLE: [bool; 256] = {
    let mut table = [false; 256];
    let symbols = b"!:+~-&|^=<>*/%";
    let mut i = 0;
    while i < symbols.len() {
        table[symbols[i] as usize] = true;
        i += 1;
    }
    table
};

#[inline(always)]
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

#[inline(always)]
fn is_ident(c: u8) -> bool {
    IDENT_TABLE[c as usize]
}

#[inline(always)]
fn is_symbol(c: u8) -> bool {
    SYMBOL_TABLE[c as usize]
}
