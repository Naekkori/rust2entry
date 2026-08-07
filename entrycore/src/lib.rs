//! 코어 라이브러리: Rust 소스 -> IR / IR -> Entry 블록 직렬화.

pub mod block;
pub mod codegen;
pub mod error;
pub mod ir;
pub mod parse;

pub use error::{Error, Result};
