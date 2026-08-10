//! Entry 블록 JSON -> IR 역변환.
//!
//! `entrycore::block`의 `Block` enum이 Entry 의미의 통합 표현임.
//! 이 모듈은 Entry project.json의 블록 Value를 `Block`으로 바꾸고
//! 다시 IR `Stmt`/`Expr`로 변환한다.

use std::vec;

use crate::Error::UnmappedBlock;
use crate::block::{Block, ParamBlock};
use crate::ir::{BinOp, Expr, Stmt, UnaryOp};
use crate::var::VarMap;
use crate::{Result, ir};
use serde_json::Value;

/// 변수 ID를 VarMap으로 lookup하여 사용자 노출 이름으로 변환.
/// 매핑이 없으면 ID 그대로 사용.
fn resolve_var(id: &str, vars: &VarMap) -> String {
    vars.get(id)
        .map(|v| v.name.clone())
        .unwrap_or_else(|| id.to_string())
}

/// Entry `script` 필드(JSON 문자열 파싱 결과) -> IR Vec<Stmt>.
///
/// Entry의 script는 블록 묶음의 배열. 최상위는 트리거 묶음 배열.
/// 각 묶음의 첫 블록이 `when_*` 트리거이고, 묶음의 나머지 블록이 본문.
/// 트리거 묶음은 IR의 `FuncDef`로 변환 (이름은 트리거 함수명).
pub fn from_script(value: &Value, vars: &VarMap) -> Result<Vec<Stmt>> {
    let outer = value
        .as_array()
        .ok_or_else(|| crate::Error::Parse("script root must be array".into()))?;
    let mut stmts = Vec::new();
    for thread in outer {
        let blocks = thread
            .as_array()
            .ok_or_else(|| crate::Error::Parse("script thread must be array".into()))?;
        if blocks.is_empty() {
            continue;
        }
        let first = block_from_value(&blocks[0], vars)?;
        if let Some((fn_name, body_blocks)) = split_trigger(&first, &blocks[1..], vars) {
            let mut body = Vec::new();
            for b in body_blocks {
                from_block_owned(&b, &mut body, vars)?;
            }
            stmts.push(Stmt::FuncDef {
                name: fn_name,
                params: Vec::new(),
                body,
            });
        } else {
            let mut body = Vec::new();
            for b in blocks {
                let block = block_from_value(b, vars)?;
                from_block_owned(&block, &mut body, vars)?;
            }
            stmts.extend(body);
        }
    }
    Ok(stmts)
}

/// 트리거 블록이면 (함수 이름, 본문 블록들) 반환. 아니면 None.
fn split_trigger(first: &Block, rest: &[Value], vars: &VarMap) -> Option<(String, Vec<Block>)> {
    let name = match first {
        Block::WhenStart => "when_start",
        Block::WhenClick => "when_click",
        Block::WhenCloneStart => "when_clone_start",
        Block::WhenMessageRecv { .. } => "when_message",
        _ => return None,
    };
    let body = rest
        .iter()
        .map(|v| block_from_value(v, vars))
        .collect::<Result<Vec<_>>>()
        .ok()?;
    Some((name.to_string(), body))
}

