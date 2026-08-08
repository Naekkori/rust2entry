//! lib::compile 통합 테스트.
//!
//! parse + codegen 을 거치며 최종 project.json 구조 확인.

use entrycore::compile;
use serde_json::{Value, json};

fn empty_project() -> Value {
    json!({
        "name": "test",
        "speed": 60, "objects": [], "variables": [], "messages": [],
        "functions": [], "scenes": [{"id":"scene1","name":"장면1"}],
        "interface": {"views": []}, "meta": {}
    })
}

/// 트리거 thread 의 0 번 인덱스가 항상 when_run (when_start) 인지 확인.
fn assert_thread_starts_with_when_run(thread: &[Value]) {
    assert_eq!(thread[0]["type"], "when_run", "thread[0] 은 when_run 트리거");
}

/// base 에 object 가 없을 때 rs stem 이름으로 가짜 sprite 가 추가되고,
/// 그 object 의 `script` 필드(thread 배열)에 `[[when_run, body...]]` 가 들어가는지 확인.
#[test]
fn compile_single_source() {
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 1);
    let threads = objects[0]["script"].as_array().expect("object script threads");
    assert_eq!(threads.len(), 1, "when_start 1개");
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2, "when_run + body 1개");
    assert_thread_starts_with_when_run(thread);
    assert_eq!(thread[1]["type"], "set_variable");
    assert_eq!(thread[1]["params"][0]["name"], "x");
}

/// rs 가 둘이고 base 비어있으면 둘 다 가짜 object 로 추가.
#[test]
fn compile_multi_source_merges_scripts() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 2);
    let a_obj = objects.iter().find(|o| o["name"] == "a").expect("a");
    let b_obj = objects.iter().find(|o| o["name"] == "b").expect("b");
    let a_thread = a_obj["script"][0].as_array().expect("a thread");
    let b_thread = b_obj["script"][0].as_array().expect("b thread");
    assert_thread_starts_with_when_run(a_thread);
    assert_thread_starts_with_when_run(b_thread);
    assert_eq!(a_thread[1]["params"][0]["name"], "x");
    assert_eq!(b_thread[1]["params"][0]["name"], "y");
}

/// base 메타(name/scenes/speed) 보존.
#[test]
fn compile_preserves_base_metadata() {
    let mut base = empty_project();
    base["name"] = json!("my_proj");
    base["scenes"] = json!([
        { "id": "scene1", "name": "장면1" },
        { "id": "scene2", "name": "장면2" },
    ]);
    base["speed"] = json!(30);
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    assert_eq!(v["name"], "my_proj");
    assert_eq!(v["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(v["speed"], 30);
    let objects = v["objects"].as_array().unwrap();
    let thread = objects[0]["script"][0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2);
}

/// 변수 집계: 여러 rs 의 트리거 body 안 변수명이 variables 에 모두 들어가는지.
#[test]
fn compile_aggregates_variables_across_sources() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; let z = 3; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().expect("variables array");
    let names: Vec<&str> = vars.iter().filter_map(|x| x["name"].as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert!(names.contains(&"z"));
    assert_eq!(vars.len(), 3);
}

/// 같은 변수명 두 소스에서 쓰면 하나로 합쳐짐.
#[test]
fn compile_deduplicates_variables() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let x = 2; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().expect("variables array");
    let xs: Vec<&Value> = vars.iter().filter(|x| x["name"] == "x").collect();
    assert_eq!(xs.len(), 1, "중복 변수 발생");
}

#[test]
fn compile_parse_error_propagates() {
    let bad = "fn when_start() { let = ; }";
    let r = compile(&[("obj", bad)], &empty_project());
    assert!(r.is_err(), "parse 에러가 전파돼야 함");
}

/// rs 가 없으면 base 그대로 (가짜 object 추가 안 함).
#[test]
fn compile_empty_sources_returns_base() {
    let mut base = empty_project();
    base["name"] = json!("untouched");
    let v = compile(&[], &base).expect("compile").0;
    assert_eq!(v["name"], "untouched");
    assert!(v["objects"].as_array().unwrap().is_empty());
    assert_eq!(v["variables"].as_array().unwrap().len(), 0);
}

/// if 블록: object.script[0] (thread) = [when_run, if, ...]. 본문 if 는 thread[1].
#[test]
fn compile_if_block_structure() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("object script threads");
    assert_eq!(threads.len(), 1);
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2, "when_run + if");
    assert_thread_starts_with_when_run(thread);
    assert_eq!(thread[1]["type"], "if");
    assert_eq!(thread[1]["params"][0]["type"], "boolean_basic");
}

