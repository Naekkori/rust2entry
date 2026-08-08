//! 코어 라이브러리: Rust 소스 -> IR / IR -> Entry 블록 직렬화.
//! + Entry 블록 -> IR 역변환.

pub mod block;
pub mod codegen;
pub mod decodegen;
pub mod deparse;
pub mod error;
pub mod ir;
pub mod parse;
pub mod var;

pub use error::{Error, Result};
pub use var::{VarInfo, VarInit, VarKind, VarMap};
use serde_json::{Value, json};

use crate::ir::{Program, Stmt};

/// 여러 .rs 소스 + base project.json -> (최종 project.json, unmapped 블록 메시지 목록).
///
/// ## 동작
/// 1. 각 .rs 소스를 `parse::parse` -> IR Program.
/// 2. 각 Program 의 stmts 를 `block::from_stmt` -> Entry 블록 Value 배열로 변환.
/// 3. rs 파일 stem 과 base 의 `objects[].name` 을 대소문자 무시 매칭해,
///    매칭된 object 의 `script` 필드를 thread 배열(`[[blocks...]]`)로 패치.
///    Entry 의 object.script 는 트리거 묶음 배열의 배열 (각 thread = `[when_*, ...body]`).
/// 4. 매칭 안 된 rs 는 새 sprite object 로 만들어 objects 에 append.
///    objects 가 처음부터 비어있으면 첫 매칭 안 된 rs 의 이름으로 단일 object 생성.
///    base 에 기존 스프라이트가 있으면 그 entity/sprite 메타를 그대로 복사해 위치/이미지가
///    0 으로 초기화되지 않도록 한다.
/// 5. `project.scripts` 는 base 값으로 복원 (codegen 이 덮어쓴 결과를 되돌림).
///    Entry 의 실제 스크립트 위치는 각 object 의 `script` 필드.
/// 6. `project.variables` 는 codegen 결과 그대로 유지 (전역 변수).
///
/// ## Unmapped 처리
/// `from_stmt`/`to_value` 가 Entry 블록으로 직렬화 불가한 IR 을 만나면
/// `UnmappedBlock` 에러를 내지 않고 (Vec, 두 번째 반환) 에 메시지를 누적한다.
/// 빌드 시 eprintln 으로 경고 출력용. 그 외 에러(parse/semantic/codegen)는
/// 그대로 propagate.
/// build 옵션. 현재는 가짜 object 의 scene 만 노출.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// 가짜 object 에 적용할 scene id. None 이면 base 의 첫 sprite scene 복사.
    pub default_scene: Option<String>,
}

pub fn compile(rs_sources: &[(&str, &str)], base: &Value) -> Result<(Value, Vec<String>)> {
    compile_with_options(rs_sources, base, &CompileOptions::default())
}

