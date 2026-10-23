mod common;

use common::generate_source;

#[test]
fn css_is_scoped_per_node() {
    let source = r#"page P {
        Card(style = "color:red") {
            Box(style = "span { color:blue }") { Text { "x" } }
        }
    }"#;
    let generated = generate_source(source);

    assert!(generated.css.contains(r#"[data-nui-scope="s1"]"#));
    assert!(generated.css.contains(r#"[data-nui-scope="s2"]"#));
    assert!(generated.css.contains(":not("));
    assert!(generated.css.contains(r#"[data-nui-scope="s2"] span:not("#));
}

#[test]
fn style_can_combine_declarations_and_rules() {
    let source = r#"page P {
        Shell(style = "color:red; padding:8px; h1 { font-size:20px }") {
            Title { "x" }
        }
    }"#;
    let generated = generate_source(source);

    assert!(generated
        .css
        .contains(r#"[data-nui-scope="s1"] { color:red; padding:8px }"#));
    assert!(generated.css.contains(r#"[data-nui-scope="s1"] h1:not("#));
    assert!(!generated.css.contains("] color:red"));
}

#[test]
fn style_block_is_scoped_like_a_style_attribute() {
    let source = r#"page P {
        Card(style {
            color: red;
            span { color: blue; }
        }) {
            Text { "x" }
        }
    }"#;
    let generated = generate_source(source);

    assert!(generated
        .css
        .contains(r#"[data-nui-scope="s1"] { color: red }"#));
    assert!(generated.css.contains(r#"[data-nui-scope="s1"] span:not("#));
}

#[test]
fn descendant_rules_ignore_outer_scope_boundaries() {
    let source = r#"page P {
        Shell(style { color: red; }) {
            Card(style {
                .item { color: blue; }
            }) {
                Item { "x" }
            }
        }
    }"#;
    let generated = generate_source(source);

    assert!(generated.css.contains(
        r#"[data-nui-scope="s2"] .item:not([data-nui-scope="s2"] [data-nui-scope]:not([data-nui-scope="s2"]))"#
    ));
}

#[test]
fn pseudo_elements_keep_the_scope_guard_before_them() {
    let source = r#"page P {
        Card(style {
            .item::after { content: attr(note); }
        }) {
            Item { "x" }
        }
    }"#;
    let generated = generate_source(source);

    assert!(!generated.css.contains("::after:not("));
    assert!(generated.css.contains("::after"));
    assert!(generated.css.contains("[data-nui-scope]:not("));
}

#[test]
fn unknown_nodes_become_block_elements() {
    let source = r#"page P {
        Card {
            NowPlaying { Title { "x" } }
            NextButton { "next" }
        }
    }"#;
    let generated = generate_source(source);

    assert!(generated.js.contains("h('div',"));
    assert!(generated.js.contains("h('button',"));
}

#[test]
fn binds_state_and_events() {
    let source = r#"page P {
        use { muat } from "./logic.js"
        state nama = ""
        Card {
            Text { "Halo {nama}" }
            Button(on:click = muat, id = "tombol") { "Muat" }
        }
    }"#;
    let generated = generate_source(source);

    assert!(generated
        .js
        .contains(r#"import { muat } from "./logic.js";"#));
    assert!(generated.js.contains(r#""nama": """#));
    assert!(generated.js.contains("(state.get()[\"nama\"] ?? '')"));
    assert!(generated.js.contains("state.subscribe("));
    assert!(generated.js.contains(r#"addEventListener("click""#));
    assert!(generated.js.contains(r#"ctx.refs["tombol"]"#));
}

#[test]
fn overlays_use_a_separate_layer() {
    let source = r#"page P {
        Card { Text { "utama" } }
        overlay Info { Panel { Text { "modal" } } }
    }"#;
    let generated = generate_source(source);

    assert!(generated.js.contains("createOverlayLayer('overlay')"));
    assert!(generated.js.contains("overlayLayer.body.appendChild("));
    assert!(generated.js.contains("overlayLayer.style.textContent ="));
}

#[test]
fn page_events_are_global_and_ready_is_deferred() {
    let source = r#"page P {
        use { siap, tekan } from "./logic.js"
        on:ready = siap
        on:keydown = tekan
        state x = "0"
        Text { "{x}" }
    }"#;
    let generated = generate_source(source);

    assert!(generated
        .js
        .contains("__ready.push(() => siap(null, ctx));"));
    assert!(generated
        .js
        .contains(r#"window.addEventListener("keydown", (e) => tekan(e, ctx));"#));
    assert!(generated.js.contains("for (const fn of app.ready) fn();"));
}

#[test]
fn css_and_js_strings_are_escaped() {
    let source = r#"page P { Text(style = "font-family:'Arial'") { "x" } }"#;
    let generated = generate_source(source);

    assert!(generated.js.contains(r#"font-family:'Arial'"#));
    assert!(!generated.js.contains("\\'Arial\\'"));
}
