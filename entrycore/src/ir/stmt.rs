//! IR 명령문.

use super::Expr;

/// Entry 블록으로 변환 가능한 명령문.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// 변수 선언 (이름, 초기값).
    VarDecl(String, Expr),

    /// 함수 정의 (이름, 인자, 본문).
    FuncDef {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    /// 표현식 평가 (expr 만나면 stmt로 감싸 변환).
    Expr(Expr),

    /// 흐름제어 (if/while/for).
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Repeat {
        times: Expr,
        body: Vec<Stmt>,
    },
    Return(Expr),
    Break,
    Continue,
}
