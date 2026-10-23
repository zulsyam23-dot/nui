//! Pipeline kompilasi dan build NodeUI.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::{ElementNode, Node, Program};
use crate::error::{CompileError, Issue};
use crate::generator::{generate, Generated};
use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use crate::validator::validate;

const RUNTIME: &str = include_str!("runtime.js");

const ASSET_ATTRS: &[&str] = &["src", "href", "poster", "data-src"];

const ASSET_EXTS: &[&str] = &[
    ".wav", ".mp3", ".ogg", ".m4a", ".webm", ".mp4", ".png", ".jpg", ".jpeg", ".gif", ".svg",
    ".webp", ".ico", ".json", ".txt", ".css", ".woff", ".woff2",
];

#[derive(Debug)]
pub struct Compiled {
    pub ast: Program,
    pub js: String,
    pub css: String,
}

#[derive(Debug)]
pub struct BuildResult {
    pub js: String,
    pub css: String,
    pub out_dir: PathBuf,
}

pub fn compile(source_file: &Path) -> Result<Compiled, CompileError> {
    compile_with(source_file, &mut |from| Ok(from.to_string()))
}

pub fn compile_with(
    source_file: &Path,
    resolve_import: &mut dyn FnMut(&str) -> Result<String, CompileError>,
) -> Result<Compiled, CompileError> {
    let file = source_file.display().to_string();
    let source = fs::read_to_string(source_file)?;
    let tokens = Tokenizer::new(&source)
        .tokenize()
        .map_err(|err| CompileError::from_syntax(&file, err))?;
    let ast = Parser::new(tokens)
        .parse()
        .map_err(|err| CompileError::from_syntax(&file, err))?;

    let issues: Vec<Issue> = validate(&ast);
    if !issues.is_empty() {
        return Err(CompileError::validation(&file, issues));
    }

    let Generated { js, css } = generate(&ast, resolve_import)?;
    Ok(Compiled { ast, js, css })
}

pub fn build(source_file: &Path, out_dir: Option<&Path>) -> Result<BuildResult, CompileError> {
    let target = out_dir.map_or_else(default_out_dir, Path::to_path_buf);
    let parent = target
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    fs::create_dir_all(&parent)?;

    let name = target
        .file_name()
        .map_or_else(|| OsString::from("dist"), OsString::from);
    let mut staging_name = OsString::from(".");
    staging_name.push(&name);
    staging_name.push(format!(".staging-{}", std::process::id()));
    let staging = parent.join(staging_name);

    fs::remove_dir_all(&staging).ok();
    fs::create_dir_all(&staging)?;

    let result = build_into(source_file, &staging);
    if let Err(err) = result {
        fs::remove_dir_all(&staging).ok();
        return Err(err);
    }

    if let Err(err) = swap_dir(&staging, &target) {
        fs::remove_dir_all(&staging).ok();
        return Err(err);
    }

    let js = fs::read_to_string(target.join("output.js"))?;
    let css = fs::read_to_string(target.join("style.css"))?;
    Ok(BuildResult {
        js,
        css,
        out_dir: target,
    })
}

fn build_into(source_file: &Path, staging: &Path) -> Result<(), CompileError> {
    let source_dir = source_file
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let source_dir = lexical_normalize(&source_dir);
    let source_root = fs::canonicalize(&source_dir).map_err(|err| {
        CompileError::new(format!(
            "Folder sumber tidak valid: {} ({err})",
            source_dir.display()
        ))
    })?;

    let mut logic_files = Vec::new();
    let mut copied = HashSet::new();
    let compiled = {
        let mut resolver = |from: &str| {
            resolve_import(
                from,
                &source_dir,
                &source_root,
                staging,
                &mut logic_files,
                &mut copied,
            )
        };
        compile_with(source_file, &mut resolver)?
    };

    copy_local_assets(&compiled.ast, &source_dir, &source_root, staging)?;
    copy_logic_assets(&logic_files, &source_root, staging)?;

    fs::write(staging.join("runtime.js"), RUNTIME)?;
    fs::write(staging.join("output.js"), &compiled.js)?;
    fs::write(staging.join("style.css"), &compiled.css)?;
    let title = source_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app.nui");
    fs::write(staging.join("index.html"), index_html(title))?;

    Ok(())
}

fn default_out_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("dist")
}

