//! Entry 釉붾줉 JSON -> IR ?????
//!
//! `entrycore::block`??`Block` enum??Entry ?섎????듯빀 ?쒗쁽??
//! ??紐⑤뱢? Entry project.json??釉붾줉 Value瑜?`Block`?쇰줈 諛붽씀怨?
//! ?ㅼ떆 IR `Stmt`/`Expr`濡?蹂?섑븳??

use std::vec;

use crate::Error::{Parse, SyntaxError, UnmappedBlock};
use crate::block::{
    Block, CalcMethod, DateKind, DialogMode, Dimension, EffectType, MathOperation, ParamBlock,
    QamMethod, RowCol,
    calc_method_to_str, change_string_case_to_str, date_kind_to_str, device_type_to_str,
    dim_to_dsl_str, effect_to_str, mouse_axis_to_str, object_coord_to_str, rgb_channel_to_str,
    row_col_to_str, str_to_calc_method, str_to_change_string_case, str_to_mouse_axis,
    str_to_object_coord, str_to_rgb_channel, str_to_row_col, str_to_text_effect, text_effect_to_str,
};
use crate::ir::{BinOp, Expr, Stmt, UnaryOp, VarRef};
use crate::var::VarMap;
use crate::{Result, ir};
use serde_json::Value;

/// 蹂??ID瑜?VarMap?쇰줈 lookup?섏뿬 ?ъ슜???몄텧 ?대쫫?쇰줈 蹂??
/// 留ㅽ븨???놁쑝硫?ID 洹몃?濡??ъ슜.
fn resolve_var(id: &str, vars: &VarMap) -> String {
    vars.get(id)
        .map(|v| v.name.clone())
        .unwrap_or_else(|| id.to_string())
}

/// Entry `script` ?꾨뱶(JSON 臾몄옄???뚯떛 寃곌낵) -> IR Vec<Stmt>.
///
/// Entry??script??釉붾줉 臾띠쓬??諛곗뿴. 理쒖긽?꾨뒗 ?몃━嫄?臾띠쓬 諛곗뿴.
/// 媛?臾띠쓬??泥?釉붾줉??`when_*` ?몃━嫄곗씠怨? 臾띠쓬???섎㉧吏 釉붾줉??蹂몃Ц.
/// ?몃━嫄?臾띠쓬? IR??`FuncDef`濡?蹂??(?대쫫? ?몃━嫄??⑥닔紐?.
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
                return_type: None,
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

