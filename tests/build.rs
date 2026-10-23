mod common;

use std::fs;
use std::process::Command;

use nodeui::compiler::{build, compile};
use nodeui::error::CompileError;

use common::{cleanup, fixture, root, temp_dir};

#[test]
fn build_writes_all_artifacts() {
    let out = temp_dir("build");
    let result = build(&fixture("home.nui"), Some(&out)).expect("build harus berhasil");

    for file in ["output.js", "runtime.js", "style.css", "index.html"] {
        assert!(out.join(file).is_file(), "{file} harus ada");
    }
    assert!(result.js.contains("createState"));
    assert!(result.css.contains("data-nui-scope"));
    assert!(fs::read_to_string(out.join("index.html"))
        .expect("index.html")
        .contains("output.js"));

    cleanup(&out);
}

#[test]
fn build_copies_external_logic_and_rewrites_import() {
    let out = temp_dir("logic");
    build(&fixture("home.nui"), Some(&out)).expect("build harus berhasil");

    assert!(out.join("logic.js").is_file());
    let output = fs::read_to_string(out.join("output.js")).expect("output.js");
    assert!(output.contains(r#"from "./logic.js""#));

    cleanup(&out);
}

#[test]
fn build_transpiles_typescript_when_available() {
    let out = temp_dir("ts");
    match build(&fixture("ts/home.nui"), Some(&out)) {
        Ok(_) => {
            let logic = fs::read_to_string(out.join("logic.js")).expect("logic.js");
            assert!(!logic.contains(": Ctx"));
            assert!(logic.contains("function muatUser"));
        }
        Err(err) if err.message.starts_with("Tidak dapat mentranspile") => {
            eprintln!("TypeScript tidak tersedia; test transpile dilewati");
        }
        Err(err) => panic!("build TS gagal: {err}"),
    }
    cleanup(&out);
}

#[test]
fn validation_errors_keep_line_and_column() {
    let err = compile(&fixture("errors.nui")).expect_err("harus gagal");
    assert_eq!(err.issues.len(), 3);
    assert_eq!(err.issues[0].line, 4);
    assert!(err.issues[0].col > 0);
}

#[test]
fn failed_build_keeps_previous_output() {
    let out = temp_dir("keep");
    build(&fixture("home.nui"), Some(&out)).expect("build awal harus berhasil");
    assert!(out.join("output.js").is_file());

    let err: CompileError = build(&fixture("errors.nui"), Some(&out)).expect_err("harus gagal");
    assert_eq!(err.issues.len(), 3);
    assert!(out.join("output.js").is_file());

    cleanup(&out);
}

#[test]
fn build_copies_attributes_and_logic_assets() {
    let source_dir = temp_dir("assets-src");
    let out = temp_dir("assets-out");
    fs::create_dir_all(source_dir.join("img")).expect("img");
    fs::create_dir_all(source_dir.join("assets")).expect("assets");
    fs::write(source_dir.join("img/hero.png"), b"png").expect("hero");
    fs::write(source_dir.join("assets/sample.wav"), b"wav").expect("wav");
    fs::write(
        source_dir.join("logic.js"),
        "export function play() { const src = 'assets/sample.wav'; return src; }\n",
    )
    .expect("logic");
    fs::write(
        source_dir.join("app.nui"),
        r#"page Demo {
            use { play } from "./logic.js"
            Image(src = "img/hero.png")
            Button(on:click = play) { "Play" }
        }"#,
    )
    .expect("app");

    build(&source_dir.join("app.nui"), Some(&out)).expect("build harus berhasil");
    assert!(out.join("img/hero.png").is_file());
    assert!(out.join("assets/sample.wav").is_file());
    assert!(out.join("logic.js").is_file());

    cleanup(&source_dir);
    cleanup(&out);
}

#[test]
fn imports_cannot_escape_the_project_folder() {
    let source_dir = temp_dir("escape-src");
    let app_dir = source_dir.join("app");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::write(
        source_dir.join("outside.js"),
        "export function dariLuar() {}\n",
    )
    .expect("outside");
    fs::write(
        app_dir.join("app.nui"),
        r#"page Demo {
            use { dariLuar } from "../outside.js"
            Text { "x" }
        }"#,
    )
    .expect("app");

    let out = temp_dir("escape-out");
    let err = build(&app_dir.join("app.nui"), Some(&out)).expect_err("impor harus ditolak");
    assert!(
        err.message.contains("keluar dari folder proyek"),
        "pesan error: {err}"
    );

    cleanup(&source_dir);
    cleanup(&out);
}

#[test]
fn page_title_is_html_escaped() {
    let source_dir = temp_dir("title-src");
    let out = temp_dir("title-out");
    let file = source_dir.join("a&b.nui");
    fs::write(&file, "page Demo { Text { \"x\" } }").expect("source");

    build(&file, Some(&out)).expect("build harus berhasil");
    let html = fs::read_to_string(out.join("index.html")).expect("index.html");
    assert!(
        html.contains("a&amp;b.nui"),
        "judul harus di-escape: {html}"
    );
    assert!(!html.contains("a&b.nui"));

    cleanup(&source_dir);
    cleanup(&out);
}

#[test]
fn generated_javascript_is_syntactically_valid() {
    if !command_exists("node") {
        eprintln!("Node.js tidak tersedia; test sintaks dilewati");
        return;
    }

    let out = temp_dir("syntax");
    build(&fixture("home.nui"), Some(&out)).expect("build harus berhasil");
    let module = out.join("output.mjs");
    fs::copy(out.join("output.js"), &module).expect("copy");
    let status = Command::new("node")
        .arg("--check")
        .arg(&module)
        .status()
        .expect("node --check");
    assert!(status.success(), "output.js harus sintaks valid");

    cleanup(&out);
}

