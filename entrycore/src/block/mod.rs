//! Entry 블록 표현.
//!
//! IR(`crate::ir`)이 Rust 의미 보존, Block은 Entry 직렬화 친화.
//! 각 variant는 Entry 컴파일러가 인식하는 슬롯 구조를 가짐.

pub mod category;
pub mod registry;

use crate::ir::{BinOp, Expr, Stmt, UnaryOp};
use crate::Error::UnmappedBlock;
use crate::Result;

pub use category::Category;

use serde_json::{json, Value};

/// 모든 Entry 블록의 통합 표현.
#[derive(Debug, Clone)]
pub enum Block {
    // ── 시작 ──
    WhenStart,
    WhenClick,
    WhenCloneStart,
    WhenMessageRecv { msg: String },

    // ── 변수 (출력: set_variable, change_variable, get_variable) ──
    SetVar { variable: String, value: ParamBlock },
    ChangeVar { variable: String, value: ParamBlock },
    GetVar { variable: String },

    ShowVar { variable: String },
    HideVar { variable: String },

    // ── 흐름 (제어) ──
    If { cond: ParamBlock, body: Vec<Block> },
    IfElse { cond: ParamBlock, then_body: Vec<Block>, else_body: Vec<Block> },
    While { cond: ParamBlock, body: Vec<Block> },
    Repeat { times: ParamBlock, body: Vec<Block> },
    Forever { body: Vec<Block> },
    Break,
    Continue,
    StopAll,

    // ── 산술 / 비교 / 논리 ──
    CalcBinOp { op: BinOp, lhs: ParamBlock, rhs: ParamBlock },
    Compare { op: BinOp, lhs: ParamBlock, rhs: ParamBlock },
    BoolOp { op: BinOp, lhs: ParamBlock, rhs: ParamBlock },
    UnaryOp { op: UnaryOp, expr: ParamBlock },

    // ── 리터럴 (단독 값) ──
    Number(f64),
    Text(String),
    Boolean(bool),

    // ── 문자열 ──
    StringConcat { parts: Vec<ParamBlock> },
    StringIncludes { haystack: ParamBlock, needle: ParamBlock },

    // ── 함수 ──
    FuncCall { name: String, args: Vec<ParamBlock> },
    FuncDef { name: String, params: Vec<String>, body: Vec<Block> },
    Return { value: Option<ParamBlock> },
}

/// 블록 파라미터 슬롯.
/// Entry 블록의 `params` 위치에 들어가는 값.
#[derive(Debug, Clone)]
pub enum ParamBlock {
    /// 빈 슬롯 (엔트리 `null`).
    Null,
    /// 숫자 리터럴.
    Number(f64),
    /// 문자열 리터럴.
    Text(String),
    /// 부울 리터럴.
    Boolean(bool),
    /// 변수 참조 (드롭다운 자리).
    Variable(String),
    /// 중첩 블록 (계산식 등).
    Sub(Box<Block>),
}

