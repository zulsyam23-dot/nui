mod common;

use common::{read_fixture, validate_source};

#[test]
fn full_fixture_is_valid() {
    let issues = validate_source(&read_fixture("home.nui"));
    assert!(issues.is_empty(), "harus valid: {issues:?}");
}

#[test]
fn unknown_binding_is_rejected() {
    let issues = validate_source(r#"page P { Card { Text { "Halo {nama}" } } }"#);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("nama"));
    assert!(issues[0].line >= 1);
    assert!(issues[0].col >= 1);
}

#[test]
fn parent_cannot_read_state_declared_in_child() {
    let issues =
        validate_source(r#"page P { Card { Text { "x = {x}" } Button { state x = "1" } } }"#);
    assert!(!issues.is_empty());
    assert!(issues
        .iter()
        .any(|issue| issue.message.contains("isolasi node")));
}

#[test]
fn descendant_can_read_ancestor_state() {
    let issues = validate_source(r#"page P { state a = "1" Card { Button { Text { "{a}" } } } }"#);
    assert!(issues.is_empty(), "harus valid: {issues:?}");
}

#[test]
fn heavy_logic_is_rejected() {
    for source in [
        r#"page P { Card { Button(on:click = "fetch('/api')") { "x" } } }"#,
        r#"page P { Card { Button(on:click = "for(;;){}") { "x" } } }"#,
        r#"page P { Card { state x = "1 + 2;" Text { "y" } } }"#,
    ] {
        let issues = validate_source(source);
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("Logika berat")),
            "harus menolak logika berat: {source}"
        );
    }
}

#[test]
fn error_has_line_and_column() {
    let issues = validate_source(r#"page P { Card { Text { "Halo {tidakAda}" } } }"#);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].line >= 1);
    assert!(issues[0].col >= 1);
    assert!(issues[0].to_string().contains("baris"));
}

#[test]
fn missing_page_is_rejected() {
    let issues = validate_source(r#"Card { Text { "x" } }"#);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("page"));
}

#[test]
fn multiple_pages_are_rejected() {
    let issues = validate_source(r#"page A { Text { "a" } } page B { Text { "b" } }"#);
    assert!(issues
        .iter()
        .any(|issue| issue.message.contains("satu node `page`")));
}

#[test]
fn handler_must_be_imported() {
    let issues = validate_source(r#"page P { Button(on:click = belum) { "x" } }"#);
    assert!(issues.iter().any(|issue| issue.message.contains("belum")));
}

#[test]
fn imported_names_must_be_valid_javascript_identifiers() {
    let issues = validate_source(r#"page P { use { class } from "./logic.js" Text { "x" } }"#);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("identifier JavaScript")),
        "harus menolak nama impor tidak valid: {issues:?}"
    );
}

#[test]
fn handlers_must_be_valid_javascript_identifiers() {
    let issues = validate_source(r#"page P { Button(on:click = class) { "x" } }"#);
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("JavaScript")),
        "harus menolak handler tidak valid: {issues:?}"
    );
}
