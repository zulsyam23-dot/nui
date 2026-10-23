//! Analisis scope bersama untuk validator dan generator.
//!
//! Setiap elemen adalah scope. State hanya terlihat oleh elemen itu dan
//! turunannya, sesuai aturan isolasi node.

use std::collections::HashMap;

use crate::ast::{ElementNode, Node, NodeId};
use crate::error::Loc;

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: usize,
    pub node_id: NodeId,
    pub parent: Option<usize>,
    pub declared: Vec<(String, Loc)>,
}

#[derive(Debug, Default)]
pub struct ScopeMap {
    by_node: HashMap<NodeId, usize>,
    scopes: Vec<Scope>,
}

impl ScopeMap {
    pub fn analyze(page: &ElementNode) -> Self {
        let mut map = Self::default();
        map.visit(page, None);
        map
    }

    fn visit(&mut self, node: &ElementNode, parent: Option<usize>) {
        let index = self.scopes.len();
        self.scopes.push(Scope {
            id: index,
            node_id: node.id,
            parent,
            declared: Vec::new(),
        });
        self.by_node.insert(node.id, index);

        for child in &node.children {
            match child {
                Node::Element(element) => self.visit(element, Some(index)),
                Node::State(state) => {
                    self.scopes[index]
                        .declared
                        .push((state.name.clone(), state.loc));
                }
                Node::Import(_) | Node::Text(_) => {}
            }
        }
    }

    pub fn id_of(&self, node_id: NodeId) -> Option<usize> {
        self.by_node.get(&node_id).copied()
    }

    /// Scope tempat `name` dideklarasikan, mulai dari node lalu ke ancestor.
    pub fn resolve(&self, node_id: NodeId, name: &str) -> Option<&Scope> {
        let mut current = self.by_node.get(&node_id).copied();
        while let Some(index) = current {
            let scope = &self.scopes[index];
            if scope.declared.iter().any(|(declared, _)| declared == name) {
                return Some(scope);
            }
            current = scope.parent;
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = &Scope> {
        self.scopes.iter()
    }
}