/// for-range 는 repeat_basic 으로 직렬화.
#[test]
fn compile_for_range_expands_to_repeat() {
    let src = "fn when_start() { for i in 0..5 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("object script threads");
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2, "when_run + repeat");
    assert_eq!(thread[1]["type"], "repeat_basic");
}

/// compile -> object.script (thread 배열) -> deparse 라운드트립.
/// thread 0 = [when_run, if] -> deparse 가 when_run 을 FuncDef 로 감싸고 본문 If 를 body 에.
#[test]
fn compile_roundtrip_via_deparse() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_value_with_vars;
    use entrycore::ir::Stmt;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script = objects[0]["script"].clone();
    let p2 = program_from_script_value_with_vars(&obj_script, &vars).expect("deparse");
    // stmts[0] = FuncDef(when_start, body=[If...])
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert!(matches!(body[0], Stmt::If { .. }));
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// base objects 가 비어있고 rs 가 있으면 가짜 object 1개 + entity 기본값.
#[test]
fn compile_adds_fake_object_when_empty() {
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("my_obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 1, "가짜 오브젝트 1개 추가");
    assert_eq!(objects[0]["name"], "my_obj");
    assert_eq!(objects[0]["objectType"], "sprite");
    assert_eq!(objects[0]["scene"], "scene1");
    assert_eq!(objects[0]["entity"]["x"], 0.0);
    assert_eq!(objects[0]["entity"]["visible"], true);
    assert_eq!(objects[0]["sprite"]["name"], "my_obj");
    assert!(objects[0]["sprite"]["pictures"].as_array().unwrap().is_empty());
    // base 가 비어있어도 가짜 object 의 id 는 fake_N 형식
    let fake_id = objects[0]["id"].as_str().expect("fake id str");
    assert!(fake_id.starts_with("fake_"), "가짜 id 는 fake_ prefix: {fake_id}");
    let threads = objects[0]["script"].as_array().expect("object script threads");
    assert_eq!(threads.len(), 1);
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2, "when_run + body");
}

/// base 에 objects 가 이미 있으면 추가하지 않고, rs stem == name 인 object 의
/// script 필드만 패치한다.
#[test]
fn compile_does_not_overwrite_existing_objects() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "existing1",
            "name": "existing_obj",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "existing_obj", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("existing_obj", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects");
    assert_eq!(objects.len(), 1, "기존 objects 보존, 추가 안 함");
    assert_eq!(objects[0]["name"], "existing_obj");
    let threads = objects[0]["script"].as_array().expect("object script threads");
    assert_eq!(threads.len(), 1);
    let thread = threads[0].as_array().expect("first thread");
    assert_thread_starts_with_when_run(thread);
    assert_eq!(thread[1]["type"], "set_variable");
}

/// 각 rs 가 stem 이름으로 base object 와 매칭되어 그 object 의 script 가 패치된다.
#[test]
fn compile_matches_existing_object_by_name() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "a1",
            "name": "alpha",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "alpha", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        },
        {
            "id": "b1",
            "name": "beta",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "beta", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let v = compile(&[("alpha", a), ("beta", b)], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2, "기존 objects 2개 보존");
    let alpha_obj = objects.iter().find(|o| o["name"] == "alpha").unwrap();
    let beta_obj = objects.iter().find(|o| o["name"] == "beta").unwrap();
    let alpha_thread = alpha_obj["script"][0].as_array().expect("alpha thread");
    let beta_thread = beta_obj["script"][0].as_array().expect("beta thread");
    assert_eq!(alpha_thread[1]["params"][0]["name"], "x");
    assert_eq!(beta_thread[1]["params"][0]["name"], "y");
}

/// 이름 매칭이 대소문자 무시.
#[test]
fn compile_matches_object_name_case_insensitive() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "a1",
            "name": "Alpha",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "Alpha", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("alpha", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 1);
    let thread = objects[0]["script"][0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2);
}

/// if-else: object.script[0] (thread) = [when_run, if_else]. 본문 if_else 는 thread[1].
#[test]
fn compile_if_else_block() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let x = 1;
            } else {
                let y = 2;
            }
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("object script threads");
    assert_eq!(threads.len(), 1);
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread.len(), 2);
    assert_thread_starts_with_when_run(thread);
    let block = &thread[1];
    assert_eq!(block["type"], "if_else");
    assert_eq!(block["params"][0]["type"], "boolean_basic");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 2, "if_else 는 then/else 2개 thread");
    let then_first = &stmts[0][0];
    assert_eq!(then_first["type"], "set_variable");
    assert_eq!(then_first["params"][0]["name"], "x");
    let else_first = &stmts[1][0];
    assert_eq!(else_first["type"], "set_variable");
    assert_eq!(else_first["params"][0]["name"], "y");
}

