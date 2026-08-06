//! IR -> Entry project.json 직렬화.

pub mod schema;

use crate::{ir::Program, Result};
use serde_json::Value;

/// IR Program -> Entry project.json (serde_json::Value).
pub fn generate(_program: &Program) -> Result<Value> {
    todo!("네가 구현 - IR -> project.json 형태")
}

/// stmt 하나 -> JSON 블록.
pub(crate) fn stmt_to_value(_stmt: &crate::ir::Stmt) -> Result<Value> {
    todo!("네가 구현")
}

/// expr 하나 -> JSON.
pub(crate) fn expr_to_value(_expr: &crate::ir::Expr) -> Result<Value> {
    todo!("네가 구현")
}