/// `CompileOptions` 적용 build.
pub fn compile_with_options(
    rs_sources: &[(&str, &str)],
    base: &Value,
    options: &CompileOptions,
) -> Result<(Value, Vec<String>)> {
    let mut unmapped: Vec<String> = Vec::new();
    // 1. 각 rs 를 두 가지로 파싱한다:
    //    - `parse::parse` (트리거 body 평탄화 포함) -> variables 집계용 Program
    //    - `parse::parse_with_triggers` (트리거 분리) -> object.script thread 구성용
    let mut per_source: Vec<(String, ThreadsAndHelpers)> =
        Vec::with_capacity(rs_sources.len());
    let mut merged_stmts: Vec<crate::ir::Stmt> = Vec::new();
    let mut all_helpers: Vec<FunctionDef> = Vec::new();
    let mut all_messages: Vec<String> = Vec::new();
    for (name, src) in rs_sources {
        // variables 집계: 트리거 body 포함 모든 stmt 평탄화
        let flat_program = parse::parse(src)?;
        merged_stmts.extend(flat_program.stmts.clone());
        // object.script thread: 트리거별 분리
        let (non_trigger_program, triggers) = parse::parse_with_triggers(src)?;
        let mut tah = build_threads(&triggers, &non_trigger_program, &mut unmapped)?;
        all_helpers.append(&mut tah.helpers);
        all_messages.append(&mut tah.messages);
        per_source.push((name.to_string(), tah));
    }
    let merged_program = Program { stmts: merged_stmts };

    // 2. variables 패치 (codegen::generate 를 통째로 부르지 않고 variables 만 직접 빌드)
    //    이유: generate 는 from_stmt 을 scripts 생성용으로 호출하는데, 이 경로의
    //    UnmappedBlock 을 우리 (Vec<String>) 에 누적할 수 없기 때문이다.
    //    scripts 는 build_threads 가 이미 object 별로 thread 배열을 만들어 두었고
    //    project.scripts 는 base 값으로 복원한다.
    let mut project = base.clone();
    // 각 rs 별로 변수 이름 -> 매핑되는 object (rs stem). 같은 변수가 여러 rs 에서
    // 등장하면 첫 등장 rs 의 object 로 매핑. Entry 의 variable 항목 `object` 필드는
    // UI 에서 어느 object 의 변수인지 보여주기 위한 표시용이다 (값 자체는 전역).
    let mut var_object: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (name, src) in rs_sources {
        let flat = parse::parse(src)?;
        for var_name in collect_var_names(&flat) {
            var_object.entry(var_name).or_insert_with(|| name.to_string());
        }
    }
    let vars_map = codegen::collect_var_map(&merged_program);
    let vars_arr: Vec<Value> = vars_map
        .iter()
        .map(|v| {
            // Entry 실제 .ent 형식 (sample 기준):
            //   name, id, visible, value, variableType, isCloud, isRealTime,
            //   cloudDate, object (null or sprite name), x, y
            // 우리 VarInfo 는 kind/init 만 가지므로 나머지는 kind 기반 기본값.
            let is_cloud = matches!(v.kind, crate::var::VarKind::Cloud);
            let is_realtime = matches!(v.kind, crate::var::VarKind::RealTime);
            json!({
                "id": v.id,
                "name": v.name,
                "variableType": match v.kind {
                    crate::var::VarKind::Variable => "variable",
                    crate::var::VarKind::Answer => "answer",
                    crate::var::VarKind::Timer => "timer",
                    crate::var::VarKind::List => "list",
                    crate::var::VarKind::Cloud => "cloud",
                    crate::var::VarKind::RealTime => "realtime",
                    crate::var::VarKind::Unknown => "variable",
                },
                "value": match v.init {
                    crate::var::VarInit::Int0 => json!(0),
                    crate::var::VarInit::Float0 => json!(0.0),
                    crate::var::VarInit::EmptyStr => json!(""),
                    crate::var::VarInit::False => json!(false),
                    crate::var::VarInit::EmptyList => json!([]),
                },
                "visible": true,
                "isCloud": is_cloud,
                "isRealTime": is_realtime,
                "cloudDate": false,
                // 변수가 등장한 object 표시 (전역 변수는 null).
                // EntryJS 가 자동 관리하는 timer / answer 와 cloud / realtime /
                // list 는 항상 전역 (어느 object 에도 묶이지 않음).
                "object": if matches!(
                    v.kind,
                    crate::var::VarKind::Timer
                        | crate::var::VarKind::Answer
                        | crate::var::VarKind::Cloud
                        | crate::var::VarKind::RealTime
                        | crate::var::VarKind::List
                ) {
                    Value::Null
                } else {
                    var_object
                        .get(&v.name)
                        .map(|s| Value::String(s.clone()))
                        .unwrap_or(Value::Null)
                },
                "x": 0,
                "y": 0,
            })
        })
        .collect();
    // base variables 와 id 기준 union
    let base_vars = project
        .get("variables")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut merged_vars: Vec<Value> = base_vars;
    for v in &vars_arr {
        let new_id = v.get("id").and_then(|x| x.as_str());
        if let Some(new_id) = new_id {
            if let Some(existing) = merged_vars.iter_mut().find(|e| {
                e.get("id").and_then(|x| x.as_str()) == Some(new_id)
            }) {
                *existing = v.clone();
                continue;
            }
        }
        merged_vars.push(v.clone());
    }
    project["variables"] = json!(merged_vars);

    // 3. project.scripts 를 base 값으로 복원 (object.script 가 진짜 위치)
    if let Some(base_scripts) = base.get("scripts") {
        project["scripts"] = base_scripts.clone();
    } else {
        project.as_object_mut().map(|m| m.remove("scripts"));
    }

    // 4. object 매핑: stem == object.name (대소문자 무시)
    //    매칭된 object 는 패치, 매칭 안 된 rs 는 unmatched 에 남긴다.
    let mut unmatched: Vec<(String, Vec<Vec<Value>>)> = Vec::new();
    if let Some(objects) = project
        .get_mut("objects")
        .and_then(|v| v.as_array_mut())
    {
        for (stem, tah) in &per_source {
            let found = objects.iter_mut().any(|o| {
                let eq = o
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.eq_ignore_ascii_case(stem))
                    .unwrap_or(false);
                if eq {
                    // Entry 의 object.script 는 트리거 묶음(thread) 배열의 배열.
                    // 각 thread 가 Value::Array(Vec<Value>) 이고 전체는 그 Vec.
                    o["script"] = threads_to_value(&tah.threads);
                    true
                } else {
                    false
                }
            });
            if !found {
                unmatched.push((stem.clone(), tah.threads.clone()));
            }
        }
    } else {
        // objects 필드가 없는 비정상 base 라도 매칭 안 된 rs 들을 unmatched 로
        for (stem, tah) in &per_source {
            unmatched.push((stem.clone(), tah.threads.clone()));
        }
    }

    // 5. 매칭 안 된 rs 처리: objects 가 비어있으면 교체, 아니면 append
    if !unmatched.is_empty() {
        let objects_empty = project
            .get("objects")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);

        if objects_empty {
            // 첫 unmatched 로 단일 가짜 object 생성
            let (name, scripts) = unmatched.remove(0);
            let fake = make_fake_object(&name, &scripts, base, &[], options);
            project["objects"] = json!([fake]);
            // 나머지 unmatched 도 동일하게 가짜 object 로 append
            // arr 가변 대여 중에는 project 를 불변 대여할 수 없으므로 taken ids 를
            // arr 의 스냅샷에서 매번 갱신한다.
            if !unmatched.is_empty() {
                for (n, s) in unmatched {
                    let taken: Vec<String> = project
                        .get("objects")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|o| o.get("id").and_then(|x| x.as_str()).map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    let fake = make_fake_object(&n, &s, base, &taken, options);
                    project["objects"].as_array_mut().unwrap().push(fake);
                }
            }
        } else {
            // 기존 objects 에 append
            for (n, s) in unmatched {
                let taken: Vec<String> = project
                    .get("objects")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|o| o.get("id").and_then(|x| x.as_str()).map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let fake = make_fake_object(&n, &s, base, &taken, options);
                project["objects"].as_array_mut().unwrap().push(fake);
            }
        }
    }

    // 6. project.functions 에 helper 함수들 emit
    if !all_helpers.is_empty() {
        let base_funcs = project
            .get("functions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut merged_funcs: Vec<Value> = base_funcs;
        for h in &all_helpers {
            // id 충돌 회피
            let id = fresh_function_id(&merged_funcs, &h.name);
            let params: Vec<Value> = h
                .params
                .iter()
                .map(|p| json!({ "name": p }))
                .collect();
            merged_funcs.push(json!({
                "id": id,
                "name": h.name,
                "content": [{
                    "blocks": h.body
                }],
                "param": params,
            }));
        }
        project["functions"] = json!(merged_funcs);
    }

    // 7. project.messages 에 when_message 트리거의 메시지 이름 emit
    //    EntryJS 의 broadcast_message_to_all trigger 는 message "이름" 으로
    //    매칭한다. id 도 name 과 동일하게 두면 EntryJS 가 양쪽으로 일관 매칭.
    //    중복 이름은 스킵 (id 충돌 회피).
    if !all_messages.is_empty() {
        let base_msgs = project
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut merged_msgs: Vec<Value> = base_msgs;
        for name in &all_messages {
            if merged_msgs.iter().any(|m| m["name"] == *name) {
                continue;
            }
            merged_msgs.push(json!({
                "id": name,
                "name": name,
            }));
        }
        project["messages"] = json!(merged_msgs);
    }

    Ok((project, unmapped))
}