impl Block {
    /// Entry 블록 ID (type 문자열).
    pub fn type_id(&self) -> &'static str {
        match self {
            Block::WhenStart => "when_run",
            Block::WhenClick => "when_click",
            Block::WhenCloneStart => "when_clone_start",
            Block::WhenMessageRecv { .. } => "when_message_cast",
            Block::SetVar { .. } => "set_variable",
            Block::ChangeVar { .. } => "change_variable",
            Block::GetVar { .. } => "get_variable",
            Block::ShowVar { .. } => "show_variable",
            Block::HideVar { .. } => "hide_variable",
            Block::If { .. } => "if",
            Block::IfElse { .. } => "if_else",
            Block::While { .. } => "repeat_while",
            Block::Repeat { .. } => "repeat_basic",
            Block::Forever { .. } => "repeat_forever",
            Block::Break => "stop_object",
            Block::Continue => "_continue",
            Block::StopAll => "stop_run_all",
            Block::CalcBinOp { .. } => "calc_basic",
            Block::Compare { .. } => "boolean_basic",
            Block::BoolOp { .. } => "boolean_and_or",
            Block::UnaryOp { .. } => "calc_unary",
            Block::Number(_) => "number",
            Block::Text(_) => "text",
            Block::Boolean(_) => "boolean",
            Block::StringConcat { .. } => "string_concat",
            Block::StringIncludes { .. } => "string_index_of",
            Block::FuncCall { .. } => "function_call",
            Block::FuncDef { .. } => "function_create",
            Block::Return { .. } => "function_return",
        }
    }

    /// BlockCategory.
    pub fn category(&self) -> Category {
        match self {
            Block::WhenStart | Block::WhenClick | Block::WhenCloneStart | Block::WhenMessageRecv { .. } => {
                Category::Start
            }
            Block::SetVar { .. }
            | Block::ChangeVar { .. }
            | Block::GetVar { .. }
            | Block::ShowVar { .. }
            | Block::HideVar { .. } => Category::Variable,
            Block::If { .. } | Block::IfElse { .. } => Category::Flow,
            Block::While { .. } | Block::Repeat { .. } | Block::Forever { .. } => Category::Flow,
            Block::Break | Block::Continue | Block::StopAll => Category::Flow,
            Block::CalcBinOp { .. } | Block::Compare { .. } | Block::BoolOp { .. } | Block::UnaryOp { .. } => {
                Category::Calc
            }
            Block::Number(_) | Block::Text(_) | Block::Boolean(_) => Category::Calc,
            Block::StringConcat { .. } | Block::StringIncludes { .. } => Category::String,
            Block::FuncCall { .. } | Block::FuncDef { .. } | Block::Return { .. } => Category::Define,
        }
    }
}

/// IR stmt -> Block 변환.
pub fn from_stmt(stmt: &crate::ir::Stmt) -> crate::Result<Block> {
    match stmt {
        Stmt::VarDecl(name, expr) | Stmt::SetVar(name, expr)=>{
            Ok(Block::SetVar { variable: name.clone(), value: from_expr(expr)? })
        }
        Stmt::FuncDef { name, params, body } =>{
            let body = body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?;
            Ok(Block::FuncDef { name: name.clone(), params: params.clone(), body })
        }
        Stmt::Expr(expr)=>{
            match expr {
                Expr::Call(fref, args)=>{
                    let args = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
                    Ok(Block::FuncCall { name: fref.name.clone(), args })
                }
                _=> Err(UnmappedBlock("stmt-level expr not a call".into()))
            }
        }
        Stmt::If { cond, then_body, else_body }=>{
            let cond = from_expr(cond)?;
            let then_body = then_body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?;
            let else_body = else_body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?;
            if else_body.is_empty() {
                Ok(Block::If { cond, body: then_body })
            }else{
                Ok(Block::IfElse { cond, then_body, else_body })
            }
        }
        Stmt::While { cond, body }=>{
            let cond = from_expr(cond)?;
            let body = body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?;
            Ok(Block::While { cond, body })
        }
        Stmt::Repeat { times, body } => {
            let times = from_expr(times)?;
            let body = body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?;
            Ok(Block::Repeat { times, body })
        }
        Stmt::For { var, iter, body } => {
            // Rust `for i in a..b` -> Entry 펼침:
            //   repeat_basic(b - a)
            //     set_variable i a
            //     <body>
            //     change_variable i 1
            let (start, end) = match iter {
                Expr::Range(s, e) => (s.as_ref().clone(), e.as_ref().clone()),
                _ => return Err(UnmappedBlock("for iter not range".into())),
            };
            let start_pb = from_expr(&start)?;
            let end_pb = from_expr(&end)?;
            // times = end - start
            let times = ParamBlock::Sub(Box::new(Block::CalcBinOp {
                op: BinOp::Sub,
                lhs: end_pb,
                rhs: start_pb.clone(),
            }));
            let mut new_body = Vec::with_capacity(body.len() + 2);
            new_body.push(Block::SetVar {
                variable: var.clone(),
                value: start_pb,
            });
            new_body.extend(body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?);
            new_body.push(Block::ChangeVar {
                variable: var.clone(),
                value: ParamBlock::Number(1.0),
            });
            Ok(Block::Repeat { times, body: new_body })
        }
        Stmt::Return(expr)=>Ok(Block::Return { value: Some(from_expr(expr)?) }),
        Stmt::Break => Ok(Block::Break),
        Stmt::Continue => Ok(Block::Continue),
    }
}

