//! Validator NodeUI.
//!
//! Aturan:
//!  1. Isolasi node  - state hanya terlihat di node itu dan turunannya.
//!  2. Logika berat  - tidak ada loop/fetch/fungsi inline di dalam .nui.
//!  3. Konsistensi   - nama node kapital, handler diimpor, state unik.
//!  4. Overlay       - pengecualian isolasi, dirender di layer terpisah.

use std::collections::{HashMap, HashSet};

use crate::ast::{ElementNode, Node, Program};
use crate::error::{Issue, Loc};
use crate::scope::ScopeMap;

pub fn validate(program: &Program) -> Vec<Issue> {
    let mut issues = Vec::new();
    let pages: Vec<&ElementNode> = program
        .body
        .iter()
        .filter_map(Node::as_element)
        .filter(|node| node.is_page)
        .collect();

    if pages.is_empty() {
        issues.push(Issue::new(
            "Tidak ada node `page` - berkas .nui harus punya satu page sebagai titik entri",
            Loc::new(1, 1),
        ));
        return issues;
    }
    if pages.len() > 1 {
        issues.push(Issue::new(
            "Hanya boleh ada satu node `page` sebagai titik entri",
            pages[1].loc,
        ));
    }

    for page in pages {
        validate_page(page, &mut issues);
    }
    issues
}

fn validate_page(page: &ElementNode, issues: &mut Vec<Issue>) {
    let scopes = ScopeMap::analyze(page);
    let imported = collect_imports(page);

    let mut state_names: HashMap<String, Loc> = HashMap::new();
    for scope in scopes.iter() {
        for (name, loc) in &scope.declared {
            if let Some(previous) = state_names.get(name) {
                issues.push(Issue::new(
                    format!(
                        "Nama state '{name}' bentrok dengan deklarasi di baris {}",
                        previous.line
                    ),
                    *loc,
                ));
            } else {
                state_names.insert(name.clone(), *loc);
            }
        }
    }

    walk(page, &scopes, &imported, issues);
}

fn collect_imports(page: &ElementNode) -> HashSet<String> {
    fn walk(node: &ElementNode, out: &mut HashSet<String>) {
        for child in &node.children {
            match child {
                Node::Import(import) => {
                    out.extend(import.names.iter().cloned());
                }
                Node::Element(element) => walk(element, out),
                Node::State(_) | Node::Text(_) => {}
            }
        }
    }

    let mut out = HashSet::new();
    walk(page, &mut out);
    out
}

fn walk(
    node: &ElementNode,
    scopes: &ScopeMap,
    imported: &HashSet<String>,
    issues: &mut Vec<Issue>,
) {
    if !node.name.starts_with(|ch: char| ch.is_ascii_uppercase()) {
        issues.push(Issue::new(
            format!(
                "Nama node '{}' harus diawali huruf kapital (mis. Card, Text, Button)",
                node.name
            ),
            node.loc,
        ));
    }

    for attr in &node.attributes {
        if attr.name == "style" {
            if has_heavy_css(&attr.value) {
                issues.push(Issue::new(
                    "CSS pada atribut 'style' tidak boleh berisi logika pemrograman",
                    attr.loc,
                ));
            }
        } else {
            check_binding(&attr.value, node, scopes, issues, attr.loc);
            check_heavy(
                &attr.value,
                attr.loc,
                &format!("atribut '{}'", attr.name),
                issues,
            );
        }

        if attr.name.starts_with("on:") {
            if !is_js_identifier(&attr.value) {
                issues.push(Issue::new(
                    format!(
                        "Handler '{}' bukan nama fungsi JavaScript yang valid",
                        attr.value
                    ),
                    attr.loc,
                ));
            } else if !imported.contains(&attr.value) {
                issues.push(Issue::new(
                    format!(
                        "Handler '{}' tidak ditemukan. Impor dengan 'use {{ {} }} from \"./logic.js\"'.",
                        attr.value, attr.value
                    ),
                    attr.loc,
                ));
            }
        }
    }

    for child in &node.children {
        match child {
            Node::Element(element) => walk(element, scopes, imported, issues),
            Node::State(state) => {
                check_heavy(
                    &state.value,
                    state.loc,
                    &format!("state '{}'", state.name),
                    issues,
                );
            }
            Node::Import(import) => {
                for name in &import.names {
                    if !is_js_identifier(name) {
                        issues.push(Issue::new(
                            format!("Nama impor '{name}' bukan identifier JavaScript yang valid"),
                            import.loc,
                        ));
                    }
                }
                if !(import.from.starts_with("./") || import.from.starts_with("../")) {
                    issues.push(Issue::new(
                        format!(
                            "Path impor harus relatif dan diawali './' atau '../' (ditemukan '{}')",
                            import.from
                        ),
                        import.loc,
                    ));
                }
                if !(import.from.ends_with(".js") || import.from.ends_with(".ts")) {
                    issues.push(Issue::new(
                        format!(
                            "Impor hanya boleh dari berkas .js atau .ts (ditemukan '{}')",
                            import.from
                        ),
                        import.loc,
                    ));
                }
            }
            Node::Text(text) => {
                check_binding(&text.value, node, scopes, issues, text.loc);
            }
        }
    }
}

