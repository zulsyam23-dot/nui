//! Generator NodeUI: AST -> JavaScript (DOM/Shadow DOM) + CSS ter-scope.

use crate::ast::{Attribute, ElementNode, ImportDecl, Node, Program, StateDecl};
use crate::error::CompileError;
use crate::scope::ScopeMap;

pub struct Generated {
    pub js: String,
    pub css: String,
}

#[derive(Debug)]
enum BindingPart {
    Text(String),
    Bind(String),
}

const TAG_MAP: &[(&str, &str)] = &[
    ("Text", "p"),
    ("Title", "h1"),
    ("Heading", "h2"),
    ("Subtitle", "p"),
    ("Caption", "figcaption"),
    ("Row", "div"),
    ("Column", "div"),
    ("Card", "div"),
    ("Panel", "div"),
    ("Box", "div"),
    ("Spacer", "div"),
    ("Badge", "span"),
    ("Label", "span"),
    ("Duration", "span"),
    ("Button", "button"),
    ("Input", "input"),
    ("List", "ul"),
    ("Playlist", "ul"),
    ("Item", "li"),
    ("Link", "a"),
    ("Image", "img"),
    ("Form", "form"),
    ("Table", "table"),
    ("Section", "section"),
    ("Header", "header"),
    ("Footer", "footer"),
    ("Nav", "nav"),
    ("Main", "main"),
    ("Aside", "aside"),
    ("Article", "article"),
    ("Strong", "strong"),
    ("Em", "em"),
    ("Code", "code"),
    ("Pre", "pre"),
    ("Select", "select"),
    ("Option", "option"),
    ("Textarea", "textarea"),
    ("Fieldset", "fieldset"),
    ("Legend", "legend"),
    ("Iframe", "iframe"),
    ("Canvas", "canvas"),
    ("Picture", "picture"),
    ("Video", "video"),
    ("Audio", "audio"),
    ("Source", "source"),
    ("Track", "track"),
    ("Progress", "progress"),
    ("Meter", "meter"),
    ("Slider", "input"),
];

const SUFFIX_TAGS: &[(&str, &str)] = &[
    ("Button", "button"),
    ("Input", "input"),
    ("Slider", "input"),
    ("Text", "p"),
    ("Label", "span"),
    ("Badge", "span"),
    ("Row", "div"),
    ("Column", "div"),
    ("Box", "div"),
    ("Panel", "div"),
    ("Card", "div"),
    ("Item", "li"),
    ("Link", "a"),
    ("Image", "img"),
    ("Video", "video"),
    ("Audio", "audio"),
    ("Sheet", "div"),
    ("Header", "header"),
    ("Footer", "footer"),
    ("Section", "section"),
    ("Duration", "span"),
    ("Subtitle", "p"),
];

