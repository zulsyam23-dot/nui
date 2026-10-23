#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nodeui::ast::{ElementNode, Node, Program};
use nodeui::error::Issue;
use nodeui::generator::{generate, Generated};
use nodeui::parser::Parser;
use nodeui::tokenizer::Tokenizer;
use nodeui::validator::validate;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn fixture(name: &str) -> PathBuf {
    root().join("tests").join("fixtures").join(name)
}

pub fn parse_source(source: &str) -> Program {
    let tokens = Tokenizer::new(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("tokenize gagal: {err}"));
    Parser::new(tokens)
        .parse()
        .unwrap_or_else(|err| panic!("parse gagal: {err}"))
}

pub fn validate_source(source: &str) -> Vec<Issue> {
    validate(&parse_source(source))
}

pub fn generate_source(source: &str) -> Generated {
    let program = parse_source(source);
    let issues = validate(&program);
    assert!(issues.is_empty(), "validasi harus lolos: {issues:?}");
    let mut resolve = |from: &str| Ok::<String, nodeui::error::CompileError>(from.to_string());
    generate(&program, &mut resolve).expect("generate harus berhasil")
}

pub fn page(program: &Program) -> &ElementNode {
    program
        .body
        .iter()
        .filter_map(Node::as_element)
        .find(|node| node.is_page)
        .expect("page harus ada")
}

pub fn child<'a>(node: &'a ElementNode, name: &str) -> &'a ElementNode {
    node.children
        .iter()
        .filter_map(Node::as_element)
        .find(|child| child.name == name)
        .unwrap_or_else(|| panic!("node {name} harus ada"))
}

pub fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture(name)).expect("fixture harus bisa dibaca")
}

pub fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let dir = std::env::temp_dir().join(format!("nui-test-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir harus dibuat");
    dir
}

pub fn cleanup(path: &Path) {
    fs::remove_dir_all(path).ok();
}
