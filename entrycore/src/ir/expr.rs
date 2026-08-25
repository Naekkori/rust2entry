//! IR 표현식.

use serde::Serialize;

/// Entry 직렬화 친화 표현식.
#[derive(Debug, Clone, Serialize)]
pub enum Expr {
    /// 정수 리터럴.
    Int(i64),
    /// 부동소수점 리터럴.
    Float(f64),
    /// 문자열 리터럴.
    Str(String),
    /// 불리언.
    Bool(bool),
    /// 변수 참조.
    Var(String),

    /// Rust 한정 경로 (예: `TextEffect::Strike`).
    Path(Vec<String>),

    /// 이항 연산 (op, lhs, rhs).
    BinOp(BinOp, Box<Expr>, Box<Expr>),

    /// 단항 연산 (op, expr).
    UnaryOp(UnaryOp, Box<Expr>),

    /// 함수 호출 (name, args).
    Call(FuncRef, Vec<Expr>),

    /// 함수 참조.
    Func(FuncRef),

    //For 용 범위
    Range(Box<Expr>, Box<Expr>)
}

/// 함수 참조 (이름 + 인자 타입).
#[derive(Debug, Clone, Serialize)]
pub struct FuncRef {
    pub name: String,
    pub arity: usize,
    /// 하드웨어 블럭의 원본 .ent 블럭 JSON (손실 없는 왕복용). 일반 함수는 None.
    /// deparse(역방향)가 채우고, decodegen이 `// @hwraw {...}` 주석으로 emit하며,
    /// parse(정방향)가 이 주석을 복구해 from_stmt 가 Block::Raw 로 되살린다.
    #[serde(default)]
    pub raw: Option<serde_json::Value>,
}

/// 이항 연산 종류.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Range
}

/// 단항 연산 종류.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Neg,
    Not,
}
