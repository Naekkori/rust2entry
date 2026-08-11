//! Entry 블록 표현.
//!
//! IR(`crate::ir`)이 Rust 의미 보존, Block은 Entry 직렬화 친화.
//! 각 variant는 Entry 컴파일러가 인식하는 슬롯 구조를 가짐.

pub mod category;
pub mod registry;

use crate::Error::UnmappedBlock;
use crate::ir::{BinOp, Expr, Stmt, UnaryOp};
use crate::{Result, VarKind};

pub use category::Category;

use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub enum QamMethod {
    Quotient,
    Mod,
}

#[derive(Debug, Clone)]
pub enum MathOperation {
    Abs,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Ln,
    Log,
    Exp,
    Pow10,
}
/// 모든 Entry 블록의 통합 표현.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogMode {
    Say,
    Think,
}

/// 모든 Entry 블록의 통합 표현.
#[derive(Debug, Clone)]
pub enum Block {
    // ── 시작 (트리거) ──
    WhenStart,
    WhenClick,
    WhenCloneStart,
    WhenMessageRecv {
        msg: String,
    },
    /// `when_some_key_pressed` — 키 코드 (Keyboard dropdown, "q" → "81")
    WhenKeyPressed {
        key_code: String,
    },
    /// `mouse_clicked`
    WhenMouseClicked,
    /// `mouse_click_cancled`
    WhenMouseReleased,
    /// `when_object_click_canceled`
    WhenObjectReleased,
    /// `when_scene_start`
    WhenSceneStart,

    // ── 시작 (액션) ──
    /// `message_cast` — 메시지 이름 (EntryJS 는 DropdownDynamic 으로 받음).
    /// args 0 = 메시지 이름, args 1 = null (Indicator 자리).
    MessageCast {
        msg: String,
    },
    /// `message_cast_wait` — 보낸 후 수신자 실행 완료까지 대기.
    MessageCastWait {
        msg: String,
    },
    /// `start_scene` — 씬 id.
    StartScene {
        scene: String,
    },
    /// `start_neighbor_scene` — next 또는 prev.
    StartNeighborScene {
        direction: String,
    },

    // ── 변수 (출력: set_variable, change_variable, get_variable) ──
    SetVar {
        variable: String,
        value: ParamBlock,
    },
    ChangeVar {
        variable: String,
        value: ParamBlock,
    },
    GetVar {
        variable: String,
    },
    ShowVar {
        variable: String,
    },
    HideVar {
        variable: String,
    },
    SetVisibleProjectTimer {
        value: bool,
    },
    SetVisibleAnswer {
        value: bool,
    },

    // ── 흐름 (제어) ──
    If {
        cond: ParamBlock,
        body: Vec<Block>,
    },
    IfElse {
        cond: ParamBlock,
        then_body: Vec<Block>,
        else_body: Vec<Block>,
    },
    While {
        cond: ParamBlock,
        body: Vec<Block>,
    },
    Repeat {
        times: ParamBlock,
        body: Vec<Block>,
    },
    Forever {
        body: Vec<Block>,
    },
    Break,
    Continue,
    StopAll,
    WaitSeconds {
        time: ParamBlock,
    },
    WaitUntilTrue {
        cond: ParamBlock,
    },
    AskAndWait {
        question: ParamBlock,
    },
    GetCanvasInputValue {},

    // ── 산술 / 비교 / 논리 ──
    CalcBinOp {
        op: BinOp,
        lhs: ParamBlock,
        rhs: ParamBlock,
    },
    Compare {
        op: BinOp,
        lhs: ParamBlock,
        rhs: ParamBlock,
    },
    BoolOp {
        op: BinOp,
        lhs: ParamBlock,
        rhs: ParamBlock,
    },
    UnaryOp {
        op: UnaryOp,
        expr: ParamBlock,
    },
    CalcOperation {
        op: MathOperation,
        expr: ParamBlock,
    },
    CalcRand {
        min: ParamBlock,
        max: ParamBlock,
    },
    ChooseProjectTimerAction {
        action: String, // start, stop, reset
    },
    GetProjectTimerValue {},
    QuotientAndMod {
        a: ParamBlock,
        b: ParamBlock,
        mode: QamMethod,
    },
    // ── 리터럴 (단독 값) ──
    Number(f64),
    Text(String),
    Boolean(bool),
    Angle(f64),
    Color(String),
    // ── 문자열 ──
    StringConcat {
        parts: Vec<ParamBlock>,
    },
    StringIncludes {
        haystack: ParamBlock,
        needle: ParamBlock,
    },

