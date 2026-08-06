//! 중간 표현 (IR).
//! 코드는 Rust AST -> IR -> Entry 직렬화 흐름.
//! IR은 Entry 직렬화에 친화적인 형태로 단순화.

pub mod expr;
pub mod stmt;

pub use expr::{BinOp, Expr, FuncRef, UnaryOp};
pub use stmt::Stmt;

/// Rust 소스 전체 표현.
#[derive(Debug, Clone, Default)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}
