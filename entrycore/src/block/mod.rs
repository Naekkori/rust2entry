//! Entry 블록 표현.
//!
//! IR(`crate::ir`)이 Rust 의미 보존, Block은 Entry 직렬화 친화.
//! 각 variant는 Entry 컴파일러가 인식하는 슬롯 구조를 가짐.

pub mod category;
pub mod registry;
use crate::Error::{SyntaxError, UnmappedBlock};
use crate::ir::{BinOp, Expr, Stmt, UnaryOp};
use crate::{Result, VarKind};
pub use registry::{BlockRegistry, HwDevice, HwSourcemap, SchemaDump, SchemaReport, Violation};

pub use category::Category;

use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub enum QamMethod {
    Quotient,
    Mod,
}
#[derive(Debug, Clone)]
pub enum Dimension {
    Width,
    Height,
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

// 효과 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Color,
    Brightness,
    Ghost,
    //엔트리에서 선언되어있지만 안쓰는것.
    Fisheye,
    Whirl,
    Pixelate,
    Mosaic,
    Negative,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEffect {
    Strike,
    UnderLine,
    FontItalic,
    FontBlold,
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
    ListValueAt {
        index: ParamBlock,
        list: String,
    },
    AddValueToList {
        value: ParamBlock,
        list: String,
    },
    RemoveValueFromList {
        index: ParamBlock,
        list: String,
    },
    InsertValueToList {
        value: ParamBlock,
        index: ParamBlock,
        list: String,
    },
    ChangeValueListIndex {
        index: ParamBlock,
        value: ParamBlock,
        list: String,
    },
    LengthOfList {
        list: String,
    },
    ShowList {
        list: String,
    },
    HideList {
        list: String,
    },
    IsIncludedInList {
        list: String,
        value: ParamBlock,
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
    RestartProject,
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
    DeleteClone,
    RemoveAllClones,
    // --- 판단 ---
    IsPressSomeKey {
        key: String,
    },
    ReachSomeThing {
        target: String,
    },
    IsClicked,
    IsObjectClicked,
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
    Dialog {
        mode: DialogMode,
        content: ParamBlock,
    },
    DialogTime {
        mode: DialogMode,
        content: ParamBlock,
        time: ParamBlock,
    },
    ChangeToSomeShape {
        picture: String,
    },
    ChangeToNextShape {},
    RemoveDialog {},
    AddEffectAmount {
        effect: EffectType,
        amount: ParamBlock,
    },
    ChangeEffectAmount {
        effect: EffectType,
        amount: ParamBlock,
    },
    EraseAllEffects {},
    ChangeScaleSize {
        amount: ParamBlock,
    },
    SetScaleSize {
        amount: ParamBlock,
    },
    ResetScaleSize {},
    FlipX {}, //상하로 뒤집힘
    FlipY {}, //좌우로 뒤집힘
    ChangeObjectIndex {
        direction: String,
    },
    StretchScaleSize {
        dim: Dimension,
        value: ParamBlock,
    },
    CreateClone {
        target: String,
    },
    // ---   붓   ---
    BrushStamp,
    StartDrawing,
    StopDrawing,
    StartFill,
    StopFill,
    SetColor {
        r: ParamBlock,
        g: ParamBlock,
        b: ParamBlock,
    },
    SetRandomColor,
    SetFillColor {
        color: ParamBlock,
    },
    ChangeThickness {
        amount: ParamBlock,
    },
    SetThickness {
        value: ParamBlock,
    },
    ChangeBrushTransparency {
        amount: ParamBlock,
    },
    SetBrushTranparency {
        value: ParamBlock,
    },
    BrushEraseAll,
    // --- 글상자 ---
    TextRead {
        value: ParamBlock,
    },
    TextWrite {
        content: ParamBlock,
    },
    TextAppend {
        content: ParamBlock,
    },
    TextPrepend {
        content: ParamBlock,
    },
    TextChangeEffect {
        effect: TextEffect,
        mode: bool,
    },
    TextFlush,
    TextChangeFont {
        font: String,
    },
    TextChangeFontColor {
        color: ParamBlock,
    },
    TextChangeBgColor {
        color: ParamBlock,
    },
    // --- 움직임 ---
    MoveDirection {
        direction: String,
        amount: ParamBlock,
    },
    BounceWall,
    MoveX {
        amount: ParamBlock,
    },
    MoveY {
        amount: ParamBlock,
    },
    RotateRelative {
        angle: ParamBlock,
    },
    DirectionRelative {
        angle: ParamBlock,
    },
    MoveXyTime {
        duration: ParamBlock,
        dx: ParamBlock,
        dy: ParamBlock,
    },
    LocateX {
        x: ParamBlock,
    },
    LocateY {
        y: ParamBlock,
    },
    LocateXY {
        x: ParamBlock,
        y: ParamBlock,
    },
    LocateXyTime {
        duration: ParamBlock,
        x: ParamBlock,
        y: ParamBlock,
    },
    Locate {
        target: ParamBlock,
    },
    LocateObjectTime {
        duration: ParamBlock,
        target: ParamBlock,
    },
    RotateByTime {
        duration: ParamBlock,
        angle: ParamBlock,
    },
    DirectionRelativeDuration {
        duration: ParamBlock,
        amount: ParamBlock,
    },
    RotateAbsolute {
        angle: ParamBlock,
    },
    DirectionAbsolute {
        angle: ParamBlock,
    },
    SeeAngleObject {
        target: ParamBlock,
    },
    MoveToAngle {
        angle: ParamBlock,
        distance: ParamBlock,
    },
    /// --- 소리 ---
    SoundSomethingWithBlock {
        sound_name: ParamBlock,
    },
    SoundSomethingSecondWithBlock {
        sound_name: ParamBlock,
        seconds: ParamBlock,
    },
    SoundFromTo {
        sound_name: ParamBlock,
        start: ParamBlock,
        end: ParamBlock,
    },
    SoundSomethingWaitWithBlock {
        sound_name: ParamBlock,
    },
    SoundSomethingSecondWaitWithBlock {
        sound_name: ParamBlock,
        seconds: ParamBlock,
    },
    SoundFromToAndWait {
        sound_name: ParamBlock,
        start: ParamBlock,
        end: ParamBlock,
    },
    SoundVolumeChange {
        amount: ParamBlock,
    },
    SoundVolumeSet {
        amount: ParamBlock,
    },
    SoundSpeedChange {
        amount: ParamBlock,
    },
    SoundSpeedSet {
        amount: ParamBlock,
    },
    GetSoundSpeed,
    SoundSilentAll {
        target: String,
    },
    PlayBgm {
        sound_name: ParamBlock,
    },
    StopBgm,
    GetSoundVolume,
    GetSoundDuration {
        sound_name: String,
    },
    /// 하드웨어 블럭 (소스맵 기반 동적 블럭). `raw` 는 원본 .ent 블럭 JSON
    /// (`{type, params, statements}`) 을 그대로 보존해 손실 없는 왕복을 보장한다.
    /// type_id 는 하드웨어 블럭 type 문자열 (예: `pyocoding_serial_set`).
    Raw {
        type_id: String,
        raw: Value,
    },
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
    pub fn type_id(&self) -> &str {
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
            Block::RestartProject => "restart_project",
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
            Block::DialogTime { .. } => "dialog_time",
            Block::ChangeToSomeShape { .. } => "change_to_some_shape",
            Block::ChangeToNextShape {} => "change_to_next_shape",
            Block::RemoveDialog {} => "remove_dialog",
            Block::AddEffectAmount { .. } => "add_effect_amount",
            Block::ChangeEffectAmount { .. } => "change_effect_amount",
            Block::EraseAllEffects {} => "erase_all_effects",
            Block::ChangeScaleSize { .. } => "change_scale_size",
            Block::SetScaleSize { .. } => "set_scale_size",
            Block::ResetScaleSize {} => "reset_scale_size",
            Block::FlipX {} => "flip_x",
            Block::FlipY {} => "flip_y",
            Block::ChangeObjectIndex { .. } => "change_object_index",
            Block::StretchScaleSize { .. } => "stretch_scale_size",
            Block::ListValueAt { .. } => "value_of_index_from_list",
            Block::AddValueToList { .. } => "add_value_to_list",
            Block::RemoveValueFromList { .. } => "remove_value_from_list",
            Block::InsertValueToList { .. } => "insert_value_to_list",
            Block::ChangeValueListIndex { .. } => "change_value_list_index",
            Block::LengthOfList { .. } => "length_of_list",
            Block::IsIncludedInList { .. } => "is_included_in_list",
            Block::ShowList { .. } => "show_list",
            Block::HideList { .. } => "hide_list",
            Block::CreateClone { .. } => "create_clone",
            Block::MoveDirection { .. } => "move_direction",
            Block::Raw { type_id, .. } => type_id.as_str(),
            Block::DeleteClone => "delete_clone",
            Block::RemoveAllClones => "remove_all_clones",
            Block::IsPressSomeKey { .. } => "is_press_some_key",
            Block::ReachSomeThing { .. } => "reach_something",
            Block::BounceWall => "bounce_wall",
            Block::MoveX { .. } => "move_x",
            Block::MoveY { .. } => "move_y",
            Block::RotateRelative { .. } => "rotate_relative",
            Block::DirectionRelative { .. } => "direction_relative",
            Block::IsClicked => "is_clicked",
            Block::IsObjectClicked => "is_object_clicked",
            Block::MoveXyTime { .. } => "move_xy_time",
            Block::LocateX { .. } => "locate_x",
            Block::LocateY { .. } => "locate_y",
            Block::LocateXY { .. } => "locate_xy",
            Block::LocateXyTime { .. } => "locate_xy_time",
            Block::Locate { .. } => "locate",
            Block::LocateObjectTime { .. } => "locate_object_time",
            Block::RotateByTime { .. } => "rotate_by_time",
            Block::DirectionRelativeDuration { .. } => "direction_relative_duration",
            Block::RotateAbsolute { .. } => "rotate_absolute",
            Block::DirectionAbsolute { .. } => "direction_absolute",
            Block::SeeAngleObject { .. } => "see_angle_object",
            Block::MoveToAngle { .. } => "move_to_angle",
            Block::BrushStamp => "brush_stamp",
            Block::StartDrawing => "start_drawing",
            Block::StopDrawing => "stop_drawing",
            Block::StartFill => "start_fill",
            Block::StopFill => "stop_fill",
            Block::SetColor { .. } => "set_color",
            Block::SetRandomColor => "set_random_color",
            Block::SetFillColor { .. } => "set_fill_color",
            Block::ChangeThickness { .. } => "change_thickness",
            Block::SetThickness { .. } => "set_thickness",
            Block::ChangeBrushTransparency { .. } => "change_brush_transparency",
            Block::SetBrushTranparency { .. } => "set_brush_tranparency",
            Block::BrushEraseAll => "brush_erase_all",
            Block::TextRead { .. } => "text_read",
            Block::TextWrite { .. } => "text_write",
            Block::TextAppend { .. } => "text_append",
            Block::TextPrepend { .. } => "text_prepend",
            Block::TextChangeEffect { .. } => "text_change_effect",
            Block::TextFlush => "text_flush",
            Block::TextChangeFont { .. } => "text_change_font",
            Block::TextChangeFontColor { .. } => "text_change_font_color",
            Block::TextChangeBgColor { .. } => "text_change_bg_color",
            Block::SoundSomethingWithBlock { .. } => "sound_something_with_block",
            Block::SoundSomethingSecondWithBlock { .. } => "sound_something_second_with_block",
            Block::SoundFromTo { .. } => "sound_from_to",
            Block::SoundSomethingWaitWithBlock { .. } => "sound_something_wait_with_block",
            Block::SoundSomethingSecondWaitWithBlock { .. } => {
                "sound_something_second_wait_with_block"
            }
            Block::SoundFromToAndWait { .. } => "sound_from_to_and_wait",
            Block::SoundVolumeChange { .. } => "sound_volume_change",
            Block::SoundVolumeSet { .. } => "sound_volume_set",
            Block::GetSoundSpeed => "get_sound_speed",
            Block::SoundSpeedChange { .. } => "sound_speed_change",
            Block::SoundSpeedSet { .. } => "sound_speed_set",
            Block::SoundSilentAll { .. } => "sound_silent_all",
            Block::PlayBgm { .. } => "play_bgm",
            Block::StopBgm => "stop_bgm",
            Block::GetSoundVolume => "get_sound_volume",
            Block::GetSoundDuration { .. } => "get_sound_duration",
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
            Block::Break | Block::Continue | Block::RestartProject | Block::StopAll => {
                Category::Flow
            }
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
            Block::DialogTime { .. } => Category::Looks,
            Block::ChangeToSomeShape { .. } => Category::Looks,
            Block::ChangeToNextShape {} => Category::Looks,
            Block::RemoveDialog {} => Category::Looks,
            Block::AddEffectAmount { .. } => Category::Looks,
            Block::ChangeEffectAmount { .. } => Category::Looks,
            Block::EraseAllEffects {} => Category::Looks,
            Block::ChangeScaleSize { .. } => Category::Looks,
            Block::SetScaleSize { .. } => Category::Looks,
            Block::ResetScaleSize {} => Category::Looks,
            Block::FlipX {} => Category::Looks,
            Block::FlipY {} => Category::Looks,
            Block::ChangeObjectIndex { .. } => Category::Looks,
            Block::StretchScaleSize { .. } => Category::Looks,
            Block::ListValueAt { .. } => Category::Variable,
            Block::AddValueToList { .. } => Category::Variable,
            Block::RemoveValueFromList { .. } => Category::Variable,
            Block::InsertValueToList { .. } => Category::Variable,
            Block::ChangeValueListIndex { .. } => Category::Variable,
            Block::LengthOfList { .. } => Category::Variable,
            Block::IsIncludedInList { .. } => Category::Variable,
            Block::ShowList { .. } => Category::Variable,
            Block::HideList { .. } => Category::Variable,
            Block::CreateClone { .. } => Category::Flow,
            Block::MoveDirection { .. } => Category::Movement,
            Block::Raw { .. } => Category::Hardware,
            Block::DeleteClone => Category::Flow,
            Block::RemoveAllClones => Category::Flow,
            Block::IsPressSomeKey { .. } => Category::Judgment,
            Block::ReachSomeThing { .. } => Category::Judgment,
            Block::BounceWall => Category::Movement,
            Block::MoveX { .. } => Category::Movement,
            Block::MoveY { .. } => Category::Movement,
            Block::RotateRelative { .. } => Category::Movement,
            Block::DirectionRelative { .. } => Category::Movement,
            Block::IsClicked => Category::Judgment,
            Block::IsObjectClicked => Category::Judgment,
            Block::MoveXyTime { .. } => Category::Movement,
            Block::LocateX { .. } => Category::Movement,
            Block::LocateY { .. } => Category::Movement,
            Block::LocateXY { .. } => Category::Movement,
            Block::LocateXyTime { .. } => Category::Movement,
            Block::Locate { .. } => Category::Movement,
            Block::LocateObjectTime { .. } => Category::Movement,
            Block::RotateByTime { .. } => Category::Movement,
            Block::DirectionRelativeDuration { .. } => Category::Movement,
            Block::RotateAbsolute { .. } => Category::Movement,
            Block::DirectionAbsolute { .. } => Category::Movement,
            Block::SeeAngleObject { .. } => Category::Movement,
            Block::MoveToAngle { .. } => Category::Movement,
            Block::BrushStamp => Category::Pen,
            Block::StartDrawing => Category::Pen,
            Block::StopDrawing => Category::Pen,
            Block::StartFill => Category::Pen,
            Block::StopFill => Category::Pen,
            Block::SetColor { .. } => Category::Pen,
            Block::SetRandomColor => Category::Pen,
            Block::SetFillColor { .. } => Category::Pen,
            Block::ChangeThickness { .. } => Category::Pen,
            Block::SetThickness { .. } => Category::Pen,
            Block::ChangeBrushTransparency { .. } => Category::Pen,
            Block::SetBrushTranparency { .. } => Category::Pen,
            Block::BrushEraseAll => Category::Pen,
            Block::TextRead { .. } => Category::Text,
            Block::TextWrite { .. } => Category::Text,
            Block::TextAppend { .. } => Category::Text,
            Block::TextPrepend { .. } => Category::Text,
            Block::TextChangeEffect { .. } => Category::Text,
            Block::TextFlush => Category::Text,
            Block::TextChangeFont { .. } => Category::Text,
            Block::TextChangeFontColor { .. } => Category::Text,
            Block::TextChangeBgColor { .. } => Category::Text,
            Block::SoundSomethingWithBlock { .. } => Category::Sound,
            Block::SoundSomethingSecondWithBlock { .. } => Category::Sound,
            Block::SoundFromTo { .. } => Category::Sound,
            Block::SoundSomethingWaitWithBlock { .. } => Category::Sound,
            Block::SoundSomethingSecondWaitWithBlock { .. } => Category::Sound,
            Block::SoundFromToAndWait { .. } => Category::Sound,
            Block::SoundVolumeChange { .. } => Category::Sound,
            Block::SoundVolumeSet { .. } => Category::Sound,
            Block::GetSoundSpeed => Category::Sound,
            Block::SoundSpeedChange { .. } => Category::Sound,
            Block::SoundSpeedSet { .. } => Category::Sound,
            Block::SoundSilentAll { .. } => Category::Sound,
            Block::PlayBgm { .. } => Category::Sound,
            Block::StopBgm => Category::Sound,
            Block::GetSoundVolume => Category::Sound,
            Block::GetSoundDuration { .. } => Category::Sound,
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
                            .ok_or_else(|| SyntaxError("wait_second needs arg".into()))?;
                        return Ok(Block::WaitSeconds {
                            time: from_expr(arg)?,
                        });
                    }
                    if fref.name == "wait_until_true" {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("wait_until_true needs arg".into()))?;
                        return Ok(Block::WaitUntilTrue {
                            cond: from_expr(arg)?,
                        });
                    }
                    if fref.name == "stop_run_all" {
                        if args.len() != 0 {
                            return Err(SyntaxError("stop_run_all needs 0 args".into()));
                        }
                        return Ok(Block::StopAll);
                    }
                    if fref.name == "restart_project" {
                        if args.len() != 0 {
                            return Err(SyntaxError("restart_project needs 0 args".into()));
                        }
                        return Ok(Block::RestartProject);
                    }
                    if fref.name == "calc_rand" {
                        if args.len() != 2 {
                            return Err(SyntaxError("calc_rand needs 2 args".into()));
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
                            .ok_or_else(|| SyntaxError("ask_and_wait needs arg".into()))?;
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
                            return Err(SyntaxError("quotient_and_mod needs 3 args".into()));
                        }
                        let mode = parse_enum_arg::<QamMethod>(&args[2], "quotient_and_mod mode")?;
                        let a = from_expr(&args[0])?;
                        let b = from_expr(&args[1])?;
                        return Ok(Block::QuotientAndMod { a, b, mode });
                    }
                    if fref.name == "say" {
                        let content_arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("say needs arg".into()))?;
                        let content = from_expr(content_arg)?;
                        if let Some(time_arg) = args.get(1) {
                            let time = from_expr(time_arg)?;
                            return Ok(Block::DialogTime {
                                mode: DialogMode::Say,
                                content,
                                time,
                            });
                        }
                        return Ok(Block::Dialog {
                            mode: DialogMode::Say,
                            content,
                        });
                    }
                    if fref.name == "think" {
                        let content_arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("think needs arg".into()))?;
                        let content = from_expr(content_arg)?;
                        if let Some(time_arg) = args.get(1) {
                            let time = from_expr(time_arg)?;
                            return Ok(Block::DialogTime {
                                mode: DialogMode::Think,
                                content,
                                time,
                            });
                        }
                        return Ok(Block::Dialog {
                            mode: DialogMode::Think,
                            content,
                        });
                    }
                    if fref.name == "change_to_some_shape" {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("change_to_some_shape needs arg".into()))?;
                        let picture = match arg {
                            Expr::Str(s) => s.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "change_to_some_shape arg must be string".into(),
                                ));
                            }
                        };
                        return Ok(Block::ChangeToSomeShape { picture });
                    }
                    if fref.name == "change_to_next_shape" {
                        return Ok(Block::ChangeToNextShape {});
                    }
                    if let Some(op) = calc_op_from_name(&fref.name) {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError(format!("{} needs arg", fref.name)))?;
                        return Ok(Block::CalcOperation {
                            op,
                            expr: from_expr(arg)?,
                        });
                    }
                    if fref.name == "remove_dialog" {
                        return Ok(Block::RemoveDialog {});
                    }
                    if fref.name == "add_effect_amount" {
                        if args.len() != 2 {
                            return Err(SyntaxError("add_effect_amount needs 2 args".into()));
                        }
                        let effect =
                            parse_enum_arg::<EffectType>(&args[0], "add_effect_amount effect")?;
                        let amount = from_expr(&args[1])?;
                        return Ok(Block::AddEffectAmount { effect, amount });
                    }
                    if fref.name == "change_effect_amount" {
                        if args.len() != 2 {
                            return Err(SyntaxError("change_effect_amount needs 2 arg".into()));
                        }
                        let effect =
                            parse_enum_arg::<EffectType>(&args[0], "change_effect_amount effect")?;
                        let amount = from_expr(&args[1])?;
                        return Ok(Block::ChangeEffectAmount { effect, amount });
                    }
                    if fref.name == "erase_all_effects" {
                        return Ok(Block::EraseAllEffects {});
                    }
                    if fref.name == "change_scale_size" {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("change_scale_size needs arg".into()))?;
                        return Ok(Block::ChangeScaleSize {
                            amount: from_expr(arg)?,
                        });
                    }
                    if fref.name == "set_scale_size" {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("set_scale_size needs arg".into()))?;
                        return Ok(Block::SetScaleSize {
                            amount: from_expr(arg)?,
                        });
                    }
                    if fref.name == "reset_scale_size" {
                        return Ok(Block::ResetScaleSize {});
                    }
                    if fref.name == "delete_clone" {
                        return Ok(Block::DeleteClone);
                    }
                    if fref.name == "remove_all_clones" {
                        return Ok(Block::RemoveAllClones);
                    }
                    if fref.name == "stretch_scale_size" {
                        if args.len() != 2 {
                            return Err(SyntaxError("stretch_scale_size needs 2 args".into()));
                        }
                        let dim =
                            parse_enum_arg::<Dimension>(&args[0], "stretch_scale_size dimension")?;
                        let value = from_expr(&args[1])?;
                        return Ok(Block::StretchScaleSize { dim, value });
                    }
                    //얘네들은 반대로 작동함.
                    if fref.name == "flip_x" {
                        return Ok(Block::FlipX {});
                    }
                    if fref.name == "flip_y" {
                        return Ok(Block::FlipY {});
                    }
                    if fref.name == "change_object_index" {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("change_object_index needs arg".into()))?;
                        let direction = match arg {
                            Expr::Str(s) => s.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "change_object_index arg must be string".into(),
                                ));
                            }
                        };
                        return Ok(Block::ChangeObjectIndex { direction });
                    }
                    if fref.name == "add_value_to_list" {
                        if args.len() != 2 {
                            return Err(SyntaxError("add_value_to_list needs 2 args".into()));
                        }

                        let value = from_expr(&args[0])?;
                        let list = match &args[1] {
                            Expr::Var(name) => name.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "add_value_to_list list must be variable".into(),
                                ));
                            }
                        };

                        return Ok(Block::AddValueToList { value, list });
                    }
                    if fref.name == "remove_value_from_list" {
                        if args.len() != 2 {
                            return Err(SyntaxError("remove_value_from_list needs 2 args".into()));
                        }

                        let index = from_expr(&args[0])?;
                        let list = match &args[1] {
                            Expr::Var(name) => name.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "remove_value_from_list list must be variable".into(),
                                ));
                            }
                        };

                        return Ok(Block::RemoveValueFromList { index, list });
                    }
                    if fref.name == "insert_value_to_list" {
                        if args.len() != 3 {
                            return Err(SyntaxError("insert_value_to_list needs 3 args".into()));
                        }
                        let value = from_expr(&args[0])?;
                        let index = from_expr(&args[1])?;
                        let list = match &args[2] {
                            Expr::Var(name) => name.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "insert_value_to_list list must be variable".into(),
                                ));
                            }
                        };
                        return Ok(Block::InsertValueToList { value, index, list });
                    }
                    if fref.name == "change_value_list_index" {
                        if args.len() != 3 {
                            return Err(SyntaxError("change_vale_list_index needs 3 args".into()));
                        }
                        let index = from_expr(&args[0])?;
                        let value = from_expr(&args[1])?;
                        let list = match &args[2] {
                            Expr::Var(name) => name.clone(),
                            _ => return Err(SyntaxError("change_value_list_index".into())),
                        };

                        return Ok(Block::ChangeValueListIndex { index, value, list });
                    }
                    if fref.name == "length_of_list" {
                        if args.len() != 1 {
                            return Err(SyntaxError("length_of_list needs 1 arg".into()));
                        }
                        let list = match &args[0] {
                            Expr::Var(name) => name.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "length_of_list list must be variable".into(),
                                ));
                            }
                        };
                        return Ok(Block::LengthOfList { list });
                    }
                    if fref.name == "show_list" {
                        if args.len() != 1 {
                            return Err(SyntaxError("show_list needs 1 arg".into()));
                        }
                        let list = match &args[0] {
                            Expr::Var(name) => name.clone(),
                            _ => return Err(SyntaxError("show_list must be variable".into())),
                        };
                        return Ok(Block::ShowList { list });
                    }
                    if fref.name == "hide_list" {
                        if args.len() != 1 {
                            return Err(SyntaxError("hide_list needs 1 arg".into()));
                        }
                        let list = match &args[0] {
                            Expr::Var(name) => name.clone(),
                            _ => return Err(SyntaxError("hide_list must be variable".into())),
                        };
                        return Ok(Block::HideList { list });
                    }
                    if fref.name == "is_included_in_list" {
                        if args.len() != 2 {
                            return Err(SyntaxError("is_included_in_list needs 2 args".into()));
                        }
                        let list = match &args[0] {
                            Expr::Var(name) => name.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "is_included_in_list list must be variable".into(),
                                ));
                            }
                        };
                        let value = from_expr(&args[1])?;
                        return Ok(Block::IsIncludedInList { list, value });
                    }
                    if fref.name == "is_clicked" {
                        if args.len() > 0 {
                            return Err(SyntaxError("is_clicked needs no args".into()));
                        }
                        return Ok(Block::IsClicked);
                    }
                    if fref.name == "is_object_clicked" {
                        if args.len() > 0 {
                            return Err(SyntaxError("is_object_clicked needs no args".into()));
                        }
                        return Ok(Block::IsObjectClicked);
                    }
                    if fref.name == "create_clone" {
                        let target = match &args.len() {
                            0 => "self".to_string(),
                            1 => match &args[0] {
                                Expr::Str(s) => s.clone(),
                                Expr::Var(name) if name == "self" => "self".to_string(),
                                Expr::Var(name) => name.clone(),
                                _ => {
                                    return Err(SyntaxError(
                                        "create_clone target must be string literal or variable"
                                            .into(),
                                    ));
                                }
                            },
                            _ => {
                                return Err(SyntaxError(format!(
                                    "create_clone needs 0 or 1 args, got {}",
                                    args.len()
                                )));
                            }
                        };
                        return Ok(Block::CreateClone { target });
                    }
                    if fref.name == "move_x" {
                        if args.len() != 1 {
                            return Err(SyntaxError("move_x needs 1 arg".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::MoveX { amount });
                    }
                    if fref.name == "move_y" {
                        if args.len() != 1 {
                            return Err(SyntaxError("move_y needs 1 arg".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::MoveY { amount });
                    }
                    if fref.name == "rotate_relative" {
                        if args.len() != 1 {
                            return Err(SyntaxError("rotate_relative needs 1 arg".into()));
                        }
                        let angle = from_expr(&args[0])?;
                        return Ok(Block::RotateRelative { angle });
                    }
                    if fref.name == "direction_relative" {
                        if args.len() != 1 {
                            return Err(SyntaxError("direction_relative needs 1 arg".into()));
                        }
                        let angle = from_expr(&args[0])?;
                        return Ok(Block::DirectionRelative { angle });
                    }
                    if fref.name == "move_xy_time" {
                        if args.len() != 3 {
                            return Err(SyntaxError("move_xy_time".into()));
                        }
                        let duration = from_expr(&args[0])?;
                        let dx = from_expr(&args[1])?;
                        let dy = from_expr(&args[2])?;
                        return Ok(Block::MoveXyTime { duration, dx, dy });
                    }
                    if fref.name == "locate_x" {
                        if args.len() != 1 {
                            return Err(SyntaxError("locate_x needs 1 arg".into()));
                        }
                        let x = from_expr(&args[0])?;
                        return Ok(Block::LocateX { x });
                    }
                    if fref.name == "locate_y" {
                        if args.len() != 1 {
                            return Err(SyntaxError("locate_y needs 1 arg".into()));
                        }
                        let y = from_expr(&args[0])?;
                        return Ok(Block::LocateY { y });
                    }
                    if fref.name == "locate_xy" {
                        if args.len() != 2 {
                            return Err(SyntaxError("locate_xy needs 2 args".into()));
                        }
                        let x = from_expr(&args[0])?;
                        let y = from_expr(&args[1])?;
                        return Ok(Block::LocateXY { x, y });
                    }
                    if fref.name == "locate_xy_time" {
                        if args.len() != 3 {
                            return Err(SyntaxError("locate_xy_time needs 3 args".into()));
                        }
                        let duration = from_expr(&args[0])?;
                        let x = from_expr(&args[1])?;
                        let y = from_expr(&args[2])?;
                        return Ok(Block::LocateXyTime { duration, x, y });
                    }
                    if fref.name == "locate_object_time" {
                        if args.len() != 2 {
                            return Err(SyntaxError("locate_object_time needs 2 args".into()));
                        }
                        let duration = from_expr(&args[0])?;
                        let target = from_expr(&args[1])?;
                        return Ok(Block::LocateObjectTime { duration, target });
                    }
                    if fref.name == "locate" {
                        if args.len() != 1 {
                            return Err(SyntaxError("locate needs 1 args".into()));
                        }
                        let target = from_expr(&args[0])?;
                        return Ok(Block::Locate { target });
                    }
                    if fref.name == "rotate_by_time" {
                        if args.len() != 2 {
                            return Err(SyntaxError("rotate_by_time needs 2 args".into()));
                        }
                        let duration = from_expr(&args[0])?;
                        let angle = from_expr(&args[1])?;
                        return Ok(Block::RotateByTime { duration, angle });
                    }
                    if fref.name == "direction_relative_duration" {
                        if args.len() != 2 {
                            return Err(SyntaxError(
                                "direction_relative_duration needs 2 args".into(),
                            ));
                        }
                        let duration = from_expr(&args[0])?;
                        let amount = from_expr(&args[1])?;
                        return Ok(Block::DirectionRelativeDuration { duration, amount });
                    }
                    if fref.name == "rotate_absolute" {
                        if args.len() != 1 {
                            return Err(SyntaxError("rotate_absolute needs 1 arg".into()));
                        }
                        let angle = from_expr(&args[0])?;
                        return Ok(Block::RotateAbsolute { angle });
                    }
                    if fref.name == "direction_absolute" {
                        if args.len() != 1 {
                            return Err(SyntaxError("direction_absolute needs 1 arg".into()));
                        }
                        let angle = from_expr(&args[0])?;
                        return Ok(Block::DirectionAbsolute { angle });
                    }
                    if fref.name == "see_angle_object" {
                        if args.len() != 1 {
                            return Err(SyntaxError("see_angle_object needs 1 arg".into()));
                        }
                        let target = from_expr(&args[0])?;
                        return Ok(Block::SeeAngleObject { target });
                    }
                    if fref.name == "move_to_angle" {
                        if args.len() != 2 {
                            return Err(SyntaxError("move_to_angle needs 2 args".into()));
                        }
                        let angle = from_expr(&args[0])?;
                        let distance = from_expr(&args[1])?;
                        return Ok(Block::MoveToAngle { angle, distance });
                    }
                    if fref.name == "bounce_wall" {
                        if args.len() > 0 {
                            return Err(SyntaxError("bounce_wall needs no args".into()));
                        }
                        return Ok(Block::BounceWall);
                    }
                    if fref.name == "move_direction" {
                        if args.len() != 2 {
                            return Err(SyntaxError("move_direction needs 2 args".into()));
                        }
                        let direction = match &args[0] {
                            Expr::Str(s) => s.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "move_direction direction must be string".into(),
                                ));
                            }
                        };
                        let amount = from_expr(&args[1])?;
                        return Ok(Block::MoveDirection { direction, amount });
                    }
                    if fref.name == "is_press_some_key" {
                        let arg = args
                            .first()
                            .ok_or_else(|| SyntaxError("is_press_some_key needs arg".into()))?;
                        let key = match arg {
                            Expr::Str(s) => s.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "is_press_some_key arg must be string".into(),
                                ));
                            }
                        };
                        return Ok(Block::IsPressSomeKey { key });
                    }

                    if fref.name == "reach_something" {
                        let target = if let Some(arg) = args.first() {
                            match arg {
                                Expr::Str(s) => s.clone(),
                                _ => {
                                    return Err(SyntaxError(
                                        "reach_something arg must be string".into(),
                                    ));
                                }
                            }
                        } else {
                            "self".to_string()
                        };
                        return Ok(Block::ReachSomeThing { target });
                    }
                    // --- 붓(stmt) ---
                    if fref.name == "brush_stamp" {
                        if args.len() > 0 {
                            return Err(SyntaxError("brush_stamp not needs arg".into()));
                        }
                        return Ok(Block::BrushStamp);
                    }
                    if fref.name == "start_drawing" {
                        if args.len() > 0 {
                            return Err(SyntaxError("start_drawing not needs arg".into()));
                        }
                        return Ok(Block::StartDrawing);
                    }
                    if fref.name == "stop_drawing" {
                        if args.len() > 0 {
                            return Err(SyntaxError("stop_drawing not needs arg".into()));
                        }
                        return Ok(Block::StopDrawing);
                    }
                    if fref.name == "start_fill" {
                        if args.len() > 0 {
                            return Err(SyntaxError("start_fill not needs arg".into()));
                        }
                        return Ok(Block::StartFill);
                    }
                    if fref.name == "stop_fill" {
                        if args.len() > 0 {
                            return Err(SyntaxError("stop_fill not needs arg".into()));
                        }
                        return Ok(Block::StopFill);
                    }
                    if fref.name == "set_color" {
                        if args.len() != 3 {
                            return Err(SyntaxError("set_color needs 3 args".into()));
                        }
                        let r = from_expr(&args[0])?;
                        let g = from_expr(&args[1])?;
                        let b = from_expr(&args[2])?;
                        return Ok(Block::SetColor { r, g, b });
                    }
                    if fref.name == "set_random_color" {
                        if args.len() > 0 {
                            return Err(SyntaxError("set_random_color not needs arg".into()));
                        }
                        return Ok(Block::SetRandomColor);
                    }
                    if fref.name == "set_fill_color" {
                        if args.len() != 1 {
                            return Err(SyntaxError("set_fill_color needs 1 arg".into()));
                        }
                        let color = from_expr(&args[0])?;
                        return Ok(Block::SetFillColor { color });
                    }
                    if fref.name == "change_thickness" {
                        if args.len() != 1 {
                            return Err(SyntaxError("change_thickness needs 1 arg".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::ChangeThickness { amount });
                    }
                    if fref.name == "set_thickness" {
                        if args.len() != 1 {
                            return Err(SyntaxError("set_thickness needs 1 arg".into()));
                        }
                        let value = from_expr(&args[0])?;
                        return Ok(Block::SetThickness { value });
                    }
                    if fref.name == "change_brush_transparency" {
                        if args.len() != 1 {
                            return Err(SyntaxError(
                                "change_brush_transparency needs 1 arg".into(),
                            ));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::ChangeBrushTransparency { amount });
                    }
                    if fref.name == "set_brush_tranparency" {
                        if args.len() != 1 {
                            return Err(SyntaxError("set_brush_tranparency needs 1 arg".into()));
                        }
                        let value = from_expr(&args[0])?;
                        return Ok(Block::SetBrushTranparency { value });
                    }
                    if fref.name == "brush_erase_all" {
                        if args.len() > 0 {
                            return Err(SyntaxError("brush_erase_all not needs arg".into()));
                        }
                        return Ok(Block::BrushEraseAll);
                    }
                    // --- 글상자(stmt) ---
                    if fref.name == "text_read" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_read needs 1 arg".into()));
                        }
                        let value = from_expr(&args[0])?;
                        return Ok(Block::TextRead { value });
                    }
                    if fref.name == "text_write" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_write needs 1 arg".into()));
                        }
                        let content = from_expr(&args[0])?;
                        return Ok(Block::TextWrite { content });
                    }
                    if fref.name == "text_append" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_append needs 1 arg".into()));
                        }
                        let content = from_expr(&args[0])?;
                        return Ok(Block::TextAppend { content });
                    }
                    if fref.name == "text_prepend" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_prepend needs 1 arg".into()));
                        }
                        let content = from_expr(&args[0])?;
                        return Ok(Block::TextPrepend { content });
                    }
                    if fref.name == "text_change_effect" {
                        if args.len() != 2 {
                            return Err(SyntaxError("text_change_effect needs 2 args".into()));
                        }
                        let effect =
                            parse_enum_arg::<TextEffect>(&args[0], "text_change_effect effect")?;
                        let mode = match &args[1] {
                            Expr::Bool(b) => *b,
                            _ => {
                                return Err(SyntaxError(
                                    "text_change_effect mode must be bool".into(),
                                ));
                            }
                        };

                        return Ok(Block::TextChangeEffect { effect, mode });
                    }
                    if fref.name == "text_flush" {
                        if args.len() > 0 {
                            return Err(SyntaxError("text_flush needs 0 args".into()));
                        }
                        return Ok(Block::TextFlush);
                    }
                    if fref.name == "text_change_font" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_change_font needs 1 arg".into()));
                        }
                        let font = match &args[0] {
                            Expr::Str(font) => font.clone(),
                            _ => {
                                return Err(SyntaxError(
                                    "text_change_font font must be string".into(),
                                ));
                            }
                        };
                        return Ok(Block::TextChangeFont { font });
                    }
                    if fref.name == "text_change_font_color" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_change_font_color needs 1 arg".into()));
                        }
                        let color = from_expr(&args[0])?;
                        return Ok(Block::TextChangeFontColor { color });
                    }
                    if fref.name == "text_change_bg_color" {
                        if args.len() != 1 {
                            return Err(SyntaxError("text_change_bg_color needs 1 arg".into()));
                        }
                        let color = from_expr(&args[0])?;
                        return Ok(Block::TextChangeBgColor { color });
                    }
                    // --- 소리(stmt) ---
                    if fref.name == "sound_something_with_block" {
                        if args.len() != 1 {
                            return Err(SyntaxError(
                                "sound_something_with_block needs 1 arg".into(),
                            ));
                        }
                        let sound_name = from_expr(&args[0])?;
                        return Ok(Block::SoundSomethingWithBlock { sound_name });
                    }
                    if fref.name == "sound_something_second_with_block" {
                        if args.len() != 2 {
                            return Err(SyntaxError(
                                "sound_something_second_with_block needs 2 args".into(),
                            ));
                        }
                        let sound_name = from_expr(&args[0])?;
                        let seconds = from_expr(&args[1])?;
                        return Ok(Block::SoundSomethingSecondWithBlock {
                            sound_name,
                            seconds,
                        });
                    }
                    if fref.name == "sound_from_to" {
                        if args.len() != 3 {
                            return Err(SyntaxError("sound_from_to needs 3 args".into()));
                        }
                        let sound_name = from_expr(&args[0])?;
                        let start = from_expr(&args[1])?;
                        let end = from_expr(&args[2])?;
                        return Ok(Block::SoundFromTo {
                            sound_name,
                            start,
                            end,
                        });
                    }
                    if fref.name == "sound_something_wait_with_block" {
                        if args.len() != 1 {
                            return Err(SyntaxError(
                                "sound_something_wait_with_block needs 1 args".into(),
                            ));
                        }
                        let sound_name = from_expr(&args[0])?;
                        return Ok(Block::SoundSomethingWaitWithBlock { sound_name });
                    }
                    if fref.name == "sound_something_second_wait_with_block" {
                        if args.len() != 2 {
                            return Err(SyntaxError(
                                "sound_something_second_wait_with_block needs 2 args".into(),
                            ));
                        }
                        let sound_name = from_expr(&args[0])?;
                        let seconds = from_expr(&args[1])?;
                        return Ok(Block::SoundSomethingSecondWaitWithBlock {
                            sound_name,
                            seconds,
                        });
                    }
                    if fref.name == "sound_from_to_and_wait" {
                        if args.len() != 3 {
                            return Err(SyntaxError("sound_from_to_and_wait needs 3 args".into()));
                        }
                        let sound_name = from_expr(&args[0])?;
                        let start = from_expr(&args[1])?;
                        let end = from_expr(&args[2])?;

                        return Ok(Block::SoundFromToAndWait {
                            sound_name,
                            start,
                            end,
                        });
                    }
                    if fref.name == "sound_volume_change" {
                        if args.len() != 1 {
                            return Err(SyntaxError("sound_volume_change needs 1 args".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::SoundVolumeChange { amount });
                    }
                    if fref.name == "sound_volume_set" {
                        if args.len() != 1 {
                            return Err(SyntaxError("sound_volume_set needs 1 args".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::SoundVolumeSet { amount });
                    }
                    if fref.name == "sound_speed_change" {
                        if args.len() != 1 {
                            return Err(SyntaxError("sound_speed_change needs 1 args".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::SoundSpeedChange { amount });
                    }
                    if fref.name == "sound_speed_set" {
                        if args.len() != 1 {
                            return Err(SyntaxError("sound_speed_set needs 1 args".into()));
                        }
                        let amount = from_expr(&args[0])?;
                        return Ok(Block::SoundSpeedSet { amount });
                    }
                    if fref.name == "sound_silent_all" {
                        if args.len() != 1 {
                            return Err(SyntaxError("sound_silent_all needs 1 arg".into()));
                        }
                        let target = match &args[0] {
                            Expr::Str(target) => target.clone(),
                            _ => return Err(SyntaxError("sound_silent_all arg must be string".into())),
                        };
                        return Ok(Block::SoundSilentAll { target });
                    }
                    if fref.name == "play_bgm" {
                        if args.len() != 1 {
                            return Err(SyntaxError("play_bgm needs 1 arg".into()));
                        }
                        let sound_name = from_expr(&args[0])?;
                        return Ok(Block::PlayBgm { sound_name });
                    }
                    if fref.name == "stop_bgm" {
                        if args.len() > 0 {
                            return Err(SyntaxError("stop_bgm not needs args".into()));
                        }
                        return Ok(Block::StopBgm);
                    }
                    // 하드웨어 블럭 (소스맵 인덱스) — @hwraw 주석 우선, 없으면 스키마+args 구성.
                    if crate::block::registry::is_hw_block(&fref.name) {
                        let raw = if let Some(r) = &fref.raw {
                            r.clone()
                        } else {
                            let pb = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
                            let params: Vec<Value> = pb.iter().map(param_to_value).collect();
                            json!({ "type": fref.name, "params": params })
                        };
                        return Ok(Block::Raw {
                            type_id: fref.name.clone(),
                            raw,
                        });
                    }
                    let args = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
                    Ok(Block::FuncCall {
                        name: fref.name.clone(),
                        args,
                    })
                }
                _ => Err(SyntaxError("stmt-level expr not a call".into())),
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
                _ => return Err(SyntaxError("for iter not range".into())),
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
        Expr::Path(_) => Err(SyntaxError("qualified path expr".into())),
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
                    return Err(SyntaxError("calc_rand needs 2 args".into()));
                }
                let min = from_expr(&args[0])?;
                let max = from_expr(&args[1])?;
                return Ok(ParamBlock::Sub(Box::new(Block::CalcRand { min, max })));
            }
            if fref.name == "say" {
                let content_arg = args
                    .first()
                    .ok_or_else(|| SyntaxError("say needs arg".into()))?;
                let content = from_expr(content_arg)?;
                if let Some(time_arg) = args.get(1) {
                    let time = from_expr(time_arg)?;
                    return Ok(ParamBlock::Sub(Box::new(Block::DialogTime {
                        mode: DialogMode::Say,
                        content,
                        time,
                    })));
                }
                return Ok(ParamBlock::Sub(Box::new(Block::Dialog {
                    mode: DialogMode::Say,
                    content,
                })));
            }
            if fref.name == "think" {
                let content_arg = args
                    .first()
                    .ok_or_else(|| SyntaxError("think needs arg".into()))?;
                let content = from_expr(content_arg)?;
                if let Some(time_arg) = args.get(1) {
                    let time = from_expr(time_arg)?;
                    return Ok(ParamBlock::Sub(Box::new(Block::DialogTime {
                        mode: DialogMode::Think,
                        content,
                        time,
                    })));
                }
                return Ok(ParamBlock::Sub(Box::new(Block::Dialog {
                    mode: DialogMode::Think,
                    content,
                })));
            }
            if fref.name == "get_project_timer_value" {
                return Ok(ParamBlock::Sub(Box::new(Block::GetProjectTimerValue {})));
            }
            if fref.name == "get_canvas_input_value" {
                return Ok(ParamBlock::Sub(Box::new(Block::GetCanvasInputValue {})));
            }
            if fref.name == "quotient_and_mod" {
                if args.len() != 3 {
                    return Err(SyntaxError("quotient_and_mod needs 3 args".into()));
                }
                let mode = parse_enum_arg::<QamMethod>(&args[2], "quotient_and_mod mode")?;
                let a = from_expr(&args[0])?;
                let b = from_expr(&args[1])?;
                return Ok(ParamBlock::Sub(Box::new(Block::QuotientAndMod {
                    a,
                    b,
                    mode,
                })));
            }
            if let Some(op) = calc_op_from_name(&fref.name) {
                let arg = args
                    .first()
                    .ok_or_else(|| SyntaxError(format!("{} needs arg", fref.name)))?;
                return Ok(ParamBlock::Sub(Box::new(Block::CalcOperation {
                    op,
                    expr: from_expr(arg)?,
                })));
            }
            if fref.name == "value_of_index_from_list" {
                if args.len() != 2 {
                    return Err(SyntaxError("value_of_index_from_list needs 2 args".into()));
                }

                let index = from_expr(&args[0])?;
                let list = match &args[1] {
                    Expr::Var(name) => name.clone(),
                    _ => {
                        return Err(SyntaxError(
                            "value_of_index_from_list list must be variable".into(),
                        ));
                    }
                };

                return Ok(ParamBlock::Sub(Box::new(Block::ListValueAt {
                    index,
                    list,
                })));
            }
            if fref.name == "length_of_list" {
                if args.len() != 1 {
                    return Err(SyntaxError("length_of_list needs 1 arg".into()));
                }
                let list = match &args[0] {
                    Expr::Var(name) => name.clone(),
                    _ => {
                        return Err(SyntaxError("length_of_list list must be variable".into()));
                    }
                };
                return Ok(ParamBlock::Sub(Box::new(Block::LengthOfList { list })));
            }
            if fref.name == "is_included_in_list" {
                if args.len() != 2 {
                    return Err(SyntaxError("is_included_in_list needs 2 args".into()));
                }
                let list = match &args[0] {
                    Expr::Var(name) => name.clone(),
                    _ => {
                        return Err(SyntaxError(
                            "is_included_in_list list must be variable".into(),
                        ));
                    }
                };
                let value = from_expr(&args[1])?;
                return Ok(ParamBlock::Sub(Box::new(Block::IsIncludedInList {
                    list,
                    value,
                })));
            }
            if fref.name == "is_press_some_key" {
                let arg = args
                    .first()
                    .ok_or_else(|| SyntaxError("is_press_some_key needs arg".into()))?;
                let key = match arg {
                    Expr::Str(s) => s.clone(),
                    _ => return Err(SyntaxError("is_press_some_key arg must be string".into())),
                };
                return Ok(ParamBlock::Sub(Box::new(Block::IsPressSomeKey { key })));
            }
            if fref.name == "reach_something" {
                let target = if let Some(arg) = args.first() {
                    match arg {
                        Expr::Str(s) => s.clone(),
                        _ => {
                            return Err(SyntaxError("reach_something arg must be string".into()));
                        }
                    }
                } else {
                    "self".to_string()
                };
                return Ok(ParamBlock::Sub(Box::new(Block::ReachSomeThing { target })));
            }
            // --- 글상자 ---
            if fref.name == "text_read" {
                if args.len() != 1 {
                    return Err(SyntaxError("text_read needs 1 arg".into()));
                }
                let value = from_expr(&args[0])?;
                return Ok(ParamBlock::Sub(Box::new(Block::TextRead { value })));
            }
            // --- 소리 ---
            if fref.name == "get_sound_speed" {
                if args.len() > 0 {
                    return Err(SyntaxError("get_sound_speed not needs args".into()));
                }
                return Ok(ParamBlock::Sub(Box::new(Block::GetSoundSpeed)));
            }
            if fref.name == "get_sound_volume" {
                if args.len() > 0 {
                    return Err(SyntaxError("get_sound_volume not needs args".into()));
                }
                return Ok(ParamBlock::Sub(Box::new(Block::GetSoundVolume)));
            }
            if fref.name == "get_sound_duration" {
                if args.len() != 1 {
                    return Err(SyntaxError("get_sound_duration needs 1 arg".into()));
                }
                let sound_name = match &args[0] {
                    Expr::Str(sound_name) => sound_name.clone(),
                    _ => return Err(SyntaxError("get_sound_duration arg must be string".into())),
                };
                return Ok(ParamBlock::Sub(Box::new(Block::GetSoundDuration { sound_name })));
            }
            // 하드웨어 getter 블럭 (소스맵 인덱스) — 값으로 사용.
            if crate::block::registry::is_hw_block(&fref.name) {
                let raw = if let Some(r) = &fref.raw {
                    r.clone()
                } else {
                    let pb = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
                    let params: Vec<Value> = pb.iter().map(param_to_value).collect();
                    json!({ "type": fref.name, "params": params })
                };
                return Ok(ParamBlock::Sub(Box::new(Block::Raw {
                    type_id: fref.name.clone(),
                    raw,
                })));
            }
            let args = args.iter().map(from_expr).collect::<Result<Vec<_>>>()?;
            Ok(ParamBlock::Sub(Box::new(Block::FuncCall {
                name: fref.name.clone(),
                args,
            })))
        }
        Expr::Func(_) => Err(SyntaxError("bare func ref".into())),
        Expr::Range(start, end) => {
            let _ = (start, end);
            Err(SyntaxError("range expr".into()))
        }
    }
}