    // ── 함수 ──
    FuncCall {
        name: String,
        args: Vec<ParamBlock>,
    },
    FuncDef {
        name: String,
        params: Vec<String>,
        body: Vec<Block>,
    },
    Return {
        value: Option<ParamBlock>,
    },

    // --- 모양 ---
    Show {},
    Hide {},
    Dialog { mode: DialogMode, content: ParamBlock}
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

/// Entry 시작 액션 reserved name → Block 변환.
/// 매칭 시 Some(Block) 반환, 매칭 안 되면 None.
fn reserved_start_call_to_block(
    fref: &crate::ir::FuncRef,
    args: &[crate::ir::Expr],
) -> Result<Option<Block>> {
    use crate::ir::Expr;
    let first_str = || -> String {
        match args.first() {
            Some(Expr::Str(s)) => s.clone(),
            _ => String::new(),
        }
    };
    Ok(Some(match fref.name.as_str() {
        "send_message" => Block::MessageCast { msg: first_str() },
        "wait_message" => Block::MessageCastWait { msg: first_str() },
        "start_scene" => Block::StartScene { scene: first_str() },
        "start_next_scene" => Block::StartNeighborScene {
            direction: "next".to_string(),
        },
        "start_prev_scene" => Block::StartNeighborScene {
            direction: "prev".to_string(),
        },
        _ => return Ok(None),
    }))
}

impl Block {
    /// Entry 블록 ID (type 문자열).
    pub fn type_id(&self) -> &'static str {
        match self {
            Block::WhenStart => "when_run",
            Block::WhenClick => "when_click",
            Block::WhenCloneStart => "when_clone_start",
            Block::WhenMessageRecv { .. } => "when_message_cast",
            Block::WhenKeyPressed { .. } => "when_some_key_pressed",
            Block::WhenMouseClicked => "mouse_clicked",
            Block::WhenMouseReleased => "mouse_click_cancled",
            Block::WhenObjectReleased => "when_object_click_canceled",
            Block::WhenSceneStart => "when_scene_start",
            Block::MessageCast { .. } => "message_cast",
            Block::MessageCastWait { .. } => "message_cast_wait",
            Block::StartScene { .. } => "start_scene",
            Block::StartNeighborScene { .. } => "start_neighbor_scene",
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
            Block::Angle(_) => "angle",
            Block::Color(_) => "color",
            Block::StringConcat { .. } => "string_concat",
            Block::StringIncludes { .. } => "string_index_of",
            Block::FuncCall { .. } => "function_call",
            Block::FuncDef { .. } => "function_create",
            Block::Return { .. } => "function_return",
            Block::WaitSeconds { .. } => "wait_second",
            Block::WaitUntilTrue { .. } => "wait_until_true",
            Block::AskAndWait { .. } => "ask_and_wait",
            Block::GetCanvasInputValue {} => "get_canvas_input_value",
            Block::CalcRand { .. } => "calc_rand",
            Block::GetProjectTimerValue {} => "get_project_timer_value",
            Block::Show {} => "show",
            Block::Hide {} => "hide",
            Block::ChooseProjectTimerAction { .. } => "choose_project_timer_action",
            Block::SetVisibleProjectTimer { .. } => "set_visible_project_timer",
            Block::SetVisibleAnswer { .. } => "set_visible_answer",
            Block::QuotientAndMod { .. } => "quotient_and_mod",
            Block::CalcOperation { .. } => "calc_operation",
            Block::Dialog { .. } => "dialog",
        }
    }