const HTML_TAGS: &[&str] = &[
    "a",
    "abbr",
    "article",
    "aside",
    "audio",
    "b",
    "blockquote",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "main",
    "map",
    "mark",
    "menu",
    "meta",
    "meter",
    "nav",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "param",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "section",
    "select",
    "small",
    "source",
    "span",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

struct DefaultStyle {
    node: &'static str,
    key: &'static str,
    decls: &'static str,
}

const DEFAULT_STYLES: &[DefaultStyle] = &[
    DefaultStyle {
        node: "Row",
        key: "row",
        decls: "display:flex;flex-direction:row;gap:8px;align-items:center",
    },
    DefaultStyle {
        node: "Column",
        key: "column",
        decls: "display:flex;flex-direction:column;gap:8px",
    },
    DefaultStyle {
        node: "Spacer",
        key: "spacer",
        decls: "flex:1",
    },
    DefaultStyle {
        node: "List",
        key: "list",
        decls: "list-style:none;margin:0;padding:0",
    },
    DefaultStyle {
        node: "Playlist",
        key: "list",
        decls: "list-style:none;margin:0;padding:0",
    },
    DefaultStyle {
        node: "Item",
        key: "item",
        decls: "list-style:none",
    },
    DefaultStyle {
        node: "Text",
        key: "text",
        decls: "margin:0",
    },
    DefaultStyle {
        node: "Title",
        key: "title",
        decls: "font-size:1.4rem;font-weight:600;margin:0;line-height:1.25",
    },
    DefaultStyle {
        node: "Heading",
        key: "heading",
        decls: "font-size:1.05rem;font-weight:600;margin:0;line-height:1.3",
    },
    DefaultStyle {
        node: "Subtitle",
        key: "subtitle",
        decls: "margin:0;opacity:.75",
    },
    DefaultStyle {
        node: "Label",
        key: "label",
        decls: "margin:0",
    },
    DefaultStyle {
        node: "Duration",
        key: "duration",
        decls: "margin:0;opacity:.7",
    },
];

pub fn generate(
    program: &Program,
    resolve_import: &mut dyn FnMut(&str) -> Result<String, CompileError>,
) -> Result<Generated, CompileError> {
    let page = program
        .body
        .iter()
        .filter_map(Node::as_element)
        .find(|node| node.is_page)
        .ok_or_else(|| CompileError::new("Tidak ada node `page` sebagai titik entri."))?;

    let scopes = ScopeMap::analyze(page);
    let imports = collect_imports(page);
    let states = collect_states(page);

    let mut import_lines = Vec::with_capacity(imports.len());
    for import in imports {
        let path = resolve_import(&import.from)?;
        import_lines.push(format!(
            "import {{ {} }} from {};",
            import.names.join(", "),
            json_string(&path)
        ));
    }

    let mut page_css = Vec::new();
    let mut overlay_css = Vec::new();
    collect_css(page, false, &scopes, &mut page_css, &mut overlay_css);

    let css = format!("{}{}", default_node_css(), page_css.join("\n"));
    let overlay_css_text = format!("{}{}", default_node_css(), overlay_css.join("\n"));

    let mut seq = 0;
    let root = emit_node(page, "  ", &scopes, &mut seq);
    let has_overlay = has_overlay(page);

    let mut js = String::new();
    js.push_str("// Generated by NodeUI - jangan diedit manual.\n");
    if !import_lines.is_empty() {
        js.push_str(&import_lines.join("\n"));
        js.push('\n');
    }
    js.push_str(
        "import { createState, h, createRoot, createOverlayLayer, mount, setAttr } from './runtime.js';\n\n",
    );
    js.push_str("const state = createState({\n");
    for state in states {
        js.push_str(&format!(
            "  {}: {},\n",
            json_string(&state.name),
            json_string(&state.value)
        ));
    }
    js.push_str("});\n\nexport function createApp() {\n");
    js.push_str("  const __ready = [];\n");
    if has_overlay {
        js.push_str("  const overlayLayer = createOverlayLayer('overlay');\n");
    } else {
        js.push_str("  const overlayLayer = null;\n");
    }
    js.push_str("  const ctx = {\n");
    js.push_str("    state,\n");
    js.push_str("    setState: state.set,\n");
    js.push_str("    refs: {},\n");
    js.push_str("    overlay: overlayLayer ? overlayLayer.host : null,\n");
    js.push_str("    overlayBody: overlayLayer ? overlayLayer.body : null,\n");
    js.push_str("  };\n");
    js.push_str("  const root = createRoot();\n");
    js.push_str(&format!(
        "  root.host.setAttribute('data-nui-page', {});\n",
        json_string(&page.name)
    ));
    js.push_str(&root.code);
    js.push('\n');
    js.push_str(&format!("  root.body.appendChild({});\n", root.name));
    js.push_str(&format!(
        "  root.style.textContent = {};\n",
        json_string(&css)
    ));
    if has_overlay {
        js.push_str(&format!(
            "  overlayLayer.style.textContent = {};\n",
            json_string(&overlay_css_text)
        ));
    }
    js.push_str(
        "  return { host: root.host, overlay: overlayLayer ? overlayLayer.host : null, state, ready: __ready };\n",
    );
    js.push_str("}\n\n");
    js.push_str("const app = createApp();\n");
    js.push_str("mount(app.host);\n");
    js.push_str("for (const fn of app.ready) fn();\n");
    js.push_str("export { app, state };\n");

    Ok(Generated { js, css })
}

struct Emitted {
    name: String,
    code: String,
}

fn emit_node(node: &ElementNode, indent: &str, scopes: &ScopeMap, seq: &mut usize) -> Emitted {
    let var_name = format!("__n{}", *seq);
    *seq += 1;
    let child_indent = format!("{indent}  ");
    let mut lines = Vec::new();
    let mut props = Vec::new();

    if node.has_style() {
        let scope = scopes.id_of(node.id).unwrap_or_default();
        props.push(format!("'data-nui-scope': 's{scope}'"));
    }
    if let Some(default) = default_style(&node.name) {
        props.push(format!("'data-nui-default': '{}'", default.key));
    }

    let tag = if node.is_page {
        "div"
    } else {
        tag_name(&node.name)
    };
    lines.push(format!(
        "{indent}const {var_name} = h('{tag}', {{ {} }}, []);",
        props.join(", ")
    ));

    for attr in &node.attributes {
        emit_attribute(node, attr, &var_name, indent, &mut lines);
    }

    for child in &node.children {
        match child {
            Node::Text(text) => {
                let parts = split_bindings(&text.value);
                if parts.is_empty() {
                    continue;
                }
                let text_var = format!("__t{}", *seq);
                *seq += 1;
                let expr = binding_expression(&parts);
                lines.push(format!(
                    "{child_indent}const {text_var} = h('#text', {{ value: {expr} }});"
                ));
                if parts
                    .iter()
                    .any(|part| matches!(part, BindingPart::Bind(_)))
                {
                    lines.push(format!(
                        "{child_indent}state.subscribe(() => {{ {text_var}.textContent = {expr}; }});"
                    ));
                }
                lines.push(format!("{child_indent}{var_name}.appendChild({text_var});"));
            }
            Node::Element(element) => {
                let child = emit_node(element, &child_indent, scopes, seq);
                lines.push(child.code);
                if element.is_overlay {
                    lines.push(format!(
                        "{child_indent}overlayLayer.body.appendChild({});",
                        child.name
                    ));
                } else {
                    lines.push(format!(
                        "{child_indent}{var_name}.appendChild({});",
                        child.name
                    ));
                }
            }
            Node::State(_) | Node::Import(_) => {}
        }
    }

    Emitted {
        name: var_name,
        code: lines.join("\n"),
    }
}

fn emit_attribute(
    node: &ElementNode,
    attr: &Attribute,
    var_name: &str,
    indent: &str,
    lines: &mut Vec<String>,
) {
    if attr.name == "style" {
        return;
    }

    if let Some(event) = attr.name.strip_prefix("on:") {
        if event == "ready" {
            lines.push(format!(
                "{indent}__ready.push(() => {}(null, ctx));",
                attr.value
            ));
        } else if node.is_page {
            lines.push(format!(
                "{indent}window.addEventListener({}, (e) => {}(e, ctx));",
                json_string(event),
                attr.value
            ));
        } else {
            lines.push(format!(
                "{indent}{var_name}.addEventListener({}, (e) => {}(e, ctx));",
                json_string(event),
                attr.value
            ));
        }
        return;
    }

    if attr.name == "id" {
        lines.push(format!(
            "{indent}ctx.refs[{}] = {var_name};",
            json_string(&attr.value)
        ));
    }

    let parts = split_bindings(&attr.value);
    if parts.is_empty() {
        lines.push(format!(
            "{indent}setAttr({var_name}, {}, '');",
            json_string(&attr.name)
        ));
    } else if let [BindingPart::Text(value)] = parts.as_slice() {
        lines.push(format!(
            "{indent}setAttr({var_name}, {}, {});",
            json_string(&attr.name),
            json_string(value)
        ));
    } else {
        emit_bound_attribute(attr, var_name, indent, lines, &parts);
    }
}

fn emit_bound_attribute(
    attr: &Attribute,
    var_name: &str,
    indent: &str,
    lines: &mut Vec<String>,
    parts: &[BindingPart],
) {
    let expr = binding_expression(parts);
    lines.push(format!(
        "{indent}setAttr({var_name}, {}, {expr});",
        json_string(&attr.name)
    ));
    lines.push(format!(
        "{indent}state.subscribe(() => setAttr({var_name}, {}, {expr}));",
        json_string(&attr.name)
    ));
}

fn binding_expression(parts: &[BindingPart]) -> String {
    let pieces: Vec<String> = parts
        .iter()
        .map(|part| match part {
            BindingPart::Text(value) => json_string(value),
            BindingPart::Bind(expr) => {
                format!("(state.get()[{}] ?? '')", json_string(expr))
            }
        })
        .collect();
    pieces.join(" + ")
}

fn collect_imports(page: &ElementNode) -> Vec<&ImportDecl> {
    fn walk<'a>(node: &'a ElementNode, out: &mut Vec<&'a ImportDecl>) {
        for child in &node.children {
            match child {
                Node::Import(import) => out.push(import),
                Node::Element(element) => walk(element, out),
                Node::State(_) | Node::Text(_) => {}
            }
        }
    }

    let mut out = Vec::new();
    walk(page, &mut out);
    out
}