/// Entry 블록 Value -> Block.
pub fn block_from_value(v: &Value, vars: &VarMap) -> Result<Block> {
    let obj = v
        .as_object()
        .ok_or_else(|| crate::Error::Parse("block must be object".into()))?;
    let type_id = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| crate::Error::Parse("block.type missing".into()))?;
    let params = obj.get("params").cloned().unwrap_or(Value::Null);

    let block = match type_id {
        // 시작 (트리거)
        "when_run_button_click" | "when_run" => Block::WhenStart,
        "when_click" | "when_object_click" => Block::WhenClick,
        "when_clone_start" => Block::WhenCloneStart,
        "when_message_cast" => {
            let msg = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Block::WhenMessageRecv { msg }
        }
        "when_some_key_pressed" => {
            let key_code = params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("81")
                .to_string();
            Block::WhenKeyPressed { key_code }
        }
        "mouse_clicked" => Block::WhenMouseClicked,
        "mouse_click_cancled" => Block::WhenMouseReleased,
        "when_object_click_canceled" => Block::WhenObjectReleased,
        "when_scene_start" => Block::WhenSceneStart,

        // 시작 (액션)
        "message_cast" => {
            let msg = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Block::MessageCast { msg }
        }
        "message_cast_wait" => {
            let msg = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Block::MessageCastWait { msg }
        }
        "start_scene" => {
            let scene = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Block::StartScene { scene }
        }
        "start_neighbor_scene" => {
            let direction = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("next")
                .to_string();
            Block::StartNeighborScene { direction }
        }

        // 변수
        "set_variable" => {
            let (variable, _name) = variable_slot(&params, 0)?;
            let variable = resolve_var(&variable, vars);
            let value = param_at(&params, 1, vars)?;
            Block::SetVar { variable, value }
        }
        "change_variable" => {
            let (variable, _name) = variable_slot(&params, 0)?;
            let variable = resolve_var(&variable, vars);
            let value = param_at(&params, 1, vars)?;
            Block::ChangeVar { variable, value }
        }
        "get_variable" => {
            let (variable, _name) = variable_slot(&params, 0)?;
            let variable = resolve_var(&variable, vars);
            Block::GetVar { variable }
        }
        "show_variable" | "hide_variable" => {
            let (variable, _name) = variable_slot(&params, 0)?;
            let variable = resolve_var(&variable, vars);
            if type_id == "show_variable" {
                Block::ShowVar { variable }
            } else {
                Block::HideVar { variable }
            }
        }

        // 흐름
        "if" => {
            let cond = param_at(&params, 0, vars)?;
            let body = statements_thread(obj, 0, vars)?;
            Block::If { cond, body }
        }
        "if_else" => {
            let cond = param_at(&params, 0, vars)?;
            let then_body = statements_thread(obj, 0, vars)?;
            let else_body = statements_thread(obj, 1, vars)?;
            Block::IfElse {
                cond,
                then_body,
                else_body,
            }
        }
        "repeat_while" => {
            let cond = param_at(&params, 0, vars)?;
            let body = statements_thread(obj, 0, vars)?;
            Block::While { cond, body }
        }
        "repeat_basic" => {
            let times = param_at(&params, 0, vars)?;
            let body = statements_thread(obj, 0, vars)?;
            Block::Repeat { times, body }
        }
        "repeat_forever" => {
            let body = statements_thread(obj, 0, vars)?;
            Block::Forever { body }
        }
        "wait_second" => {
            let time = param_at(&params, 0, vars)?;
            Block::WaitSeconds { time }
        }
        "wait_until_true" => {
            let cond = param_at(&params, 0, vars)?;
            Block::WaitUntilTrue { cond }
        }
        "stop_object" => Block::Break,
        "_continue" => Block::Continue,
        "stop_run_all" => Block::StopAll,

        // 산술/비교/논리
        "calc_basic" => {
            let lhs = param_at(&params, 0, vars)?;
            let op = op_at(&params, 1)?;
            let rhs = param_at(&params, 2, vars)?;
            Block::CalcBinOp { op, lhs, rhs }
        }
        "boolean_basic" | "boolean_basic_operator" => {
            let lhs = param_at(&params, 0, vars)?;
            let op = op_at(&params, 1)?;
            let rhs = param_at(&params, 2, vars)?;
            Block::Compare { op, lhs, rhs }
        }
        "boolean_and_or" => {
            let lhs = param_at(&params, 0, vars)?;
            let op = op_at(&params, 1)?;
            let rhs = param_at(&params, 2, vars)?;
            Block::BoolOp { op, lhs, rhs }
        }
        "calc_rand" => {
            let min = param_at(&params, 0, vars)?;
            let max = param_at(&params, 1, vars)?;
            Block::CalcRand { min, max }
        }
        "set_visible_project_timer" => {
            let value = params.get(0).and_then(Value::as_bool).unwrap_or(true);
            Block::SetVisibleProjectTimer { value }
        }
        "set_visible_answer" => {
            let value = params.get(0).and_then(Value::as_bool).unwrap_or(true);
            Block::SetVisibleAnswer { value }
        }
        "calc_unary" => {
            let expr = param_at(&params, 0, vars)?;
            let op_str = params.get(1).and_then(Value::as_str).unwrap_or("");
            let op = match op_str {
                "-" => UnaryOp::Neg,
                "!" => UnaryOp::Not,
                other => {
                    return Err(UnmappedBlock(format!("calc_unary op: {other}")));
                }
            };
            Block::UnaryOp { op, expr }
        }
        "get_project_timer_value" => Block::GetProjectTimerValue {},
        "ask_and_wait" => {
            let q = params
                .get(0)
                .map(|v| value_to_param(v, vars))
                .transpose()?
                .unwrap_or(ParamBlock::Null);
            Block::AskAndWait { question: q }
        }
        "get_canvas_input_value" => Block::GetCanvasInputValue {},
        "choose_project_timer_action" => Block::ChooseProjectTimerAction { action: params
                                                                                    .get(0)
                                                                                    .and_then(Value::as_str)
                                                                                    .unwrap_or("start")
                                                                                    .to_string(),
         },
        // 리터럴
        "number" => {
            let n = params
                .get(0)
                .and_then(|v| match v {
                    Value::Number(n) => n.as_f64(),
                    Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                })
                .ok_or_else(|| crate::Error::Parse("number param".into()))?;
            Block::Number(n)
        }
        "text" => {
            let s = params
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("text param".into()))?;
            Block::Text(s.to_string())
        }
        "boolean" => {
            let b = params
                .get(0)
                .and_then(Value::as_bool)
                .ok_or_else(|| crate::Error::Parse("boolean param".into()))?;
            Block::Boolean(b)
        }

        // 문자열
        "string_concat" => {
            let parts = params
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|v| value_to_param(v, vars))
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();
            Block::StringConcat { parts }
        }
        "string_index_of" => {
            let haystack = param_at(&params, 0, vars)?;
            let needle = param_at(&params, 1, vars)?;
            Block::StringIncludes { haystack, needle }
        }
        // 모양
        "show" => Block::Show {},
        "hide" => Block::Hide {},
        // 함수
        "function_call" => {
            let name = params
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("function_call name".into()))?
                .to_string();
            let args = match params.get(2) {
                Some(Value::Array(a)) => a
                    .iter()
                    .map(|v| value_to_param(v, vars))
                    .collect::<Result<Vec<_>>>()?,
                _ => Vec::new(),
            };
            Block::FuncCall { name, args }
        }
        // EntryJS 의 동적 함수 호출 블록. type = `func_<id>` 형식이며
        // id 는 project.functions[].id 와 매칭된다. args 슬롯은
        // EntryJS 가 동적 확장하므로 params[0] 만 (Indicator) 있다.
        // name 으로 id 를 그대로 두고 FuncCall 변환 (라운드트립 시
        // id 가 보존되어 build 가 다시 같은 func_<id> 블록을 생성).
        t if t.starts_with("func_") => {
            let name = t.to_string();
            Block::FuncCall {
                name,
                args: Vec::new(),
            }
        }
        "function_create" => {
            let name = params
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("function_create name".into()))?
                .to_string();
            let pnames = match params.get(1) {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect(),
                _ => Vec::new(),
            };
            let body = statements_thread(obj, 0, vars)?;
            Block::FuncDef {
                name,
                params: pnames,
                body,
            }
        }
        "function_return" => {
            let value = match params.get(0) {
                Some(v) if !v.is_null() => Some(value_to_param(v, vars)?),
                _ => None,
            };
            Block::Return { value }
        }

        other => return Err(UnmappedBlock(format!("entry block type: {other}"))),
    };
    Ok(block)
}

