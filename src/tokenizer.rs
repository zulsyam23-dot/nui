//! Tokenizer NodeUI (.nui).

use crate::error::{Loc, SyntaxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Keyword,
    String,
    Number,
    StyleBlock,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Equals,
    Colon,
    Comma,
    Hash,
    Dot,
    Eof,
}

impl TokenKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ident => "IDENT",
            Self::Keyword => "KEYWORD",
            Self::String => "STRING",
            Self::Number => "NUMBER",
            Self::StyleBlock => "STYLE_BLOCK",
            Self::LBrace => "LBRACE",
            Self::RBrace => "RBRACE",
            Self::LParen => "LPAREN",
            Self::RParen => "RPAREN",
            Self::Equals => "EQUALS",
            Self::Colon => "COLON",
            Self::Comma => "COMMA",
            Self::Hash => "HASH",
            Self::Dot => "DOT",
            Self::Eof => "EOF",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub loc: Loc,
}

impl Token {
    fn new(kind: TokenKind, value: impl Into<String>, loc: Loc) -> Self {
        Self {
            kind,
            value: value.into(),
            loc,
        }
    }
}

fn is_keyword(value: &str) -> bool {
    matches!(value, "use" | "from" | "state" | "page" | "overlay")
}

fn dedent_block(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return String::new();
    };
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .unwrap_or(start);
    let block = &lines[start..=end];
    let indent = block
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    block
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                return String::new();
            }
            let stripped = line.get(indent..).unwrap_or_else(|| line.trim_start());
            stripped.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Tokenizer {
    pub fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, SyntaxError> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            if self.skip_comment() {
                continue;
            }

            let loc = self.loc();
            let Some(ch) = self.peek() else { break };

            let token = match ch {
                '"' | '\'' => self.read_string()?,
                '{' => self.single(TokenKind::LBrace, ch),
                '}' => self.single(TokenKind::RBrace, ch),
                '(' => self.single(TokenKind::LParen, ch),
                ')' => self.single(TokenKind::RParen, ch),
                '=' => self.single(TokenKind::Equals, ch),
                ':' => self.single(TokenKind::Colon, ch),
                ',' => self.single(TokenKind::Comma, ch),
                '#' => self.single(TokenKind::Hash, ch),
                '.' => self.single(TokenKind::Dot, ch),
                _ if is_ident_start(ch) => self.read_ident(),
                _ if ch.is_ascii_digit() => self.read_number(),
                _ => {
                    return Err(SyntaxError::new(
                        format!("Karakter tidak dikenal '{ch}'"),
                        loc,
                    ))
                }
            };

            if token.kind == TokenKind::Ident && token.value == "style" {
                if let Some(mut extra) = self.try_style_block(token.loc)? {
                    tokens.push(token);
                    tokens.append(&mut extra);
                    continue;
                }
            }

            tokens.push(token);
        }

        tokens.push(Token::new(TokenKind::Eof, "", self.loc()));
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn loc(&self) -> Loc {
        Loc::new(self.line, self.col)
    }

    fn single(&mut self, kind: TokenKind, ch: char) -> Token {
        let loc = self.loc();
        self.next();
        Token::new(kind, ch.to_string(), loc)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    fn read_ident(&mut self) -> Token {
        let loc = self.loc();
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            let hyphen = ch == '-' && self.peek_next().is_some_and(is_ident_continue);
            if is_ident_continue(ch) || hyphen {
                value.push(ch);
                self.next();
            } else {
                break;
            }
        }

        let kind = if is_keyword(&value) {
            TokenKind::Keyword
        } else {
            TokenKind::Ident
        };
        Token::new(kind, value, loc)
    }

    fn read_string(&mut self) -> Result<Token, SyntaxError> {
        let Some(quote) = self.next() else {
            return Err(SyntaxError::new("String tidak ditutup", self.loc()));
        };
        let loc = self.loc();
        let mut value = String::new();

        loop {
            let Some(ch) = self.peek() else {
                return Err(SyntaxError::new("String tidak ditutup", loc));
            };
            if ch == quote {
                self.next();
                break;
            }
            if ch == '\\' {
                self.next();
                let Some(escaped) = self.next() else {
                    return Err(SyntaxError::new("String tidak ditutup", loc));
                };
                value.push(match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    other => other,
                });
            } else {
                value.push(ch);
                self.next();
            }
        }

        Ok(Token::new(TokenKind::String, value, loc))
    }

    fn read_number(&mut self) -> Token {
        let loc = self.loc();
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.next();
            } else {
                break;
            }
        }
        Token::new(TokenKind::Number, value, loc)
    }

    fn restore(&mut self, mark: (usize, usize, usize)) {
        self.pos = mark.0;
        self.line = mark.1;
        self.col = mark.2;
    }

    fn try_style_block(&mut self, loc: Loc) -> Result<Option<Vec<Token>>, SyntaxError> {
        let mark = (self.pos, self.line, self.col);
        self.skip_whitespace();

        if self.peek() == Some('{') {
            let css = self.read_style_block(loc)?;
            return Ok(Some(vec![Token::new(TokenKind::StyleBlock, css, loc)]));
        }

        if self.peek() == Some('=') {
            let eq_loc = self.loc();
            self.next();
            self.skip_whitespace();
            if self.peek() == Some('{') {
                let css = self.read_style_block(loc)?;
                return Ok(Some(vec![
                    Token::new(TokenKind::Equals, "=", eq_loc),
                    Token::new(TokenKind::StyleBlock, css, loc),
                ]));
            }
        }

        self.restore(mark);
        Ok(None)
    }

    fn read_style_block(&mut self, loc: Loc) -> Result<String, SyntaxError> {
        self.next();
        let mut value = String::new();
        let mut depth = 1usize;
        let mut quote: Option<char> = None;

        while let Some(ch) = self.peek() {
            if let Some(active) = quote {
                value.push(ch);
                self.next();
                if ch == '\\' {
                    if let Some(escaped) = self.peek() {
                        value.push(escaped);
                        self.next();
                    }
                } else if ch == active {
                    quote = None;
                }
                continue;
            }

            match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    value.push(ch);
                    self.next();
                }
                '/' if self.peek_next() == Some('*') => {
                    value.push('/');
                    self.next();
                    value.push('*');
                    self.next();
                    while let Some(inner) = self.peek() {
                        value.push(inner);
                        self.next();
                        if inner == '*' && self.peek() == Some('/') {
                            value.push('/');
                            self.next();
                            break;
                        }
                    }
                }
                '{' => {
                    depth += 1;
                    value.push(ch);
                    self.next();
                }
                '}' => {
                    depth -= 1;
                    self.next();
                    if depth == 0 {
                        break;
                    }
                    value.push('}');
                }
                _ => {
                    value.push(ch);
                    self.next();
                }
            }
        }

        if depth != 0 {
            return Err(SyntaxError::new(
                "Style block tidak ditutup dengan '}'",
                loc,
            ));
        }

        Ok(dedent_block(&value))
    }

    fn skip_comment(&mut self) -> bool {
        if self.peek() == Some('/') && self.peek_next() == Some('/') {
            while let Some(ch) = self.peek() {
                if ch == '\n' {
                    break;
                }
                self.next();
            }
            return true;
        }

        if self.peek() == Some('/') && self.peek_next() == Some('*') {
            self.next();
            self.next();
            while let Some(ch) = self.peek() {
                if ch == '*' && self.peek_next() == Some('/') {
                    self.next();
                    self.next();
                    break;
                }
                self.next();
            }
            return true;
        }

        false
    }
}