#[test]
fn generated_output_runs_against_dom_stub() {
    if !command_exists("node") {
        eprintln!("Node.js tidak tersedia; test runtime dilewati");
        return;
    }

    let out = temp_dir("runtime");
    build(&fixture("home.nui"), Some(&out)).expect("build harus berhasil");

    let script = r#"
const vm = require('vm');
const fs = require('fs');
const path = require('path');
const { createDocument, createWindow } = require(process.env.NUI_DOM_STUB);
const out = process.env.NUI_OUT;
const toScript = (code) => code.split('\n').filter((line) => !/^\s*import\s/.test(line)).join('\n').replace(/^export\s+/gm, '');
const document = createDocument();
const window = createWindow();
const sandbox = { document, window, console };
vm.createContext(sandbox);
for (const file of ['runtime.js', 'logic.js']) {
  vm.runInContext(toScript(fs.readFileSync(path.join(out, file), 'utf8')), sandbox);
}
vm.runInContext(toScript(fs.readFileSync(path.join(out, 'output.js'), 'utf8')) + '\nglobalThis.__app = app;', sandbox);
const app = sandbox.__app;
const shadow = app.host.shadowRoot;
const halo = shadow.findAll((node) => node.tagName === 'p').find((node) => node.textContent.startsWith('Halo'));
if (!halo) throw new Error('teks Halo tidak ditemukan');
const muat = shadow.findAll((node) => node.tagName === 'button').find((node) => node.textContent === 'Muat');
if (!muat) throw new Error('tombol Muat tidak ditemukan');
muat.dispatch('click', {});
if (halo.textContent !== 'Halo Budi Santoso') throw new Error('binding tidak reaktif: ' + halo.textContent);
const overlay = document.body.childNodes.find((node) => node.getAttribute('data-nui-overlay'));
if (!overlay) throw new Error('overlay tidak berada di layer terpisah');
"#;

    let status = Command::new("node")
        .arg("-e")
        .arg(script)
        .env("NUI_OUT", &out)
        .env("NUI_DOM_STUB", root().join("tests").join("dom-stub.js"))
        .status()
        .expect("node runtime smoke");
    assert!(status.success(), "output build harus berjalan di DOM stub");

    cleanup(&out);
}

#[test]
fn runtime_blocks_dangerous_attributes() {
    if !command_exists("node") {
        eprintln!("Node.js tidak tersedia; test runtime security dilewati");
        return;
    }

    let out = temp_dir("runtime-security");
    build(&fixture("home.nui"), Some(&out)).expect("build harus berhasil");

    let script = r#"
const vm = require('vm');
const fs = require('fs');
const path = require('path');
const { createDocument, createWindow } = require(process.env.NUI_DOM_STUB);
const out = process.env.NUI_OUT;
const code = fs.readFileSync(path.join(out, 'runtime.js'), 'utf8').replace(/^export\s+/gm, '');
const document = createDocument();
const sandbox = { document, window: createWindow(), console };
vm.createContext(sandbox);
vm.runInContext(code, sandbox);

const link = document.createElement('a');
sandbox.setAttr(link, 'href', 'javascript:alert(1)');
if (link.getAttribute('href') !== '#') throw new Error('javascript: tidak diblokir');
sandbox.setAttr(link, 'href', 'https://contoh.test');
if (link.getAttribute('href') !== 'https://contoh.test') throw new Error('URL aman ikut diblokir');
sandbox.setAttr(link, 'onclick', 'alert(1)');
if (link.hasAttribute('onclick')) throw new Error('atribut on* tidak diblokir');
sandbox.setAttr(link, 'srcdoc', '<script>alert(1)</script>');
if (link.hasAttribute('srcdoc')) throw new Error('srcdoc tidak diblokir');

const box = sandbox.h('div', { onclick: 'alert(1)', title: 'aman' });
if (box.hasAttribute('onclick')) throw new Error('h memasang atribut on*');
if (box.getAttribute('title') !== 'aman') throw new Error('atribut biasa hilang');
"#;

    let status = Command::new("node")
        .arg("-e")
        .arg(script)
        .env("NUI_OUT", &out)
        .env("NUI_DOM_STUB", root().join("tests").join("dom-stub.js"))
        .status()
        .expect("node runtime security");
    assert!(
        status.success(),
        "runtime harus memblokir atribut berbahaya"
    );

    cleanup(&out);
}

#[test]
fn cli_build_and_check_work() {
    let out = temp_dir("cli");
    let binary = env!("CARGO_BIN_EXE_nui");

    let build_status = Command::new(binary)
        .args([
            "build",
            fixture("home.nui").to_str().expect("path"),
            "--out",
        ])
        .arg(&out)
        .status()
        .expect("CLI build");
    assert!(build_status.success());
    assert!(out.join("output.js").is_file());

    let check = Command::new(binary)
        .args(["check", fixture("home.nui").to_str().expect("path")])
        .output()
        .expect("CLI check");
    assert!(check.status.success());
    assert!(String::from_utf8_lossy(&check.stdout).contains("OK"));

    let bad = Command::new(binary)
        .args(["build", fixture("errors.nui").to_str().expect("path")])
        .output()
        .expect("CLI build error");
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("baris"));

    cleanup(&out);
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}
