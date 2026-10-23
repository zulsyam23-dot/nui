//! CLI NodeUI: `nui build`, `nui dev`, `nui check`.

mod dev;

use std::env;
use std::path::PathBuf;
use std::process;

use nodeui::ast::Node;
use nodeui::compiler::{build, compile};
use nodeui::error::CompileError;

const USAGE: &str = "Pemakaian: nui build <file.nui> [--out dir] | nui dev [<file.nui>] [--port N] | nui check <file.nui>";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(err) => {
            print_error(&err);
            1
        }
    };
    process::exit(code);
}

fn run(args: &[String]) -> Result<i32, CompileError> {
    match args.first().map(String::as_str) {
        Some("build") => command_build(args),
        Some("dev") => command_dev(args),
        Some("check") => command_check(args),
        _ => Err(CompileError::new(USAGE)),
    }
}

fn command_build(args: &[String]) -> Result<i32, CompileError> {
    let file = args
        .get(1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| CompileError::new("Pemakaian: nui build <file.nui> [--out dir]"))?;
    let source = absolute(file);
    let out = option(args, "out")?.as_deref().map(absolute);
    let result = build(&source, out.as_deref())?;
    println!(
        "[build] {} -> {}",
        source.display(),
        result.out_dir.join("output.js").display()
    );
    Ok(0)
}

fn command_dev(args: &[String]) -> Result<i32, CompileError> {
    let source = match args.get(1).filter(|value| !value.starts_with("--")) {
        Some(file) => absolute(file),
        None => default_dev_file()?,
    };
    let port = match option(args, "port")? {
        Some(value) => value
            .parse::<u16>()
            .map_err(|_| CompileError::new(format!("Port tidak valid: {value}")))?,
        None => env::var("NUI_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3456),
    };

    dev::run(&source, port)?;
    Ok(0)
}

fn command_check(args: &[String]) -> Result<i32, CompileError> {
    let file = args
        .get(1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| CompileError::new("Pemakaian: nui check <file.nui>"))?;
    let source = absolute(file);

    match compile(&source) {
        Ok(compiled) => {
            let nodes = compiled.ast.body.iter().map(count_nodes).sum::<usize>();
            println!("[check] {} OK ({nodes} node)", source.display());
            Ok(0)
        }
        Err(err) if !err.issues.is_empty() => {
            eprintln!(
                "[check] {} gagal: {} masalah",
                source.display(),
                err.issues.len()
            );
            for issue in &err.issues {
                eprintln!("  - {issue}");
                eprintln!("    -> {}:{}:{}", source.display(), issue.line, issue.col);
            }
            Ok(1)
        }
        Err(err) => Err(err),
    }
}

fn count_nodes(node: &Node) -> usize {
    let Node::Element(element) = node else {
        return 0;
    };
    1 + element.children.iter().map(count_nodes).sum::<usize>()
}

fn option(args: &[String], name: &str) -> Result<Option<String>, CompileError> {
    let flag = format!("--{name}");
    let Some(index) = args.iter().position(|arg| arg == &flag) else {
        return Ok(None);
    };
    let value = args
        .get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| CompileError::new(format!("Opsi {flag} memerlukan nilai")))?;
    Ok(Some(value.clone()))
}

fn absolute(value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn default_dev_file() -> Result<PathBuf, CompileError> {
    for candidate in ["examples/piano/piano.nui", "examples/home.nui"] {
        let path = absolute(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(CompileError::new(
        "Tidak ada berkas default. Jalankan: nui dev <file.nui>",
    ))
}

fn print_error(err: &CompileError) {
    if err.issues.is_empty() {
        eprintln!("[error] {err}");
        if let (Some(file), Some(line), Some(col)) = (&err.file, err.line, err.col) {
            eprintln!("  -> {file}:{line}:{col}");
        }
        return;
    }

    eprintln!("[error] Validasi gagal ({} masalah):", err.issues.len());
    for issue in &err.issues {
        eprintln!("  - {issue}");
        if let Some(file) = &err.file {
            eprintln!("    -> {file}:{}:{}", issue.line, issue.col);
        }
    }
}