/// ?몃━嫄?釉붾줉?대㈃ (?⑥닔 ?대쫫, 蹂몃Ц 釉붾줉?? 諛섑솚. ?꾨땲硫?None.
fn split_trigger(first: &Block, rest: &[Value], vars: &VarMap) -> Option<(String, Vec<Block>)> {
    let name = match first {
        Block::WhenStart => "when_start",
        Block::WhenClick => "when_click",
        Block::WhenCloneStart => "when_clone_start",
        Block::WhenSceneStart => "when_scene_start",
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

/// Entry 釉붾줉 Value -> Block.
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
        // ?쒖옉 (?몃━嫄?
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

        // ?쒖옉 (?≪뀡)
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

        // 蹂??
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
        "set_func_variable" => {
            let (variable, _name) = variable_slot(&params, 0)?;
            // ?⑥닔 local var ??id ?뺤떇 `<func_id>_<hash>` ?먯꽌 func_id 遺遺꾨쭔 異붿텧?섏뿬
            // 蹂?섎챸?쇰줈???ъ슜. ?⑥닚?? variable name 洹몃?濡??ъ슜.
            let variable = resolve_var(&variable, vars);
            let value = param_at(&params, 1, vars)?;
            Block::SetFuncVariable { variable, value }
        }
        "get_func_variable" => {
            let (variable, _name) = variable_slot(&params, 0)?;
            let variable = resolve_var(&variable, vars);
            Block::GetFuncVariable { variable }
        }
        "show_list" | "hide_list" => {
            let (list, _name) = variable_slot(&params, 0)?;
            let list = resolve_var(&list, vars);
            if type_id == "show_list" {
                Block::ShowList { list }
            } else {
                Block::HideList { list }
            }
        }
        "value_of_index_from_list" => {
            let index = param_at(&params, 0, vars)?;
            let (list, _name) = variable_slot(&params, 1)?;
            let list = resolve_var(&list, vars);

            Block::ListValueAt { index, list }
        }
        "add_value_to_list" => {
            let value = param_at(&params, 0, vars)?;
            let (list, _name0) = variable_slot(&params, 1)?;
            let list = resolve_var(&list, vars);

            Block::AddValueToList { value, list }
        }
        "remove_value_from_list" => {
            let index = param_at(&params, 0, vars)?;
            let (list, _name) = variable_slot(&params, 1)?;
            let list = resolve_var(&list, vars);

            Block::RemoveValueFromList { index, list }
        }
        "insert_value_to_list" => {
            let value = param_at(&params, 0, vars)?;
            let index = param_at(&params, 1, vars)?;
            let (list, _name) = variable_slot(&params, 2)?;
            let list = resolve_var(&list, vars);
            Block::InsertValueToList { value, index, list }
        }
        "change_value_list_index" => {
            let index = param_at(&params, 0, vars)?;
            let value = param_at(&params, 1, vars)?;
            let (list, _name) = variable_slot(&params, 2)?;
            let list = resolve_var(&list, vars);

            Block::ChangeValueListIndex { index, value, list }
        }
        "length_of_list" => {
            // params = [Text, list, Text] ??list dropdown at index 1
            let (list, _name) = variable_slot(&params, 1)?;
            let list = resolve_var(&list, vars);
            Block::LengthOfList { list }
        }
        "is_included_in_list" => {
            // params = [Text, list, Text, value, Text]
            let (list, _name) = variable_slot(&params, 1)?;
            let list = resolve_var(&list, vars);
            let value = param_at(&params, 3, vars)?;
            Block::IsIncludedInList { list, value }
        }
        // ── 데이터분석 (테이블) — block_from_value type 매칭 ──
        // 첫 슬롯 = DropdownDynamic (런타임 채움) — params[0] 가 null 이거나
        // 실제 table id 일 수 있어 string 으로 안전 파싱.
        "append_row_to_table" => {
            let table = table_param(&params, 0);
            let dimension = row_col_param(&params, 1)?;
            Block::AppendRowToTable { table, dimension }
        }
        "insert_row_to_table" => {
            let table = table_param(&params, 0);
            let index = param_at(&params, 1, vars)?;
            let dimension = row_col_param(&params, 2)?;
            Block::InsertRowToTable { table, index, dimension }
        }
        "delete_row_from_table" => {
            let table = table_param(&params, 0);
            let index = param_at(&params, 1, vars)?;
            let dimension = row_col_param(&params, 2)?;
            Block::DeleteRowFromTable { table, index, dimension }
        }
        "set_value_from_table" => {
            let table = table_param(&params, 0);
            let row = param_at(&params, 1, vars)?;
            let field = param_at(&params, 2, vars)?;
            let value = param_at(&params, 3, vars)?;
            Block::SetValueFromTable { table, row, field, value }
        }
        "save_current_table" => {
            let table = table_param(&params, 0);
            Block::SaveCurrentTable { table }
        }
        "get_table_count" => {
            let table = table_param(&params, 0);
            let dimension = row_col_param(&params, 1)?;
            Block::GetTableCount { table, dimension }
        }
        "get_value_from_table" => {
            let table = table_param(&params, 0);
            let row = param_at(&params, 1, vars)?;
            let field = param_at(&params, 2, vars)?;
            Block::GetValueFromTable { table, row, field }
        }
        "get_value_from_last_row" => {
            let table = table_param(&params, 0);
            let field = param_at(&params, 1, vars)?;
            Block::GetValueFromLastRow { table, field }
        }
        "calc_values_from_table" => {
            let table = table_param(&params, 0);
            let field = param_at(&params, 1, vars)?;
            let method = calc_method_param(&params, 2)?;
            Block::CalcValuesFromTable { table, field, method }
        }
        "open_table" => {
            let table = table_param(&params, 0);
            Block::OpenTable { table }
        }
        "open_table_wait" => {
            let table = table_param(&params, 0);
            let seconds = param_at(&params, 1, vars)?;
            Block::OpenTableWait { table, seconds }
        }
        "open_table_chart" => {
            let table = table_param(&params, 0);
            // params[1] = DropdownDynamic (chart index) — string 으로 보관.
            let chart_index = params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Block::OpenTableChart { table, chart_index }
        }
        "close_table_chart" => Block::CloseTableChart,
        "get_coefficient" => {
            let table = table_param(&params, 0);
            let field1 = param_at(&params, 1, vars)?;
            let field2 = param_at(&params, 2, vars)?;
            Block::GetCoefficient { table, field1, field2 }
        }
        "set_value_from_cell" => {
            let table = table_param(&params, 0);
            let cell = param_at(&params, 1, vars)?;
            let value = param_at(&params, 2, vars)?;
            Block::SetValueFromCell { table, cell, value }
        }
        "get_value_from_cell" => {
            let table = table_param(&params, 0);
            let cell = param_at(&params, 1, vars)?;
            Block::GetValueFromCell { table, cell }
        }
        "get_value_v_lookup" => {
            let table = table_param(&params, 0);
            let field = param_at(&params, 1, vars)?;
            let value = param_at(&params, 2, vars)?;
            let return_field = param_at(&params, 3, vars)?;
            Block::GetValueVLookup { table, field, value, return_field }
        }
        // ?먮쫫
        "if" | "_if" => {
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
        "repeat_while" | "repeat_while_true" => {
            let cond = param_at(&params, 0, vars)?;
            let body = statements_thread(obj, 0, vars)?;
            // EntryJS `repeat_while_true` 의 cond 가 literal true 면 무한
            // 루프. Rust idiomatic 표현인 `loop { }` (EntryJS 의 `repeat_inf`)
            // 로 normalize 해서 .rs 가 자연스럽게 emit 되도록 한다.
            if matches!(cond, crate::block::ParamBlock::Boolean(true)) {
                Block::Forever { body }
            } else {
                Block::While { cond, body }
            }
        }
        "repeat_basic" => {
            let times = param_at(&params, 0, vars)?;
            let body = statements_thread(obj, 0, vars)?;
            Block::Repeat { times, body }
        }
        "repeat_forever" | "repeat_inf" => {
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
        "restart_project" => Block::RestartProject,
        "create_clone" => {
            let target = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("self")
                .to_string();
            Block::CreateClone { target }
        }
        "move_direction" => {
            let direction = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("forward")
                .to_string();
            let amount = param_at(&params, 1, vars)?;
            Block::MoveDirection { direction, amount }
        }
        "move_x" => {
            let amount = param_at(&params, 0, vars)?;
            Block::MoveX { amount }
        }
        "move_y" => {
            let amount = param_at(&params, 0, vars)?;
            Block::MoveY { amount }
        }
        "direction_relative" => {
            let angle = param_at(&params, 0, vars)?;
            Block::DirectionRelative { angle }
        }
        "move_xy_time" => {
            let duration = param_at(&params, 0, vars)?;
            let dx = param_at(&params, 1, vars)?;
            let dy = param_at(&params, 2, vars)?;
            Block::MoveXyTime { duration, dx, dy }
        }
        "locate_x" => {
            let x = param_at(&params, 0, vars)?;
            Block::LocateX { x }
        }
        "locate_y" => {
            let y = param_at(&params, 0, vars)?;
            Block::LocateY { y }
        }
        "locate_xy" => {
            let x = param_at(&params, 0, vars)?;
            let y = param_at(&params, 1, vars)?;
            Block::LocateXY { x, y }
        }
        "locate_xy_time" => {
            let duration = param_at(&params, 0, vars)?;
            let x = param_at(&params, 1, vars)?;
            let y = param_at(&params, 2, vars)?;
            Block::LocateXyTime { duration, x, y }
        }
        "locate" => {
            let target = param_at(&params, 0, vars)?;
            Block::Locate { target }
        }
        "locate_object_time" => {
            let duration = param_at(&params, 0, vars)?;
            let target = param_at(&params, 1, vars)?;
            Block::LocateObjectTime { duration, target }
        }
        "rotate_relative" => {
            let angle = param_at(&params, 0, vars)?;
            Block::RotateRelative { angle }
        }
        "rotate_by_time" => {
            let duration = param_at(&params, 0, vars)?;
            let angle = param_at(&params, 1, vars)?;
            Block::RotateByTime { duration, angle }
        }
        "rotate_absolute" => {
            let angle = param_at(&params, 0, vars)?;
            Block::RotateAbsolute { angle }
        }
        "direction_absolute" => {
            let angle = param_at(&params, 0, vars)?;
            Block::DirectionAbsolute { angle }
        }
        "see_angle_object" => {
            let target = param_at(&params, 0, vars)?;
            Block::SeeAngleObject { target }
        }
        "move_to_angle" => {
            let angle = param_at(&params, 0, vars)?;
            let distance = param_at(&params, 1, vars)?;
            Block::MoveToAngle { angle, distance }
        }
        // 遺?
        "brush_stamp" => Block::BrushStamp,
        "start_drawing" => Block::StartDrawing,
        "stop_drawing" => Block::StopDrawing,
        "start_fill" => Block::StartFill,
        "stop_fill" => Block::StopFill,
        "set_color" => {
            let r = param_at(&params, 0, vars)?;
            let g = param_at(&params, 1, vars)?;
            let b = param_at(&params, 2, vars)?;
            Block::SetColor { r, g, b }
        }
        "set_random_color" => Block::SetRandomColor,
        "direction_relative_duration" => {
            let duration = param_at(&params, 0, vars)?;
            let amount = param_at(&params, 1, vars)?;
            Block::DirectionRelativeDuration { duration, amount }
        }
        "change_brush_transparency" => {
            let amount = param_at(&params, 0, vars)?;
            Block::ChangeBrushTransparency { amount }
        }
        "set_brush_tranparency" => {
            let value = param_at(&params, 0, vars)?;
            Block::SetBrushTranparency { value }
        }
        "delete_clone" => Block::DeleteClone,
        "remove_all_clones" => Block::RemoveAllClones,
        "bounce_wall" => Block::BounceWall,
        "set_fill_color" => {
            let color = param_at(&params, 0, vars)?;
            Block::SetFillColor { color }
        }
        "change_thickness" => {
            let amount = param_at(&params, 0, vars)?;
            Block::ChangeThickness { amount }
        }
        "set_thickness" => {
            let value = param_at(&params, 0, vars)?;
            Block::SetThickness { value }
        }
        "brush_erase_all" => Block::BrushEraseAll,
        // 湲?곸옄
        "text_read" => {
            let value = param_at(&params, 0, vars)?;
            Block::TextRead { value }
        }
        "text_write" => {
            let content = param_at(&params, 0, vars)?;
            Block::TextWrite { content }
        }
        "text_append" => {
            let content = param_at(&params, 0, vars)?;
            Block::TextAppend { content }
        }
        "text_prepend" => {
            let content = param_at(&params, 0, vars)?;
            Block::TextPrepend { content }
        }
        "text_change_effect" => {
            let effect_pb = param_at(&params, 0, vars)?;
            let mode_pb = param_at(&params, 1, vars)?;
            let effect_str = match &effect_pb {
                ParamBlock::Text(s) => s,
                _ => {
                    return Err(SyntaxError(
                        "text_change_effect effect must be string".into(),
                    ));
                }
            };
            let effect = str_to_text_effect(effect_str)
                .ok_or_else(|| SyntaxError(format!("unknown text effect: {effect_str}")))?;
            let mode = match &mode_pb {
                ParamBlock::Text(s) if s == "on" => true,
                ParamBlock::Text(s) if s == "off" => false,
                _ => return Err(SyntaxError("text_change_effect mode must be on/off".into())),
            };
            Block::TextChangeEffect { effect, mode }
        }
        "text_flush" => Block::TextFlush, // null=?щ’?놁쓬
        "text_change_font" => {
            let font = match param_at(&params, 0, vars)? {
                ParamBlock::Text(font) => font,
                _ => return Err(SyntaxError("text_change_font font must be string".into())),
            };
            Block::TextChangeFont { font }
        }
        "text_change_font_color" => {
            let color = param_at(&params, 0, vars)?;
            Block::TextChangeFontColor { color }
        }
        "text_change_bg_color" => {
            let color = param_at(&params, 0, vars)?;
            Block::TextChangeBgColor { color }
        }
        // ?곗닠/鍮꾧탳/?쇰━
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
        "is_press_some_key" => {
            let key = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("space")
                .to_string();
            Block::IsPressSomeKey { key }
        }
        "is_clicked" => Block::IsClicked,
        "is_object_clicked" => Block::IsObjectClicked,
        "is_boost_mode" => Block::IsBoostMode,
        "is_touch_supported" => Block::IsTouchSupported,
        "is_type" => {
            let value = param_at(&params, 0, vars)?;
            let type_name = match params.get(2).and_then(Value::as_str).unwrap_or("number") {
                "number" => crate::block::EntryType::Number,
                "en" => crate::block::EntryType::En,
                "ko" => crate::block::EntryType::Ko,
                _ => return Err(crate::Error::Parse("is_type invalid type".into())),
            };
            Block::IsType { value, type_name }
        }
        "get_date" => {
            let kind_str = params.get(1).and_then(Value::as_str).unwrap_or("YEAR");
            let kind = match kind_str {
                "YEAR" => DateKind::Year,
                "MONTH" => DateKind::Month,
                "DAY" => DateKind::Day,
                "HOUR" => DateKind::Hour,
                "MINUTE" => DateKind::Minute,
                "SECOND" => DateKind::Second,
                _ => return Err(Parse("get_date invalid kind".into())),
            };
            Block::GetDate { kind }
        }
        "get_user_name" => Block::GetUserName,
        "get_nickname" => Block::GetNickName,
        "length_of_string" => {
            let value = param_at(&params, 1, vars)?;
            Block::LengthOfString { value }
        }
        "reverse_of_string" => {
            let value = param_at(&params, 1, vars)?;
            Block::ReverseOfString { value }
        }
        "combine_something" => {
            let a = param_at(&params, 1, vars)?;
            let b = param_at(&params, 3, vars)?;
            Block::CombineSomething { a, b }
        }
        "char_at" => {
            let string = param_at(&params, 1, vars)?;
            let index = param_at(&params, 3, vars)?;
            Block::CharAt { string, index }
        }
        "substring" => {
            let string = param_at(&params, 1, vars)?;
            let start = param_at(&params, 3, vars)?;
            let end = param_at(&params, 5, vars)?;
            Block::Substring { string, start, end }
        }
        "replace_string" => {
            let target = param_at(&params, 1, vars)?;
            let old = param_at(&params, 3, vars)?;
            let new = param_at(&params, 5, vars)?;
            Block::ReplaceString { target, old, new }
        }
        "count_match_string" => {
            let target = param_at(&params, 1, vars)?;
            let pattern = param_at(&params, 3, vars)?;
            Block::CountMatchString { target, pattern }
        }
        "index_of_string" => {
            let target = param_at(&params, 1, vars)?;
            let pattern = param_at(&params, 3, vars)?;
            Block::IndexOfString { target, pattern }
        }
        "change_string_case" => {
            let value = param_at(&params, 1, vars)?;
            let case_str = params
                .get(3)
                .and_then(Value::as_str)
                .unwrap_or("toUpperCase");
            let case = str_to_change_string_case(case_str).ok_or_else(|| {
                crate::Error::Parse(format!("change_string_case invalid case:{case_str}"))
            })?;
            Block::ChangeStringCase {
                target: value,
                case,
            }
        }
        "get_block_count" => {
            let target = param_at(&params, 0, vars)?;
            Block::GetBlockCount { target }
        }
        "change_rgb_to_hex" => {
            let r = param_at(&params, 0, vars)?;
            let g = param_at(&params, 1, vars)?;
            let b = param_at(&params, 2, vars)?;
            Block::ChangeRgbToHex { r, g, b }
        }
        "change_hex_to_rgb" => {
            let hex = param_at(&params, 0, vars)?;
            let channel_str = params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("r");
            let channel = str_to_rgb_channel(channel_str).ok_or_else(|| {
                crate::Error::Parse(format!("change_hex_to_rgb invalid channel:{channel_str}"))
            })?;
            Block::ChangeHexToRgb { hex, channel }
        }
        "get_boolean_value" => {
            let value = param_at(&params, 0, vars)?;
            Block::GetBooleanValue { value }
        }
        "reach_something" => {
            let target = params
                // EntryJS ?щ’? [Indicator, DropdownDynamic, Indicator]
                .get(1)
                .and_then(Value::as_str)
                // ?댁쟾???앹꽦??2?щ’ ?뺤떇([target, Indicator])???쎌쓬
                .or_else(|| params.get(0).and_then(Value::as_str))
                .unwrap_or("self")
                .to_string();
            Block::ReachSomeThing { target }
        }
        "coordinate_object" => {
            let target = params
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("coordinate_object target".into()))?
                .to_string();
            let coordinate = params
                .get(3)
                .and_then(Value::as_str)
                .and_then(str_to_object_coord)
                .ok_or_else(|| crate::Error::Parse("coordinate_object coordinate".into()))?;
            Block::CoordinateObject { target, coordinate }
        }
        "distance_something" => {
            let target = params
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("distance_something target".into()))?
                .to_string();
            Block::DistanceSomething { target }
        }
        "coordinate_mouse" => {
            let axis = params
                .get(1)
                .and_then(Value::as_str)
                .and_then(str_to_mouse_axis)
                .ok_or_else(|| crate::Error::Parse("coordinate_mouse axis".into()))?;
            Block::CoordinateMouse { axis }
        }
        "boolean_and_or" => {
            let lhs = param_at(&params, 0, vars)?;
            let op = op_at(&params, 1)?;
            let rhs = param_at(&params, 2, vars)?;
            Block::BoolOp { op, lhs, rhs }
        }
        "calc_rand" => {
            let min = param_at(&params, 1, vars)?;
            let max = param_at(&params, 3, vars)?;
            Block::CalcRand { min, max }
        }
        "set_visible_project_timer" => {
            let value = params
                .get(1)
                .and_then(Value::as_str)
                .map(|v| v == "SHOW")
                .unwrap_or(true);
            Block::SetVisibleProjectTimer { value }
        }
        "set_visible_answer" => {
            let value = params
                .get(1)
                .and_then(Value::as_str)
                .map(|v| v == "SHOW")
                .unwrap_or(true);
            Block::SetVisibleAnswer { value }
        }
        "calc_unary" | "boolean_not" => {
            let expr = param_at(&params, 0, vars)?;
            let op_str = params.get(1).and_then(Value::as_str).unwrap_or("");
            let op = match op_str {
                "-" => UnaryOp::Neg,
                "!" => UnaryOp::Not,
                other => {
                    return Err(SyntaxError(format!("calc_unary op: {other}")));
                }
            };
            Block::UnaryOp { op, expr }
        }
        "calc_operation" => {
            let op = match params.get(3).and_then(Value::as_str) {
                Some("abs") => MathOperation::Abs,
                Some("sqrt") => MathOperation::Sqrt,
                Some("sin") => MathOperation::Sin,
                Some("cos") => MathOperation::Cos,
                Some("tan") => MathOperation::Tan,
                Some("asin") => MathOperation::Asin,
                Some("acos") => MathOperation::Acos,
                Some("atan") => MathOperation::Atan,
                Some("ln") => MathOperation::Ln,
                Some("log") => MathOperation::Log,
                Some("exp") => MathOperation::Exp,
                Some("pow10") => MathOperation::Pow10,
                _ => MathOperation::Abs,
            };
            let expr = params
                .get(1)
                .map(|v| value_to_param(v, vars))
                .transpose()?
                .unwrap_or(ParamBlock::Null);
            Block::CalcOperation { op, expr }
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
        "choose_project_timer_action" => Block::ChooseProjectTimerAction {
            action: params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("start")
                .to_ascii_lowercase()
                .to_string(),
        },
        "quotient_and_mod" => {
            let mode = match params.get(5).and_then(Value::as_str) {
                Some("quotient") => QamMethod::Quotient,
                Some("modulo") => QamMethod::Mod,
                _ => QamMethod::Quotient,
            };
            let a = params
                .get(1)
                .map(|v| value_to_param(v, vars))
                .transpose()?
                .unwrap_or(ParamBlock::Null);
            let b = params
                .get(3)
                .map(|v| value_to_param(v, vars))
                .transpose()?
                .unwrap_or(ParamBlock::Null);

            Block::QuotientAndMod { a, b, mode }
        }
        // 由ы꽣??
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
        "angle" => {
            let n = params
                .get(0)
                .and_then(|v| match v {
                    Value::Number(n) => n.as_f64(),
                    Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                })
                .ok_or_else(|| crate::Error::Parse("angle param".into()))?;
            Block::Angle(n)
        }
        "color" => {
            let s = params
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("color param".into()))?;
            Block::Color(s.to_string())
        }

        // 臾몄옄??
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
        // 紐⑥뼇
        "show" => Block::Show {},
        "hide" => Block::Hide {},
        "remove_dialog" => Block::RemoveDialog {},
        "dialog" => {
            let content = param_at(&params, 0, vars)?;
            let mode = match params.get(1).and_then(Value::as_str) {
                Some("think") => DialogMode::Think,
                _ => DialogMode::Say,
            };
            Block::Dialog { mode, content }
        }
        "dialog_time" => {
            let content = param_at(&params, 0, vars)?;
            let time = param_at(&params, 2, vars)?;
            let mode = match params.get(1).and_then(Value::as_str) {
                Some("think") => DialogMode::Think,
                _ => DialogMode::Say,
            };
            Block::DialogTime {
                mode,
                content,
                time,
            }
        }
        "set_scale_size" => {
            let amount = param_at(&params, 0, vars)?;
            Block::SetScaleSize { amount }
        }
        "reset_scale_size" => Block::ResetScaleSize {},
        "flip_x" => Block::FlipX {},
        "flip_y" => Block::FlipY {},
        "change_object_index" => {
            let direction = params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("front")
                .to_string();
            Block::ChangeObjectIndex { direction }
        }
        // ?뚮━
        "sound_something_with_block" => {
            let sound_name = param_at(&params, 0, vars)?;
            Block::SoundSomethingWithBlock { sound_name }
        }
        "sound_something_second_with_block" => {
            let sound_name = param_at(&params, 0, vars)?;
            let seconds = param_at(&params, 1, vars)?;
            Block::SoundSomethingSecondWithBlock {
                sound_name,
                seconds,
            }
        }
        "sound_from_to" => {
            let sound_name = param_at(&params, 0, vars)?;
            let start = param_at(&params, 1, vars)?;
            let end = param_at(&params, 2, vars)?;
            Block::SoundFromTo {
                sound_name,
                start,
                end,
            }
        }
        "sound_something_wait_with_block" => {
            let sound_name = param_at(&params, 0, vars)?;
            Block::SoundSomethingWaitWithBlock { sound_name }
        }
        "sound_something_second_wait_with_block" => {
            let sound_name = param_at(&params, 0, vars)?;
            let seconds = param_at(&params, 1, vars)?;
            Block::SoundSomethingSecondWaitWithBlock {
                sound_name,
                seconds,
            }
        }
        "sound_from_to_and_wait" => {
            let sound_name = param_at(&params, 0, vars)?;
            let start = param_at(&params, 1, vars)?;
            let end = param_at(&params, 2, vars)?;
            Block::SoundFromToAndWait {
                sound_name,
                start,
                end,
            }
        }
        "sound_volume_change" => {
            let amount = param_at(&params, 0, vars)?;
            Block::SoundVolumeChange { amount }
        }
        "sound_volume_set" => {
            let amount = param_at(&params, 0, vars)?;
            Block::SoundVolumeSet { amount }
        }
        "sound_speed_change" => {
            let amount = param_at(&params, 0, vars)?;
            Block::SoundSpeedChange { amount }
        }
        "sound_speed_set" => {
            let amount = param_at(&params, 0, vars)?;
            Block::SoundSpeedSet { amount }
        }
        "get_sound_speed" => Block::GetSoundSpeed,
        "sound_silent_all" => {
            let target = params
                .as_array()
                .and_then(|values| values.first())
                .and_then(Value::as_str)
                .unwrap_or("all")
                .to_string();
            Block::SoundSilentAll { target }
        }
        "play_bgm" => {
            let sound_name = param_at(&params, 0, vars)?;
            Block::PlayBgm { sound_name }
        }
        "stop_bgm" => Block::StopBgm,
        "get_sound_volume" => Block::GetSoundVolume,
        "get_sound_duration" => {
            let sound_name = params
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("get_sound_duration sound".into()))?
                .to_string();
            Block::GetSoundDuration { sound_name }
        }
        // ?⑥닔
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
        "change_to_some_shape" => {
            // ?꾩옱 EntryJS ?뺤떇? `get_pictures` 媛?釉붾줉 ?덉뿉 ?대?吏 ID瑜??붾떎.
            // ?댁쟾???앹꽦???먯떆 臾몄옄???뺤떇??extract ?명솚???꾪빐 ?쎈뒗??
            let picture = params
                .get(0)
                .and_then(|value| {
                    value
                        .get("type")
                        .filter(|kind| *kind == "get_pictures")
                        .and_then(|_| value.get("params"))
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .or_else(|| value.as_str())
                })
                .unwrap_or("")
                .to_string();
            Block::ChangeToSomeShape { picture }
        }
        "change_to_next_shape" => Block::ChangeToNextShape {},
        "stretch_scale_size" => {
            let dim = match params.get(0).and_then(Value::as_str).unwrap_or("WIDTH") {
                "HEIGHT" => Dimension::Height,
                _ => Dimension::Width,
            };
            let value = param_at(&params, 1, vars)?;
            Block::StretchScaleSize { dim, value }
        }
        "add_effect_amount" => {
            let effect_s = params.get(0).and_then(Value::as_str).unwrap_or("color");
            let effect = match effect_s {
                "color" => EffectType::Color,
                "brightness" => EffectType::Brightness,
                "ghost" => EffectType::Ghost,
                "fisheye" => EffectType::Fisheye,
                "whirl" => EffectType::Whirl,
                "pixelate" => EffectType::Pixelate,
                "mosaic" => EffectType::Mosaic,
                "negative" => EffectType::Negative,
                _ => EffectType::Color,
            };
            let amount = param_at(&params, 1, vars)?;
            Block::AddEffectAmount { effect, amount }
        }
        "change_effect_amount" => {
            let effect_s = params.get(0).and_then(Value::as_str).unwrap_or("color");
            let effect = match effect_s {
                "color" => EffectType::Color,
                "brightness" => EffectType::Brightness,
                "ghost" => EffectType::Ghost,
                "fisheye" => EffectType::Fisheye,
                "whirl" => EffectType::Whirl,
                "pixelate" => EffectType::Pixelate,
                "mosaic" => EffectType::Mosaic,
                "negative" => EffectType::Negative,
                _ => EffectType::Color,
            };
            let amount = param_at(&params, 1, vars)?;
            Block::ChangeEffectAmount { effect, amount }
        }
        "erase_all_effects" => Block::EraseAllEffects {},
        "change_scale_size" => {
            let amount = param_at(&params, 0, vars)?;
            Block::ChangeScaleSize { amount }
        }
        // EntryJS ???숈쟻 ?⑥닔 ?몄텧 釉붾줉. type = `func_<id>` ?뺤떇?대ŉ
        // id ??project.functions[].id ? 留ㅼ묶?쒕떎. args ?щ’?
        // EntryJS 媛 ?숈쟻 ?뺤옣?섎?濡?params[0] 留?(Indicator) ?덈떎.
        // name ?쇰줈 id 瑜?洹몃?濡??먭퀬 FuncCall 蹂??(?쇱슫?쒗듃由???
        // id 媛 蹂댁〈?섏뼱 build 媛 ?ㅼ떆 媛숈? func_<id> 釉붾줉???앹꽦).
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
        "function_create_value" => {
            // function_create_value ??paramsKeyMap: FIELD=0, VALUE=3
            // params = [function_field_label_chain, Indicator(null), LineBreak(null), VALUE block]
            let name = params
                .get(0)
                .and_then(Value::as_str)
                .ok_or_else(|| crate::Error::Parse("function_create_value name".into()))?
                .to_string();
            let pnames = match params.get(1) {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect(),
                _ => Vec::new(),
            };
            let mut body = statements_thread(obj, 0, vars)?;
            // VALUE ?щ’ (params[3]) ??Block::Return { value } 濡?蹂????蹂몃Ц ?앹뿉 push.
            let return_value: Option<ParamBlock> = params
                .get(3)
                .filter(|v| !v.is_null())
                .map(|v| value_to_param(v, vars))
                .transpose()?;
            if let Some(value) = return_value {
                body.push(Block::Return { value: Some(value) });
            }
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

        // ?섎뱶?⑥뼱 釉붾윮 (?뚯뒪留??몃뜳?? ???먮낯 .ent 釉붾윮 JSON ??raw 濡?蹂댁〈.
        other if crate::block::registry::is_hw_block(other) => Block::Raw {
            type_id: other.to_string(),
            raw: v.clone(),
        },
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
        // variable dropdown: codegen ??`{id, name, variableType}` ?뺥깭濡?emit.
        // `type` ???놁쓬 ??block_from_value ?몄텧?섎㈃ "block.type missing" ?먮윭.
        // ??遺꾧린瑜?癒쇱? 泥섎━??ParamBlock::Variable 濡?蹂??
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
                "boolean" | "True" | "False" => {
                    if let Some(b) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(Value::as_bool)
                    {
                        return Ok(ParamBlock::Boolean(b));
                    }
                    // True/False literal 블록은 params 가 빈 경우도 있음 — 그 경우 true/false 자체가 의미.
                    return Ok(ParamBlock::Boolean(t == "True"));
                }
                "get_sounds" => {
                    if let Some(id) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                    {
                        return Ok(ParamBlock::Text(id.to_string()));
                    }
                }
                "get_pictures" => {
                    if let Some(id) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                    {
                        return Ok(ParamBlock::Text(id.to_string()));
                    }
                }
                "text_color" => {
                    if let Some(c) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                    {
                        return Ok(ParamBlock::Text(c.to_string()));
                    }
                }
                "function_field_label" => {
                    // 함수 정의 헤드. 라운드트립은 lib.rs 가 처리 — 자리 유지.
                    let name = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    return Ok(ParamBlock::Text(name));
                }
                "function_field_string" => {
                    let name = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    return Ok(ParamBlock::Text(name));
                }
                "function_field_boolean" => {
                    let b = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    return Ok(ParamBlock::Boolean(b));
                }
                "text_box_with_self" | "textBoxWithSelf" => {
                    // 글상자 dropdown — 라운드트립용 placeholder.
                    let id = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .unwrap_or("self")
                        .to_string();
                    return Ok(ParamBlock::Text(id));
                }
                "get_table_fields" => {
                    let id = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    return Ok(ParamBlock::Text(id));
                }
                "angle" => {
                    if let Some(n) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|a| a.first())
                        .and_then(Value::as_f64)
                    {
                        return Ok(ParamBlock::Number(n));
                    }
                }
                "wildcard_string" | "wildcard_boolean" => {
                    // 함수 param placeholder. 값 슬롯 자리.
                    return Ok(ParamBlock::Null);
                }
                "boolean_not" => {
                    // 단항 부정 — 값 자리 placeholder.
                    let expr = if let Some(p) = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                    {
                        value_to_param(p, vars)?
                    } else {
                        ParamBlock::Null
                    };
                    return Ok(ParamBlock::Sub(Box::new(Block::UnaryOp {
                        op: UnaryOp::Not,
                        expr,
                    })));
                }
                "get_sound_volume" => {
                    return Ok(ParamBlock::Sub(Box::new(Block::GetSoundVolume)));
                }
                "get_sound_duration" => {
                    let sound_name = v
                        .get("params")
                        .and_then(Value::as_array)
                        .and_then(|items| items.get(1))
                        .and_then(Value::as_str)
                        .ok_or_else(|| crate::Error::Parse("get_sound_duration sound".into()))?
                        .to_string();
                    return Ok(ParamBlock::Sub(Box::new(Block::GetSoundDuration {
                        sound_name,
                    })));
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

/// `params` 諛곗뿴?먯꽌 ?몃뜳???꾩튂??媛?-> ParamBlock.
fn param_at(params: &Value, idx: usize, vars: &VarMap) -> Result<ParamBlock> {
    match params.get(idx) {
        Some(v) => value_to_param(v, vars),
        None => Ok(ParamBlock::Null),
    }
}

/// `params[idx]` 에서 table id string 추출. DropdownDynamic 이 비어있으면 "".
fn table_param(params: &Value, idx: usize) -> String {
    params
        .get(idx)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// `params[idx]` 에서 RowCol 드롭다운 문자열 파싱.
fn row_col_param(params: &Value, idx: usize) -> Result<RowCol> {
    let s = params
        .get(idx)
        .and_then(Value::as_str)
        .unwrap_or("ROW");
    str_to_row_col(s).map_err(|_| SyntaxError(format!("invalid row/col dropdown: {s}")))
}

/// `params[idx]` 에서 CalcMethod 드롭다운 문자열 파싱.
fn calc_method_param(params: &Value, idx: usize) -> Result<CalcMethod> {
    let s = params
        .get(idx)
        .and_then(Value::as_str)
        .unwrap_or("SUM");
    str_to_calc_method(s).map_err(|_| SyntaxError(format!("invalid calc method dropdown: {s}")))
}

/// `params` 諛곗뿴?먯꽌 蹂??ID(泥?踰덉㎏ ?щ’) 異붿텧.
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

/// Entry 釉붾줉 obj??`statements[N]` ?щ’?먯꽌 釉붾줉 諛곗뿴 異붿텧.
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

/// `params` 諛곗뿴?먯꽌 ?곗궛??臾몄옄??異붿텧.
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
        other => return Err(SyntaxError(format!("op: {other}"))),
    })
}