fn is_js_identifier(name: &str) -> bool {
    const RESERVED: &[&str] = &[
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "implements",
        "import",
        "in",
        "instanceof",
        "interface",
        "let",
        "new",
        "null",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
    ];
    if RESERVED.contains(&name) {
        return false;
    }

    let mut chars = name.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn check_binding(
    text: &str,
    node: &ElementNode,
    scopes: &ScopeMap,
    issues: &mut Vec<Issue>,
    loc: Loc,
) {
    for name in binding_names(text) {
        if scopes.resolve(node.id, &name).is_none() {
            issues.push(Issue::new(
                format!(
                    "Binding '{name}' di luar scope node ini dan induknya (pelanggaran isolasi node)"
                ),
                loc,
            ));
        }
    }
}

fn check_heavy(text: &str, loc: Loc, what: &str, issues: &mut Vec<Issue>) {
    if has_heavy_logic(text) {
        issues.push(Issue::new(
            format!(
                "Logika berat tidak diizinkan di dalam .nui ({what}). Pindahkan ke file JS/TS lalu impor dengan 'use'."
            ),
            loc,
        ));
    }
}

fn binding_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] != '{' {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != '}' {
            end += 1;
        }
        if end < chars.len() && end > start {
            let name: String = chars[start..end].iter().collect();
            names.push(name.trim().to_string());
            index = end + 1;
        } else {
            index = start;
        }
    }

    names
}

fn strip_bindings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    for ch in text.chars() {
        match ch {
            '{' => inside = true,
            '}' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn contains_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let target = word.as_bytes();
    if target.is_empty() || target.len() > bytes.len() {
        return false;
    }

    bytes
        .windows(target.len())
        .enumerate()
        .any(|(index, part)| {
            if part != target {
                return false;
            }
            let before_ok = index == 0 || !is_word_byte(bytes[index - 1]);
            let after = index + target.len();
            let after_ok = after == bytes.len() || !is_word_byte(bytes[after]);
            before_ok && after_ok
        })
}

fn has_heavy_logic(text: &str) -> bool {
    let text = strip_bindings(text);
    [
        "fetch",
        "XMLHttpRequest",
        "for",
        "while",
        "do",
        "function",
        "return",
    ]
    .iter()
    .any(|word| contains_word(&text, word))
        || text.contains("=>")
        || text.contains(';')
        || text.contains('{')
        || text.contains('}')
        || text.contains("++")
        || text.contains("==")
        || text.contains("!=")
        || text.contains(">=")
        || text.contains("<=")
}

fn has_heavy_css(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["fetch", "for", "while", "function"]
        .iter()
        .any(|word| contains_word(&lower, word))
        || lower.contains("javascript:")
        || lower.contains("@import")
        || lower.contains("=>")
        || has_expression_call(&lower)
        || has_new_function(&lower)
}

fn has_expression_call(lower: &str) -> bool {
    let mut rest = lower;
    while let Some(index) = rest.find("expression") {
        let after = &rest[index + "expression".len()..];
        if after.trim_start().starts_with('(') {
            return true;
        }
        rest = after;
    }
    false
}

fn has_new_function(lower: &str) -> bool {
    let mut rest = lower;
    while let Some(index) = rest.find("new") {
        let before_ok = index == 0 || !is_word_byte(rest.as_bytes()[index - 1]);
        let after = &rest[index + 3..];
        let after_ok = after
            .as_bytes()
            .first()
            .map_or(true, |byte| !is_word_byte(*byte));
        if before_ok && after_ok {
            let tail = after.trim_start();
            if tail.starts_with("function")
                && tail
                    .as_bytes()
                    .get("function".len())
                    .map_or(true, |byte| !is_word_byte(*byte))
            {
                return true;
            }
        }
        rest = after;
    }
    false
}
