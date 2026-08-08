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

use crate::ir::Program;

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
pub fn compile(rs_sources: &[(&str, &str)], base: &Value) -> Result<(Value, Vec<String>)> {
    let mut unmapped: Vec<String> = Vec::new();
    // 1. 각 rs 를 두 가지로 파싱한다:
    //    - `parse::parse` (트리거 body 평탄화 포함) -> variables 집계용 Program
    //    - `parse::parse_with_triggers` (트리거 분리) -> object.script thread 구성용
    let mut per_source: Vec<(String, Vec<Vec<Value>>)> = Vec::with_capacity(rs_sources.len());
    let mut merged_stmts: Vec<crate::ir::Stmt> = Vec::new();
    for (name, src) in rs_sources {
        // variables 집계: 트리거 body 포함 모든 stmt 평탄화
        let flat_program = parse::parse(src)?;
        merged_stmts.extend(flat_program.stmts.clone());
        // object.script thread: 트리거별 분리
        let (non_trigger_program, triggers) = parse::parse_with_triggers(src)?;
        let threads = build_threads(&triggers, &non_trigger_program, &mut unmapped)?;
        per_source.push((name.to_string(), threads));
    }
    let merged_program = Program { stmts: merged_stmts };

    // 2. variables 패치 (codegen::generate 를 통째로 부르지 않고 variables 만 직접 빌드)
    //    이유: generate 는 from_stmt 을 scripts 생성용으로 호출하는데, 이 경로의
    //    UnmappedBlock 을 우리 (Vec<String>) 에 누적할 수 없기 때문이다.
    //    scripts 는 build_threads 가 이미 object 별로 thread 배열을 만들어 두었고
    //    project.scripts 는 base 값으로 복원한다.
    let mut project = base.clone();
    let vars_map = codegen::collect_var_map(&merged_program);
    let vars_arr: Vec<Value> = vars_map
        .iter()
        .map(|v| {
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
                }
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
        for (stem, scripts) in &per_source {
            let found = objects.iter_mut().any(|o| {
                let eq = o
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.eq_ignore_ascii_case(stem))
                    .unwrap_or(false);
                if eq {
                    // Entry 의 object.script 는 트리거 묶음(thread) 배열의 배열.
                    // 각 thread 가 Value::Array(Vec<Value>) 이고 전체는 그 Vec.
                    o["script"] = threads_to_value(scripts);
                    true
                } else {
                    false
                }
            });
            if !found {
                unmatched.push((stem.clone(), scripts.clone()));
            }
        }
    } else {
        // objects 필드가 없는 비정상 base 라도 매칭 안 된 rs 들을 unmatched 로
        unmatched.extend(per_source.iter().cloned());
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
            let fake = make_fake_object(&name, &scripts, base, &[]);
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
                    let fake = make_fake_object(&n, &s, base, &taken);
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
                let fake = make_fake_object(&n, &s, base, &taken);
                project["objects"].as_array_mut().unwrap().push(fake);
            }
        }
    }

    Ok((project, unmapped))
}

/// 매칭 안 된 rs 이름으로 가짜 sprite object 를 만든다.
/// base 에 기존 스프라이트가 있으면 entity/sprite/scene 메타를 복사해 위치 등이
/// 0 으로 초기화되지 않도록 한다. 단 `id` 는 base + `taken_ids` 와 충돌하지 않게
/// 새로 발급한다 (그대로 복사하면 Entry 에서 중복 id 오류 발생).
/// `taken_ids` 는 호출 시점의 project objects id 목록 (매 push 마다 갱신됨).
fn make_fake_object(
    name: &str,
    scripts: &[Vec<Value>],
    base: &Value,
    taken_ids: &[String],
) -> Value {
    // base 에서 복사할 기존 스프라이트 골라내기 (objectType == "sprite" 우선)
    let base_sprite = base
        .get("objects")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|o| o.get("objectType").and_then(|x| x.as_str()) == Some("sprite"))
                .or_else(|| arr.first())
        });

    // entity 는 base 가 있으면 통째로 복사, 없으면 기본값
    let entity = base_sprite
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

    // sprite 메타(name/pictures/sounds) 복사, 없으면 빈 메타
    let sprite_meta = base_sprite
        .and_then(|o| o.get("sprite").cloned())
        .unwrap_or_else(|| {
            json!({
                "name": name,
                "pictures": [],
                "sounds": []
            })
        });

    // scene 은 base 스프라이트의 것을 그대로 (가짜 오브젝트는 동일 scene 에 둔다)
    let scene = base_sprite
        .and_then(|o| o.get("scene").and_then(|x| x.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "scene1".to_string());

    // selectedPictureId 는 pictures 메타에 실제 id 가 있으면 그걸로, 없으면 Null
    let selected_picture = sprite_meta
        .get("pictures")
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.first())
        .and_then(|p| p.get("id").cloned())
        .unwrap_or(Value::Null);

    // id 는 base + taken_ids 와 충돌하지 않게 새로 발급
    let id = fresh_object_id(base, taken_ids, "fake");

    json!({
        "id": id,
        "name": name,
        "script": threads_to_value(scripts),
        "objectType": "sprite",
        "scene": scene,
        "selectedPictureId": selected_picture,
        "sprite": sprite_meta,
        "entity": entity
    })
}