fn collect_states(page: &ElementNode) -> Vec<&StateDecl> {
    fn walk<'a>(node: &'a ElementNode, out: &mut Vec<&'a StateDecl>) {
        for child in &node.children {
            match child {
                Node::State(state) => out.push(state),
                Node::Element(element) => walk(element, out),
                Node::Import(_) | Node::Text(_) => {}
            }
        }
    }

    let mut out = Vec::new();
    walk(page, &mut out);
    out
}

fn has_overlay(node: &ElementNode) -> bool {
    node.is_overlay
        || node.children.iter().any(|child| match child {
            Node::Element(element) => has_overlay(element),
            _ => false,
        })
}

fn collect_css(
    node: &ElementNode,
    inside_overlay: bool,
    scopes: &ScopeMap,
    page_css: &mut Vec<String>,
    overlay_css: &mut Vec<String>,
) {
    let inside = inside_overlay || node.is_overlay;
    if let Some(style) = node.attribute("style") {
        let scope = scopes.id_of(node.id).unwrap_or_default();
        let chunk = scope_css(&style.value, scope);
        if inside {
            overlay_css.push(chunk);
        } else {
            page_css.push(chunk);
        }
    }

    for child in &node.children {
        if let Node::Element(element) = child {
            collect_css(element, inside, scopes, page_css, overlay_css);
        }
    }
}