/// Block -> serde_json::Value 변환.
///
/// Entry project.json 형식: `{type, params, statements?}`.
/// `statements[N]`은 본문 thread 배열 (없으면 키 생략).
pub fn to_value(block: &Block) -> crate::Result<Value> {
    // 하드웨어 블럭은 원본 .ent JSON 을 그대로 반환 (손실 없는 왕복).
    if let Block::Raw { raw, .. } = block {
        return Ok(raw.clone());
    }
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

/// 오브젝트별 이미지 이름을 Entry 자산 ID로 치환해 블록 JSON을 만든다.
pub fn to_value_with_assets(
    block: &Block,
    assets: &crate::AssetMap,
    object_name: &str,
) -> crate::Result<Value> {
    let mut value = to_value(block)?;
    if let Block::ChangeToSomeShape { picture } = block {
        if let Some(id) = assets.picture_id_by_name(object_name, picture) {
            // `change_to_some_shape`은 문자열이 아니라 `get_pictures` 값 블록을 받는다.
            value["params"][0]["params"][0] = Value::String(id.to_string());
        }
    }
    if let Block::SoundSomethingWithBlock {
        sound_name: ParamBlock::Text(sound_name),
    }
    | Block::SoundSomethingSecondWithBlock {
        sound_name: ParamBlock::Text(sound_name),
        ..
    }
    | Block::SoundFromTo {
        sound_name: ParamBlock::Text(sound_name),
        ..
    }
    | Block::SoundSomethingWaitWithBlock {
        sound_name: ParamBlock::Text(sound_name),
    }
    | Block::SoundSomethingSecondWaitWithBlock {
        sound_name: ParamBlock::Text(sound_name),
        ..
    }
    | Block::SoundFromToAndWait {
        sound_name: ParamBlock::Text(sound_name),
        ..
    } = block
    {
        let id = assets
            .sound_id_by_name(object_name, sound_name)
            .ok_or_else(|| {
                crate::Error::Semantic(format!("{object_name}: sound not found: {sound_name}"))
            })?;
        // 소리 선택도 EntryJS의 `get_sounds` 값 블록으로 저장한다.
        value["params"][0] = json!({ "type": "get_sounds", "params": [id] });
    }
    if let Block::PlayBgm {
        sound_name: ParamBlock::Text(sound_name),
    } = block
    {
        let id = assets
            .sound_id_by_name(object_name, sound_name)
            .ok_or_else(|| {
                crate::Error::Semantic(format!("{object_name}: sound not found: {sound_name}"))
            })?;
        value["params"][0] = json!({ "type": "get_sounds", "params": [id] });
    }
    if let Block::GetSoundDuration { sound_name } = block {
        if let Some(id) = assets.sound_id_by_name(object_name, sound_name) {
            value["params"][1] = Value::String(id.to_string());
        }
    }
    fn resolve_nested_sound_duration(
        value: &mut Value,
        assets: &crate::AssetMap,
        object_name: &str,
    ) {
        if let Some(obj) = value.as_object_mut() {
            if obj.get("type").and_then(Value::as_str) == Some("get_sound_duration") {
                if let Some(Value::String(name)) = obj
                    .get("params")
                    .and_then(Value::as_array)
                    .and_then(|params| params.get(1))
                {
                    if let Some(id) = assets.sound_id_by_name(object_name, name) {
                        if let Some(params) = obj.get_mut("params").and_then(Value::as_array_mut) {
                            params[1] = Value::String(id.to_string());
                        }
                    }
                }
            }
            for child in obj.values_mut() {
                resolve_nested_sound_duration(child, assets, object_name);
            }
        } else if let Some(items) = value.as_array_mut() {
            for child in items {
                resolve_nested_sound_duration(child, assets, object_name);
            }
        }
    }
    resolve_nested_sound_duration(&mut value, assets, object_name);
    Ok(value)
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
        Block::Break | Block::Continue => (vec![], None),
        Block::StopAll => (vec![Value::Null], None),
        Block::RestartProject => (vec![Value::Null], None),
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
                param_to_value(expr),
                Value::String(match op {
                    UnaryOp::Neg => "-".into(),
                    UnaryOp::Not => "!".into(),
                }),
                Value::Null,
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
        Block::DialogTime {
            mode,
            content,
            time,
        } => (
            vec![
                param_to_value(content),
                Value::String(match mode {
                    DialogMode::Say => "say".into(),
                    DialogMode::Think => "think".into(),
                }),
                param_to_value(time),
                Value::Null,
            ],
            None,
        ),
        Block::ChangeToSomeShape { picture } => {
            // EntryJS는 이미지 선택 값을 `get_pictures` 블록으로 저장한다.
            (
                vec![
                    json!({ "type": "get_pictures", "params": [picture] }),
                    Value::Null,
                ],
                None,
            )
        }
        Block::ChangeToNextShape {} => (vec![], None),
        Block::RemoveDialog {} => (vec![], None),
        Block::AddEffectAmount { effect, amount } => (
            vec![
                Value::String(effect_to_str(*effect).to_string()),
                param_to_value(amount),
                Value::Null,
            ],
            None,
        ),
        Block::StretchScaleSize { dim, value } => (
            vec![
                Value::String(dim_to_str(dim).to_string()),
                param_to_value(value),
                Value::Null,
            ],
            None,
        ),
        Block::ChangeEffectAmount { effect, amount } => (
            vec![
                Value::String(effect_to_str(*effect).to_string()),
                param_to_value(amount),
                Value::Null,
            ],
            None,
        ),
        Block::EraseAllEffects {} => (vec![], None),
        Block::ChangeScaleSize { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::SetScaleSize { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::ResetScaleSize {} => (vec![], None),
        Block::FlipX {} => (vec![], None),
        Block::FlipY {} => (vec![], None),
        Block::ChangeObjectIndex { direction } => (vec![Value::String(direction.clone())], None),
        Block::ListValueAt { index, list } => {
            (vec![param_to_value(index), list_variable_param(list)], None)
        }
        Block::AddValueToList { value, list } => (
            vec![
                param_to_value(value),
                list_variable_param(list),
                Value::Null,
            ],
            None,
        ),
        Block::RemoveValueFromList { index, list } => (
            vec![
                param_to_value(index),
                list_variable_param(list),
                Value::Null,
            ],
            None,
        ),
        Block::InsertValueToList { value, index, list } => (
            vec![
                param_to_value(value),
                param_to_value(index),
                list_variable_param(list),
                Value::Null,
            ],
            None,
        ),
        Block::ChangeValueListIndex { index, value, list } => (
            vec![
                param_to_value(index),
                param_to_value(value),
                list_variable_param(list),
                Value::Null,
            ],
            None,
        ),
        Block::LengthOfList { list } => (
            vec![Value::Null, list_variable_param(list), Value::Null],
            None,
        ),
        Block::IsIncludedInList { list, value } => (
            vec![
                Value::Null,
                list_variable_param(list),
                Value::Null,
                param_to_value(value),
                Value::Null,
            ],
            None,
        ),
        Block::ShowList { list } => (vec![list_variable_param(list), Value::Null], None),
        Block::HideList { list } => (vec![list_variable_param(list), Value::Null], None),
        Block::CreateClone { target } => (vec![Value::String(target.clone()), Value::Null], None),
        Block::MoveDirection { direction, amount } => (
            vec![
                Value::String(direction.clone()),
                param_to_value(amount),
                Value::Null,
            ],
            None,
        ),
        // to_value 가 Raw 는 조기 반환하므로 이 arm 은 도달하지 않는다 (완전 매치용).
        Block::Raw { .. } => (vec![], None),
        Block::DeleteClone => (vec![], None),
        Block::RemoveAllClones => (vec![], None),
        Block::IsPressSomeKey { key } => (vec![Value::String(key.clone()), Value::Null], None),
        Block::ReachSomeThing { target } => {
            (vec![Value::String(target.clone()), Value::Null], None)
        }
        Block::BounceWall => (vec![], None),
        Block::MoveX { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::MoveY { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::RotateRelative { angle } => (vec![param_to_value(angle), Value::Null], None),
        Block::DirectionRelative { angle } => (vec![param_to_value(angle), Value::Null], None),
        Block::IsClicked => (vec![], None),
        Block::IsObjectClicked => (vec![], None),
        Block::MoveXyTime { duration, dx, dy } => (
            vec![
                param_to_value(duration),
                param_to_value(dx),
                param_to_value(dy),
                Value::Null,
            ],
            None,
        ),
        Block::LocateX { x } => (vec![param_to_value(x), Value::Null], None),
        Block::LocateY { y } => (vec![param_to_value(y), Value::Null], None),
        Block::LocateXY { x, y } => (
            vec![param_to_value(x), param_to_value(y), Value::Null],
            None,
        ),
        Block::LocateXyTime { duration, x, y } => (
            vec![
                param_to_value(duration),
                param_to_value(x),
                param_to_value(y),
                Value::Null,
            ],
            None,
        ),
        Block::LocateObjectTime { duration, target } => (
            vec![
                param_to_value(duration),
                param_to_value(target),
                Value::Null,
            ],
            None,
        ),
        Block::Locate { target } => (vec![param_to_value(target), Value::Null], None),
        Block::RotateByTime { duration, angle } => (
            vec![param_to_value(duration), param_to_value(angle), Value::Null],
            None,
        ),

        Block::DirectionRelativeDuration { duration, amount } => (
            vec![
                param_to_value(duration),
                param_to_value(amount),
                Value::Null,
            ],
            None,
        ),
        Block::RotateAbsolute { angle } => (vec![param_to_value(angle), Value::Null], None),
        Block::DirectionAbsolute { angle } => (vec![param_to_value(angle), Value::Null], None),
        Block::SeeAngleObject { target } => (vec![param_to_value(target), Value::Null], None),
        Block::MoveToAngle { angle, distance } => (
            vec![param_to_value(angle), param_to_value(distance), Value::Null],
            None,
        ),
        Block::BrushStamp => (vec![], None),
        Block::StartDrawing => (vec![], None),
        Block::StopDrawing => (vec![], None),
        Block::StartFill => (vec![], None),
        Block::StopFill => (vec![], None),
        Block::SetColor { r, g, b } => (
            vec![param_to_value(r), param_to_value(g), param_to_value(b)],
            None,
        ),
        Block::SetRandomColor => (vec![], None),
        Block::SetFillColor { color } => (vec![param_to_value(color), Value::Null], None),
        Block::ChangeThickness { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::SetThickness { value } => (vec![param_to_value(value), Value::Null], None),
        Block::ChangeBrushTransparency { amount } => {
            (vec![param_to_value(amount), Value::Null], None)
        }
        Block::SetBrushTranparency { value } => (vec![param_to_value(value), Value::Null], None),
        Block::BrushEraseAll => (vec![], None),
        Block::TextRead { value } => (vec![param_to_value(value), Value::Null], None),
        Block::TextWrite { content } => (vec![param_to_value(content), Value::Null], None),
        Block::TextAppend { content } => (vec![param_to_value(content), Value::Null], None),
        Block::TextPrepend { content } => (vec![param_to_value(content), Value::Null], None),
        Block::TextChangeEffect { effect, mode } => (
            vec![
                Value::String(text_effect_to_str(*effect).to_string()),
                Value::String(if *mode { "on" } else { "off" }.to_string()),
                Value::Null,
            ],
            None,
        ),
        Block::TextFlush => (vec![], None),
        Block::TextChangeFont { font } => (vec![Value::String(font.clone()), Value::Null], None),
        Block::TextChangeFontColor { color } => (vec![param_to_value(color), Value::Null], None),
        Block::TextChangeBgColor { color } => (vec![param_to_value(color), Value::Null], None),
        Block::SoundSomethingWithBlock { sound_name } => {
            (vec![param_to_value(sound_name), Value::Null], None)
        }
        Block::SoundSomethingSecondWithBlock {
            sound_name,
            seconds,
        } => (
            vec![
                param_to_value(sound_name),
                param_to_value(seconds),
                Value::Null,
            ],
            None,
        ),
        Block::SoundFromTo {
            sound_name,
            start,
            end,
        } => (
            vec![
                param_to_value(sound_name),
                param_to_value(start),
                param_to_value(end),
                Value::Null,
            ],
            None,
        ),
        Block::SoundSomethingWaitWithBlock { sound_name } => {
            (vec![param_to_value(sound_name), Value::Null], None)
        }
        Block::SoundSomethingSecondWaitWithBlock {
            sound_name,
            seconds,
        } => (
            vec![
                param_to_value(sound_name),
                param_to_value(seconds),
                Value::Null,
            ],
            None,
        ),
        Block::SoundFromToAndWait {
            sound_name,
            start,
            end,
        } => (
            vec![
                param_to_value(sound_name),
                param_to_value(start),
                param_to_value(end),
                Value::Null,
            ],
            None,
        ),
        Block::SoundVolumeChange { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::SoundVolumeSet { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::GetSoundSpeed => (vec![], None),
        Block::SoundSpeedChange { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::SoundSpeedSet { amount } => (vec![param_to_value(amount), Value::Null], None),
        Block::SoundSilentAll { target } => {
            (vec![Value::String(target.clone()), Value::Null], None)
        }
        Block::PlayBgm { sound_name } => (vec![param_to_value(sound_name), Value::Null], None),
        Block::StopBgm => (vec![], None),
        Block::GetSoundVolume => (vec![], None),
        Block::GetSoundDuration { sound_name } => (
            vec![Value::Null, Value::String(sound_name.clone()), Value::Null],
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

/// EffectType -> Entry effects 문자열.
pub fn effect_to_str(e: EffectType) -> &'static str {
    match e {
        EffectType::Color => "color",
        EffectType::Brightness => "brightness",
        EffectType::Ghost => "ghost",
        EffectType::Fisheye => "fisheye",
        EffectType::Whirl => "whirl",
        EffectType::Pixelate => "pixelate",
        EffectType::Mosaic => "mosaic",
        EffectType::Negative => "negative",
    }
}

/// Dimension -> EntryJS dropdown 값 (대문자).
pub fn dim_to_str(d: &Dimension) -> &'static str {
    match d {
        Dimension::Width => "WIDTH",
        Dimension::Height => "HEIGHT",
    }
}

/// Dimension -> DSL 신택스 문자열 (소문자, `str_to_dim` 의 역).
pub fn dim_to_dsl_str(d: &Dimension) -> &'static str {
    match d {
        Dimension::Width => "width",
        Dimension::Height => "height",
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

fn list_variable_param(name: &str) -> Value {
    json!({ "id": id_for(name), "name": name, "variableType": "list" })
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

//effect helper
fn str_to_effect(s: &str) -> Option<EffectType> {
    Some(match s {
        "color" => EffectType::Color,
        "brightness" => EffectType::Brightness,
        "ghost" => EffectType::Ghost,
        "fisheye" => EffectType::Fisheye,
        "whirl" => EffectType::Whirl,
        "pixelate" => EffectType::Pixelate,
        "mosaic" => EffectType::Mosaic,
        "negative" => EffectType::Negative,
        _ => return None,
    })
}
pub fn str_to_text_effect(s: &str) -> Option<TextEffect> {
    Some(match s {
        "strike" => TextEffect::Strike,
        "underLine" => TextEffect::UnderLine,
        "fontItalic" => TextEffect::FontItalic,
        "fontBold" => TextEffect::FontBlold,
        _ => return None,
    })
}

pub fn text_effect_to_str(t: TextEffect) -> &'static str {
    match t {
        TextEffect::Strike => "strike",
        TextEffect::UnderLine => "underLine",
        TextEffect::FontItalic => "fontItalic",
        TextEffect::FontBlold => "fontBold",
    }
}

/// 문자열 리터럴과 Rust enum 경로를 함께 받는 dropdown enum 규약.
trait DslEnum: Sized {
    const TYPE_NAME: &'static str;

    fn from_dsl_str(value: &str) -> Option<Self>;
    fn from_variant(variant: &str) -> Option<Self>;
}

/// dropdown 인자를 기존 문자열 또는 `EnumType::Variant` 경로로 변환한다.
fn parse_enum_arg<T: DslEnum>(expr: &Expr, arg_name: &str) -> crate::Result<T> {
    match expr {
        Expr::Str(value) => T::from_dsl_str(value)
            .ok_or_else(|| SyntaxError(format!("unknown {} value: {value}", T::TYPE_NAME))),
        Expr::Path(segments) if segments.len() == 2 && segments[0] == T::TYPE_NAME => {
            T::from_variant(&segments[1]).ok_or_else(|| {
                SyntaxError(format!("unknown {} variant: {}", T::TYPE_NAME, segments[1]))
            })
        }
        _ => Err(SyntaxError(format!(
            "{arg_name} must be string or {} variant",
            T::TYPE_NAME
        ))),
    }
}

impl DslEnum for EffectType {
    const TYPE_NAME: &'static str = "EffectType";

    fn from_dsl_str(value: &str) -> Option<Self> {
        str_to_effect(value)
    }

    fn from_variant(variant: &str) -> Option<Self> {
        Some(match variant {
            "Color" => Self::Color,
            "Brightness" => Self::Brightness,
            "Ghost" => Self::Ghost,
            "Fisheye" => Self::Fisheye,
            "Whirl" => Self::Whirl,
            "Pixelate" => Self::Pixelate,
            "Mosaic" => Self::Mosaic,
            "Negative" => Self::Negative,
            _ => return None,
        })
    }
}

impl DslEnum for TextEffect {
    const TYPE_NAME: &'static str = "TextEffect";

    fn from_dsl_str(value: &str) -> Option<Self> {
        str_to_text_effect(value)
    }

    fn from_variant(variant: &str) -> Option<Self> {
        Some(match variant {
            "Strike" => Self::Strike,
            "UnderLine" => Self::UnderLine,
            "FontItalic" => Self::FontItalic,
            "FontBlold" => Self::FontBlold,
            _ => return None,
        })
    }
}

impl DslEnum for Dimension {
    const TYPE_NAME: &'static str = "Dimension";

    fn from_dsl_str(value: &str) -> Option<Self> {
        str_to_dim(value)
    }

    fn from_variant(variant: &str) -> Option<Self> {
        Some(match variant {
            "Width" => Self::Width,
            "Height" => Self::Height,
            _ => return None,
        })
    }
}

impl DslEnum for QamMethod {
    const TYPE_NAME: &'static str = "QamMethod";

    fn from_dsl_str(value: &str) -> Option<Self> {
        Some(match value {
            "quotient" => Self::Quotient,
            "modulo" => Self::Mod,
            _ => return None,
        })
    }

    fn from_variant(variant: &str) -> Option<Self> {
        Some(match variant {
            "Quotient" => Self::Quotient,
            "Mod" => Self::Mod,
            _ => return None,
        })
    }
}
//dim helper
fn str_to_dim(s: &str) -> Option<Dimension> {
    Some(match s {
        "width" => Dimension::Width,
        "height" => Dimension::Height,
        _ => return None,
    })
}