/// project.functions 의 기존 항목과 충돌하지 않는 새 function id 발급.
/// `fn_<djb2(name)>` 시도 -> 충돌 시 `_2`, `_3`, ... suffix.
/// `Program` 에 등장한 모든 변수 이름 (등장 순서, unique).
fn collect_var_names(program: &Program) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    codegen::collect_vars_program(program, &mut names);
    names
}

/// unmapped 메시지 누적 헬퍼. 중복 메시지는 한 번만 (stderr 노이즈 방지).
/// 메시지 포맷이 정확히 일치해야 dedup. 동일 미매핑이 여러 stmt 에서 나올 때
/// 한 줄로 합쳐짐.
fn push_unmapped(unmapped: &mut Vec<String>, msg: String) {
    if !unmapped.iter().any(|m| m == &msg) {
        unmapped.push(msg);
    }
}

fn fresh_function_id(existing: &[Value], name: &str) -> String {
    let taken: Vec<String> = existing
        .iter()
        .filter_map(|f| f.get("id").and_then(|x| x.as_str()).map(String::from))
        .collect();
    let base = format!("fn_{}", crate::block::id_for(name));
    if !taken.iter().any(|x| x == &base) {
        return base;
    }
    for i in 2.. {
        let candidate = format!("{base}_{i}");
        if !taken.iter().any(|x| x == &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// 매칭 안 된 rs 이름으로 가짜 sprite object 를 만든다.
/// base 에 기존 스프라이트가 있으면 entity 메타는 복사해 위치 등이
/// 0 으로 초기화되지 않도록 한다. 단 다음 필드는 새로 발급 / 비운다:
/// - `id`: stem 기반 hash 로 stable id (라운드트립 시 동일 stem 이면 동일 id).
///   충돌 시 `_2`, `_3` suffix.
/// - `selectedPictureId`: None (가짜 object 가 base picture 를 가리키면 scene 안에서
///   picture 가 두 object 에 공유되어 부작용 발생. Entry 사용자가 수동 지정).
/// - `sprite.pictures` / `sounds`: 빈 배열 (asset 은 사용자 수동 추가).
/// - `objectType`: base 의 첫 object 의 objectType 복사 (text 등 non-sprite 도 지원).
/// - `scene`: options.default_scene 우선, 없으면 base 첫 sprite 의 scene, 없으면 "scene1".
fn make_fake_object(
    name: &str,
    scripts: &[Vec<Value>],
    base: &Value,
    taken_ids: &[String],
    options: &CompileOptions,
) -> Value {
    // base 에서 복사할 기존 object (objectType == "sprite" 우선, 그 외 첫 object)
    let base_object = base
        .get("objects")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|o| o.get("objectType").and_then(|x| x.as_str()) == Some("sprite"))
                .or_else(|| arr.first())
        });

    // entity 는 base 가 있으면 통째로 복사, 없으면 기본값
    let entity = base_object
        .and_then(|o| o.get("entity").cloned())
        .unwrap_or_else(|| {
            json!({
                "rotation": 0.0,
                "direction": 90.0,
                "x": 0.0,
                "y": 0.0,
                "regX": 0.0,
                "regY": 0.0,
                "scaleX": 1.0,
                "scaleY": 1.0,
                "width": 0.0,
                "height": 0.0,
                "visible": true
            })
        });

    // sprite 메타는 base 에서 가져오되 pictures / sounds 는 비운다 (asset 공유 방지).
    // base 에 sprite 메타가 없으면 새로 만든다.
    let sprite_meta = base_object
        .and_then(|o| o.get("sprite"))
        .map(|s| {
            json!({
                "name": s.get("name").cloned().unwrap_or_else(|| json!(name)),
                "pictures": json!([]),
                "sounds": json!([]),
            })
        })
        .unwrap_or_else(|| {
            json!({
                "name": name,
                "pictures": [],
                "sounds": []
            })
        });

    // objectType: base 첫 object 의 objectType, 없으면 "sprite"
    let object_type = base_object
        .and_then(|o| o.get("objectType").and_then(|x| x.as_str()).map(String::from))
        .unwrap_or_else(|| "sprite".to_string());

    // scene: options.default_scene 우선, 없으면 base 첫 sprite 의 scene, 없으면 "scene1"
    let scene = options
        .default_scene
        .clone()
        .or_else(|| {
            base_object
                .and_then(|o| o.get("scene").and_then(|x| x.as_str()).map(String::from))
        })
        .unwrap_or_else(|| "scene1".to_string());

    // id: stem 기반 stable hash + collision 시 suffix
    let id = stable_object_id(base, taken_ids, name);

    json!({
        "id": id,
        "name": name,
        "script": threads_to_value(scripts),
        "objectType": object_type,
        "scene": scene,
        "selectedPictureId": Value::Null,
        "sprite": sprite_meta,
        "entity": entity,
        // Entry 실제 .ent 형식 (sample 기준) 에 등장하는 부수 필드:
        // - rotateMethod: 회전 방식 ("free" / "vertical" / ...)
        // - lock: 편집 잠금
        // 비어있으면 EntryJS 가 default 적용하나 명시해두는 편이 안전.
        "rotateMethod": "free",
        "lock": false,
    })
}