    /// BlockCategory.
    pub fn category(&self) -> Category {
        match self {
            Block::WhenStart
            | Block::WhenClick
            | Block::WhenCloneStart
            | Block::WhenMessageRecv { .. } => Category::Start,
            Block::SetVar { .. }
            | Block::ChangeVar { .. }
            | Block::GetVar { .. }
            | Block::ShowVar { .. }
            | Block::HideVar { .. } => Category::Variable,
            Block::If { .. } | Block::IfElse { .. } => Category::Flow,
            Block::While { .. } | Block::Repeat { .. } | Block::Forever { .. } => Category::Flow,
            Block::Break | Block::Continue | Block::StopAll => Category::Flow,
            Block::WhenKeyPressed { .. }
            | Block::WhenMouseClicked
            | Block::WhenMouseReleased
            | Block::WhenObjectReleased
            | Block::WhenSceneStart => Category::Start,
            Block::MessageCast { .. }
            | Block::MessageCastWait { .. }
            | Block::StartScene { .. }
            | Block::StartNeighborScene { .. } => Category::Start,
            Block::CalcBinOp { .. }
            | Block::Compare { .. }
            | Block::BoolOp { .. }
            | Block::UnaryOp { .. } => Category::Calc,
            Block::Number(_)
            | Block::Text(_)
            | Block::Boolean(_)
            | Block::Angle(_)
            | Block::Color(_) => Category::Calc,
            Block::CalcRand { .. } => Category::Calc,
            Block::StringConcat { .. } | Block::StringIncludes { .. } => Category::String,
            Block::FuncCall { .. } | Block::FuncDef { .. } | Block::Return { .. } => {
                Category::Define
            }
            Block::WaitSeconds { .. } => Category::Flow,
            Block::WaitUntilTrue { .. } => Category::Flow,
            Block::GetProjectTimerValue {} => Category::Calc,
            Block::AskAndWait { .. } => Category::Variable,
            Block::GetCanvasInputValue {} => Category::Variable,
            Block::Show {} => Category::Looks,
            Block::Hide {} => Category::Looks,
            Block::ChooseProjectTimerAction { .. } => Category::Calc,
            Block::SetVisibleProjectTimer { .. } => Category::Calc,
            Block::SetVisibleAnswer { .. } => Category::Variable,
            Block::QuotientAndMod { .. } => Category::Calc,
            Block::CalcOperation { .. } => Category::Calc,
            Block::Dialog { .. } => Category::Looks,
        }
    }
}