/// IR expr -> ParamBlock 변환.
pub fn from_expr(expr: &crate::ir::Expr) -> crate::Result<ParamBlock> {
    match expr {
        Expr::Int(n)=>Ok(ParamBlock::Number(*n as f64)),
        Expr::Float(f) => Ok(ParamBlock::Number(*f)),
        Expr::Str(s) => Ok(ParamBlock::Text(s.clone())),
        Expr::Bool(b) => Ok(ParamBlock::Boolean(b.clone())),
        Expr::Var(name) => Ok(ParamBlock::Variable(name.clone())),
        Expr::BinOp(op, lhs, rhs) => {
            let lhs = from_expr(lhs)?;
            let rhs = from_expr(rhs)?;
            let block = match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    Block::Compare { op: *op, lhs, rhs }
                }
                BinOp::And | BinOp::Or => Block::BoolOp { op: *op, lhs, rhs },
                _ => Block::CalcBinOp { op: *op, lhs, rhs },
            };
            Ok(ParamBlock::Sub(Box::new(block)))
        }
        Expr::UnaryOp(op, expr) => Ok(ParamBlock::Sub(Box::new(
            Block::UnaryOp { op: *op, expr: from_expr(expr)?, }
        ))),
        Expr::Call(fref, args) => {
            let args = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
            Ok(ParamBlock::Sub(Box::new(Block::FuncCall { name: fref.name.clone(), args })))
        },
        Expr::Func(_) => Err(UnmappedBlock("bare func ref".into())),
        Expr::Range(start, end)=>{
            let _ = (start,end);
            Err(UnmappedBlock("range expr".into()))
        },
    }
}

/// Block -> serde_json::Value 변환.
///
/// Entry project.json 형식: `{type, params, statements?}`.
/// `statements[N]`은 본문 thread 배열 (없으면 키 생략).
pub fn to_value(block: &Block) -> crate::Result<Value> {
    let type_id = block.type_id();
    let (params, statements) = build_params_and_statements(block)?;
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), Value::String(type_id.into()));
    obj.insert("params".into(), Value::Array(params));
    if let Some(stmts) = statements {
        obj.insert("statements".into(), Value::Array(stmts));
    }
    Ok(Value::Object(obj))
}

