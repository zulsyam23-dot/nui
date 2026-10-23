//! Definisi AST NodeUI.

use crate::error::Loc;

pub type NodeId = usize;

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Node>,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
pub enum Node {
    Element(ElementNode),
    State(StateDecl),
    Import(ImportDecl),
    Text(TextNode),
}

impl Node {
    pub fn as_element(&self) -> Option<&ElementNode> {
        match self {
            Self::Element(node) => Some(node),
            _ => None,
        }
    }

    pub fn location(&self) -> Loc {
        match self {
            Self::Element(node) => node.loc,
            Self::State(node) => node.loc,
            Self::Import(node) => node.loc,
            Self::Text(node) => node.loc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ElementNode {
    pub id: NodeId,
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
    pub loc: Loc,
    pub is_page: bool,
    pub is_overlay: bool,
}

impl ElementNode {
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|attr| attr.name == name)
    }

    pub fn has_style(&self) -> bool {
        self.attribute("style").is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
pub struct StateDecl {
    pub name: String,
    pub value: String,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub names: Vec<String>,
    pub from: String,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
pub struct TextNode {
    pub value: String,
    pub loc: Loc,
}