/// Entry `Value` -> `ParamBlock`.
fn value_to_param(v: &Value, vars: &VarMap) -> Result<ParamBlock> {
    if v.is_null() {
        return Ok(ParamBlock::Null);
    }
    if v.is_object() {
        // variable dropdown: codegen 이 `{id, name, variableType}` 형태로 emit.
        // `type` 키 없음 → block_from_value 호출하면 "block.type missing" 에러.
        // 이 분기를 먼저 처리해 ParamBlock::Variable 로 변환.
        if v.get("type").is_none() && v.get("id").is_some() && v.get("name").is_some() {
            let id = v["id"].as_str().unwrap_or("");
            let name = resolve_var(id, vars);
            return Ok(ParamBlock::Variable(name));
        }
        if let Some(t) = v.get("type").and_then(Value::as_str) {
            match t {
                "number" => {
                    if let Some(n) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(Value::as_f64)
                    {
                        return Ok(ParamBlock::Number(n));
                    }
                }
                "text" => {
                    if let Some(s) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(Value::as_str)
                    {
                        return Ok(ParamBlock::Text(s.to_string()));
                    }
                }
                "boolean" => {
                    if let Some(b) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(Value::as_bool)
                    {
                        return Ok(ParamBlock::Boolean(b));
                    }
                }
                _ => {}
            }
        }
        return Ok(ParamBlock::Sub(Box::new(block_from_value(v, vars)?)));
    }
    if let Some(n) = v.as_f64() {
        return Ok(ParamBlock::Number(n));
    }
    if let Some(s) = v.as_str() {
        return Ok(ParamBlock::Text(s.to_string()));
    }
    if let Some(b) = v.as_bool() {
        return Ok(ParamBlock::Boolean(b));
    }
    Err(crate::Error::Parse("unknown param shape".into()))
}

