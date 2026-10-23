//! Parser NodeUI: token stream -> AST.

use crate::ast::{Attribute, ElementNode, ImportDecl, Node, NodeId, Program, StateDecl, TextNode};
use crate::error::{Loc, SyntaxError};
use crate::tokenizer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    next_id: NodeId,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            next_id: 0,
        }
    }

    pub fn parse(mut self) -> Result<Program, SyntaxError> {
        let mut body = Vec::new();
        while !self.is_at_end() {
            body.push(self.parse_item()?);
        }
        Ok(Program {
            body,
            loc: self.peek_loc(),
        })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn peek_loc(&self) -> Loc {
        self.peek().loc
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn match_kind(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<Token, SyntaxError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            Err(SyntaxError::new(
                format!("{message} (menemukan '{}')", token.kind.as_str()),
                token.loc,
            ))
        }
    }

    fn new_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn parse_item(&mut self) -> Result<Node, SyntaxError> {
        let token = self.peek();
        match token.kind {
            TokenKind::String => self.parse_text(),
            TokenKind::Keyword => match token.value.as_str() {
                "state" => self.parse_state(),
                "use" => self.parse_import(),
                "page" => self.parse_page(),
                "overlay" => self.parse_overlay(),
                _ => self.parse_node(),
            },
            _ => self.parse_node(),
        }
    }

    fn parse_text(&mut self) -> Result<Node, SyntaxError> {
        let token = self.consume(TokenKind::String, "Diperlukan teks string")?;
        Ok(Node::Text(TextNode {
            value: token.value,
            loc: token.loc,
        }))
    }

    fn parse_page(&mut self) -> Result<Node, SyntaxError> {
        let keyword = self.advance();
        let name = self.consume(TokenKind::Ident, "Diperlukan nama page setelah 'page'")?;
        self.parse_node_body(name, true, false, keyword.loc)
    }

    fn parse_overlay(&mut self) -> Result<Node, SyntaxError> {
        let keyword = self.advance();
        let name = self.consume(
            TokenKind::Ident,
            "Diperlukan nama overlay setelah 'overlay'",
        )?;
        self.parse_node_body(name, false, true, keyword.loc)
    }

    fn parse_node(&mut self) -> Result<Node, SyntaxError> {
        let name = self.consume(TokenKind::Ident, "Diperlukan nama node")?;
        let loc = name.loc;
        self.parse_node_body(name, false, false, loc)
    }

    fn parse_node_body(
        &mut self,
        name: Token,
        is_page: bool,
        is_overlay: bool,
        loc: Loc,
    ) -> Result<Node, SyntaxError> {
        let mut attributes = Vec::new();
        let mut children = Vec::new();

        if self.match_kind(TokenKind::LParen) {
            attributes.append(&mut self.parse_attributes()?);
            self.consume(
                TokenKind::RParen,
                "Diperlukan ')' untuk menutup daftar atribut",
            )?;
        }

        if self.match_kind(TokenKind::LBrace) {
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                if self.looks_like_attribute() {
                    attributes.push(self.parse_attribute()?);
                } else {
                    children.push(self.parse_item()?);
                }
            }
            self.consume(TokenKind::RBrace, "Diperlukan '}' untuk menutup tubuh node")?;
        }

        Ok(Node::Element(ElementNode {
            id: self.new_id(),
            name: name.value,
            attributes,
            children,
            loc,
            is_page,
            is_overlay,
        }))
    }

    fn looks_like_attribute(&self) -> bool {
        if !self.check(TokenKind::Ident) {
            return false;
        }
        self.peek_next().is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::Colon | TokenKind::Equals | TokenKind::StyleBlock
            )
        })
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, SyntaxError> {
        let mut attributes = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            attributes.push(self.parse_attribute()?);
            if self.match_kind(TokenKind::Comma) || self.check(TokenKind::Ident) {
                continue;
            }
            break;
        }
        Ok(attributes)
    }

    fn parse_attribute(&mut self) -> Result<Attribute, SyntaxError> {
        let name = self.consume(TokenKind::Ident, "Diperlukan nama atribut")?;
        let mut attr_name = name.value;

        if self.match_kind(TokenKind::Colon) {
            let suffix = self.consume(TokenKind::Ident, "Diperlukan nama atribut setelah ':'")?;
            attr_name.push(':');
            attr_name.push_str(&suffix.value);
        }

        let value = if self.match_kind(TokenKind::Equals) {
            self.consume_attribute_value()?
        } else if self.check(TokenKind::StyleBlock) {
            self.advance().value
        } else {
            String::new()
        };

        Ok(Attribute {
            name: attr_name,
            value,
            loc: name.loc,
        })
    }

    fn consume_attribute_value(&mut self) -> Result<String, SyntaxError> {
        let token = self.peek();
        match token.kind {
            TokenKind::String | TokenKind::Number | TokenKind::Ident | TokenKind::StyleBlock => {
                Ok(self.advance().value)
            }
            _ => Err(SyntaxError::new(
                format!("Nilai atribut tidak valid '{}'", token.kind.as_str()),
                token.loc,
            )),
        }
    }

    fn parse_state(&mut self) -> Result<Node, SyntaxError> {
        self.advance();
        let name = self.consume(TokenKind::Ident, "Diperlukan nama state")?;
        self.consume(TokenKind::Equals, "Diperlukan '=' setelah nama state")?;
        let value = self.consume(
            TokenKind::String,
            "Diperlukan nilai state berupa string literal",
        )?;
        Ok(Node::State(StateDecl {
            name: name.value,
            value: value.value,
            loc: name.loc,
        }))
    }

    fn parse_import(&mut self) -> Result<Node, SyntaxError> {
        self.advance();
        self.consume(TokenKind::LBrace, "Diperlukan '{' setelah 'use'")?;

        let mut names = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let name = self.consume(TokenKind::Ident, "Diperlukan nama impor")?;
            names.push(name.value);
            if !self.match_kind(TokenKind::Comma) {
                break;
            }
        }
        self.consume(
            TokenKind::RBrace,
            "Diperlukan '}' untuk menutup daftar impor",
        )?;

        let from = self.consume(TokenKind::Keyword, "Diperlukan kata kunci 'from'")?;
        if from.value != "from" {
            return Err(SyntaxError::new(
                format!(
                    "Diperlukan kata kunci 'from' tapi menemukan '{}'",
                    from.value
                ),
                from.loc,
            ));
        }

        let path = self.consume(TokenKind::String, "Diperlukan path impor berupa string")?;
        Ok(Node::Import(ImportDecl {
            names,
            from: path.value,
            loc: path.loc,
        }))
    }
}