/// `Block` ??媛쒕? IR `Vec<Stmt>`???꾩쟻.
#[allow(unreachable_patterns)]
fn from_block_owned(block: &Block, stmts: &mut Vec<Stmt>, vars: &VarMap) -> Result<()> {
    match block {
        Block::WhenStart => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "when_start".to_string(),
                    arity: 0,
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::WhenSceneStart => {
            // Entry `when_scene_start` 트리거 의미 보존: Expr::Call 로
            // 떨어뜨리면 codegen 의 reserved 매칭이 거부된다 (when_scene_start
            // 은 EntryJS 의 트리거 함수). 빈 본문 FuncDef 로 emit 해서
            // from_script 의 split_trigger 가 `when_scene_start` 트리거로 인식.
            stmts.push(Stmt::FuncDef {
                name: "when_scene_start".to_string(),
                params: Vec::new(),
                return_type: None,
                body: Vec::new(),
            });
            Ok(())
        }
        Block::MessageCast { msg } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "send_message".to_string(),
                    arity: 1,
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SetVar { variable, value } => {
            stmts.push(Stmt::SetVar(
                VarRef::new(variable.clone()),
                expr_from_param(value, vars)?,
            ));
            Ok(())
        }
        Block::SetFuncVariable { variable, value } => {
            // ?⑥닔 蹂몃Ц local var ?ㅼ젙. IR ?덈꺼?뿉?쒕뒗 ?쇰컲 Stmt::SetVar (scope=Local ?쒖떆 ??????
            // from_stmt_with_fn_scope ?ㅇ≪?????ш린 ?ㅼ뼱?ㅻ㈃ set_func_variable ?쇰835로 emit ??.
            stmts.push(Stmt::SetVar(
                VarRef::new(variable.clone()),
                expr_from_param(value, vars)?,
            ));
            Ok(())
        }
        Block::GetFuncVariable { variable } => {
            // ?⑥닔 蹂몃Ц local var ?쎄린. 媛??щ’ ?먮━??Expr::Var 濡??쒗쁽.
            stmts.push(Stmt::Expr(Expr::Var(variable.clone())));
            Ok(())
        }
        Block::ChangeVar { variable, value } => {
            // Entry `change_variable` 의미 보존: SetVar(BinOp(Add, ...)) 로
            // 평탄화하면 라운드트립 시 set_variable 로 떨어진다. ChangeVariable
            // variant 로 그대로 들고 가서 codegen 에서 change_variable 로 emit.
            stmts.push(Stmt::ChangeVariable {
                variable: VarRef::new(variable.clone()),
                value: expr_from_param(value, vars)?,
            });
            Ok(())
        }
        Block::GetVar { .. } => Ok(()),
        Block::ShowVar { variable } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "show_var".to_string(),
                    arity: 1,
                    raw: None,
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
                    raw: None,
                },
                vec![Expr::Var(variable.clone())],
            )));
            Ok(())
        }
        Block::ShowList { list } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "show_list".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Var(list.clone())],
            )));
            Ok(())
        }
        Block::HideList { list } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "hide_list".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Var(list.clone())],
            )));
            Ok(())
        }
        // ?? ?곗씠?곕텇??(?뚯씠釉? ??
        Block::AppendRowToTable { table, dimension } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "append_row_to_table".to_string(), arity: 2, raw: None },
                vec![Expr::Str(table.clone()), Expr::Str(row_col_to_str(*dimension).to_string())],
            )));
            Ok(())
        }
        Block::InsertRowToTable { table, index, dimension } => {
            let index = expr_from_param(index, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "insert_row_to_table".to_string(), arity: 3, raw: None },
                vec![Expr::Str(table.clone()), index, Expr::Str(row_col_to_str(*dimension).to_string())],
            )));
            Ok(())
        }
        Block::DeleteRowFromTable { table, index, dimension } => {
            let index = expr_from_param(index, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "delete_row_from_table".to_string(), arity: 3, raw: None },
                vec![Expr::Str(table.clone()), index, Expr::Str(row_col_to_str(*dimension).to_string())],
            )));
            Ok(())
        }
        Block::SetValueFromTable { table, row, field, value } => {
            let row = expr_from_param(row, vars)?;
            let field = expr_from_param(field, vars)?;
            let value = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "set_value_from_table".to_string(), arity: 4, raw: None },
                vec![Expr::Str(table.clone()), row, field, value],
            )));
            Ok(())
        }
        Block::SaveCurrentTable { table } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "save_current_table".to_string(), arity: 1, raw: None },
                vec![Expr::Str(table.clone())],
            )));
            Ok(())
        }
        Block::GetTableCount { .. } => Err(SyntaxError(
            "get_table_count is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromTable { .. } => Err(SyntaxError(
            "get_value_from_table is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromLastRow { .. } => Err(SyntaxError(
            "get_value_from_last_row is a value block and cannot be used as a statement".into(),
        )),
        Block::CalcValuesFromTable { .. } => Err(SyntaxError(
            "calc_values_from_table is a value block and cannot be used as a statement".into(),
        )),
        Block::OpenTable { table } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "open_table".to_string(), arity: 1, raw: None },
                vec![Expr::Str(table.clone())],
            )));
            Ok(())
        }
        Block::OpenTableWait { table, seconds } => {
            let seconds = expr_from_param(seconds, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "open_table_wait".to_string(), arity: 2, raw: None },
                vec![Expr::Str(table.clone()), seconds],
            )));
            Ok(())
        }
        Block::OpenTableChart { table, chart_index } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "open_table_chart".to_string(), arity: 2, raw: None },
                vec![Expr::Str(table.clone()), Expr::Str(chart_index.clone())],
            )));
            Ok(())
        }
        Block::CloseTableChart => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "close_table_chart".to_string(), arity: 0, raw: None },
                Vec::new(),
            )));
            Ok(())
        }
        Block::GetCoefficient { .. } => Err(SyntaxError(
            "get_coefficient is a value block and cannot be used as a statement".into(),
        )),
        Block::SetValueFromCell { table, cell, value } => {
            let cell = expr_from_param(cell, vars)?;
            let value = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef { name: "set_value_from_cell".to_string(), arity: 3, raw: None },
                vec![Expr::Str(table.clone()), cell, value],
            )));
            Ok(())
        }
        Block::GetValueFromCell { .. } => Err(SyntaxError(
            "get_value_from_cell is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueVLookup { .. } => Err(SyntaxError(
            "get_value_v_lookup is a value block and cannot be used as a statement".into(),
        )),

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
            // Entry `repeat_inf` 의미 보존: While(Bool(true)) 로 떨어뜨리면
            // IR 일관성이 깨진다 (의미는 같지만 Entry -> IR 매핑이 무한 루프와
            // 일반 while 루프를 구분하지 못함). Loop variant 로 직접 emit.
            let mut bb = Vec::new();
            for b in body {
                from_block_owned(b, &mut bb, vars)?;
            }
            stmts.push(Stmt::Loop { body: bb });
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
            // Entry `stop_run_all` 의미 보존: Expr::Call("stop_all") 로
            // 떨어뜨리면 codegen 의 reserved 매칭이 안 돼 function_call 로
            // emit 된다. 전용 Stmt::StopAll 로 직접 들고 간다.
            stmts.push(Stmt::StopAll);
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
        Block::Number(_)
        | Block::Text(_)
        | Block::Boolean(_)
        | Block::Angle(_)
        | Block::Color(_) => Ok(()),
        Block::StringConcat { parts } => {
            let mut args = Vec::new();
            for p in parts {
                args.push(expr_from_param(p, vars)?);
            }
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: "string_concat".to_string(),
                    arity: args.len(),
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
            // Block::FuncDef ??param name 留?蹂댁쑀. kind (String/Bool) ??
            // block layer ?먯꽌 ?먯떎 ??蹂듭썝 遺덇?. default String 泥섎━.
            let param_pairs: Vec<(String, crate::ir::ParamKind)> = params
                .iter()
                .map(|n| (n.clone(), crate::ir::ParamKind::String))
                .collect();
            stmts.push(Stmt::FuncDef {
                name: name.clone(),
                params: param_pairs,
                return_type: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
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
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::ChooseProjectTimerAction { action } => {
            let fn_name = match action.as_str() {
                "start" => "start_timer",
                "stop" => "stop_timer",
                "reset" => "reset_timer",
                _ => "start_timer",
            };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: fn_name.to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SetVisibleProjectTimer { value } => {
            let name = if *value { "show_timer" } else { "hide_timer" };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: name.to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SetVisibleAnswer { value } => {
            let name = if *value { "show_answer" } else { "hide_answer" };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: name.to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::QuotientAndMod { a, b, mode } => {
            let av = expr_from_param(a, vars)?;
            let bv = expr_from_param(b, vars)?;
            let mode_str = match mode {
                QamMethod::Quotient => "quotient",
                QamMethod::Mod => "modulo",
            };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "quotient_and_mod".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![av, bv, Expr::Str(mode_str.to_string())],
            )));
            Ok(())
        }
        Block::CalcOperation { op, expr } => {
            let fn_name = math_op_to_name(op);
            let e = expr_from_param(expr, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: fn_name.to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![e],
            )));
            Ok(())
        }
        Block::Dialog { mode, content } => {
            // Entry `dialog` 의 Say/Think mode 의미 보존: Expr::Call 로
            // 떨어뜨리면 codegen 이 reserved 매칭 없이 function_call 로
            // emit 해서 mode 가 사라진다. 전용 Stmt::Dialog 로 직접 emit.
            stmts.push(Stmt::Dialog {
                value: expr_from_param(content, vars)?,
                mode: *mode,
            });
            Ok(())
        }
        Block::DialogTime {
            mode,
            content,
            time,
        } => {
            let content_arg = expr_from_param(content, vars)?;
            let time_arg = expr_from_param(time, vars)?;
            let name = match mode {
                DialogMode::Say => "say",
                DialogMode::Think => "think",
            };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: name.to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![content_arg, time_arg],
            )));
            Ok(())
        }
        Block::ChangeToSomeShape { picture } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_to_some_shape".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(picture.clone())],
            )));
            Ok(())
        }
        Block::ChangeToNextShape {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_to_next_shape".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::AddEffectAmount { effect, amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "add_effect_amount".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(effect_to_str(*effect).to_string()), a],
            )));
            Ok(())
        }
        Block::StretchScaleSize { dim, value } => {
            let v = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "stretch_scale_size".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(dim_to_dsl_str(dim).to_string()), v],
            )));
            Ok(())
        }
        Block::RemoveDialog {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "remove_dialog".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::ChangeEffectAmount { effect, amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_effect_amount".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(effect_to_str(*effect).to_string()), a],
            )));
            Ok(())
        }
        Block::EraseAllEffects {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "erase_all_effects".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::ChangeScaleSize { amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_scale_size".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::SetScaleSize { amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "set_scale_size".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::ResetScaleSize {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "reset_scale_size".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::FlipX {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "flip_x".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::FlipY {} => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "flip_y".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::ChangeObjectIndex { direction } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_object_index".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(direction.clone())],
            )));
            Ok(())
        }
        Block::DeleteClone => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "delete_clone".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::RemoveAllClones => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "remove_all_clones".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::ListValueAt { index, list } => {
            let index = expr_from_param(index, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "value_of_index_from_list".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![index, Expr::Var(list.clone())],
            )));
            Ok(())
        }
        Block::AddValueToList { value, list } => {
            let value = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "add_value_to_list".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![value, Expr::Var(list.clone())],
            )));
            Ok(())
        }
        Block::RemoveValueFromList { index, list } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "remove_value_from_list".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![expr_from_param(index, vars)?, Expr::Var(list.clone())],
            )));
            Ok(())
        }
        Block::InsertValueToList { value, index, list } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "insert_value_to_list".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![
                    expr_from_param(value, vars)?,
                    expr_from_param(index, vars)?,
                    Expr::Var(list.clone()),
                ],
            )));
            Ok(())
        }
        Block::ChangeValueListIndex { index, value, list } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_value_list_index".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![
                    expr_from_param(index, vars)?,
                    expr_from_param(value, vars)?,
                    Expr::Var(list.clone()),
                ],
            )));
            Ok(())
        }
        Block::LengthOfList { list } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "length_of_list".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Var(list.clone())],
            )));
            Ok(())
        }
        Block::IsIncludedInList { list, value } => {
            let value = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_included_in_list".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Var(list.clone()), value],
            )));
            Ok(())
        }
        Block::RestartProject => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "restart_project".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::CreateClone { target } => {
            let args = if target == "self" {
                Vec::new()
            } else {
                vec![Expr::Str(target.clone())]
            };
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "create_clone".to_string(),
                    arity: args.len(),
                    raw: None,
                },
                args,
            )));
            Ok(())
        }
        Block::MoveDirection { direction, amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "move_direction".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(direction.clone()), a],
            )));
            Ok(())
        }
        Block::Raw { type_id, raw } => {
            stmts.push(Stmt::Expr(Expr::Call(
                crate::ir::FuncRef {
                    name: type_id.clone(),
                    arity: hw_raw_arg_count(raw),
                    raw: Some(raw.clone()),
                },
                hw_raw_args(raw, vars),
            )));
            Ok(())
        }
        Block::IsPressSomeKey { key } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_press_some_key".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(key.clone())],
            )));
            Ok(())
        }
        Block::ReachSomeThing { target } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "reach_something".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(target.clone())],
            )));
            Ok(())
        }
        Block::CoordinateMouse { .. } => Err(SyntaxError(
            "coordinate_mouse is a value block and cannot be used as a statement".into(),
        )),
        Block::GetTableCount { .. } => Err(SyntaxError(
            "get_table_count is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromTable { .. } => Err(SyntaxError(
            "get_value_from_table is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromLastRow { .. } => Err(SyntaxError(
            "get_value_from_last_row is a value block and cannot be used as a statement".into(),
        )),
        Block::CalcValuesFromTable { .. } => Err(SyntaxError(
            "calc_values_from_table is a value block and cannot be used as a statement".into(),
        )),
        Block::GetCoefficient { .. } => Err(SyntaxError(
            "get_coefficient is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromCell { .. } => Err(SyntaxError(
            "get_value_from_cell is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueVLookup { .. } => Err(SyntaxError(
            "get_value_v_lookup is a value block and cannot be used as a statement".into(),
        )),
        Block::BounceWall => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "bounce_wall".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::MoveX { amount } => {
            let arg = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "move_x".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![arg],
            )));
            Ok(())
        }
        Block::MoveY { amount } => {
            let arg = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "move_y".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![arg],
            )));
            Ok(())
        }
        Block::RotateRelative { angle } => {
            let arg = expr_from_param(angle, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "rotate_relative".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![arg],
            )));
            Ok(())
        }
        Block::DirectionRelative { angle } => {
            let arg = expr_from_param(angle, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "direction_relative".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![arg],
            )));
            Ok(())
        }
        Block::IsClicked => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_clicked".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::IsObjectClicked => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_object_clicked".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::IsType { value, type_name } => {
            let value = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_type".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![
                    value,
                    Expr::Str(
                        match type_name {
                            crate::block::EntryType::Number => "number",
                            crate::block::EntryType::En => "en",
                            crate::block::EntryType::Ko => "ko",
                        }
                        .to_string(),
                    ),
                ],
            )));
            Ok(())
        }
        Block::MoveXyTime { duration, dx, dy } => {
            let duration = expr_from_param(duration, vars)?;
            let dx = expr_from_param(dx, vars)?;
            let dy = expr_from_param(dy, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "move_xy_time".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![duration, dx, dy],
            )));
            Ok(())
        }
        Block::LocateX { x } => {
            let x = expr_from_param(x, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "locate_x".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![x],
            )));
            Ok(())
        }
        Block::LocateY { y } => {
            let y = expr_from_param(y, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "locate_y".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![y],
            )));
            Ok(())
        }
        Block::LocateXY { x, y } => {
            let x = expr_from_param(x, vars)?;
            let y = expr_from_param(y, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "locate_xy".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![x, y],
            )));
            Ok(())
        }
        Block::LocateXyTime { duration, x, y } => {
            let duration = expr_from_param(duration, vars)?;
            let x = expr_from_param(x, vars)?;
            let y = expr_from_param(y, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "locate_xy_time".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![duration, x, y],
            )));
            Ok(())
        }
        Block::LocateObjectTime { duration, target } => {
            let duration = expr_from_param(duration, vars)?;
            let target = expr_from_param(target, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "locate_object_time".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![duration, target],
            )));
            Ok(())
        }
        Block::Locate { target } => {
            let target = expr_from_param(target, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "locate".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![target],
            )));
            Ok(())
        }
        Block::RotateByTime { duration, angle } => {
            let d = expr_from_param(duration, vars)?;
            let a = expr_from_param(angle, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "rotate_by_time".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![d, a],
            )));
            Ok(())
        }
        Block::DirectionRelativeDuration { duration, amount } => {
            let d = expr_from_param(duration, vars)?;
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "direction_relative_duration".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![d, a],
            )));
            Ok(())
        }
        Block::RotateAbsolute { angle } => {
            let a = expr_from_param(angle, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "rotate_absolute".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::DirectionAbsolute { angle } => {
            let a = expr_from_param(angle, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "direction_absolute".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::SeeAngleObject { target } => {
            let t = expr_from_param(target, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "see_angle_object".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![t],
            )));
            Ok(())
        }
        Block::MoveToAngle { angle, distance } => {
            let a = expr_from_param(angle, vars)?;
            let d = expr_from_param(distance, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "move_to_angle".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![a, d],
            )));
            Ok(())
        }
        Block::BrushStamp => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "brush_stamp".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::StartDrawing => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "start_drawing".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::StopDrawing => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "stop_drawing".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::StartFill => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "start_fill".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::StopFill => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "stop_fill".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SetColor { r, g, b } => {
            let red = expr_from_param(r, vars)?;
            let green = expr_from_param(g, vars)?;
            let blue = expr_from_param(b, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "set_color".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![red, green, blue],
            )));
            Ok(())
        }
        Block::SetRandomColor => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "set_random_color".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SetFillColor { color } => {
            let c = expr_from_param(color, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "set_fill_color".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            )));
            Ok(())
        }
        Block::ChangeThickness { amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_thickness".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::SetThickness { value } => {
            let v = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "set_thickness".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![v],
            )));
            Ok(())
        }
        Block::ChangeBrushTransparency { amount } => {
            let a = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "change_brush_transparency".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::SetBrushTranparency { value } => {
            let a = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "set_brush_tranparency".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![a],
            )));
            Ok(())
        }
        Block::BrushEraseAll => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "brush_erase_all".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::TextRead { value } => {
            let v = expr_from_param(value, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_read".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![v],
            )));
            Ok(())
        }
        Block::TextWrite { content } => {
            let c = expr_from_param(content, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_write".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            )));
            Ok(())
        }
        Block::TextAppend { content } => {
            let c = expr_from_param(content, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_append".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            )));
            Ok(())
        }
        Block::TextPrepend { content } => {
            let c = expr_from_param(content, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_prepend".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            )));
            Ok(())
        }
        Block::TextChangeEffect { effect, mode } => {
            let effect_str = text_effect_to_str(*effect);
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_change_effect".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(effect_str.to_string()), Expr::Bool(*mode)],
            )));
            Ok(())
        }
        Block::TextFlush => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_flush".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::TextChangeFont { font } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_change_font".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(font.clone())],
            )));
            Ok(())
        }
        Block::TextChangeFontColor { color } => {
            let c = expr_from_param(color, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_change_font_color".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            )));
            Ok(())
        }
        Block::TextChangeBgColor { color } => {
            let c = expr_from_param(color, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "text_change_bg_color".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            )));
            Ok(())
        }
        Block::SoundSomethingWithBlock { sound_name } => {
            let sn = expr_from_param(sound_name, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_with_block".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![sn],
            )));
            Ok(())
        }
        Block::SoundSomethingSecondWithBlock {
            sound_name,
            seconds,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let sec = expr_from_param(seconds, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_second_with_block".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![sn, sec],
            )));
            Ok(())
        }
        Block::SoundFromTo {
            sound_name,
            start,
            end,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let from = expr_from_param(start, vars)?;
            let to = expr_from_param(end, vars)?;

            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_from_to".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![sn, from, to],
            )));
            Ok(())
        }
        Block::SoundSomethingWaitWithBlock { sound_name } => {
            let sn = expr_from_param(sound_name, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_wait_with_block".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![sn],
            )));
            Ok(())
        }
        Block::SoundSomethingSecondWaitWithBlock {
            sound_name,
            seconds,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let sec = expr_from_param(seconds, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_second_wait_with_block".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![sn, sec],
            )));
            Ok(())
        }
        Block::SoundFromToAndWait {
            sound_name,
            start,
            end,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let from = expr_from_param(start, vars)?;
            let to = expr_from_param(end, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_from_to_and_wait".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![sn, from, to],
            )));
            Ok(())
        }
        Block::SoundVolumeChange { amount } => {
            let am = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_volume_change".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            )));
            Ok(())
        }
        Block::SoundVolumeSet { amount } => {
            let am = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_volume_set".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            )));
            Ok(())
        }
        Block::GetSoundSpeed => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "get_sound_speed".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::SoundSpeedChange { amount } => {
            let am = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_speed_change".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            )));
            Ok(())
        }
        Block::SoundSpeedSet { amount } => {
            let am = expr_from_param(amount, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_speed_set".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            )));
            Ok(())
        }
        Block::SoundSilentAll { target } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "sound_silent_all".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(target.clone())],
            )));
            Ok(())
        }
        Block::PlayBgm { sound_name } => {
            let sound_name = expr_from_param(sound_name, vars)?;
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "play_bgm".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![sound_name],
            )));
            Ok(())
        }
        Block::StopBgm => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "stop_bgm".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::GetSoundVolume => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "get_sound_volume".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::GetSoundDuration { sound_name } => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "get_sound_duration".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(sound_name.clone())],
            )));
            Ok(())
        }
        Block::IsBoostMode => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_boost_mode".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::IsTouchSupported => {
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_touch_supported".to_string(),
                    arity: 0,
                    raw: None,
                },
                Vec::new(),
            )));
            Ok(())
        }
        Block::IsCurrentDeviceType { device_type } => {
            let dt = device_type_to_str(*device_type);
            stmts.push(Stmt::Expr(Expr::Call(
                ir::FuncRef {
                    name: "is_current_device_type".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(dt.to_string())],
            )));
            Ok(())
        }
        Block::CoordinateObject { .. } => Err(SyntaxError(
            "coordinate_object is a value block and cannot be used as a statement".into(),
        )),
        Block::GetDate { .. } => Err(SyntaxError(
            "get_date is a value block and cannot be used as a statement".into(),
        )),
        Block::DistanceSomething { .. } => Err(SyntaxError(
            "distance_something is a value block and cannot be used as a statement".into(),
        )),
        Block::GetUserName => Err(SyntaxError(
            "get_user_name is a value block and cannot be used as a statement".into(),
        )),
        Block::GetNickName => Err(SyntaxError(
            "get_nickname is a value block and cannot be used as a statement".into(),
        )),
        Block::LengthOfString { .. } => Err(SyntaxError(
            "length_of_string is a value block and cannot be used as a statement".into(),
        )),
        Block::ReverseOfString { .. } => Err(SyntaxError(
            "reverse_of_string is a value block and cannot be used as a statement".into(),
        )),
        Block::CombineSomething { .. } => Err(SyntaxError(
            "combine_something is a value block and cannot be used as a statement".into(),
        )),
        Block::CharAt { .. } => Err(SyntaxError(
            "char_at is a value block and cannot be used as a statement".into(),
        )),
        Block::Substring { .. } => Err(SyntaxError(
            "substring is a value block and cannot be used as a statement".into(),
        )),
        Block::ReplaceString { .. } => Err(SyntaxError(
            "replace_string is a value block and cannot be used as a statement".into(),
        )),
        Block::CountMatchString { .. } => Err(SyntaxError(
            "count_match_string is a value block and cannot be used as a statement".into(),
        )),
        Block::IndexOfString { .. } => Err(SyntaxError(
            "index_of_string is a value block and cannot be used as a statement".into(),
        )),
        Block::ChangeStringCase { .. } => Err(SyntaxError(
            "change_string_case is a value block and cannot be used as a statement".into(),
        )),
        Block::GetBlockCount { .. } => Err(SyntaxError(
            "get_block_count is a value block and cannot be used as a statement".into(),
        )),
        Block::ChangeRgbToHex { .. } => Err(SyntaxError(
            "change_rgb_to_hex is a value block and cannot be used as a statement".into(),
        )),
        Block::ChangeHexToRgb { .. } => Err(SyntaxError(
            "change_hex_to_rgb is a value block and cannot be used as a statement".into(),
        )),
        Block::GetBooleanValue { .. } => Err(SyntaxError(
            "get_boolean_value is a value block and cannot be used as a statement".into(),
        )),
        Block::GetTableCount { .. } => Err(SyntaxError(
            "get_table_count is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromTable { .. } => Err(SyntaxError(
            "get_value_from_table is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromLastRow { .. } => Err(SyntaxError(
            "get_value_from_last_row is a value block and cannot be used as a statement".into(),
        )),
        Block::CalcValuesFromTable { .. } => Err(SyntaxError(
            "calc_values_from_table is a value block and cannot be used as a statement".into(),
        )),
        Block::GetCoefficient { .. } => Err(SyntaxError(
            "get_coefficient is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueFromCell { .. } => Err(SyntaxError(
            "get_value_from_cell is a value block and cannot be used as a statement".into(),
        )),
        Block::GetValueVLookup { .. } => Err(SyntaxError(
            "get_value_v_lookup is a value block and cannot be used as a statement".into(),
        )),
    }
}