/// stem 기반 stable object id.
/// `obj_<djb2(stem)>` 시도 -> base + taken_ids 와 충돌 시 `_2`, `_3`, ... suffix.
fn stable_object_id(base: &Value, taken_ids: &[String], stem: &str) -> String {
    let mut taken: Vec<String> = taken_ids.to_vec();
    if let Some(arr) = base.get("objects").and_then(|v| v.as_array()) {
        for o in arr {
            if let Some(id) = o.get("id").and_then(|x| x.as_str()) {
                taken.push(id.to_string());
            }
        }
    }
    let base_id = format!("obj_{}", crate::block::id_for(stem));
    if !taken.iter().any(|x| x == &base_id) {
        return base_id;
    }
    for i in 2.. {
        let candidate = format!("{base_id}_{i}");
        if !taken.iter().any(|x| x == &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// thread 배열 (`Vec<Vec<Value>>`) 을 object.script 값으로 변환.
/// Entry 실제 .ent 에서는 `object.script` 가 JSON 문자열로 저장된다 (예:
/// `"script":"[[{...},...]]"`). 우리 build 도 같은 형식으로 emit 한다.
/// (EntryJS 가 양쪽 다 읽지만, sample 확인 결과 String 이 표준.)
fn threads_to_value(threads: &[Vec<Value>]) -> Value {
    let arr = Value::Array(threads.iter().map(|t| Value::Array(t.clone())).collect());
    // `to_string` 이 항상 유효한 JSON 을 내므로 별도 직렬화 실패 가능성 없음.
    Value::String(arr.to_string())
}

/// build_threads 결과.
/// - `threads`: object.script 의 trigger 묶음 배열 (각 thread = `[when_*, ...body]`)
/// - `helpers`: project.functions 로 옮길 helper 함수 정의들
/// - `messages`: when_message 트리거의 메시지 이름들 (project.messages 에 emit)
struct ThreadsAndHelpers {
    threads: Vec<Vec<Value>>,
    helpers: Vec<FunctionDef>,
    messages: Vec<String>,
}

/// project.functions 1개 항목.
#[derive(Clone)]
struct FunctionDef {
    name: String,
    params: Vec<String>,
    /// 함수의 본문 블록들 (1개 thread = linear).
    body: Vec<Value>,
}

/// 한 rs 소스의 트리거 + 본문 stmts 를 Entry object.script 의 thread 배열로 변환.
/// 각 thread = `[when_*, ...body_blocks]`.
///
/// trigger 가 하나도 없고 `program.stmts` 에도 일반 stmt 가 없으면 thread 가 0 개.
/// (EntryJS 가 빈 object.script 를 받아도 무해하므로 그대로 둔다.)
///
/// `Stmt::FuncDef` (트리거 아닌 함수) 는 helpers 로 분리해 project.functions 에
/// emit. Entry object script 안에 function_create 블록을 그대로 두면 EntryJS 가
/// 무시하거나 오동작할 수 있어 project-level function pool 로 옮긴다.
///
/// `from_stmt`/`to_value` 가 `UnmappedBlock` 을 반환하면 thread 에서 빼고
/// `unmapped` 에 메시지를 누적한다. 그 외 에러(parse/semantic/codegen)는 propagate.
fn build_threads(
    triggers: &[parse::TriggerDef],
    program: &Program,
    unmapped: &mut Vec<String>,
) -> Result<ThreadsAndHelpers> {
    let mut threads: Vec<Vec<Value>> = Vec::new();
    let mut messages: Vec<String> = Vec::new();
    for t in triggers {
        let trigger_block = match trigger_block_for(&t.name, &t.params) {
            Ok(b) => b,
            Err(Error::UnmappedBlock(m)) => {
                push_unmapped(unmapped, m);
                continue;
            }
            Err(e) => return Err(e),
        };
        // when_message 트리거면 메시지 이름 수집 (project.messages 등록용)
        if matches!(trigger_block, crate::block::Block::WhenMessageRecv { .. }) {
            if let Some(name) = t.params.first() {
                messages.push(name.clone());
            }
        }
        let mut thread = Vec::new();
        match crate::block::to_value(&trigger_block) {
            Ok(v) => thread.push(v),
            Err(Error::UnmappedBlock(m)) => unmapped.push(m),
            Err(e) => return Err(e),
        }
        for s in &t.body {
            let b = match crate::block::from_stmt(s) {
                Ok(b) => b,
                Err(Error::UnmappedBlock(m)) => {
                    push_unmapped(unmapped, m);
                    continue;
                }
                Err(e) => return Err(e),
            };
            match crate::block::to_value(&b) {
                Ok(v) => thread.push(v),
                Err(Error::UnmappedBlock(m)) => unmapped.push(m),
                Err(e) => return Err(e),
            }
        }
        threads.push(thread);
    }
    // 트리거 외 top-level stmt 분리:
    // - Stmt::FuncDef → helpers (project.functions)
    // - 그 외 (VarDecl/SetVar/If/While/Repeat/For/Return/Expr) → 단일 init thread
    let mut helpers: Vec<FunctionDef> = Vec::new();
    let mut init_stmts: Vec<&Stmt> = Vec::new();
    for s in &program.stmts {
        if let Stmt::FuncDef { name, params, body } = s {
            let mut body_blocks: Vec<Value> = Vec::new();
            for bs in body {
                let b = match crate::block::from_stmt(bs) {
                    Ok(b) => b,
                    Err(Error::UnmappedBlock(m)) => {
                        push_unmapped(unmapped, m);
                        continue;
                    }
                    Err(e) => return Err(e),
                };
                match crate::block::to_value(&b) {
                    Ok(v) => body_blocks.push(v),
                    Err(Error::UnmappedBlock(m)) => unmapped.push(m),
                    Err(e) => return Err(e),
                }
            }
            if !body_blocks.is_empty() {
                helpers.push(FunctionDef {
                    name: name.clone(),
                    params: params.clone(),
                    body: body_blocks,
                });
            }
        } else {
            init_stmts.push(s);
        }
    }
    if !init_stmts.is_empty() {
        let mut init_thread: Vec<Value> = Vec::with_capacity(init_stmts.len());
        for s in &init_stmts {
            let b = match crate::block::from_stmt(s) {
                Ok(b) => b,
                Err(Error::UnmappedBlock(m)) => {
                    push_unmapped(unmapped, m);
                    continue;
                }
                Err(e) => return Err(e),
            };
            match crate::block::to_value(&b) {
                Ok(v) => init_thread.push(v),
                Err(Error::UnmappedBlock(m)) => unmapped.push(m),
                Err(e) => return Err(e),
            }
        }
        if !init_thread.is_empty() {
            threads.push(init_thread);
        }
    }
    Ok(ThreadsAndHelpers {
        threads,
        helpers,
        messages,
    })
}

/// 트리거 함수 이름을 Entry Block 으로 매핑.
fn trigger_block_for(
    name: &str,
    params: &[String],
) -> Result<crate::block::Block> {
    use crate::block::Block;
    let b = match name {
        "when_start" | "when_run" => Block::WhenStart,
        "when_click" | "when_object_click" => Block::WhenClick,
        "when_clone_start" => Block::WhenCloneStart,
        "when_message" | "when_message_cast" => {
            let msg = params.first().cloned().unwrap_or_default();
            Block::WhenMessageRecv { msg }
        }
        // 알 수 없는 when_* 는 when_start 로 fallback (EntryJS 가 무시하더라도
        // 잘못된 트리거로 시작되지 않게).
        _ if name.starts_with("when_") => Block::WhenStart,
        _ => return Err(crate::Error::UnmappedBlock(format!("non-trigger: {name}"))),
    };
    Ok(b)
}
