pub mod category;
pub mod registry;

use serde::Serialize;

use crate::ir::Stmt;

/// 모든 Entry 블록의 통합 표현.
/// 각 변형이 자체 직렬화 구현 가져옴.
/// Deserialize 안 함 — Entry project.json은 출력 전용.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// 시작 블록 (when_start, when_click, ...).
    Start(StartBlock),

    /// 흐름: if.
    If(IfBlock),
    /// 흐름: if-else.
    IfElse(IfElseBlock),
    /// 흐름: while.
    While(WhileBlock),
    /// 흐름: for-range.
    ForRange(ForRangeBlock),
    /// 흐름: repeat.
    Repeat(RepeatBlock),
    /// 흐름: break.
    Break,
    /// 흐름: continue.
    Continue,

    /// 산술: 이항.
    CalcBinOp(CalcBinOpBlock),
    /// 산술: 단항.
    CalcUnaryOp(CalcUnaryOpBlock),

    /// 비교/논리.
    Compare(CompareBlock),
    /// 논리합/논리곱.
    BoolOp(BoolOpBlock),

    /// 변수: 읽기.
    GetVar(GetVarBlock),
    /// 변수: 쓰기.
    SetVar(SetVarBlock),
    /// 변수: 변경.
    ChangeVar(ChangeVarBlock),

    /// 문자열: 연결.
    StringConcat(StringConcatBlock),
    /// 문자열: 포함.
    StringIncludes(StringIncludesBlock),

    /// 함수 호출.
    FuncCall(FuncCallBlock),
    /// 함수 정의.
    FuncDef(FuncDefBlock),
    /// 함수 파라미터 참조.
    FuncParam(FuncParamBlock),
    /// return.
    Return(ReturnBlock),

    /// 표현식 평가 (값 없음).
    Expr(ExprBlock),
}

/// 블록 카테고리.
pub use category::Category;

/// 블록 그룹 trait — IR 변환용.
pub trait FromRust {
    /// 이 블록이 처리 가능한 Rust 노드 종별.
    fn matches(stmt: &Stmt) -> bool;

    /// IR -> Block.
    fn from_ir(stmt: &Stmt) -> crate::Result<Block>;
}

/// ── 블록 데이터 정의 ─────────────────────────────────
/// 모두 public 필드. 직렬화 시 struct 형식으로 출력.

#[derive(Debug, Clone, Serialize)]
pub struct StartBlock {
    pub op_code: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct IfBlock {
    pub op_code: &'static str,
    pub cond: Box<Block>,
    pub then_body: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IfElseBlock {
    pub op_code: &'static str,
    pub cond: Box<Block>,
    pub then_body: Vec<Block>,
    pub else_body: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhileBlock {
    pub op_code: &'static str,
    pub cond: Box<Block>,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForRangeBlock {
    pub op_code: &'static str,
    pub var: String,
    pub from: Box<Block>,
    pub to: Box<Block>,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepeatBlock {
    pub op_code: &'static str,
    pub times: Box<Block>,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalcBinOpBlock {
    pub op_code: &'static str,
    pub op: crate::ir::BinOp,
    pub lhs: Box<Block>,
    pub rhs: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalcUnaryOpBlock {
    pub op_code: &'static str,
    pub op: crate::ir::UnaryOp,
    pub expr: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareBlock {
    pub op_code: &'static str,
    pub op: crate::ir::BinOp,
    pub lhs: Box<Block>,
    pub rhs: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoolOpBlock {
    pub op_code: &'static str,
    pub op: crate::ir::BinOp,
    pub lhs: Box<Block>,
    pub rhs: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetVarBlock {
    pub op_code: &'static str,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetVarBlock {
    pub op_code: &'static str,
    pub name: String,
    pub value: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeVarBlock {
    pub op_code: &'static str,
    pub name: String,
    pub delta: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StringConcatBlock {
    pub op_code: &'static str,
    pub parts: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StringIncludesBlock {
    pub op_code: &'static str,
    pub haystack: Box<Block>,
    pub needle: Box<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FuncCallBlock {
    pub op_code: &'static str,
    pub name: String,
    pub args: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FuncDefBlock {
    pub op_code: &'static str,
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FuncParamBlock {
    pub op_code: &'static str,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReturnBlock {
    pub op_code: &'static str,
    pub value: Option<Box<Block>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExprBlock {
    pub op_code: &'static str,
    pub expr: Box<Block>,
}