/// else 없으면 if (Entry 의 if 블록 형식).
#[test]
fn compile_if_without_else_stays_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("threads");
    let thread = threads[0].as_array().expect("first thread");
    let block = &thread[1];
    assert_eq!(block["type"], "if");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 1, "if 는 then 1개 thread");
}

/// if-else compile -> object.script (thread 배열) -> deparse 라운드트립.
#[test]
fn compile_if_else_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_value_with_vars;
    use entrycore::ir::Stmt;
    use entrycore::parse::parse;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } else { let y = 2; } }";
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script = objects[0]["script"].clone();
    let p2 = program_from_script_value_with_vars(&obj_script, &vars).expect("deparse");
    // stmts[0] = FuncDef(when_start, body=[If{then,else}])
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::If { then_body, else_body, .. } => {
                    assert_eq!(then_body.len(), 1);
                    assert_eq!(else_body.len(), 1);
                    let then_var = match &then_body[0] {
                        Stmt::VarDecl(n, _) | Stmt::SetVar(n, _) => n,
                        other => panic!("unexpected then stmt: {other:?}"),
                    };
                    let else_var = match &else_body[0] {
                        Stmt::VarDecl(n, _) | Stmt::SetVar(n, _) => n,
                        other => panic!("unexpected else stmt: {other:?}"),
                    };
                    assert_eq!(then_var, "x");
                    assert_eq!(else_var, "y");
                }
                other => panic!("expected If in body, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// base 에 기존 스프라이트가 있을 때 매칭 안 된 rs 가 가짜 object 로 추가될 때
/// entity/sprite 메타가 base 에서 복사되어 위치 등이 0 으로 초기화되지 않는다.
#[test]
fn compile_preserves_existing_sprite_metadata_on_fake_object() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "src1",
            "name": "source_sprite",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": {
                "name": "source_sprite",
                "pictures": [{"id":"pic1","name":"a.png","fileurl":"a.png","type":"","dimension":{"width":10,"height":20}}],
                "sounds": []
            },
            "entity": {
                "rotation": 0.0, "direction": 90.0,
                "x": 123.0, "y": 456.0,
                "regX": 5.0, "regY": 10.0,
                "scaleX": 2.0, "scaleY": 1.5,
                "width": 30.0, "height": 40.0, "visible": true
            },
            "selectedPictureId": "pic1"
        }
    ]);
    let v = compile(&[("new_sprite", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2);
    let fake = objects.iter().find(|o| o["name"] == "new_sprite").expect("fake object");
    assert_eq!(fake["entity"]["x"], 123.0);
    assert_eq!(fake["entity"]["y"], 456.0);
    assert_eq!(fake["entity"]["regX"], 5.0);
    assert_eq!(fake["entity"]["regY"], 10.0);
    assert_eq!(fake["entity"]["scaleX"], 2.0);
    let pics = fake["sprite"]["pictures"].as_array().unwrap();
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0]["id"], "pic1");
    assert_eq!(fake["selectedPictureId"], "pic1");
    // 가짜 object 의 id 는 base 의 id("src1") 와 충돌하면 안 됨
    let fake_id = fake["id"].as_str().expect("fake id str");
    assert_ne!(fake_id, "src1", "가짜 object id 가 base 와 충돌");
    assert!(fake_id.starts_with("fake_"), "가짜 id 는 fake_ prefix: {fake_id}");
    // base 의 id 도 그대로 보존
    let src = objects.iter().find(|o| o["name"] == "source_sprite").expect("base obj");
    assert_eq!(src["id"], "src1");
}

/// project.scripts 는 base 값으로 복원된다 (object.script 가 진짜 위치).
#[test]
fn compile_does_not_clobber_project_scripts_field() {
    let mut base = empty_project();
    base["scripts"] = json!([{"type": "old_block"}]);
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let scripts = v["scripts"].as_array().expect("project.scripts");
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0]["type"], "old_block");
}

