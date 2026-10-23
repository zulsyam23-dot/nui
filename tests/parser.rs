mod common;

use nodeui::ast::Node;

use common::{child, parse_source, read_fixture};

#[test]
fn parses_full_page_with_isolation_links() {
    let ast = parse_source(&read_fixture("home.nui"));
    assert_eq!(ast.body.len(), 1);

    let page = common::page(&ast);
    assert_eq!(page.name, "Home");
    assert!(page.is_page);

    let card = child(page, "Card");
    let title = child(card, "Title");
    assert_eq!(title.name, "Title");
    let row = child(card, "Row");
    let button = child(row, "Button");
    assert_eq!(button.name, "Button");

    let overlay = page
        .children
        .iter()
        .filter_map(Node::as_element)
        .find(|node| node.is_overlay)
        .expect("overlay harus ada");
    assert_eq!(overlay.name, "InfoModal");

    let state = page
        .children
        .iter()
        .find_map(|node| match node {
            Node::State(state) => Some(state),
            _ => None,
        })
        .expect("state harus ada");
    assert_eq!(state.name, "nama");
    assert_eq!(state.value, "");

    let import = page
        .children
        .iter()
        .find_map(|node| match node {
            Node::Import(import) => Some(import),
            _ => None,
        })
        .expect("import harus ada");
    assert_eq!(import.names, ["muatUser", "tampilkanModal"]);
    assert_eq!(import.from, "./logic.js");

    let click = button
        .attributes
        .iter()
        .find(|attr| attr.name == "on:click")
        .expect("on:click harus ada");
    assert_eq!(click.value, "muatUser");
}

#[test]
fn parses_text_binding() {
    let ast = parse_source(r#"page P { state nama = "x" Card { Text { "Halo {nama}" } } }"#);
    let page = common::page(&ast);
    let card = child(page, "Card");
    let text_element = child(card, "Text");
    let text = text_element
        .children
        .iter()
        .find_map(|node| match node {
            Node::Text(text) => Some(text),
            _ => None,
        })
        .expect("text harus ada");
    assert_eq!(text.value, "Halo {nama}");
}

#[test]
fn attributes_can_be_separated_by_comma_or_space() {
    let ast = parse_source(r#"page P { Button(on:click = f class = "utama" disabled) { "x" } }"#);
    let page = common::page(&ast);
    let button = child(page, "Button");
    let names: Vec<&str> = button
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();
    assert_eq!(names, ["on:click", "class", "disabled"]);
    assert_eq!(button.attributes[0].value, "f");
    assert_eq!(button.attributes[1].value, "utama");
    assert_eq!(button.attributes[2].value, "");
}

#[test]
fn style_block_supports_multiline_css() {
    let source = r#"page P {
        Card(style {
            color: red;
            border: 1px solid #ccc;

            span {
                color: blue;
            }
        }) {
            Text { "x" }
        }
    }"#;
    let ast = parse_source(source);
    let card = child(common::page(&ast), "Card");
    let style = card.attribute("style").expect("style harus ada");

    assert_eq!(
        style.value,
        "color: red;\nborder: 1px solid #ccc;\n\nspan {\n    color: blue;\n}"
    );
    assert_eq!(card.children.len(), 1);
    assert_eq!(child(card, "Text").name, "Text");
}

#[test]
fn style_block_can_be_an_attribute_in_the_body() {
    let source = r#"page P {
        Card {
            style = {
                color: red;
            }
            Text { "x" }
        }
    }"#;
    let ast = parse_source(source);
    let card = child(common::page(&ast), "Card");
    let style = card.attribute("style").expect("style harus ada");

    assert_eq!(style.value, "color: red;");
    assert_eq!(card.children.len(), 1);
}

#[test]
fn string_style_attribute_still_works() {
    let ast = parse_source(r#"page P { Card(style = "color:red") { "x" } }"#);
    let card = child(common::page(&ast), "Card");
    assert_eq!(card.attribute("style").unwrap().value, "color:red");
}

#[test]
fn node_without_body_is_allowed() {
    let ast = parse_source(r#"page P { Card { Input(placeholder = "cari") } }"#);
    let page = common::page(&ast);
    let card = child(page, "Card");
    let input = child(card, "Input");
    assert_eq!(input.name, "Input");
    assert!(input.children.is_empty());
}

#[test]
fn unbalanced_braces_report_line_and_column() {
    let err = parse_source_result(r#"page P { Card { Text { "x" } }"#).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("baris"),
        "pesan harus menyebut baris: {message}"
    );
    assert!(
        message.contains("kolom"),
        "pesan harus menyebut kolom: {message}"
    );
}

#[test]
fn sibling_nodes_stay_on_the_same_scope() {
    let ast = parse_source(r#"page P { Card { Text { "a" } } Button { "b" } }"#);
    let page = common::page(&ast);
    let names: Vec<&str> = page
        .children
        .iter()
        .filter_map(Node::as_element)
        .map(|node| node.name.as_str())
        .collect();
    assert_eq!(names, ["Card", "Button"]);
    assert_eq!(child(page, "Button").name, "Button");
}

fn parse_source_result(source: &str) -> Result<nodeui::ast::Program, nodeui::error::SyntaxError> {
    let tokens = nodeui::tokenizer::Tokenizer::new(source).tokenize()?;
    nodeui::parser::Parser::new(tokens).parse()
}