/// `params` 배열에서 인덱스 위치의 값 -> ParamBlock.
fn param_at(params: &Value, idx: usize, vars: &VarMap) -> Result<ParamBlock> {
    match params.get(idx) {
        Some(v) => value_to_param(v, vars),
        None => Ok(ParamBlock::Null),
    }
}

/// `params` 배열에서 변수 ID(첫 번째 슬롯) 추출.
fn variable_slot(params: &Value, idx: usize) -> Result<(String, Option<String>)> {
    let v = params.get(idx).cloned().unwrap_or(Value::Null);
    if v.is_null() {
        return Err(crate::Error::Parse("variable slot null".into()));
    }
    let id = if let Some(id) = v.get("id").and_then(Value::as_str) {
        id.to_string()
    } else if let Some(s) = v.as_str() {
        s.to_string()
    } else {
        return Err(crate::Error::Parse("variable slot shape".into()));
    };
    let name = v.get("name").and_then(Value::as_str).map(String::from);
    Ok((id, name))
}

/// Entry 블록 obj의 `statements[N]` 슬롯에서 블록 배열 추출.
fn statements_thread(
    obj: &serde_json::Map<String, Value>,
    idx: usize,
    vars: &VarMap,
) -> Result<Vec<Block>> {
    match obj.get("statements").and_then(Value::as_array) {
        Some(arr) => match arr.get(idx) {
            Some(Value::Array(b)) => b.iter().map(|v| block_from_value(v, vars)).collect(),
            _ => Ok(Vec::new()),
        },
        None => Ok(Vec::new()),
    }
}

/// `params` 배열에서 연산자 문자열 추출.
fn op_at(params: &Value, idx: usize) -> Result<BinOp> {
    let s = params
        .get(idx)
        .and_then(Value::as_str)
        .ok_or_else(|| crate::Error::Parse("operator slot".into()))?;
    Ok(match s {
        "+" | "PLUS" => BinOp::Add,
        "-" | "MINUS" => BinOp::Sub,
        "*" | "MULTI" => BinOp::Mul,
        "/" | "DIVIDE" => BinOp::Div,
        "%" | "MOD" => BinOp::Mod,
        "==" | "EQUAL" => BinOp::Eq,
        "!=" | "NOT_EQUAL" => BinOp::Ne,
        "<" | "LESS" => BinOp::Lt,
        "<=" | "LESS_OR_EQUAL" => BinOp::Le,
        ">" | "GREATER" => BinOp::Gt,
        ">=" | "GREATER_OR_EQUAL" => BinOp::Ge,
        "&&" | "AND" => BinOp::And,
        "||" | "OR" => BinOp::Or,
        other => return Err(UnmappedBlock(format!("op: {other}"))),
    })
}

