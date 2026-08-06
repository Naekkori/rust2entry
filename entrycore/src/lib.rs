//! 코어 라이브러리: Rust 소스 -> Entry .ent 패키징.

pub mod block;
pub mod codegen;
pub mod error;
pub mod ir;
pub mod parse;
pub mod project;

pub use error::{Error, Result};

/// 입력 소스 + 스프라이트 폴더를 받아 .ent 바이트스트림 생성.
/// 스프라이트는 None이면 패키지에 미포함.
pub fn compile(_source: &str, _sprites: Option<&std::path::Path>) -> Result<Vec<u8>> {
    todo!("네가 구현 - parse -> ir -> codegen -> pack")
}
