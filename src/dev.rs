//! Dev server: build + static file + live reload (SSE).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use nodeui::ast::Node;
use nodeui::error::CompileError;
use nodeui::parser::Parser;
use nodeui::tokenizer::Tokenizer;

const REQUEST_LINE_LIMIT: usize = 8 * 1024;
const MAX_SSE_CLIENTS: usize = 64;

pub fn run(source: &Path, port: u16) -> Result<(), CompileError> {
    let output = nodeui::build(source, None)?;
    let root = fs::canonicalize(&output.out_dir).map_err(|err| {
        CompileError::new(format!(
            "Folder hasil build tidak valid: {} ({err})",
            output.out_dir.display()
        ))
    })?;
    let root = Arc::new(root);
    let clients = Arc::new(Mutex::new(Vec::new()));

    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|err| CompileError::new(format!("Tidak dapat membuka port {port}: {err}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| CompileError::new(err.to_string()))?;

    println!("[dev] http://localhost:{port}  (ctrl+c untuk berhenti)");

    let watched = snapshot(source);
    println!(
        "[dev] memantau: {}",
        watched
            .iter()
            .filter_map(|(path, _)| path.file_name().and_then(|name| name.to_str()))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut last = watched;
    loop {
        while let Ok((stream, _)) = listener.accept() {
            let _ = stream.set_nonblocking(false);
            let root = Arc::clone(&root);
            let clients = Arc::clone(&clients);
            thread::spawn(move || {
                if let Err(err) = handle(stream, &root, &clients) {
                    eprintln!("[dev] request gagal: {err}");
                }
            });
        }

        thread::sleep(Duration::from_millis(250));
        let current = snapshot(source);
        if current == last {
            continue;
        }
        last = current;
        thread::sleep(Duration::from_millis(150));

        match nodeui::build(source, None) {
            Ok(_) => {
                println!("[dev] rebuild selesai -> reload browser");
                broadcast(&clients);
            }
            Err(err) => eprintln!("[dev] rebuild gagal: {err}"),
        }
    }
}

fn snapshot(source: &Path) -> Vec<(PathBuf, Option<SystemTime>)> {
    let mut files = vec![source.to_path_buf()];
    files.extend(imported_files(source));
    files.sort();
    files.dedup();

    files
        .into_iter()
        .map(|path| {
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .ok();
            (path, modified)
        })
        .collect()
}

fn imported_files(source: &Path) -> Vec<PathBuf> {
    let Ok(code) = fs::read_to_string(source) else {
        return Vec::new();
    };
    let Ok(tokens) = Tokenizer::new(&code).tokenize() else {
        return Vec::new();
    };
    let Ok(program) = Parser::new(tokens).parse() else {
        return Vec::new();
    };

    fn walk(node: &nodeui::ast::ElementNode, base: &Path, root: &Path, out: &mut Vec<PathBuf>) {
        for child in &node.children {
            match child {
                Node::Import(import) => {
                    let path = base.join(&import.from);
                    let Ok(canonical) = fs::canonicalize(&path) else {
                        continue;
                    };
                    if canonical.starts_with(root) && canonical.is_file() {
                        out.push(canonical);
                    }
                }
                Node::Element(element) => walk(element, base, root, out),
                Node::State(_) | Node::Text(_) => {}
            }
        }
    }

    let base = source.parent().unwrap_or_else(|| Path::new("."));
    let Ok(root) = fs::canonicalize(base) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for node in &program.body {
        if let Node::Element(element) = node {
            walk(element, base, &root, &mut out);
        }
    }
    out
}

fn handle(
    mut stream: TcpStream,
    root: &Path,
    clients: &Mutex<Vec<TcpStream>>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut reader = BufReader::new(&stream);
    let mut line = Vec::new();
    let read = (&mut reader)
        .take(REQUEST_LINE_LIMIT as u64)
        .read_until(b'\n', &mut line)?;
    drop(reader);

    if read == 0 {
        return Ok(());
    }
    if read >= REQUEST_LINE_LIMIT && !line.ends_with(b"\n") {
        return write_response(
            &mut stream,
            414,
            "text/plain; charset=utf-8",
            b"uri too long",
            false,
        );
    }

    let request = String::from_utf8_lossy(&line);
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let version = parts.next().unwrap_or("");

    if !version.starts_with("HTTP/") {
        return write_response(
            &mut stream,
            400,
            "text/plain; charset=utf-8",
            b"bad request",
            false,
        );
    }

    let head_only = method == "HEAD";
    if method != "GET" && !head_only {
        return write_response(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed",
            false,
        );
    }

    if target == "/__reload" && !head_only {
        let full = clients
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .len()
            >= MAX_SSE_CLIENTS;
        if full {
            return write_response(
                &mut stream,
                503,
                "text/plain; charset=utf-8",
                b"too many clients",
                false,
            );
        }

        stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: keep-alive\r\n\r\n",
        )?;
        stream.flush()?;
        clients
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .push(stream);
        return Ok(());
    }

    serve(stream, root, target, head_only)
}

fn serve(mut stream: TcpStream, root: &Path, url: &str, head_only: bool) -> std::io::Result<()> {
    let clean = percent_decode(url.split('?').next().unwrap_or("/"));
    let relative = clean.trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };

    let path = Path::new(relative);
    let safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if !safe {
        return write_response(
            &mut stream,
            403,
            "text/plain; charset=utf-8",
            b"forbidden",
            head_only,
        );
    }

    let file = root.join(path);
    let Ok(canonical) = fs::canonicalize(&file) else {
        return write_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"not found",
            head_only,
        );
    };
    if !canonical.starts_with(root) {
        return write_response(
            &mut stream,
            403,
            "text/plain; charset=utf-8",
            b"forbidden",
            head_only,
        );
    }
    if !canonical.is_file() {
        return write_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"not found",
            head_only,
        );
    }

    let body = match fs::read(&canonical) {
        Ok(body) => body,
        Err(_) => {
            return write_response(
                &mut stream,
                404,
                "text/plain; charset=utf-8",
                b"not found",
                head_only,
            )
        }
    };

    write_response(&mut stream, 200, mime_type(&canonical), &body, head_only)
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        414 => "URI Too Long",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn broadcast(clients: &Mutex<Vec<TcpStream>>) {
    let mut clients = clients.lock().unwrap_or_else(|lock| lock.into_inner());
    clients.retain_mut(|client| {
        client.write_all(b"data: reload\n\n").is_ok() && client.flush().is_ok()
    });
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1]);
            let low = hex_value(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