/// IR stmt -> Block 변환.
pub fn from_stmt(stmt: &crate::ir::Stmt) -> crate::Result<Block> {
    match stmt {
        Stmt::VarDecl(name, expr, _, _) | Stmt::SetVar(name, expr) => {
            // Timer/Answer/List 변수는 Entry 전용 슬롯만 받음. 일반 let/set 불가.
            if matches!(kind_for(name), VarKind::Timer | VarKind::Answer) {
                return Err(UnmappedBlock(format!(
                    "{name} is reserved Entry variable (use dedicated block)"
                )));
            }
            Ok(Block::SetVar {
                variable: name.clone(),
                value: from_expr(expr)?,
            })
        }
        Stmt::FuncDef { name, params, body } => {
            let body = body.iter().map(from_stmt).collect::<Result<Vec<_>>>()?;
            // IR param 의 (name, kind) → Block 은 name 만. kind 는 outer scope
            // (`lib.rs`) 에서 function_create head 빌드 시 사용.
            let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
            Ok(Block::FuncDef {
                name: name.clone(),
                params: param_names,
                body,
            })
        }
        Stmt::Expr(expr) => {
            match expr {
                Expr::Call(fref, args) => {
                    // Entry 시작 액션 — reserved name 으로 매칭되는 호출은
                    // 별도 Block 으로 변환 (EntryJS 가 정의한 function 이 아님).
                    if let Some(block) = reserved_start_call_to_block(fref, args)? {
                        return Ok(block);
                    }
                    if fref.name == "wait_second" {
                        let arg = args
                            .first()
                            .ok_or_else(|| UnmappedBlock("wait_second needs arg".into()))?;
                        return Ok(Block::WaitSeconds {
                            time: from_expr(arg)?,
                        });
                    }
                    if fref.name == "wait_until_true" {
                        let arg = args
                            .first()
                            .ok_or_else(|| UnmappedBlock("wait_until_true needs arg".into()))?;
                        return Ok(Block::WaitUntilTrue {
                            cond: from_expr(arg)?,
                        });
                    }
                    if fref.name == "calc_rand" {
                        if args.len() != 2 {
                            return Err(UnmappedBlock("calc_rand needs 2 args".into()));
                        }
                        let min = from_expr(&args[0])?;
                        let max = from_expr(&args[1])?;
                        return Ok(Block::CalcRand { min, max });
                    }
                    if fref.name == "get_project_timer_value" {
                        return Ok(Block::GetProjectTimerValue {});
                    }
                    if fref.name == "ask_and_wait" {
                        let arg = args
                            .first()
                            .ok_or_else(|| UnmappedBlock("ask_and_wait needs arg".into()))?;
                        return Ok(Block::AskAndWait {
                            question: from_expr(arg)?,
                        });
                    }
                    if fref.name == "get_canvas_input_value" {
                        return Ok(Block::GetCanvasInputValue {});
                    }
                    if fref.name == "show" {
                        return Ok(Block::Show {});
                    }
                    if fref.name == "hide" {
                        return Ok(Block::Hide {});
                    }
                    if fref.name == "show_timer" {
                        return Ok(Block::SetVisibleProjectTimer { value: true });
                    }
                    if fref.name == "hide_timer" {
                        return Ok(Block::SetVisibleProjectTimer { value: false });
                    }
                    if fref.name == "start_timer" {
                        return Ok(Block::ChooseProjectTimerAction {
                            action: "start".into(),
                        });
                    }
                    if fref.name == "stop_timer" {
                        return Ok(Block::ChooseProjectTimerAction {
                            action: "stop".into(),
                        });
                    }
                    if fref.name == "reset_timer" {
                        return Ok(Block::ChooseProjectTimerAction {
                            action: "reset".into(),
                        });
                    }
                    if fref.name == "show_answer" {
                        return Ok(Block::SetVisibleAnswer { value: true });
                    }
                    if fref.name == "hide_answer" {
                        return Ok(Block::SetVisibleAnswer { value: false });
                    }
                    if fref.name == "quotient_and_mod" {
                        if args.len() != 3 {
                            return Err(UnmappedBlock("quotient_and_mod needs 3 args".into()));
                        }
                        let mode = match &args[2] {
                            Expr::Str(s) if s == "quotient" => QamMethod::Quotient,
                            Expr::Str(s) if s == "modulo" => QamMethod::Mod,
                            _ => {
                                return Err(UnmappedBlock(
                                    "quotient_and_mod mode must be \"quotient\" \"modulo\" ".into(),
                                ));
                            }
                        };
                        let a = from_expr(&args[0])?;
                        let b = from_expr(&args[1])?;
                        return Ok(Block::QuotientAndMod { a, b, mode });
                    }
                    if fref.name == "say" {
                        let arg = args.first().ok_or_else(|| UnmappedBlock("say needs arg".into()))?;
                        return Ok(Block::Dialog { mode: DialogMode::Say, content: from_expr(arg)? });
                    }
                    if fref.name == "think" {
                        let arg = args.first().ok_or_else(|| UnmappedBlock("think needs arg".into()))?;
                        return Ok(Block::Dialog { mode: DialogMode::Think, content: from_expr(arg)? });
                    }
                    if let Some(op) = calc_op_from_name(&fref.name) {
                        let arg = args
                            .first()
                            .ok_or_else(|| UnmappedBlock(format!("{} needs arg", fref.name)))?;
                        return Ok(Block::CalcOperation {
                            op,
                            expr: from_expr(arg)?,
                        });
                    }
                    let args = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
                    Ok(Block::FuncCall {
                        name: fref.name.clone(),
                        args,
                    })
                }
                _ => Err(UnmappedBlock("stmt-level expr not a call".into())),
            }
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            let cond = from_expr(cond)?;
            let then_body = then_body
                .iter()
                .map(from_stmt)
                .collect::<Result<Vec<_>>>()?;
            let else_body = else_body
                .iter()
                .map(from_stmt)
                .collect::<Result<Vec<_>>>()?;
            if else_body.is_empty() {
                Ok(Block::If {
                    cond,
                    body: then_body,
                })
            } else {
                Ok(Block::IfElse {
                    cond,
                    then_body,
                    else_body,
                })
            }
        }
        Stmt::While { cond, body } => {
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
            Ok(Block::Repeat {
                times,
                body: new_body,
            })
        }
        Stmt::Return(expr) => Ok(Block::Return {
            value: Some(from_expr(expr)?),
        }),
        Stmt::Break => Ok(Block::Break),
        Stmt::Continue => Ok(Block::Continue),
    }
}