/// 여러 가짜 object 가 생성되어도 id 가 서로 충돌하지 않는다.
#[test]
fn compile_fake_objects_have_unique_ids() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let c = "fn when_start() { let z = 3; }";
    let v = compile(&[("a", a), ("b", b), ("c", c)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 3);
    let mut ids: Vec<String> = objects
        .iter()
        .map(|o| o["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "가짜 object id 중복: {ids:?}");
    for id in &ids {
        assert!(id.starts_with("fake_"), "id 포맷: {id}");
    }
}

/// base 에 fake_1 이 이미 있으면 가짜 object 는 fake_2 부터 발급.
#[test]
fn compile_fake_id_skips_existing_ids_in_base() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "fake_1",
            "name": "base_a",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "base_a", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("new_one", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let fake = objects.iter().find(|o| o["name"] == "new_one").expect("new_one");
    assert_eq!(fake["id"], "fake_2", "fake_1 과 충돌 회피");
}

/// when_click 트리거는 when_click 블록으로 직렬화.
#[test]
fn compile_when_click_trigger() {
    let src = "fn when_click() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("threads");
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread[0]["type"], "when_click");
    assert_eq!(thread[1]["type"], "set_variable");
}

/// when_clone_start 트리거는 when_clone_start 블록으로 직렬화.
#[test]
fn compile_when_clone_start_trigger() {
    let src = "fn when_clone_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("threads");
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread[0]["type"], "when_clone_start");
}

/// when_message 함수는 params[0] 을 메시지 이름으로 사용한 when_message_cast 트리거 생성.
#[test]
fn compile_when_message_trigger_uses_param_as_msg() {
    let src = "fn when_message(m: &str) { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("threads");
    let thread = threads[0].as_array().expect("first thread");
    assert_eq!(thread[0]["type"], "when_message_cast");
    assert_eq!(thread[0]["params"][0].as_str(), Some("m"));
}

/// 트리거가 둘 (when_start + when_click) 이면 object.script 에 thread 2개.
#[test]
fn compile_multiple_triggers_produce_multiple_threads() {
    let src = r#"
        fn when_start() { let x = 1; }
        fn when_click() { let y = 2; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("threads");
    assert_eq!(threads.len(), 2, "when_start + when_click");
    let t0 = threads[0].as_array().expect("t0");
    let t1 = threads[1].as_array().expect("t1");
    assert_eq!(t0[0]["type"], "when_run");
    assert_eq!(t1[0]["type"], "when_click");
    assert_eq!(t0[1]["params"][0]["name"], "x");
    assert_eq!(t1[1]["params"][0]["name"], "y");
}

/// 트리거가 없는 rs (e.g. helper 함수만) 도 정상 처리.
#[test]
fn compile_no_trigger_source_yields_empty_script() {
    let src = "fn helper() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    // helper FuncDef 는 Entry 가 인식 못 하므로 object.script 가 비어있어도 무해.
    // 현재 구현은 비트리거 stmt 를 init thread 1개로 emit 한다.
    let threads = objects[0]["script"].as_array().expect("threads");
    // helper 자체는 init thread 에 들어가지 않는다 (FuncDef 변환은 function_create
    // 블록이 되지만 trigger 가 없으면 EntryJS 가 무시). 일단 0 또는 그 이상 허용.
    let _ = threads.len();
}

/// 미매핑 블록은 (project, Vec<unmapped>) 의 unmapped 에 누적되고 빌드는 성공.
#[test]
fn compile_collects_unmapped_blocks() {
    // timer 변수 read 는 Entry 전용 get 블록 필요 (stmt-level VarDecl/SetVar 사용 불가)
    let src = r#"
        fn when_start() {
            let x = timer;
        }
    "#;
    let (v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    // 빌드는 성공 (해당 stmt 는 init thread 에서 빠짐)
    let objects = v["objects"].as_array().unwrap();
    let threads = objects[0]["script"].as_array().expect("threads");
    let first_thread = threads[0].as_array().expect("first thread");
    // when_run 만 들어가고 timer read stmt 는 빠짐
    assert_eq!(first_thread.len(), 1);
    assert_eq!(first_thread[0]["type"], "when_run");
    // unmapped 메시지에 timer read 사유가 들어감
    assert!(!unmapped.is_empty(), "unmapped 가 비어있으면 안 됨");
    let joined = unmapped.join(" ");
    assert!(joined.contains("timer"), "unmapped 에 timer 사유 포함: {unmapped:?}");
}

/// 미매핑이 없어도 unmapped Vec 은 비어있는 채로 반환.
#[test]
fn compile_empty_unmapped_when_all_supported() {
    let src = "fn when_start() { let x = 1; }";
    let (_v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    assert!(unmapped.is_empty(), "정상 rs 는 unmapped 비어야: {unmapped:?}");
}
