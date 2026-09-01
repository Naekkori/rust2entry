//! IR 명령문.

use super::Expr;
use crate::var::VarKind;

/// 변수 scope — EntryJS 의 `variables[].object` 필드 결정.
/// `static x = ...` (top-level) → Global (object: null, 모든 object 공유)
/// `let x = ...` (함수 내)    → Local  (object: 현재 rs stem, 특정 object 에 묶임)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VarScope {
    /// `let` — 로컬 (EntryJS variables[].object = rs stem).
    #[default]
    Local,
    /// `static` — 전역 (EntryJS variables[].object = null).
    Global,
}

/// 함수 param 타입. EntryJS 의 `function_param_string` / `function_param_boolean` 매핑.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// `StringParam` (default) — EntryJS function_param_string.
    String,
    /// `BoolParam` — EntryJS function_param_boolean.
    Bool,
}

/// 함수 결괏값 타입 — EntryJS function_create_value 의 VALUE 슬롯 매핑.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// 숫자 (i32 / f64 / Number / 그 외) — EntryJS function_create_value 의 값 슬롯.
    Number,
    /// 문자열 (`String` / `&str` / `str`).
    String,
    /// 불리언 (`bool` / `Bool`).
    Boolean,
}

/// Entry 블록으로 변환 가능한 명령문.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// 변수 선언 (이름, 초기값, kind, scope).
    /// `kind` 는 `let x: T = ...` 타입 어노테이션으로 지정 (None 이면 이름 기반 자동).
    /// `scope` 는 `let` (함수 내, Local) vs `static` (top-level, Global) 키워드로 결정.
    VarDecl(String, Expr, Option<VarKind>, VarScope),

    /// 변수 값 정하기 (이름, 새값).
    SetVar(String, Expr),

    /// 함수 정의 (이름, 인자, 결괏값 타입, 본문).
    FuncDef {
        name: String,
        /// (이름, kind) 쌍. kind 는 String(default) 또는 Bool.
        params: Vec<(String, ParamKind)>,
        /// `Some` 이면 결괏값 반환 함수 — EntryJS function_create_value 로 emit.
        /// 본문 마지막 stmt 는 `Stmt::Return(Expr)` 이어야 함.
        /// `None` 이면 statement 본문 함수 — EntryJS function_create 로 emit.
        return_type: Option<ReturnType>,
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