/// base + taken_ids 의 기존 id 와 충돌하지 않는 새 id 발급.
/// `prefix_N` (N=1..) 형태, prefix 가 겹치면 N 을 증가시켜 회피.
fn fresh_object_id(base: &Value, taken_ids: &[String], prefix: &str) -> String {
    let mut taken: Vec<String> = taken_ids.to_vec();
    if let Some(arr) = base.get("objects").and_then(|v| v.as_array()) {
        for o in arr {
            if let Some(id) = o.get("id").and_then(|x| x.as_str()) {
                taken.push(id.to_string());
            }
        }
    }
    for i in 1.. {
        let candidate = format!("{prefix}_{i}");
        if !taken.iter().any(|x| x == &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// thread 배열 (`Vec<Vec<Value>>`) 을 object.script 값으로 변환.
fn threads_to_value(threads: &[Vec<Value>]) -> Value {
    Value::Array(threads.iter().map(|t| Value::Array(t.clone())).collect())
}

/// 한 rs 소스의 트리거 + 본문 stmts 를 Entry object.script 의 thread 배열로 변환.
/// 각 thread = `[when_*, ...body_blocks]`.
///
/// trigger 가 하나도 없고 `program.stmts` 에도 일반 stmt 가 없으면 thread 가 0 개.
/// (EntryJS 가 빈 object.script 를 받아도 무해하므로 그대로 둔다.)
///
/// `from_stmt`/`to_value` 가 `UnmappedBlock` 을 반환하면 thread 에서 빼고
/// `unmapped` 에 메시지를 누적한다. 그 외 에러(parse/semantic/codegen)는 propagate.
fn build_threads(
    triggers: &[parse::TriggerDef],
    program: &Program,
    unmapped: &mut Vec<String>,
) -> Result<Vec<Vec<Value>>> {
    let mut threads: Vec<Vec<Value>> = Vec::new();
    for t in triggers {
        let trigger_block = match trigger_block_for(&t.name, &t.params) {
            Ok(b) => b,
            Err(Error::UnmappedBlock(m)) => {
                unmapped.push(m);
                continue;
            }
            Err(e) => return Err(e),
        };
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
                    unmapped.push(m);
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
    // 트리거 외 top-level stmt (helper FuncDef 의 body 평탄화는 parse 단계에서
    // 이미 처리됨. 여기 남은 건 VarDecl/SetVar/If/While/Repeat/For/Return 등)
    // → 단일 thread 에 모아 둔다. 트리거가 있는 경우엔 부수적인 initialization
    // 코드로 보고 같이 묶지 않고 별도 thread 로 두는 편이 EntryJS 가 더 자연스럽게
    // import 한다.
    if !program.stmts.is_empty() {
        let mut init_thread: Vec<Value> = Vec::with_capacity(program.stmts.len());
        for s in &program.stmts {
            let b = match crate::block::from_stmt(s) {
                Ok(b) => b,
                Err(Error::UnmappedBlock(m)) => {
                    unmapped.push(m);
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
    Ok(threads)
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