fn math_op_to_name(op: &MathOperation) -> &'static str {
    match op {
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
    }
}

/// `ParamBlock` -> IR `Expr`.
fn expr_from_param(p: &ParamBlock, _vars: &VarMap) -> Result<Expr> {
    match p {
        ParamBlock::Null => Ok(Expr::Int(0)),
        ParamBlock::Number(n) => Ok(Expr::Float(*n)),
        ParamBlock::Text(s) => {
            // Entry `text` 釉붾줉???レ옄泥섎읆 蹂댁씠硫??뺤닔濡?蹂??(?곗닠/鍮꾧탳 而⑦뀓?ㅽ듃).
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

/// `Block` -> IR `Expr` (媛믪쑝濡??곗씠??釉붾줉).
#[allow(unreachable_patterns)]
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
        Block::Angle(n) => Ok(Expr::Float(*n)),
        Block::Color(s) => Ok(Expr::Str(s.clone())),
        Block::GetVar { variable } => Ok(Expr::Var(variable.clone())),
        Block::GetFuncVariable { variable } => Ok(Expr::Var(variable.clone())),
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
                    raw: None,
                },
                args,
            ))
        }
        Block::CoordinateMouse { axis } => Ok(Expr::Call(
            ir::FuncRef {
                name: "coordinate_mouse".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(mouse_axis_to_str(*axis).to_string())],
        )),
        Block::StringIncludes { haystack, needle } => {
            let h = expr_from_param(haystack, vars)?;
            let n = expr_from_param(needle, vars)?;
            Ok(Expr::Call(
                crate::ir::FuncRef {
                    name: "string_contains".to_string(),
                    arity: 2,
                    raw: None,
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
                    raw: None,
                },
                ir_args,
            ))
        }
        Block::CalcRand { min, max } => {
            let m = expr_from_param(min, vars)?;
            let mx = expr_from_param(max, vars)?;
            Ok(Expr::Call(
                crate::ir::FuncRef {
                    name: "calc_rand".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![m, mx],
            ))
        }
        Block::GetProjectTimerValue {} => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_project_timer_value".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::GetCanvasInputValue {} => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_canvas_input_value".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::CalcOperation { op, expr } => {
            let fn_name = math_op_to_name(op);
            let e = expr_from_param(expr, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: fn_name.to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![e],
            ))
        }
        Block::SetVar { .. }
        | Block::ChangeVar { .. }
        | Block::ShowVar { .. }
        | Block::HideVar { .. }
        | Block::CreateClone { .. }
        | Block::MoveDirection { .. }
        | Block::HideList { .. }
        | Block::ShowList { .. }
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
        | Block::AskAndWait { .. }
        | Block::AddValueToList { .. }
        | Block::RemoveValueFromList { .. }
        | Block::InsertValueToList { .. }
        | Block::ChangeValueListIndex { .. }
        | Block::RestartProject
        | Block::DeleteClone
        | Block::RemoveAllClones
        | Block::ChooseProjectTimerAction { .. }
        | Block::SetVisibleProjectTimer { .. }
        | Block::SetVisibleAnswer { .. }
        | Block::Show {}
        | Block::Hide {}
        | Block::Dialog { .. }
        | Block::DialogTime { .. }
        | Block::ChangeToSomeShape { .. }
        | Block::ChangeToNextShape {}
        | Block::RemoveDialog {}
        | Block::AddEffectAmount { .. }
        | Block::ChangeEffectAmount { .. }
        | Block::EraseAllEffects {}
        | Block::ChangeScaleSize { .. }
        | Block::SetScaleSize { .. }
        | Block::ResetScaleSize {}
        | Block::FlipX {}
        | Block::FlipY {}
        | Block::ChangeObjectIndex { .. }
        | Block::StretchScaleSize { .. }
        | Block::TextChangeEffect { .. }
        | Block::BounceWall
        | Block::MoveX { .. }
        | Block::MoveY { .. }
        | Block::RotateRelative { .. }
        | Block::DirectionRelative { .. }
        | Block::MoveXyTime { .. }
        | Block::LocateX { .. }
        | Block::LocateY { .. }
        | Block::LocateXY { .. }
        | Block::LocateXyTime { .. }
        | Block::LocateObjectTime { .. }
        | Block::Locate { .. }
        | Block::RotateByTime { .. }
        | Block::DirectionRelativeDuration { .. }
        | Block::RotateAbsolute { .. }
        | Block::DirectionAbsolute { .. }
        | Block::SeeAngleObject { .. }
        | Block::MoveToAngle { .. }
        | Block::BrushStamp
        | Block::StartDrawing
        | Block::StopDrawing
        | Block::StartFill
        | Block::StopFill
        | Block::SetColor { .. }
        | Block::SetRandomColor
        | Block::SetFillColor { .. }
        | Block::ChangeThickness { .. }
        | Block::SetThickness { .. }
        | Block::ChangeBrushTransparency { .. }
        | Block::SetBrushTranparency { .. }
        | Block::BrushEraseAll
        | Block::TextWrite { .. }
        | Block::TextAppend { .. }
        | Block::TextPrepend { .. }
        | Block::Return { .. } => Err(UnmappedBlock(format!(
            "block used as expr: {}",
            b.type_id()
        ))),
        Block::QuotientAndMod { a, b, mode } => {
            let av = expr_from_param(a, vars)?;
            let bv = expr_from_param(b, vars)?;
            let mode_str = match mode {
                QamMethod::Quotient => "quotient",
                QamMethod::Mod => "modulo",
            };
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "quotient_and_mod".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![av, bv, Expr::Str(mode_str.to_string())],
            ))
        }
        Block::ListValueAt { index, list } => {
            let index = expr_from_param(index, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "value_of_index_from_list".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![index, Expr::Var(list.clone())],
            ))
        }
        Block::LengthOfList { list } => Ok(Expr::Call(
            ir::FuncRef {
                name: "length_of_list".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Var(list.clone())],
        )),
        Block::IsIncludedInList { list, value } => {
            let value = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "is_included_in_list".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Var(list.clone()), value],
            ))
        }
        // ── 데이터분석 (테이블) stmt 전용 — expr 자리 거부 (expr_from_block) ──
        Block::AppendRowToTable { .. } => Err(SyntaxError(
            "append_row_to_table is a statement block and cannot be used as a value".into(),
        )),
        Block::InsertRowToTable { .. } => Err(SyntaxError(
            "insert_row_to_table is a statement block and cannot be used as a value".into(),
        )),
        Block::DeleteRowFromTable { .. } => Err(SyntaxError(
            "delete_row_from_table is a statement block and cannot be used as a value".into(),
        )),
        Block::SetValueFromTable { .. } => Err(SyntaxError(
            "set_value_from_table is a statement block and cannot be used as a value".into(),
        )),
        Block::SaveCurrentTable { .. } => Err(SyntaxError(
            "save_current_table is a statement block and cannot be used as a value".into(),
        )),
        Block::OpenTable { .. } => Err(SyntaxError(
            "open_table is a statement block and cannot be used as a value".into(),
        )),
        Block::OpenTableWait { .. } => Err(SyntaxError(
            "open_table_wait is a statement block and cannot be used as a value".into(),
        )),
        Block::OpenTableChart { .. } => Err(SyntaxError(
            "open_table_chart is a statement block and cannot be used as a value".into(),
        )),
        Block::CloseTableChart => Err(SyntaxError(
            "close_table_chart is a statement block and cannot be used as a value".into(),
        )),
        Block::SetValueFromCell { .. } => Err(SyntaxError(
            "set_value_from_cell is a statement block and cannot be used as a value".into(),
        )),
        Block::GetTableCount { table, dimension } => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_table_count".to_string(),
                arity: 2,
                raw: None,
            },
            vec![Expr::Str(table.clone()), Expr::Str(row_col_to_str(*dimension).to_string())],
        )),
        Block::GetValueFromTable { table, row, field } => {
            let row = expr_from_param(row, vars)?;
            let field = expr_from_param(field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_from_table".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), row, field],
            ))
        }
        Block::GetValueFromLastRow { table, field } => {
            let field = expr_from_param(field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_from_last_row".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), field],
            ))
        }
        Block::CalcValuesFromTable { table, field, method } => {
            let field = expr_from_param(field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "calc_values_from_table".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![
                    Expr::Str(table.clone()),
                    field,
                    Expr::Str(calc_method_to_str(*method).to_string()),
                ],
            ))
        }
        Block::GetCoefficient { table, field1, field2 } => {
            let field1 = expr_from_param(field1, vars)?;
            let field2 = expr_from_param(field2, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_coefficient".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), field1, field2],
            ))
        }
        Block::GetValueFromCell { table, cell } => {
            let cell = expr_from_param(cell, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_from_cell".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), cell],
            ))
        }
        Block::GetValueVLookup { table, field, value, return_field } => {
            let field = expr_from_param(field, vars)?;
            let value = expr_from_param(value, vars)?;
            let return_field = expr_from_param(return_field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_v_lookup".to_string(),
                    arity: 4,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), field, value, return_field],
            ))
        }
        // ?? ?곗씠?곕텇??(?뚯씠釉? ??媛??щ’ ?먮━ ??
        Block::GetTableCount { table, dimension } => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_table_count".to_string(),
                arity: 2,
                raw: None,
            },
            vec![Expr::Str(table.clone()), Expr::Str(row_col_to_str(*dimension).to_string())],
        )),
        Block::GetValueFromTable { table, row, field } => {
            let row = expr_from_param(row, vars)?;
            let field = expr_from_param(field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_from_table".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), row, field],
            ))
        }
        Block::GetValueFromLastRow { table, field } => {
            let field = expr_from_param(field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_from_last_row".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), field],
            ))
        }
        Block::CalcValuesFromTable { table, field, method } => {
            let field = expr_from_param(field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "calc_values_from_table".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![
                    Expr::Str(table.clone()),
                    field,
                    Expr::Str(calc_method_to_str(*method).to_string()),
                ],
            ))
        }
        Block::GetCoefficient { table, field1, field2 } => {
            let field1 = expr_from_param(field1, vars)?;
            let field2 = expr_from_param(field2, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_coefficient".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), field1, field2],
            ))
        }
        Block::GetValueFromCell { table, cell } => {
            let cell = expr_from_param(cell, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_from_cell".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), cell],
            ))
        }
        Block::GetValueVLookup { table, field, value, return_field } => {
            let field = expr_from_param(field, vars)?;
            let value = expr_from_param(value, vars)?;
            let return_field = expr_from_param(return_field, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_value_v_lookup".to_string(),
                    arity: 4,
                    raw: None,
                },
                vec![Expr::Str(table.clone()), field, value, return_field],
            ))
        }
        Block::Raw { type_id, raw } => Ok(Expr::Call(
            ir::FuncRef {
                name: type_id.clone(),
                arity: hw_raw_arg_count(raw),
                raw: Some(raw.clone()),
            },
            hw_raw_args(raw, vars),
        )),
        Block::IsPressSomeKey { key } => Ok(Expr::Call(
            ir::FuncRef {
                name: "is_press_some_key".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(key.clone())],
        )),
        Block::ReachSomeThing { target } => Ok(Expr::Call(
            ir::FuncRef {
                name: "reach_something".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(target.clone())],
        )),
        Block::IsClicked => Ok(Expr::Call(
            ir::FuncRef {
                name: "is_clicked".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::IsObjectClicked => Ok(Expr::Call(
            ir::FuncRef {
                name: "is_object_clicked".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::IsType { value, type_name } => {
            let value = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "is_type".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![
                    value,
                    Expr::Str(
                        match type_name {
                            crate::block::EntryType::Number => "number",
                            crate::block::EntryType::En => "en",
                            crate::block::EntryType::Ko => "ko",
                        }
                        .to_string(),
                    ),
                ],
            ))
        }
        Block::TextRead { value } => {
            let v = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "text_read".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![v],
            ))
        }
        Block::TextFlush => Ok(Expr::Call(
            ir::FuncRef {
                name: "text_flush".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::TextChangeFont { font } => Ok(Expr::Call(
            ir::FuncRef {
                name: "text_change_font".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(font.clone())],
        )),
        Block::TextChangeFontColor { color } => {
            let c = expr_from_param(color, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "text_change_font_color".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            ))
        }
        Block::TextChangeBgColor { color } => {
            let c = expr_from_param(color, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "text_change_bg_color".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![c],
            ))
        }
        Block::SoundSomethingWithBlock { sound_name } => {
            let sn = expr_from_param(sound_name, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_with_block".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![sn],
            ))
        }
        Block::SoundSomethingSecondWithBlock {
            sound_name,
            seconds,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let sec = expr_from_param(seconds, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_second_with_block".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![sn, sec],
            ))
        }
        Block::SoundFromTo {
            sound_name,
            start,
            end,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let from = expr_from_param(start, vars)?;
            let to = expr_from_param(end, vars)?;

            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_from_to".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![sn, from, to],
            ))
        }
        Block::SoundSomethingWaitWithBlock { sound_name } => {
            let sn = expr_from_param(sound_name, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_wait_with_block".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![sn],
            ))
        }
        Block::SoundSomethingSecondWaitWithBlock {
            sound_name,
            seconds,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let sec = expr_from_param(seconds, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_something_second_wait_with_block".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![sn, sec],
            ))
        }
        Block::SoundFromToAndWait {
            sound_name,
            start,
            end,
        } => {
            let sn = expr_from_param(sound_name, vars)?;
            let start = expr_from_param(start, vars)?;
            let end = expr_from_param(end, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_from_to_and_wait".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![sn, start, end],
            ))
        }
        Block::SoundVolumeChange { amount } => {
            let am = expr_from_param(amount, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_volume_change".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            ))
        }
        Block::SoundVolumeSet { amount } => {
            let am = expr_from_param(amount, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_volume_set".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            ))
        }
        Block::GetSoundSpeed => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_sound_speed".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::SoundSpeedChange { amount } => {
            let am = expr_from_param(amount, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_speed_change".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            ))
        }
        Block::SoundSpeedSet { amount } => {
            let am = expr_from_param(amount, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "sound_speed_set".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![am],
            ))
        }
        Block::SoundSilentAll { target } => Ok(Expr::Call(
            ir::FuncRef {
                name: "sound_silent_all".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(target.clone())],
        )),
        Block::PlayBgm { sound_name } => {
            let sound_name = expr_from_param(sound_name, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "play_bgm".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![sound_name],
            ))
        }
        Block::StopBgm => Ok(Expr::Call(
            ir::FuncRef {
                name: "stop_bgm".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::GetSoundVolume => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_sound_volume".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::GetSoundDuration { sound_name } => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_sound_duration".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(sound_name.clone())],
        )),
        Block::IsBoostMode => Ok(Expr::Call(
            ir::FuncRef {
                name: "is_boost_mode".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::IsTouchSupported => Ok(Expr::Call(
            ir::FuncRef {
                name: "is_touch_supported".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::IsCurrentDeviceType { device_type } => {
            let dt = device_type_to_str(*device_type);
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "is_current_device_type".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![Expr::Str(dt.to_string())],
            ))
        }
        Block::CoordinateObject { target, coordinate } => Ok(Expr::Call(
            ir::FuncRef {
                name: "coordinate_object".to_string(),
                arity: 2,
                raw: None,
            },
            vec![
                Expr::Str(target.clone()),
                Expr::Str(object_coord_to_str(*coordinate).to_string()),
            ],
        )),
        Block::GetDate { kind } => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_date".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(date_kind_to_str(*kind).to_string())],
        )),
        Block::DistanceSomething { target } => Ok(Expr::Call(
            ir::FuncRef {
                name: "distance_something".to_string(),
                arity: 1,
                raw: None,
            },
            vec![Expr::Str(target.clone())],
        )),
        Block::GetUserName => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_user_name".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::GetNickName => Ok(Expr::Call(
            ir::FuncRef {
                name: "get_nickname".to_string(),
                arity: 0,
                raw: None,
            },
            Vec::new(),
        )),
        Block::LengthOfString { value } => {
            let v = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "length_of_string".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![v],
            ))
        }
        Block::ReverseOfString { value } => {
            let v = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "reverse_of_string".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![v],
            ))
        }
        Block::CombineSomething { a, b } => {
            let va = expr_from_param(a, vars)?;
            let vb = expr_from_param(b, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "combine_something".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![va, vb],
            ))
        }
        Block::CharAt { string, index } => {
            let s = expr_from_param(string, vars)?;
            let i = expr_from_param(index, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "char_at".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![s, i],
            ))
        }
        Block::Substring { string, start, end } => {
            let s = expr_from_param(string, vars)?;
            let st = expr_from_param(start, vars)?;
            let e = expr_from_param(end, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "substring".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![s, st, e],
            ))
        }
        Block::ReplaceString { target, old, new } => {
            let tg = expr_from_param(target, vars)?;
            let ow = expr_from_param(old, vars)?;
            let nw = expr_from_param(new, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "replace_string".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![tg, ow, nw],
            ))
        }
        Block::CountMatchString { target, pattern } => {
            let ss = expr_from_param(target, vars)?;
            let sp = expr_from_param(pattern, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "count_match_string".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![ss, sp],
            ))
        }
        Block::IndexOfString { target, pattern } => {
            let tg = expr_from_param(target, vars)?;
            let pn = expr_from_param(pattern, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "index_of_string".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![tg, pn],
            ))
        }
        Block::ChangeStringCase { target, case } => Ok(Expr::Call(
            ir::FuncRef {
                name: "change_string_case".to_string(),
                arity: 2,
                raw: None,
            },
            vec![
                expr_from_param(target, vars)?,
                Expr::Str(change_string_case_to_str(*case).to_string()),
            ],
        )),
        Block::GetBlockCount { target } => {
            let t = expr_from_param(target, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_block_count".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![t],
            ))
        }
        Block::ChangeRgbToHex { r, g, b } => {
            let vr = expr_from_param(r, vars)?;
            let vg = expr_from_param(g, vars)?;
            let vb = expr_from_param(b, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "change_rgb_to_hex".to_string(),
                    arity: 3,
                    raw: None,
                },
                vec![vr, vg, vb],
            ))
        }
        Block::ChangeHexToRgb { hex, channel } => {
            let h = expr_from_param(hex, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "change_hex_to_rgb".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![h, Expr::Str(rgb_channel_to_str(*channel).to_string())],
            ))
        }
        Block::GetBooleanValue { value } => {
            let v = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "get_boolean_value".to_string(),
                    arity: 1,
                    raw: None,
                },
                vec![v],
            ))
        }
        Block::SetFuncVariable { variable, value } => {
            // stmt ?먮━. expr 而⑦뀓?ㅽ듃濡????쇱? 嫄곗쓽 ?놁?留??덉쟾?섍쾶 ?⑦빆 emit.
            let v = expr_from_param(value, vars)?;
            Ok(Expr::Call(
                ir::FuncRef {
                    name: "set_func_variable".to_string(),
                    arity: 2,
                    raw: None,
                },
                vec![Expr::Var(variable.clone()), v],
            ))
        }
    }
}

