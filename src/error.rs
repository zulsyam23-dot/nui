//! Tipe lokasi dan error yang dipakai seluruh pipeline.

use std::fmt;

/// Posisi satu baris/kolom (1-based) di dalam berkas sumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Loc {
    pub line: usize,
    pub col: usize,
}

impl Loc {
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// Satu masalah validasi tanpa nama berkas.
#[derive(Debug, Clone)]
pub struct Issue {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl Issue {
    pub fn new(message: impl Into<String>, loc: Loc) -> Self {
        Self {
            message: message.into(),
            line: loc.line,
            col: loc.col,
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (baris {}, kolom {})",
            self.message, self.line, self.col
        )
    }
}

/// Error tokenisasi/parsing; lokasi disimpan terpisah agar tidak tercetak ganda.
#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub message: String,
    pub loc: Loc,
}

impl SyntaxError {
    pub fn new(message: impl Into<String>, loc: Loc) -> Self {
        Self {
            message: message.into(),
            loc,
        }
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (baris {}, kolom {})",
            self.message, self.loc.line, self.loc.col
        )
    }
}

impl std::error::Error for SyntaxError {}

/// Error kompilasi dengan konteks berkas dan (opsional) daftar masalah validasi.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub issues: Vec<Issue>,
}

impl CompileError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            file: None,
            line: None,
            col: None,
            issues: Vec::new(),
        }
    }

    pub fn at(message: impl Into<String>, file: impl Into<String>, loc: Loc) -> Self {
        Self {
            message: message.into(),
            file: Some(file.into()),
            line: Some(loc.line),
            col: Some(loc.col),
            issues: Vec::new(),
        }
    }

    pub fn from_syntax(file: &str, err: SyntaxError) -> Self {
        Self::at(err.message, file, err.loc)
    }

    pub fn validation(file: &str, issues: Vec<Issue>) -> Self {
        let first = issues.first();
        Self {
            message: "Validasi gagal".to_string(),
            file: Some(file.to_string()),
            line: first.map(|issue| issue.line),
            col: first.map(|issue| issue.col),
            issues,
        }
    }

    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.col) {
            (Some(line), Some(col)) => {
                write!(f, "{} (baris {}, kolom {})", self.message, line, col)
            }
            _ => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(err: std::io::Error) -> Self {
        Self::new(err.to_string())
    }
}