/// `Block` 한 개를 IR `Vec<Stmt>`에 누적.
fn from_block_owned(block: &Block, stmts: &mut Vec<Stmt>, vars: &VarMap) -> Result<()> {
    match block {
        Block::WhenStart => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_start".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenClick => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_click".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenCloneStart => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_clone_start".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenMessageRecv { msg } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_message".to_string(),
                    arity: 1,
                },
                vec![Expr::Str(msg.clone())],
            )));
            Ok(())
        }
        Block::WhenKeyPressed { key_code } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_key_pressed".to_string(),
                    arity: 1,
                },
                vec![Expr::Str(key_code.clone())],
            )));
            Ok(())
        }
        Block::WhenMouseClicked => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_mouse_clicked".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenMouseReleased => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_mouse_released".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenObjectReleased => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_object_released".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenSceneStart => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_scene_start".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::MessageCast { msg } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "send_message".to_string(),
                    arity: 1,
                },
                vec![Expr::Str(msg.clone())],
            )));
            Ok(())
        }
        Block::MessageCastWait { msg } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "wait_message".to_string(),
                    arity: 1,
                },
                vec![Expr::Str(msg.clone())],
            )));
            Ok(())
        }
        Block::StartScene { scene } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "start_scene".to_string(),
                    arity: 1,
                },
                vec![Expr::Str(scene.clone())],
            )));
            Ok(())
        }
        Block::StartNeighborScene { direction } => {
            let name = match direction.as_str() {
                "prev" => "start_prev_scene",
                _ => "start_next_scene",
            };
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: name.to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SetVar { variable, value } => {
            stmts.push(Stmt::SetVar(
                variable.clone(),
                expr_from_param(value, vars)?,
            ));
            Ok(())
        }
        Block::ChangeVar { variable, value } => {
            let cur = Expr::Var(variable.clone());
            let rhs = expr_from_param(value, vars)?;
            stmts.push(Stmt::SetVar(
                variable.clone(),
                Expr::BinOp(BinOp::Add, Box::new(cur), Box::new(rhs)),
            ));
            Ok(())
        }
        Block::GetVar { .. } => Ok(()),
        Block::ShowVar { variable } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "show_var".to_string(),
                    arity: 1,
                },
                vec![Expr::Var(variable.clone())],
            )));
            Ok(())
        }
        Block::HideVar { variable } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "hide_var".to_string(),
                    arity: 1,
                },
                vec![Expr::Var(variable.clone())],
            )));
            Ok(())
        }
        Block::If { cond, body } => {
            let cond = expr_from_param(cond, vars)?;
            let mut then_body = Vec::new();
            for b in body {
                from_block_owned(b, &mut then_body, vars)?;
            }
            stmts.push(Stmt::If {
                cond,
                then_body,
                else_body: Vec::new(),
            });
            Ok(())
        }
        Block::IfElse {
            cond,
            then_body,
            else_body,
        } => {
            let cond = expr_from_param(cond, vars)?;
            let mut tb = Vec::new();
            for b in then_body {
                from_block_owned(b, &mut tb, vars)?;
            }
            let mut eb = Vec::new();
            for b in else_body {
                from_block_owned(b, &mut eb, vars)?;
            }
            stmts.push(Stmt::If {
                cond,
                then_body: tb,
                else_body: eb,
            });
            Ok(())
        }
        Block::While { cond, body } => {
            let cond = expr_from_param(cond, vars)?;
            let mut bb = Vec::new();
            for b in body {
                from_block_owned(b, &mut bb, vars)?;
            }
            stmts.push(Stmt::While { cond, body: bb });
            Ok(())
        }
        Block::Repeat { times, body } => {
            let times = expr_from_param(times, vars)?;
            let mut bb = Vec::new();
            for b in body {
                from_block_owned(b, &mut bb, vars)?;
            }
            stmts.push(Stmt::Repeat { times, body: bb });
            Ok(())
        }
        Block::Forever { body } => {
            let mut bb = Vec::new();
            for b in body {
                from_block_owned(b, &mut bb, vars)?;
            }
            stmts.push(Stmt::While {
                cond: Expr::Bool(true),
                body: bb,
            });
            Ok(())
        }
        Block::Break => {
            stmts.push(Stmt::Break);
            Ok(())
        }
        Block::Continue => {
            stmts.push(Stmt::Continue);
            Ok(())
        }
        Block::StopAll => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "stop_all".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::CalcBinOp { op, lhs, rhs }
        | Block::Compare { op, lhs, rhs }
        | Block::BoolOp { op, lhs, rhs } => {
            let lhs = expr_from_param(lhs, vars)?;
            let rhs = expr_from_param(rhs, vars)?;
            stmts.push(Stmt::Expr(Expr::BinOp(*op, Box::new(lhs), Box::new(rhs))));
            Ok(())
        }
        Block::UnaryOp { op, expr } => {
            let e = expr_from_param(expr, vars)?;
            stmts.push(Stmt::Expr(Expr::UnaryOp(*op, Box::new(e))));
            Ok(())
        }
        Block::Number(_) | Block::Text(_) | Block::Boolean(_) => Ok(()),
        Block::StringConcat { parts } => {
            let mut args = Vec::new();
            for p in parts {
                args.push(expr_from_param(p, vars)?);
            }
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "string_concat".to_string(),
                    arity: args.len(),
                },
                args,
            )));
            Ok(())
        }
        Block::StringIncludes { haystack, needle } => {
            let h = expr_from_param(haystack, vars)?;
            let n = expr_from_param(needle, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "string_contains".to_string(),
                    arity: 2,
                },
                vec![h, n],
            )));
            Ok(())
        }
        Block::FuncCall { name, args } => {
            let mut ir_args = Vec::new();
            for a in args {
                ir_args.push(expr_from_param(a, vars)?);
            }
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: name.clone(),
                    arity: ir_args.len(),
                },
                ir_args,
            )));
            Ok(())
        }
        Block::FuncDef { name, params, body } => {
            let mut bb = Vec::new();
            for b in body {
                from_block_owned(b, &mut bb, vars)?;
            }
            // Block::FuncDef 는 param name 만 보유. kind (String/Bool) 는
            // block layer 에서 손실 → 복원 불가. default String 처리.
            let param_pairs: Vec<(String, crate::ir::ParamKind)> = params
                .iter()
                .map(|n| (n.clone(), crate::ir::ParamKind::String))
                .collect();
            stmts.push(Stmt::FuncDef {
                name: name.clone(),
                params: param_pairs,
                body: bb,
            });
            Ok(())
        }
        Block::Return { value } => {
            let v = match value {
                Some(p) => expr_from_param(p, vars)?,
                None => Expr::Int(0),
            };
            stmts.push(Stmt::Return(v));
            Ok(())
        }
        Block::WaitSeconds { time } => {
            let arg = expr_from_param(time, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "wait_second".to_string(),
                    arity: 1,
                },
                vec![arg],
            )));
            Ok(())
        }
        Block::WaitUntilTrue { cond } => {
            let arg = expr_from_param(cond, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "wait_until_true".to_string(),
                    arity: 1,
                },
                vec![arg],
            )));
            Ok(())
        }
        Block::CalcRand { min, max } => {
            let m = expr_from_param(min, vars)?;
            let mx = expr_from_param(max, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "calc_rand".to_string(),
                    arity: 2,
                },
                vec![m, mx],
            )));
            Ok(())
        }
        Block::GetProjectTimerValue {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "get_project_timer_value".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::AskAndWait { question } => {
            let q = expr_from_param(question, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "ask_and_wait".to_string(),
                    arity: 1,
                },
                vec![q],
            )));
            Ok(())
        }
        Block::GetCanvasInputValue {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "get_canvas_input_value".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::Show {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "show".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::Hide {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "hide".to_string(),
                    arity: 0,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::ChooseProjectTimerAction { action } => {
            let fn_name = match action.as_str()
            {
                "start" => "start_timer",
                "stop" => "stop_timer",
                "reset" => "reset_timer",
                _ => "start_timer",   
            };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: fn_name.to_string(),
                    arity: 0,
                },
                Vec::new()
            )));
            Ok(())
        },
        Block::SetVisibleProjectTimer { value } => {
            let name = if *value { "show_timer" } else { "hide_timer" };
            stmts.push(Stmt::Expr(
                Expr::Call(
                  ir::FuncRef { name: name.to_string(), arity: 0 },
                  Vec::new()
                ),
            ));
            Ok(())
        },
        Block::SetVisibleAnswer { value }=>{
            let name = if *value { "show_answer" } else { "hide_answer"};
            stmts.push(Stmt::Expr(
                Expr::Call(
                    ir::FuncRef {
                        name: name.to_string(),
                        arity: 0
                    },
                    Vec::new()
                ),
            ));
            Ok(())
        }
    }
}