fn resolve_import(
    from: &str,
    source_dir: &Path,
    source_root: &Path,
    out_dir: &Path,
    logic_files: &mut Vec<PathBuf>,
    copied: &mut HashSet<PathBuf>,
) -> Result<String, CompileError> {
    let abs = lexical_normalize(&source_dir.join(from));
    if !abs.is_file() {
        return Err(CompileError::new(format!(
            "Berkas impor tidak ditemukan: {from} ({})",
            abs.display()
        )));
    }

    let canonical = fs::canonicalize(&abs)
        .map_err(|err| CompileError::new(format!("Path impor tidak valid: {from} ({err})")))?;
    if !canonical.starts_with(source_root) {
        return Err(CompileError::new(format!(
            "Impor keluar dari folder proyek: {from} ({})",
            canonical.display()
        )));
    }

    let output_rel = abs
        .strip_prefix(source_dir)
        .map_err(|_| CompileError::new(format!("Path impor tidak valid: {from}")))?
        .to_path_buf();
    logic_files.push(abs.clone());

    if from.ends_with(".ts") {
        let transpiled = transpile_type_script(&abs)?;
        let output_rel = output_rel.with_extension("js");
        let destination = out_dir.join(&output_rel);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, transpiled)?;
        return Ok(format!("./{}", slash_path(&output_rel)));
    }

    let destination = out_dir.join(&output_rel);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if copied.insert(destination.clone()) {
        fs::copy(&abs, &destination)?;
    }
    Ok(format!("./{}", slash_path(&output_rel)))
}

fn slash_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn copy_local_assets(
    program: &Program,
    source_dir: &Path,
    source_root: &Path,
    out_dir: &Path,
) -> Result<(), CompileError> {
    fn walk(
        node: &ElementNode,
        source_dir: &Path,
        source_root: &Path,
        out_dir: &Path,
    ) -> Result<(), CompileError> {
        for child in &node.children {
            let Node::Element(element) = child else {
                continue;
            };
            for attr in &element.attributes {
                if !ASSET_ATTRS.contains(&attr.name.as_str()) {
                    continue;
                }
                copy_asset(&attr.value, source_dir, source_root, out_dir)?;
            }
            walk(element, source_dir, source_root, out_dir)?;
        }
        Ok(())
    }

    for node in &program.body {
        if let Node::Element(element) = node {
            walk(element, source_dir, source_root, out_dir)?;
        }
    }
    Ok(())
}

fn copy_asset(
    value: &str,
    source_dir: &Path,
    source_root: &Path,
    out_dir: &Path,
) -> Result<(), CompileError> {
    if value.is_empty()
        || value.starts_with("data:")
        || value.starts_with("//")
        || value.starts_with("http://")
        || value.starts_with("https://")
    {
        return Ok(());
    }

    let source = lexical_normalize(&source_dir.join(value));
    if !source.is_file() {
        return Ok(());
    }
    let Ok(canonical) = fs::canonicalize(&source) else {
        return Ok(());
    };
    if !canonical.starts_with(source_root) {
        return Ok(());
    }
    let Ok(relative) = source.strip_prefix(source_dir) else {
        return Ok(());
    };
    let destination = out_dir.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&source, &destination)?;
    Ok(())
}

fn copy_logic_assets(
    logic_files: &[PathBuf],
    source_root: &Path,
    out_dir: &Path,
) -> Result<(), CompileError> {
    for file in logic_files {
        let code = match fs::read_to_string(file) {
            Ok(code) => code,
            Err(_) => continue,
        };
        let base = file.parent().unwrap_or(source_root);

        for value in quoted_strings(&code) {
            if value.is_empty()
                || value.starts_with("data:")
                || value.starts_with("//")
                || value.starts_with("http://")
                || value.starts_with("https://")
                || !has_asset_extension(&value)
            {
                continue;
            }

            let source = lexical_normalize(&base.join(&value));
            if !source.is_file() {
                continue;
            }
            let Ok(canonical) = fs::canonicalize(&source) else {
                continue;
            };
            if !canonical.starts_with(source_root) {
                continue;
            }
            let Ok(relative) = canonical.strip_prefix(source_root) else {
                continue;
            };
            let destination = out_dir.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&canonical, &destination)?;
        }
    }
    Ok(())
}

fn has_asset_extension(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ASSET_EXTS.iter().any(|ext| lower.ends_with(ext))
}

