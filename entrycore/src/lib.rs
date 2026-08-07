//! 코어 라이브러리: Rust 소스 -> IR / IR -> Entry 블록 직렬화.
//! + Entry 블록 -> IR 역변환.

pub mod block;
pub mod codegen;
pub mod decodegen;
pub mod deparse;
pub mod error;
pub mod ir;
pub mod parse;
pub mod var;

pub use error::{Error, Result};
pub use var::{VarInfo, VarInit, VarKind, VarMap};