/// `ParamBlock` -> IR `Expr`.
fn expr_from_param(p: &ParamBlock, _vars: &VarMap) -> Result<Expr> {
    match p {
        ParamBlock::Null => Err(UnmappedBlock("null in expr slot".into())),
        ParamBlock::Number(n) => Ok(Expr::Float(*n)),
        ParamBlock::Text(s) => {
            // Entry `text` 블록이 숫자처럼 보이면 정수로 변환 (산술/비교 컨텍스트).
            if let Ok(i) = s.parse::<i64>() {
                Ok(Expr::Int(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Expr::Float(f))
            } else {
                Ok(Expr::Str(s.clone()))
            }
        }
        ParamBlock::Boolean(b) => Ok(Expr::Bool(*b)),
        ParamBlock::Variable(name) => Ok(Expr::Var(name.clone())),
        ParamBlock::Sub(b) => expr_from_block(b, _vars),
    }
}

/// `Block` -> IR `Expr` (값으로 쓰이는 블록).
fn expr_from_block(b: &Block, vars: &VarMap) -> Result<Expr> {
    match b {
        Block::Number(n) => Ok(Expr::Float(*n)),
        Block::Text(s) => {
            if let Ok(i) = s.parse::<i64>() {
                Ok(Expr::Int(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Expr::Float(f))
            } else {
                Ok(Expr::Str(s.clone()))
            }
        }
        Block::Boolean(b) => Ok(Expr::Bool(*b)),
        Block::GetVar { variable } => Ok(Expr::Var(variable.clone())),
        Block::CalcBinOp { op, lhs, rhs } => {
            let l = expr_from_param(lhs, vars)?;
            let r = expr_from_param(rhs, vars)?;
            Ok(Expr::BinOp(*op, Box::new(l), Box::new(r)))
        }
        Block::Compare { op, lhs, rhs } => {
            let l = expr_from_param(lhs, vars)?;
            let r = expr_from_param(rhs, vars)?;
            Ok(Expr::BinOp(*op, Box::new(l), Box::new(r)))
        }
        Block::BoolOp { op, lhs, rhs } => {
            let l = expr_from_param(lhs, vars)?;
            let r = expr_from_param(rhs, vars)?;
            Ok(Expr::BinOp(*op, Box::new(l), Box::new(r)))
        }
        Block::UnaryOp { op, expr } => {
            let e = expr_from_param(expr, vars)?;
            Ok(Expr::UnaryOp(*op, Box::new(e)))
        }
        Block::StringConcat { parts } => {
            let mut args = Vec::new();
            for p in parts {
                args.push(expr_from_param(p, vars)?);
            }
            Ok(Expr::Call(
                crate::ir::FuncRef {
                    name: "string_concat".to_string(),
                    arity: args.len(),
                },
                args,
            ))
        }
        Block::StringIncludes { haystack, needle } => {
            let h = expr_from_param(haystack, vars)?;
            let n = expr_from_param(needle, vars)?;
            Ok(Expr::Call(
                crate::ir::FuncRef {
                    name: "string_contains".to_string(),
                    arity: 2,
                },
                vec![h, n],
            ))
        }
        Block::FuncCall { name, args } => {
            let mut ir_args = Vec::new();
            for a in args {
                ir_args.push(expr_from_param(a, vars)?);
            }
            Ok(Expr::Call(
                crate::ir::FuncRef {
                    name: name.clone(),
                    arity: ir_args.len(),
                },
                ir_args,
            ))
        }
        Block::CalcRand { min, max } => {
            let m = expr_from_param(min, vars)?;
            let M = expr_from_param(max, vars)?;
            Ok(Expr::Call(
                crate::ir::FuncRef {
                    name: "calc_rand".to_string(),
                    arity: 2,
                },
                vec![m, M],
            ))
        }
        Block::GetProjectTimerValue {} => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_project_timer_value".to_string(),
                arity: 0,
            },
            Vec::new(),
        )),
        Block::AskAndWait { question } => {
            let q = expr_from_param(question, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "ask_and_wait".to_string(),
                    arity: 1,
                },
                vec![q],
            ))
        }
        Block::GetCanvasInputValue {} => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_canvas_input_value".to_string(),
                arity: 0,
            },
            Vec::new(),
        )),
        Block::Show {  } =>Ok(Expr::Call(
            ir::FuncRef {
                name: "show".to_string(),
                arity: 0,
            },
            Vec::new(),
        )),
        Block::Hide {  } => Ok(Expr::Call(
            ir::FuncRef {
                name: "hide".to_string(),
                arity: 0,
            },
            Vec::new(),
        )),
        Block::SetVar { .. }
        | Block::ChangeVar { .. }
        | Block::ShowVar { .. }
        | Block::HideVar { .. }
        | Block::If { .. }
        | Block::IfElse { .. }
        | Block::While { .. }
        | Block::Repeat { .. }
        | Block::Forever { .. }
        | Block::Break
        | Block::Continue
        | Block::StopAll
        | Block::WhenStart
        | Block::WhenClick
        | Block::WhenCloneStart
        | Block::WhenMessageRecv { .. }
        | Block::WhenKeyPressed { .. }
        | Block::WhenMouseClicked
        | Block::WhenMouseReleased
        | Block::WhenObjectReleased
        | Block::WhenSceneStart
        | Block::MessageCast { .. }
        | Block::MessageCastWait { .. }
        | Block::StartScene { .. }
        | Block::StartNeighborScene { .. }
        | Block::FuncDef { .. }
        | Block::WaitSeconds { .. }
        | Block::WaitUntilTrue { .. }
        | Block::Return { .. } => Err(UnmappedBlock(format!(
            "block used as expr: {}",
            b.type_id()
        ))),
        Block::ChooseProjectTimerAction { action } => {
            let fn_name = match action.as_str()
            {
                "start" => "start_timer",
                "stop" => "stop_timer",
                "reset" => "reset_timer",
                _ => "start_timer",   
            };
            Ok(Expr::Call(
                ir::FuncRef {
                    name: fn_name.to_string(),
                    arity: 0,
                },
                Vec::new()
            ))
        },
        Block::SetVisibleProjectTimer { value } => {
            Ok(Expr::Bool(*value))
        },
        Block::SetVisibleAnswer { value } => {
            Ok(Expr::Bool(*value))
        },
    }
}

