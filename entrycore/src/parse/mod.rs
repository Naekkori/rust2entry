//! Rust 소스 파싱 -> IR.

use crate::{ir::Program, Error, Result};

/// Rust 소스 문자열을 IR Program으로 변환.
pub fn parse(_source: &str) -> Result<Program> {
    todo!("네가 구현 - syn::parse_str -> AST -> IR 변환")
}

/// 파싱 실패 시 에러 변환 헬퍼.
pub(crate) fn map_syn_err(_e: syn::Error) -> Error {
    todo!("네가 구현 - syn::Error -> crate::Error")
}
