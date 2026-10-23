//! NodeUI: compiler bahasa UI web (.nui).
//!
//! Pipeline: tokenizer -> parser -> validator -> generator.
//! Runtime browser tetap JavaScript (`runtime.js`) karena berjalan di DOM.

pub mod ast;
pub mod compiler;
pub mod error;
pub mod generator;
pub mod parser;
pub mod scope;
pub mod tokenizer;
pub mod validator;

pub use compiler::{build, compile, compile_with, BuildResult, Compiled};
pub use error::{CompileError, Issue, Loc, SyntaxError};
pub use generator::{generate, Generated};