fn quoted_strings(code: &str) -> Vec<String> {
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let quote = bytes[index];
        if quote != b'\'' && quote != b'"' {
            index += 1;
            continue;
        }

        let start = index + 1;
        let mut end = start;
        while end < bytes.len() {
            match bytes[end] {
                b'\\' => end += 2,
                byte if byte == quote || byte == b'\n' => break,
                _ => end += 1,
            }
        }

        if end < bytes.len() && bytes[end] == quote {
            if let Some(value) = code.get(start..end) {
                out.push(value.to_string());
            }
            index = end + 1;
        } else {
            index = start;
        }
    }

    out
}

enum Tsc {
    NodeScript(PathBuf),
    Binary(OsString),
}

fn transpile_type_script(file: &Path) -> Result<String, CompileError> {
    let Some(tsc) = find_tsc() else {
        return Err(CompileError::new(format!(
            "Tidak dapat mentranspile '{}'. Pasang TypeScript (npm i -D typescript) atau tulis logika dalam berkas .js.",
            file.display()
        )));
    };

    let temp = unique_temp_dir();
    fs::create_dir_all(&temp)?;

    let result = (|| -> Result<String, CompileError> {
        let mut command = match &tsc {
            Tsc::NodeScript(script) => {
                let mut command = Command::new("node");
                command.arg(script);
                command
            }
            Tsc::Binary(binary) => Command::new(binary),
        };
        command
            .args(["--target", "es2020", "--module", "esnext", "--skipLibCheck"])
            .arg("--outDir")
            .arg(&temp)
            .arg(file);

        let output = command
            .output()
            .map_err(|err| CompileError::new(format!("Gagal menjalankan TypeScript: {err}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CompileError::new(format!(
                "TypeScript gagal mentranspile '{}': {}",
                file.display(),
                stderr.trim()
            )));
        }

        let name = file.file_name().unwrap_or_else(|| OsStr::new("logic.ts"));
        let produced = temp.join(Path::new(name).with_extension("js"));
        fs::read_to_string(&produced).map_err(|_| {
            CompileError::new(format!(
                "TypeScript tidak menghasilkan berkas untuk '{}'",
                file.display()
            ))
        })
    })();

    fs::remove_dir_all(&temp).ok();
    result
}

fn find_tsc() -> Option<Tsc> {
    if let Some(script) = find_local_tsc() {
        return Some(Tsc::NodeScript(script));
    }
    for binary in ["tsc", "tsc.cmd"] {
        let output = Command::new(binary).arg("--version").output();
        if output.is_ok_and(|output| output.status.success()) {
            return Some(Tsc::Binary(OsString::from(binary)));
        }
    }
    None
}

fn find_local_tsc() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        let candidate = current
            .join("node_modules")
            .join("typescript")
            .join("bin")
            .join("tsc");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("nui-ts-{}-{nanos}", std::process::id()))
}

fn swap_dir(staging: &Path, target: &Path) -> Result<(), CompileError> {
    let mut backup_name = target.as_os_str().to_os_string();
    backup_name.push(".old");
    let backup = PathBuf::from(backup_name);

    fs::remove_dir_all(&backup).ok();
    if target.exists() && fs::rename(target, &backup).is_err() {
        fs::remove_dir_all(target)?;
    }

    if fs::rename(staging, target).is_err() {
        fs::create_dir_all(target)?;
        copy_tree(staging, target)?;
        fs::remove_dir_all(staging).ok();
    }

    fs::remove_dir_all(&backup).ok();
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), CompileError> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&destination)?;
            copy_tree(&source, &destination)?;
        } else {
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

fn index_html(title: &str) -> String {
    let title = html_escape(title);
    format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>NodeUI - {title}</title>
<link rel="stylesheet" href="style.css">
<style>
  html, body {{ margin: 0; padding: 0; min-height: 100%; background: Canvas; color-scheme: light dark; }}
  body {{ -webkit-font-smoothing: antialiased; }}
</style>
</head>
<body>
<script type="module" src="output.js"></script>
<script>
  // Live reload sederhana (mode dev).
  if (location.protocol === 'http:' || location.protocol === 'https:') {{
    const es = new EventSource('/__reload');
    es.onmessage = () => location.reload();
  }}
</script>
</body>
</html>"#
    )
}

pub fn runtime_source() -> &'static str {
    RUNTIME
}
