mod common;

use std::process::Command;

use common::{cleanup, root, temp_dir};

#[test]
fn piano_app_runs_against_dom_stub() {
    if !command_exists("node") {
        eprintln!("Node.js tidak tersedia; test piano dilewati");
        return;
    }

    let out = temp_dir("piano");
    nodeui::compiler::build(&root().join("examples/piano/piano.nui"), Some(&out))
        .expect("build piano harus berhasil");

    let script = r#"
const vm = require('vm');
const fs = require('fs');
const path = require('path');
const { createDocument, createWindow } = require(process.env.NUI_DOM_STUB);
const out = process.env.NUI_OUT;
const toScript = (code) => code.split('\n').filter((line) => !/^\s*import\s/.test(line)).join('\n').replace(/^export\s+/gm, '');

function load() {
  const document = createDocument();
  const window = createWindow();
  const sandbox = { document, window, console, setInterval, clearInterval };
  vm.createContext(sandbox);
  for (const file of ['runtime.js', 'piano-logic.js']) {
    vm.runInContext(toScript(fs.readFileSync(path.join(out, file), 'utf8')), sandbox);
  }
  vm.runInContext(toScript(fs.readFileSync(path.join(out, 'output.js'), 'utf8')) + '\nglobalThis.__app = app;', sandbox);
  return { app: sandbox.__app, window, shadow: sandbox.__app.host.shadowRoot };
}

const assert = (value, message) => { if (!value) throw new Error(message); };
const eq = (actual, expected, message) => { if (actual !== expected) throw new Error(message + ': ' + JSON.stringify(actual)); };
const keys = (shadow, name) => shadow.findAll((node) => String(node.getAttribute('class') || '').includes(name));

{
  const { shadow } = load();
  const white = keys(shadow, 'white');
  const black = keys(shadow, 'black');
  eq(white.length, 10, 'jumlah tombol putih');
  eq(black.length, 7, 'jumlah tombol hitam');
  eq(white[0].getAttribute('note'), 'C4', 'nada putih pertama');
  eq(black[0].getAttribute('note'), 'C#4', 'nada hitam pertama');
  eq(black[0].style.left, 'calc(10.0000% - 23px)', 'posisi C#4');
  eq(black[5].style.left, 'calc(80.0000% - 23px)', 'posisi C#5');
}

{
  const { app, window, shadow } = load();
  const white = keys(shadow, 'white');
  window.dispatch('keydown', { key: 'a' });
  eq(app.state.get().nada, 'C4', 'keyboard A');
  eq(app.state.get().ditekan, 'C4', 'state ditekan');
  assert(white[0].classList.contains('tekan'), 'class tekan harus aktif');
  window.dispatch('keyup', { key: 'a' });
  eq(app.state.get().ditekan, '-', 'keyup melepas nada');
  assert(!white[0].classList.contains('tekan'), 'class tekan harus hilang');

  window.dispatch('keydown', { key: 'w' });
  eq(app.state.get().nada, 'C#4', 'keyboard W');

  window.dispatch('keydown', { key: 'x' });
  eq(app.state.get().octav, '5', 'naik oktaf');
  eq(white[0].getAttribute('note'), 'C5', 'label mengikuti oktaf');
  window.dispatch('keydown', { key: 'a' });
  eq(app.state.get().nada, 'C5', 'nada mengikuti oktaf');
}

{
  const { app, window } = load();
  window.dispatch('keydown', { key: 'a' });
  window.dispatch('keydown', { key: ' ' });
  eq(app.state.get().sustain, 'nyala', 'sustain aktif');
  window.dispatch('keyup', { key: 'a' });
  eq(app.state.get().ditekan, 'C4', 'sustain menahan nada');
  window.dispatch('keyup', { key: ' ' });
  eq(app.state.get().sustain, 'mati', 'sustain mati');
  eq(app.state.get().ditekan, '-', 'nada lepas setelah sustain');
}

{
  const { app, shadow } = load();
  const buttons = shadow.findAll((node) => node.tagName === 'button');
  const find = (label) => buttons.find((button) => button.textContent === label);
  find('Oktav +').dispatch('click', {});
  eq(app.state.get().octav, '5', 'tombol oktav naik');
  find('Oktav -').dispatch('click', {});
  eq(app.state.get().octav, '4', 'tombol oktav turun');
  find('Sustain').dispatch('click', {});
  eq(app.state.get().sustain, 'nyala', 'tombol sustain nyala');
  find('Sustain').dispatch('click', {});
  eq(app.state.get().sustain, 'mati', 'tombol sustain mati');

  const slider = shadow.findAll((node) => node.tagName === 'input')[0];
  slider.value = '25';
  slider.dispatch('input', { currentTarget: slider });
  eq(app.state.get().volume, '25', 'slider volume');

  const white = keys(shadow, 'white');
  white[2].dispatch('mousedown', { currentTarget: white[2] });
  eq(app.state.get().nada, 'E4', 'klik mouse');
  white[2].dispatch('mouseup', { currentTarget: white[2] });
  eq(app.state.get().ditekan, '-', 'mouseup melepas nada');
}
"#;

    let status = Command::new("node")
        .arg("-e")
        .arg(script)
        .env("NUI_OUT", &out)
        .env("NUI_DOM_STUB", root().join("tests").join("dom-stub.js"))
        .status()
        .expect("node piano smoke");
    assert!(
        status.success(),
        "aplikasi piano harus berjalan di DOM stub"
    );

    cleanup(&out);
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}