/// IR expr -> ParamBlock 변환.
pub fn from_expr(expr: &crate::ir::Expr) -> crate::Result<ParamBlock> {
    match expr {
        Expr::Int(n) => Ok(ParamBlock::Number(*n as f64)),
        Expr::Float(f) => Ok(ParamBlock::Number(*f)),
        Expr::Str(s) => Ok(ParamBlock::Text(s.clone())),
        Expr::Bool(b) => Ok(ParamBlock::Boolean(b.clone())),
        Expr::Var(name) => {
            // Timer/Answer는 전용 슬롯(get_xxx)에서만 읽음.
            if matches!(kind_for(name), VarKind::Timer | VarKind::Answer) {
                return Err(UnmappedBlock(format!("{name} read needs dedicated block")));
            }
            Ok(ParamBlock::Variable(name.clone()))
        }
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
        Expr::UnaryOp(op, expr) => Ok(ParamBlock::Sub(Box::new(Block::UnaryOp {
            op: *op,
            expr: from_expr(expr)?,
        }))),
        Expr::Call(fref, args) => {
            if fref.name == "calc_rand" {
                if args.len() != 2 {
                    return Err(UnmappedBlock("calc_rand needs 2 args".into()));
                }
                let min = from_expr(&args[0])?;
                let max = from_expr(&args[1])?;
                return Ok(ParamBlock::Sub(Box::new(Block::CalcRand { min, max })));
            }
            if fref.name == "say" {
                let arg = args.first().ok_or_else(|| UnmappedBlock("say needs arg".into()))?;
                return Ok(ParamBlock::Sub(Box::new(Block::Dialog { mode: DialogMode::Say, content: from_expr(arg)? })));
            }
            if fref.name == "think" {
                let arg = args.first().ok_or_else(|| UnmappedBlock("think needs arg".into()))?;
                return Ok(ParamBlock::Sub(Box::new(Block::Dialog { mode: DialogMode::Think, content: from_expr(arg)? })));
            }
            if fref.name == "get_project_timer_value" {
                return Ok(ParamBlock::Sub(Box::new(Block::GetProjectTimerValue {})));
            }
            if fref.name == "get_canvas_input_value" {
                return Ok(ParamBlock::Sub(Box::new(Block::GetCanvasInputValue {})));
            }
            if fref.name == "quotient_and_mod" {
                if args.len() != 3 {
                    return Err(UnmappedBlock("quotient_and_mod needs 3 args".into()));
                }
                let mode = match &args[2] {
                    Expr::Str(s) if s == "quotient" => QamMethod::Quotient,
                    Expr::Str(s) if s == "modulo" => QamMethod::Mod,
                    _ => {
                        return Err(UnmappedBlock(
                            "quotient_and_mod mode must be \"quotient\" \"modulo\"".into(),
                        ));
                    }
                };
                let a = from_expr(&args[0])?;
                let b = from_expr(&args[1])?;
                return Ok(ParamBlock::Sub(Box::new(Block::QuotientAndMod {
                    a,
                    b,
                    mode,
                })));
            }
            if let Some(op) = calc_op_from_name(&fref.name) {
                let arg = args.first().ok_or_else(|| UnmappedBlock(format!("{} needs arg", fref.name)))?;
                return Ok(
                    ParamBlock::Sub(
                        Box::new(
                            Block::CalcOperation { op, expr: from_expr(arg)? }
                        )
                    )
                );
            }
            let args = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
            Ok(ParamBlock::Sub(Box::new(Block::FuncCall {
                name: fref.name.clone(),
                args,
            })))
        }
        Expr::Func(_) => Err(UnmappedBlock("bare func ref".into())),
        Expr::Range(start, end) => {
            let _ = (start, end);
            Err(UnmappedBlock("range expr".into()))
        }
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
fn build_params_and_statements(block: &Block) -> crate::Result<(Vec<Value>, Option<Vec<Value>>)> {
    Ok(match block {
        Block::SetVar { variable, value } => (
            vec![variable_param(variable), param_to_value(value), Value::Null],
            None,
        ),
        Block::ChangeVar { variable, value } => (
            vec![variable_param(variable), param_to_value(value), Value::Null],
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
        Block::IfElse {
            cond,
            then_body,
            else_body,
        } => (
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
        Block::CalcOperation { op, expr } => {
            let op_str = match op {
                MathOperation::Abs => "abs",
                MathOperation::Sqrt => "sqrt",
                MathOperation::Sin => "sin",
                MathOperation::Cos => "cos",
                MathOperation::Tan => "tan",
                MathOperation::Asin => "asin",
                MathOperation::Acos => "acos",
                MathOperation::Atan => "atan",
                MathOperation::Ln => "ln",
                MathOperation::Log => "log",
                MathOperation::Exp => "exp",
                MathOperation::Pow10 => "pow10",
            };
            (vec![json!(op_str), param_to_value(expr)], None)
        }
        Block::Number(n) => (vec![Value::from(*n)], None),
        Block::Text(s) => (vec![Value::String(s.clone())], None),
        Block::Boolean(b) => (vec![Value::Bool(*b)], None),
        Block::Angle(n) => (vec![Value::from(*n)], None),
        Block::Color(s) => (vec![Value::String(s.clone())], None),
        Block::StringConcat { parts } => (parts.iter().map(param_to_value).collect(), None),
        Block::StringIncludes { haystack, needle } => {
            (vec![param_to_value(haystack), param_to_value(needle)], None)
        }
        Block::FuncCall { name, args } => (
            vec![
                Value::String(name.clone()),
                Value::Null,
                args.iter()
                    .map(param_to_value)
                    .collect::<Value>()
                    .as_array()
                    .cloned()
                    .map(Value::Array)
                    .unwrap_or(Value::Null),
            ],
            None,
        ),
        Block::FuncDef { name, params, body } => (
            vec![
                Value::String(name.clone()),
                params
                    .iter()
                    .map(|p| Value::String(p.clone()))
                    .collect::<Value>()
                    .as_array()
                    .cloned()
                    .map(Value::Array)
                    .unwrap_or(Value::Null),
            ],
            Some(vec![blocks_to_thread(body)?]),
        ),
        Block::Return { value } => (
            vec![value.as_ref().map(param_to_value).unwrap_or(Value::Null)],
            None,
        ),
        Block::WhenStart | Block::WhenClick | Block::WhenCloneStart => (vec![], None),
        Block::WhenMessageRecv { msg } => (vec![Value::String(msg.clone())], None),
        // when_some_key_pressed: [Indicator, Keyboard dropdown (key code)]
        // 우리 DSL: `when_key_pressed(key: &str)` — key code (예: "q"→"81").
        // EntryJS 기본 key 코드 = "81" ('q'). param 이 없으면 기본값.
        Block::WhenKeyPressed { key_code } => {
            (vec![Value::Null, Value::String(key_code.clone())], None)
        }
        Block::WhenMouseClicked
        | Block::WhenMouseReleased
        | Block::WhenObjectReleased
        | Block::WhenSceneStart => (vec![], None),
        // message_cast / message_cast_wait / start_scene:
        // [DropdownDynamic 메시지/씬, Indicator]. 우리 DSL 은 String literal 전달.
        Block::MessageCast { msg } | Block::MessageCastWait { msg } => {
            (vec![Value::String(msg.clone()), Value::Null], None)
        }
        Block::StartScene { scene } => (vec![Value::String(scene.clone()), Value::Null], None),
        // start_neighbor_scene: [Dropdown next/prev, Indicator]
        Block::StartNeighborScene { direction } => {
            (vec![Value::String(direction.clone()), Value::Null], None)
        }
        Block::WaitSeconds { time } => (vec![param_to_value(time)], None),
        Block::WaitUntilTrue { cond } => (vec![param_to_value(cond), Value::Null], None),
        Block::CalcRand { min, max } => (vec![param_to_value(min), param_to_value(max)], None),
        Block::GetProjectTimerValue {} => (vec![], None),
        Block::AskAndWait { question } => (vec![param_to_value(question), Value::Null], None),
        Block::GetCanvasInputValue {} => (vec![], None),
        Block::Show {} => (vec![], None),
        Block::Hide {} => (vec![], None),
        Block::ChooseProjectTimerAction { action } => (vec![json!(action)], None),
        Block::SetVisibleProjectTimer { value } => (vec![Value::Bool(*value), Value::Null], None),
        Block::SetVisibleAnswer { value } => (vec![Value::Bool(*value), Value::Null], None),
        Block::QuotientAndMod { a, b, mode } => {
            let mode_str = match mode {
                QamMethod::Quotient => "quotient",
                QamMethod::Mod => "modulo",
            };
            (
                vec![param_to_value(a), param_to_value(b), json!(mode_str)],
                None,
            )
        }
        Block::Dialog { mode, content } => (
            vec![
                param_to_value(content),
                Value::String(match mode {
                    DialogMode::Say => "say".into(),
                    DialogMode::Think => "think".into(),
                }),
                Value::Null,
            ],
            None,
        ),
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
        BinOp::Range => "..",
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
    let kind = kind_for(name);
    json!({ "id": id, "name": name, "variableType": kind_to_str(&kind) })
}

/// 이름 -> 해시 ID (간단한 해시).
pub fn id_for(name: &str) -> String {
    let mut h: u64 = 5381;
    for b in name.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    format!("{:x}", h)
}
/// 이름 -> kind?
pub fn kind_for(name: &str) -> crate::var::VarKind {
    match name {
        "초시계" | "timer" | "Timer" => VarKind::Timer,
        "대답" | "answer" | "Answer" => VarKind::Answer,
        "리스트" | "list" | "List" => VarKind::List,
        _ => VarKind::Variable,
    }
}

// VarKind -> Entry variableType 문자열
fn kind_to_str(kind: &crate::var::VarKind) -> &'static str {
    match kind {
        VarKind::Variable => "variable",
        VarKind::Timer => "timer",
        VarKind::List => "list",
        VarKind::Cloud => "cloud",
        VarKind::Answer => "answer",
        VarKind::RealTime => "realtime",
        VarKind::Unknown => "variable",
    }
}
/// Vec<Block> -> Thread.
fn blocks_to_thread(blocks: &[Block]) -> Result<Value> {
    let arr: Result<Vec<_>> = blocks.iter().map(to_value).collect();
    Ok(Value::Array(arr?))
}

// calc_op helper
fn calc_op_from_name(name: &str) -> Option<MathOperation> {
    Some(match name {
        "abs" => MathOperation::Abs,
        "sqrt" => MathOperation::Sqrt,
        "sin" => MathOperation::Sin,
        "cos" => MathOperation::Cos,
        "tan" => MathOperation::Tan,
        "asin" => MathOperation::Asin,
        "acos" => MathOperation::Acos,
        "atan" => MathOperation::Atan,
        "ln" => MathOperation::Ln,
        "log" => MathOperation::Log,
        "exp" => MathOperation::Exp,
        "pow10" => MathOperation::Pow10,
        _ => return None,
    })
}