/// Block::Raw ??raw.params ?먯꽌 Rust ?몄옄 ?쒗쁽??best-effort 濡?戮묐뒗??
/// (?뺥솗???ш뎄?깆? @hwraw 二쇱꽍???대떦?섎?濡? ?ш린???쎄린 醫뗭? ?섏??쇰줈留?)
fn hw_raw_arg_count(raw: &Value) -> usize {
    raw.get("params")
        .and_then(|p| p.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

/// Block::Raw ??raw.params 瑜?IR Expr ?몄옄濡?蹂??(?ㅽ뙣 ?붿냼??嫄대꼫?).
fn hw_raw_args(raw: &Value, vars: &VarMap) -> Vec<Expr> {
    let mut out = Vec::new();
    if let Some(Value::Array(params)) = raw.get("params") {
        for p in params {
            if let Ok(pb) = value_to_param(p, vars)
                && let Ok(e) = expr_from_param(&pb, vars)
            {
                out.push(e);
            }
        }
    }
    out
}

/// Entry ?꾨줈?앺듃 `script` (JSON 臾몄옄?? -> IR `Program`. 蹂???놁쓬.
pub fn program_from_script_string(s: &str) -> Result<crate::ir::Program> {
    program_from_script_string_with_vars(s, &VarMap::new())
}

/// Entry ?꾨줈?앺듃 `script` (JSON 臾몄옄?? -> IR `Program`. 蹂??留??꾨떖.
pub fn program_from_script_string_with_vars(s: &str, vars: &VarMap) -> Result<crate::ir::Program> {
    let v: Value = serde_json::from_str(s).map_err(|e| crate::Error::Parse(e.to_string()))?;
    program_from_script_value_with_vars(&v, vars)
}

/// Entry ?ㅻ툕?앺듃 script瑜?蹂듭썝?섎ŉ ?꾩옱 ?ㅻ툕?앺듃???먯궛 ID瑜??대쫫?쇰줈 諛붽씔??
pub fn program_from_script_string_with_vars_and_assets(
    s: &str,
    vars: &VarMap,
    assets: &crate::AssetMap,
    object_name: &str,
) -> Result<crate::ir::Program> {
    let program = program_from_script_string_with_vars(s, vars)?;
    Ok(resolve_asset_ids(program, assets, object_name))
}

/// Entry ?ㅻ툕?앺듃 `script` (`Value::String` ?덉쓽 JSON) -> IR `Program`.
pub fn program_from_script_value(v: &Value) -> Result<crate::ir::Program> {
    program_from_script_value_with_vars(v, &VarMap::new())
}

/// Entry ?ㅻ툕?앺듃 `script` (`Value::String` ?덉쓽 JSON) -> IR `Program`. 蹂??留??꾨떖.
pub fn program_from_script_value_with_vars(v: &Value, vars: &VarMap) -> Result<crate::ir::Program> {
    let stmts = from_script(v, vars)?;
    Ok(crate::ir::Program { stmts })
}

/// Entry ?ㅻ툕?앺듃 script Value瑜?蹂듭썝?섎ŉ ?꾩옱 ?ㅻ툕?앺듃???먯궛 ID瑜??대쫫?쇰줈 諛붽씔??
pub fn program_from_script_value_with_vars_and_assets(
    v: &Value,
    vars: &VarMap,
    assets: &crate::AssetMap,
    object_name: &str,
) -> Result<crate::ir::Program> {
    let program = program_from_script_value_with_vars(v, vars)?;
    Ok(resolve_asset_ids(program, assets, object_name))
}

/// ?먯궛 ID瑜?DSL?먯꽌 ?ъ슜???먯궛 ?대쫫?쇰줈 蹂듭썝?쒕떎.
fn resolve_asset_ids(
    mut program: crate::ir::Program,
    assets: &crate::AssetMap,
    object_name: &str,
) -> crate::ir::Program {
    fn resolve_expr(expr: &mut Expr, assets: &crate::AssetMap, object_name: &str) {
        if let Expr::Call(fref, args) = expr {
            if fref.name == "get_sound_duration"
                && let Some(Expr::Str(id)) = args.first_mut()
                && let Some(name) = assets.sound_name_by_id(object_name, id)
            {
                *id = name.to_string();
            }
            // ?ㅻ툕?앺듃 dropdown ?щ’ ??IR args ??target ?꾩튂 id ??name.
            // (forward ??EntryJS params idx ? ?ㅻ쫫 ??IR ? ?쇰꺼 ?щ’ ?쒖쇅?섍퀬 value 留?args ???대뒗??)
            let object_target_idx: Option<usize> = match fref.name.as_str() {
                "create_clone" | "see_angle_object" | "locate" | "distance_something"
                | "reach_something" => Some(0),
                "locate_object_time" | "coordinate_object" => Some(1),
                _ => None,
            };
            if let Some(idx) = object_target_idx
                && let Some(arg) = args.get_mut(idx)
                && let Expr::Str(id) = arg
                && let Some(name) = assets.object_name_by_id(id)
            {
                *id = name.to_string();
            }
            for arg in args {
                resolve_expr(arg, assets, object_name);
            }
        }
    }
    fn resolve_stmts(stmts: &mut [Stmt], assets: &crate::AssetMap, object_name: &str) {
        for stmt in stmts {
            match stmt {
                Stmt::SetVar(_, expr) | Stmt::VarDecl(_, expr, _, _) => {
                    resolve_expr(expr, assets, object_name);
                }
                Stmt::Expr(Expr::Call(fref, args)) if fref.name == "change_to_some_shape" => {
                    if let Some(Expr::Str(id)) = args.first_mut()
                        && let Some(name) = assets.picture_name_by_id(object_name, id)
                    {
                        *id = name.to_string();
                    }
                }
                Stmt::Expr(Expr::Call(fref, args))
                    if matches!(
                        fref.name.as_str(),
                        "sound_something_with_block"
                            | "sound_something_second_with_block"
                            | "sound_from_to"
                            | "sound_something_wait_with_block"
                            | "sound_something_second_wait_with_block"
                            | "sound_from_to_and_wait"
                            | "play_bgm"
                            | "get_sound_duration"
                    ) =>
                {
                    if let Some(Expr::Str(id)) = args.first_mut()
                        && let Some(name) = assets.sound_name_by_id(object_name, id)
                    {
                        *id = name.to_string();
                    }
                }
                // ?ㅻ툕?앺듃 dropdown ?щ’ ??EntryJS 媛 emit ??id 瑜?DSL ??name ?쇰줈 蹂듭썝.
                Stmt::Expr(Expr::Call(fref, args))
                    if matches!(
                        fref.name.as_str(),
                        "create_clone" | "see_angle_object" | "locate" | "reach_something"
                    ) =>
                {
                    if let Some(Expr::Str(id)) = args.first_mut()
                        && let Some(name) = assets.object_name_by_id(id)
                    {
                        *id = name.to_string();
                    }
                }
                Stmt::Expr(Expr::Call(fref, args))
                    if matches!(
                        fref.name.as_str(),
                        "locate_object_time" | "coordinate_object" | "distance_something"
                    ) =>
                {
                    if let Some(arg) = args.get_mut(1)
                        && let Expr::Str(id) = arg
                        && let Some(name) = assets.object_name_by_id(id)
                    {
                        *id = name.to_string();
                    }
                }
                Stmt::FuncDef { body, .. } => resolve_stmts(body, assets, object_name),
                Stmt::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    resolve_stmts(then_body, assets, object_name);
                    resolve_stmts(else_body, assets, object_name);
                }
                Stmt::While { body, .. } | Stmt::Repeat { body, .. } | Stmt::For { body, .. } => {
                    resolve_stmts(body, assets, object_name);
                }
                _ => {}
            }
        }
    }
    resolve_stmts(&mut program.stmts, assets, object_name);
    program
}

/// scripts Value (`[[block, ...], ...]` ?뺥깭) 瑜??쒗쉶?섎ŉ 留ㅽ븨 ???섎뒗 釉붾줉 ??낆쓣 吏묎퀎.
/// 鍮꾪뙆愿댁쟻 ??IR 蹂???놁씠 吏곸젒 walk. ?ш?濡?`statements` ?덉쓽 釉붾줉???먯깋.
///
/// `block_from_value` 媛 single source of truth ???붿씠?몃━?ㅽ듃 ?좎? 遺덊븘??
/// ??釉붾줉 異붽? ??`block_from_value` ??留ㅽ븨留?異붽??섎㈃ ?먮룞 諛섏쁺.
///
/// ## 諛섑솚
/// `(type_name, count)` 紐⑸줉. count ?대┝李⑥닚 ???대쫫 ?ㅻ쫫李⑥닚 ?뺣젹.
///
/// ## ?ъ슜
/// extract ???ㅻ툕?앺듃蹂?raw ?대갚 ?몄뿉, ?꾩껜 ?꾨줈?앺듃??誘몃ℓ??釉붾줉???붿빟 異쒕젰????
pub fn collect_unmapped_blocks(scripts: &Value, vars: &VarMap) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, usize> = HashMap::new();
    walk_blocks(scripts, &mut |block: &Value| {
        if let Some(t) = block.get("type").and_then(|x| x.as_str()) {
            // ?섎뱶?⑥뼱 釉붾윮? ?뚯뒪留??몃뜳?ㅻ줈 ?몄떇?섎?濡?誘몃ℓ??吏묎퀎?먯꽌 ?쒖쇅.
            if !crate::block::registry::is_hw_block(t) && block_from_value(block, vars).is_err() {
                *counts.entry(t.to_string()).or_insert(0) += 1;
            }
        }
    });
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// scripts ?몃━瑜??ш? walk. 媛?block 留덈떎 `f` ?몄텧.
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