/// `to_value` 내부 헬퍼. (params, Option<statements>) 분리 산출.
fn build_params_and_statements(
    block: &Block,
) -> crate::Result<(Vec<Value>, Option<Vec<Value>>)> {
    Ok(match block {
        Block::SetVar { variable, value } => (
            vec![
                variable_param(variable),
                param_to_value(value),
                Value::Null,
            ],
            None,
        ),
        Block::ChangeVar { variable, value } => (
            vec![
                variable_param(variable),
                param_to_value(value),
                Value::Null,
            ],
            None,
        ),
        Block::GetVar { variable } => (vec![variable_param(variable)], None),
        Block::ShowVar { variable } | Block::HideVar { variable } => {
            (vec![variable_param(variable), Value::Null], None)
        }
        Block::If { cond, body } => (
            vec![param_to_value(cond), Value::Null],
            Some(vec![blocks_to_thread(body)?]),
        ),
        Block::IfElse { cond, then_body, else_body } => (
            vec![param_to_value(cond), Value::Null],
            Some(vec![
                blocks_to_thread(then_body)?,
                blocks_to_thread(else_body)?,
            ]),
        ),
        Block::While { cond, body } => (
            vec![param_to_value(cond), Value::Null],
            Some(vec![blocks_to_thread(body)?]),
        ),
        Block::Repeat { times, body } => (
            vec![param_to_value(times), Value::Null],
            Some(vec![blocks_to_thread(body)?]),
        ),
        Block::Forever { body } => (vec![], Some(vec![blocks_to_thread(body)?])),
        Block::Break | Block::Continue | Block::StopAll => (vec![], None),
        Block::CalcBinOp { op, lhs, rhs } => (
            vec![
                param_to_value(lhs),
                Value::String(op_str(*op).to_string()),
                param_to_value(rhs),
            ],
            None,
        ),
        Block::Compare { op, lhs, rhs } => (
            vec![
                param_to_value(lhs),
                Value::String(op_str(*op).to_string()),
                param_to_value(rhs),
            ],
            None,
        ),
        Block::BoolOp { op, lhs, rhs } => (
            vec![
                param_to_value(lhs),
                Value::String(op_str(*op).to_string()),
                param_to_value(rhs),
            ],
            None,
        ),
        Block::UnaryOp { op, expr } => (
            vec![
                Value::String(match op {
                    UnaryOp::Neg => "-".into(),
                    UnaryOp::Not => "!".into(),
                }),
                param_to_value(expr),
            ],
            None,
        ),
        Block::Number(n) => (vec![Value::from(*n)], None),
        Block::Text(s) => (vec![Value::String(s.clone())], None),
        Block::Boolean(b) => (vec![Value::Bool(*b)], None),
        Block::StringConcat { parts } => (parts.iter().map(param_to_value).collect(), None),
        Block::StringIncludes { haystack, needle } => {
            (vec![param_to_value(haystack), param_to_value(needle)], None)
        }
        Block::FuncCall { name, args } => (
            vec![
                Value::String(name.clone()),
                Value::Null,
                args.iter().map(param_to_value).collect::<Value>().as_array()
                    .cloned()
                    .map(Value::Array)
                    .unwrap_or(Value::Null),
            ],
            None,
        ),
        Block::FuncDef { name, params, body } => (
            vec![
                Value::String(name.clone()),
                params.iter().map(|p| Value::String(p.clone())).collect::<Value>()
                    .as_array().cloned().map(Value::Array).unwrap_or(Value::Null),
            ],
            Some(vec![blocks_to_thread(body)?]),
        ),
        Block::Return { value } => (
            vec![value.as_ref().map(param_to_value).unwrap_or(Value::Null)],
            None,
        ),
        Block::WhenStart | Block::WhenClick | Block::WhenCloneStart => (vec![], None),
        Block::WhenMessageRecv { msg } => (vec![Value::String(msg.clone())], None),
    })
}

/// BinOp -> Entry 산술 비교 기호 문자열.
fn op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::Range => ".."
    }
}

/// ParamBlock -> JSON Value.
fn param_to_value(p: &ParamBlock) -> Value {
    match p {
        ParamBlock::Null => Value::Null,
        ParamBlock::Number(n) => json!({ "type": "number", "params": [n] }),
        ParamBlock::Text(s) => json!({ "type": "text", "params": [s] }),
        ParamBlock::Boolean(b) => json!({ "type": "boolean", "params": [b] }),
        ParamBlock::Variable(name) => variable_param(name),
        ParamBlock::Sub(b) => to_value(b).unwrap_or(Value::Null),
    }
}

/// 변수 드롭다운 슬롯.
fn variable_param(name: &str) -> Value {
    let id = id_for(name);
    json!({ "id": id, "name": name, "variableType": "variable" })
}

/// 이름 -> 해시 ID (간단한 해시).
pub fn id_for(name: &str) -> String {
    let mut h: u64 = 5381;
    for b in name.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    format!("{:x}", h)
}

/// Vec<Block> -> Thread.
fn blocks_to_thread(blocks: &[Block]) -> Result<Value> {
    let arr: Result<Vec<_>> = blocks.iter().map(to_value).collect();
    Ok(Value::Array(arr?))
}
