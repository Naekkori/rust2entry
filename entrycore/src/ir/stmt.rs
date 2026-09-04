//! IR 명령문.

use super::Expr;
use crate::block::DialogMode;
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

/// Entry 의미 보존용 variable 참조. id 가 있으면 EntryJS variable list 의
/// 실제 socket id 를 그대로 emit 하고, 없으면 name 만으로 emit 한다.
/// (e.g. codegen 시점에 VarMap 에 없으면 name 만, 있으면 id 사용)
#[derive(Debug, Clone)]
pub struct VarRef {
    pub name: String,
    pub id: Option<String>,
}

impl VarRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: None,
        }
    }
    pub fn with_id(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: Some(id.into()),
        }
    }
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

    /// 변수 값 정하기 (ref, 새값). ref.name 은 사용자 변수 이름,
    /// ref.id 가 Some 이면 EntryJS variable list 의 실제 socket id.
    SetVar(VarRef, Expr),

    /// Entry `change_variable` 의미 보존 (Rust 의 `x = x + delta` 와 구별).
    /// 라운드트립 시 Entry JSON 이 정확히 `change_variable` 로 복원된다.
    ChangeVariable {
        variable: VarRef,
        value: Expr,
    },

    /// Entry `dialog` 의미 보존. mode 가 Say / Think 로 분기.
    Dialog {
        value: Expr,
        mode: DialogMode,
    },

    /// Entry `stop_run_all` 의미 보존. Rust 의 `break` 와는 다른 Entry 글로벌 stop.
    StopAll,

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
    Loop {
        body: Vec<Stmt>,
    },
    Return(Expr),
    Break,
    Continue,
}