fn tag_name(name: &str) -> &'static str {
    if let Some((_, tag)) = TAG_MAP.iter().find(|(node, _)| *node == name) {
        return tag;
    }

    if let Some(tag) = HTML_TAGS.iter().find(|tag| name.eq_ignore_ascii_case(tag)) {
        return tag;
    }

    for (suffix, tag) in SUFFIX_TAGS {
        if name.len() > suffix.len() && name.ends_with(suffix) {
            return tag;
        }
    }

    "div"
}

fn default_style(name: &str) -> Option<&'static DefaultStyle> {
    DEFAULT_STYLES.iter().find(|style| style.node == name)
}

fn default_node_css() -> String {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for style in DEFAULT_STYLES {
        if seen.contains(&style.key) {
            continue;
        }
        seen.push(style.key);
        out.push(format!(
            "[data-nui-default=\"{}\"] {{ {} }}",
            style.key, style.decls
        ));
    }
    out.join("\n") + "\n"
}

fn css_guard(scope: usize) -> String {
    let inner = format!(
        "[data-nui-scope=\"s{scope}\"] [data-nui-scope]:not([data-nui-scope=\"s{scope}\"])"
    );
    format!(":not({inner}):not({inner} *)")
}

fn guard_selector(selector: &str, guard: &str) -> String {
    match selector.find("::") {
        Some(index) => format!("{}{guard}{}", &selector[..index], &selector[index..]),
        None => format!("{selector}{guard}"),
    }
}

fn scope_css(css: &str, scope: usize) -> String {
    let text = css.trim();
    if text.is_empty() {
        return String::new();
    }

    let node_selector = format!("[data-nui-scope=\"s{scope}\"]");
    let guard = css_guard(scope);
    let mut out = Vec::new();
    let mut last = 0;

    while let Some(open_rel) = text[last..].find('{') {
        let open = last + open_rel;
        let Some(close_rel) = text[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + close_rel;
        let before = &text[last..open];
        let (decls, selector) = split_decls_and_selector(before);
        flush_declarations(&mut out, &node_selector, &decls);

        let body = text[open + 1..close].trim();
        let selectors: Vec<&str> = selector
            .split(',')
            .map(str::trim)
            .filter(|selector| !selector.is_empty())
            .collect();

        if !selectors.is_empty() && !body.is_empty() {
            let scoped = selectors
                .iter()
                .map(|selector| {
                    let combined = if *selector == "&" {
                        node_selector.clone()
                    } else if let Some(rest) = selector.strip_prefix('&') {
                        format!("{node_selector}{rest}")
                    } else {
                        format!("{node_selector} {selector}")
                    };
                    guard_selector(&combined, &guard)
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!("{scoped} {{ {body} }}"));
        }

        last = close + 1;
    }

    flush_declarations(&mut out, &node_selector, text[last..].trim());
    out.join("\n") + "\n"
}

fn flush_declarations(out: &mut Vec<String>, node_selector: &str, decls: &str) {
    let clean = decls
        .replace(['{', '}'], " ")
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string();
    if !clean.is_empty() {
        out.push(format!("{node_selector} {{ {clean} }}"));
    }
}

fn split_decls_and_selector(text: &str) -> (String, String) {
    let mut parts = split_top_level(text, ';');
    let selector = parts.pop().unwrap_or_default().trim().to_string();
    let decls = parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    (decls, selector)
}

fn split_top_level(text: &str, separator: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if ch == separator && depth == 0 {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    out.push(current);
    out
}

fn split_bindings(text: &str) -> Vec<BindingPart> {
    if text.is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut last = 0;
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
            if index > last {
                out.push(BindingPart::Text(chars[last..index].iter().collect()));
            }
            let expr: String = chars[start..end].iter().collect();
            out.push(BindingPart::Bind(expr.trim().to_string()));
            last = end + 1;
            index = end + 1;
        } else {
            index = start;
        }
    }

    if last < chars.len() {
        out.push(BindingPart::Text(chars[last..].iter().collect()));
    }
    out
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