/// Entry 프로젝트 `script` (JSON 문자열) -> IR `Program`. 변수 없음.
pub fn program_from_script_string(s: &str) -> Result<crate::ir::Program> {
    program_from_script_string_with_vars(s, &VarMap::new())
}

/// Entry 프로젝트 `script` (JSON 문자열) -> IR `Program`. 변수 맵 전달.
pub fn program_from_script_string_with_vars(s: &str, vars: &VarMap) -> Result<crate::ir::Program> {
    let v: Value = serde_json::from_str(s).map_err(|e| crate::Error::Parse(e.to_string()))?;
    program_from_script_value_with_vars(&v, vars)
}

/// Entry 오브젝트 `script` (`Value::String` 안의 JSON) -> IR `Program`.
pub fn program_from_script_value(v: &Value) -> Result<crate::ir::Program> {
    program_from_script_value_with_vars(v, &VarMap::new())
}

/// Entry 오브젝트 `script` (`Value::String` 안의 JSON) -> IR `Program`. 변수 맵 전달.
pub fn program_from_script_value_with_vars(v: &Value, vars: &VarMap) -> Result<crate::ir::Program> {
    let stmts = from_script(v, vars)?;
    Ok(crate::ir::Program { stmts })
}

/// scripts Value (`[[block, ...], ...]` 형태) 를 순회하며 매핑 안 되는 블록 타입을 집계.
/// 비파괴적 — IR 변환 없이 직접 walk. 재귀로 `statements` 안의 블록도 탐색.
///
/// `block_from_value` 가 single source of truth — 화이트리스트 유지 불필요.
/// 새 블록 추가 시 `block_from_value` 에 매핑만 추가하면 자동 반영.
///
/// ## 반환
/// `(type_name, count)` 목록. count 내림차순 → 이름 오름차순 정렬.
///
/// ## 사용
/// extract 시 오브젝트별 raw 폴백 외에, 전체 프로젝트의 미매핑 블록을 요약 출력할 때.
pub fn collect_unmapped_blocks(scripts: &Value, vars: &VarMap) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, usize> = HashMap::new();
    walk_blocks(scripts, &mut |block: &Value| {
        if let Some(t) = block.get("type").and_then(|x| x.as_str()) {
            if block_from_value(block, vars).is_err() {
                *counts.entry(t.to_string()).or_insert(0) += 1;
            }
        }
    });
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// scripts 트리를 재귀 walk. 각 block 마다 `f` 호출.
fn walk_blocks(value: &Value, f: &mut impl FnMut(&Value)) {
    match value {
        Value::Array(arr) => arr.iter().for_each(|v| walk_blocks(v, f)),
        Value::Object(_) => {
            f(value);
            if let Some(s) = value.get("statements").and_then(|x| x.as_array()) {
                s.iter().for_each(|t| walk_blocks(t, f));
            }
            if let Some(p) = value.get("params").and_then(|x| x.as_array()) {
                p.iter().for_each(|p| walk_blocks(p, f));
            }
        }
        _ => {}
    }
}
