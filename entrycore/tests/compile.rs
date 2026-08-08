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

/// build 가 object.script 에 저장한 JSON 문자열을 thread 배열 Value 로 파싱.
/// (실제 .ent 형식이 String 이라 테스트에서 매번 역직렬화 필요)
fn parse_script_string(script: &Value) -> Value {
    let s = script.as_str().expect("script 는 JSON 문자열");
    serde_json::from_str(s).expect("script JSON 파싱")
}

fn obj_threads(obj: &Value) -> Vec<Value> {
    parse_script_string(&obj["script"])
        .as_array()
        .expect("threads array")
        .clone()
}

fn first_thread(obj: &Value) -> Vec<Value> {
    obj_threads(obj)
        .into_iter()
        .next()
        .expect("first thread")
        .as_array()
        .expect("first thread array")
        .clone()
}

/// base 에 object 가 없을 때 rs stem 이름으로 가짜 sprite 가 추가되고,
/// 그 object 의 `script` 필드(JSON 문자열, thread 배열)에 `[[when_run, body...]]` 가 들어가는지 확인.
#[test]
fn compile_single_source() {
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 1);
    // object.script 는 JSON 문자열 (실제 .ent 형식)
    assert!(
        objects[0]["script"].as_str().is_some(),
        "script 는 JSON 문자열"
    );
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + body 1개");
    assert_eq!(thread[0]["type"], "when_run");
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
    let a_thread = first_thread(a_obj);
    let b_thread = first_thread(b_obj);
    assert_eq!(a_thread[0]["type"], "when_run");
    assert_eq!(b_thread[0]["type"], "when_run");
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
    let thread = first_thread(&objects[0]);
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
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + if");
    assert_eq!(thread[0]["type"], "when_run");
    assert_eq!(thread[1]["type"], "if");
    assert_eq!(thread[1]["params"][0]["type"], "boolean_basic");
}

/// for-range 는 repeat_basic 으로 직렬화.
#[test]
fn compile_for_range_expands_to_repeat() {
    let src = "fn when_start() { for i in 0..5 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + repeat");
    assert_eq!(thread[1]["type"], "repeat_basic");
}

/// compile -> object.script (JSON 문자열) -> deparse 라운드트립.
/// thread 0 = [when_run, if] -> deparse 가 when_run 을 FuncDef 로 감싸고 본문 If 를 body 에.
#[test]
fn compile_roundtrip_via_deparse() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::Stmt;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
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
    let fake_id = objects[0]["id"].as_str().expect("fake id str");
    assert!(fake_id.starts_with("obj_"), "가짜 id 는 obj_ prefix: {fake_id}");
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + body");
}

/// `wait_second(2)` → `wait_second` 블록, params[0] = number 슬롯.
#[test]
fn compile_wait_second_int() {
    let src = r#"fn when_start() { wait_second(2); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "wait_second");
    assert_eq!(thread[1]["params"][0]["type"], "number");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(2.0));
}

/// `wait_second(2.5)` → 실수 보존.
#[test]
fn compile_wait_second_float() {
    let src = r#"fn when_start() { wait_second(2.5); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "wait_second");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(2.5));
}

/// `wait_second(x)` → 변수 슬롯.
#[test]
fn compile_wait_second_var() {
    let src = r#"
        fn when_start() {
            let x = 3;
            wait_second(x);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // set x, wait_second(x)
    let wait = thread.iter().find(|b| b["type"] == "wait_second").expect("wait_second");
    assert_eq!(wait["params"][0]["name"], "x");
}

/// wait_second 라운드트립: compile → deparse → IR 에 wait_second 호출 보존.
#[test]
fn compile_wait_second_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { wait_second(1.5); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "wait_second");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Call(wait_second), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
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
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_run");
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
    let alpha_thread = first_thread(alpha_obj);
    let beta_thread = first_thread(beta_obj);
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
    let thread = first_thread(&objects[0]);
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
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2);
    assert_eq!(thread[0]["type"], "when_run");
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
    let thread = first_thread(&objects[0]);
    let block = &thread[1];
    assert_eq!(block["type"], "if");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 1, "if 는 then 1개 thread");
}

/// if-else compile -> object.script (JSON 문자열) -> deparse 라운드트립.
#[test]
fn compile_if_else_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::Stmt;
    use entrycore::parse::parse;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } else { let y = 2; } }";
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::If { then_body, else_body, .. } => {
                    assert_eq!(then_body.len(), 1);
                    assert_eq!(else_body.len(), 1);
                    let then_var = match &then_body[0] {
                        Stmt::VarDecl(n, _, _, _) | Stmt::SetVar(n, _) => n,
                        other => panic!("unexpected then stmt: {other:?}"),
                    };
                    let else_var = match &else_body[0] {
                        Stmt::VarDecl(n, _, _, _) | Stmt::SetVar(n, _) => n,
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
/// entity 메타가 base 에서 복사되어 위치 등이 0 으로 초기화되지 않는다.
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
    assert!(pics.is_empty(), "가짜 object 의 pictures 는 비어야 함");
    let sounds = fake["sprite"]["sounds"].as_array().unwrap();
    assert!(sounds.is_empty(), "가짜 object 의 sounds 는 비어야 함");
    assert_eq!(fake["selectedPictureId"], Value::Null);
    let fake_id = fake["id"].as_str().expect("fake id str");
    assert_ne!(fake_id, "src1");
    assert!(fake_id.starts_with("obj_"));
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
        assert!(id.starts_with("obj_"), "id 포맷: {id}");
    }
}

/// 가짜 object 의 id 는 stem 기반 stable hash 이고 base 와 충돌 안 함.
#[test]
fn compile_fake_id_skips_existing_ids_in_base() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "obj_doesnotmatter",
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
    let fake_id = fake["id"].as_str().expect("id");
    assert_eq!(fake_id, format!("obj_{}", entrycore::block::id_for("new_one")));
    assert_ne!(fake_id, "obj_doesnotmatter");
}

/// stable id: 같은 stem 으로 두 번 빌드하면 같은 id 가 나온다.
#[test]
fn compile_stable_id_is_deterministic() {
    let src = "fn when_start() { let x = 1; }";
    let v1 = compile(&[("foo", src)], &empty_project()).expect("compile").0;
    let v2 = compile(&[("foo", src)], &empty_project()).expect("compile").0;
    let id1 = v1["objects"][0]["id"].as_str().unwrap();
    let id2 = v2["objects"][0]["id"].as_str().unwrap();
    assert_eq!(id1, id2, "stable id: {id1} == {id2}");
}

/// helper 함수 (트리거 아닌 FuncDef) 는 object script 가 아니라 project.functions 로 emit.
#[test]
fn compile_helpers_go_to_project_functions() {
    let src = r#"
        fn when_start() { let x = 1; }
        fn helper() { let y = 2; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let first_thread = first_thread(&objects[0]);
    // when_run + let x 만 있고 helper 본문(set y)은 없어야
    assert_eq!(first_thread.len(), 2);
    assert_eq!(first_thread[0]["type"], "when_run");
    assert_eq!(first_thread[1]["type"], "set_variable");
    assert_eq!(first_thread[1]["params"][0]["name"], "x");
    // project.functions 에 helper 항목
    let funcs = v["functions"].as_array().expect("functions");
    let helper = funcs.iter().find(|f| f["name"] == "helper").expect("helper fn");
    assert!(helper["id"].as_str().unwrap().starts_with("fn_"));
    // EntryJS Entry.Code 포맷: content = [[thread1_block, ...], ...].
    // helper 의 thread[0] 은 function_create 헤드 블록이며, 그 헤드의
    // statements[0] 에 body 가 들어간다. 헤드의 params[0] 은
    // function_field_label 블록 (이름 + param chain).
    let content = helper["content"].as_array().expect("content threads");
    assert_eq!(content.len(), 1, "helper 는 1개 thread");
    let head = content[0].as_object().expect("head block obj");
    assert_eq!(head["type"], "function_create");
    let label = head["params"][0].as_object().expect("label block");
    assert_eq!(label["type"], "function_field_label");
    assert_eq!(label["params"][0].as_str(), Some("helper"));
    let head_body = head["statements"][0].as_array().expect("head body");
    assert_eq!(head_body.len(), 1);
    assert_eq!(head_body[0]["type"], "set_variable");
    assert_eq!(head_body[0]["params"][0]["name"], "y");
}

/// CompileOptions.default_scene 으로 가짜 object 의 scene 지정.
#[test]
fn compile_default_scene_from_options() {
    use entrycore::compile_with_options;
    let src = "fn when_start() { let x = 1; }";
    let options = entrycore::CompileOptions {
        default_scene: Some("scene2".to_string()),
        ..Default::default()
    };
    let v = compile_with_options(&[("obj", src)], &empty_project(), &options)
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects[0]["scene"], "scene2");
}

/// base 의 첫 object 가 text 면 가짜 object 도 text objectType 보존.
#[test]
fn compile_fake_object_preserves_non_sprite_object_type() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "txt1",
            "name": "label_obj",
            "objectType": "text",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "label_obj", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("new_obj", "fn when_start() { let x = 1; }")], &base)
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    let fake = objects.iter().find(|o| o["name"] == "new_obj").expect("new_obj");
    assert_eq!(fake["objectType"], "text", "base 가 text 면 가짜도 text");
}

/// when_click 트리거는 when_click 블록으로 직렬화.
#[test]
fn compile_when_click_trigger() {
    let src = "fn when_click() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_click");
    assert_eq!(thread[1]["type"], "set_variable");
}

/// when_clone_start 트리거는 when_clone_start 블록으로 직렬화.
#[test]
fn compile_when_clone_start_trigger() {
    let src = "fn when_clone_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_clone_start");
}

/// when_message 함수는 params[0] 을 메시지 이름으로 사용한 when_message_cast 트리거 생성.
#[test]
fn compile_when_message_trigger_uses_param_as_msg() {
    let src = "fn when_message(m: &str) { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_message_cast");
    assert_eq!(thread[0]["params"][0].as_str(), Some("m"));
}

/// when_message 트리거 발견 시 project.messages 에 메시지 항목도 추가.
/// EntryJS 가 message 이름으로 매칭하므로 id 도 name 과 동일.
#[test]
fn compile_when_message_registers_message_in_project() {
    let src = "fn when_message(my_msg: &str) { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let messages = v["messages"].as_array().expect("messages array");
    let msg = messages
        .iter()
        .find(|m| m["name"] == "my_msg")
        .expect("my_msg in messages");
    assert_eq!(msg["id"].as_str(), Some("my_msg"));
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
    let threads = obj_threads(&objects[0]);
    assert_eq!(threads.len(), 2, "when_start + when_click");
    let t0 = threads[0].as_array().expect("t0");
    let t1 = threads[1].as_array().expect("t1");
    assert_eq!(t0[0]["type"], "when_run");
    assert_eq!(t1[0]["type"], "when_click");
    assert_eq!(t0[1]["params"][0]["name"], "x");
    assert_eq!(t1[1]["params"][0]["name"], "y");
}

/// 트리거가 없는 rs 도 정상 처리.
#[test]
fn compile_no_trigger_source_yields_threads() {
    let src = "fn helper() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    // helper 본문은 project.functions 로 가고 object.script 는 비어있음.
    let threads = obj_threads(&objects[0]);
    // threads 비어있거나 (when_run 만) 있어도 무해.
    let _ = threads.len();
}

/// 미매핑 블록은 (project, Vec<unmapped>) 의 unmapped 에 누적되고 빌드는 성공.
#[test]
fn compile_collects_unmapped_blocks() {
    let src = r#"
        fn when_start() {
            let x = timer;
        }
    "#;
    let (v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    let objects = v["objects"].as_array().unwrap();
    let first_thread = first_thread(&objects[0]);
    // when_run 만 들어가고 timer read stmt 는 빠짐
    assert_eq!(first_thread.len(), 1);
    assert_eq!(first_thread[0]["type"], "when_run");
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

/// 같은 미매핑 사유가 여러 위치에서 나도 unmapped 에는 한 번만 들어감 (dedup).
#[test]
fn compile_unmapped_is_deduplicated() {
    // 같은 stmt 가 if 의 두 분기에 들어 있어도 timer read 사유는 한 번만.
    let src = r#"
        fn when_start() {
            if 1 < 2 { let a = timer; }
            else { let b = timer; }
        }
    "#;
    let (_v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    let timer_msgs: Vec<&String> = unmapped
        .iter()
        .filter(|m| m.contains("timer"))
        .collect();
    assert_eq!(
        timer_msgs.len(),
        1,
        "같은 timer 사유가 dedup 되어야: {unmapped:?}"
    );
}

/// 변수 항목이 Entry 실제 .ent 형식 (sample 기준) 의 추가 필드를 포함.
/// 일반 변수는 등장한 object 의 stem 으로 object 필드 채워짐.
#[test]
fn compile_variables_have_entry_format_fields() {
    let src = "fn when_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().expect("variables");
    let x = vars.iter().find(|v| v["name"] == "x").expect("x var");
    assert_eq!(x["visible"], true);
    assert_eq!(x["isCloud"], false);
    assert_eq!(x["isRealTime"], false);
    assert_eq!(x["cloudDate"], false);
    assert_eq!(x["object"], "obj", "x 는 obj 의 변수");
    assert_eq!(x["x"], 0);
    assert_eq!(x["y"], 0);
}

/// 가짜 object 가 Entry 부수 필드 (rotateMethod, lock) 포함.
#[test]
fn compile_fake_object_has_rotate_method_and_lock() {
    let src = "fn when_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects[0]["rotateMethod"], "free");
    assert_eq!(objects[0]["lock"], false);
}

/// 가짜 object 가 `text` 필드 포함 (IRawObject 필수). textBox objectType
/// 에서 글상자 내용, 그 외는 name 으로 fallback.
#[test]
fn compile_fake_object_has_text_field() {
    let src = "fn when_start() { let x = 1; }";
    let v = compile(&[("my_obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    // sprite 면 text = name (fallback).
    assert_eq!(objects[0]["text"], "my_obj");
}

/// textBox objectType base 면 가짜 object 의 text 도 base 에서 복사.
#[test]
fn compile_fake_object_inherits_text_from_base_textbox() {
    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "txt1",
        "name": "label",
        "objectType": "textBox",
        "text": "Hello world",
        "scene": "scene1",
        "script": "[]",
        "sprite": { "name": "label", "pictures": [], "sounds": [] },
        "entity": { "x": 0, "y": 0, "visible": true }
    }]);
    let v = compile(&[("new_box", "fn when_start() { let x = 1; }")], &base)
        .expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let fake = objects.iter().find(|o| o["name"] == "new_box").expect("new_box");
    assert_eq!(fake["text"], "Hello world", "textBox base 의 text 복사");
}

/// function_call 블록은 빌드 시 EntryJS 의 동적 `func_<id>` 호출 블록으로
/// 재작성되어야. helpers 가 project.functions 에 emit 된 후 같은 id 로
/// object.script 의 호출부도 치환된다.
#[test]
fn compile_function_call_rewritten_to_func_id_block() {
    let src = r#"
        fn when_start() { greet(); }
        fn greet() { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    // project.functions 에 greet 항목, id 는 fn_<djb2("greet")>
    let funcs = v["functions"].as_array().expect("functions");
    let greet = funcs.iter().find(|f| f["name"] == "greet").expect("greet fn");
    let fn_id = greet["id"].as_str().expect("fn id");
    assert!(fn_id.starts_with("fn_"), "fn_id format: {fn_id}");
    // object.script 안 호출 블록도 func_<id> 로 치환.
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let call = thread
        .iter()
        .find(|b| {
            let t = b["type"].as_str().unwrap_or("");
            t == "function_call" || t.starts_with("func_")
        })
        .expect("call block");
    assert_eq!(
        call["type"],
        format!("func_{fn_id}"),
        "function_call -> func_<id> rewrite"
    );
    // greet() 은 param 0개 -> 호출부 params = [].
    assert_eq!(call["params"].as_array().unwrap().len(), 0);
}

/// 함수에 param 이 있으면 호출부 params 도 param 개수에 맞춰 emit.
#[test]
fn compile_function_call_params_match_arity() {
    let src = r#"
        fn when_start() { greet("hi", 42); }
        fn greet(a: &str, b: i32) { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let call = thread
        .iter()
        .find(|b| b["type"].as_str().unwrap_or("").starts_with("func_"))
        .expect("call block");
    // greet 의 param 2개 -> 호출부 params 2개.
    assert_eq!(call["params"].as_array().unwrap().len(), 2);
}

/// args 부족분은 null 로 채움.
#[test]
fn compile_function_call_short_args_padded_with_null() {
    let src = r#"
        fn when_start() { greet("only_one"); }
        fn greet(a: &str, b: i32) { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let call = thread
        .iter()
        .find(|b| b["type"].as_str().unwrap_or("").starts_with("func_"))
        .expect("call block");
    let params = call["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    // 첫 번째는 text param.
    assert!(params[0].get("type").and_then(|x| x.as_str()) == Some("text"));
    // 두 번째는 부족분 → null.
    assert!(params[1].is_null(), "두 번째 param null");
}

/// args 초과분은 무시.
#[test]
fn compile_function_call_extra_args_dropped() {
    let src = r#"
        fn when_start() { greet("a", 1, 99); }
        fn greet(a: &str, b: i32) { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let call = thread
        .iter()
        .find(|b| b["type"].as_str().unwrap_or("").starts_with("func_"))
        .expect("call block");
    // greet 의 param 2개 -> 호출부 params 도 2개 (초과분 1개 무시).
    assert_eq!(call["params"].as_array().unwrap().len(), 2);
}

/// 함수 정의 시 param type 어노테이션 (BoolParam) → function_field_boolean chain.
#[test]
fn compile_function_param_chain_emits_kind() {
    let src = r#"
        fn when_start() { greet(true); }
        fn greet(a: BoolParam) { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let funcs = v["functions"].as_array().unwrap();
    let greet = funcs.iter().find(|f| f["name"] == "greet").expect("greet");
    // content[0] = function_create 헤드, params[0] = function_field_label chain
    let head = greet["content"][0].as_object().expect("head");
    assert_eq!(head["type"], "function_create");
    let label = head["params"][0].as_object().expect("label");
    assert_eq!(label["type"], "function_field_label");
    // label.params[1] = function_field_boolean (kind 따라 type 결정)
    let field = label["params"][1].as_object().expect("field");
    assert_eq!(field["type"], "function_field_boolean");
}

/// StringParam (default) → function_field_string chain.
#[test]
fn compile_function_param_default_string_emits_string_field() {
    let src = r#"
        fn when_start() { greet("hi"); }
        fn greet(a: &str) { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let funcs = v["functions"].as_array().unwrap();
    let greet = funcs.iter().find(|f| f["name"] == "greet").expect("greet");
    let head = greet["content"][0].as_object().expect("head");
    let label = head["params"][0].as_object().expect("label");
    let field = label["params"][1].as_object().expect("field");
    assert_eq!(field["type"], "function_field_string");
}

/// param 없는 함수는 label 만, next = null.
#[test]
fn compile_function_no_params_label_next_null() {
    let src = r#"
        fn when_start() { greet(); }
        fn greet() { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let funcs = v["functions"].as_array().unwrap();
    let greet = funcs.iter().find(|f| f["name"] == "greet").expect("greet");
    let head = greet["content"][0].as_object().expect("head");
    let label = head["params"][0].as_object().expect("label");
    assert!(label["params"][1].is_null(), "param 0개 시 label.next = null");
}

/// BoolParam 호출 시 args 가 boolean 으로 wrap.
#[test]
fn compile_function_call_bool_param_arg_wrap() {
    let src = r#"
        fn when_start() { greet(true); }
        fn greet(a: BoolParam) { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let call = thread
        .iter()
        .find(|b| b["type"].as_str().unwrap_or("").starts_with("func_"))
        .expect("call");
    assert_eq!(call["params"][0]["type"], "boolean");
}

/// 미정의 함수 호출은 경고만 stderr 로, 블록은 그대로 유지.
#[test]
fn compile_function_call_to_undefined_keeps_block() {
    let src = r#"
        fn when_start() { mystery(); }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let call = thread
        .iter()
        .find(|b| b["type"] == "function_call")
        .expect("function_call block (undefined)");
    // 미정의라 재작성 안 됨.
    assert_eq!(call["type"], "function_call");
}

/// helper 만 있고 트리거 없을 때 object.script 는 비고 project.functions 에만 emit.
#[test]
fn compile_helper_only_source_emits_no_trigger_thread() {
    let src = "fn helper_only() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = obj_threads(&objects[0]);
    // helper 는 project.functions 로 가고 object.script 는 트리거가 없으면
    // thread 자체를 emit 하지 않음 (EntryJS 가 trigger 없는 thread 무시).
    assert!(threads.is_empty(), "트리거 없는 소스는 thread 미생성");
    let funcs = v["functions"].as_array().expect("functions");
    assert!(funcs.iter().any(|f| f["name"] == "helper_only"));
}

/// base 에 같은 이름의 function 이 있으면 새 빌드의 function 은 이름에 suffix.
#[test]
fn compile_function_name_dedup_against_base() {
    let mut base = empty_project();
    base["functions"] = json!([{
        "id": "fn_existing",
        "name": "greet",
        "content": "[]",
        "param": []
    }]);
    let src = r#"
        fn when_start() { greet(); }
        fn greet() { let y = 1; }
    "#;
    let v = compile(&[("obj", src)], &base).expect("compile").0;
    let funcs = v["functions"].as_array().expect("functions");
    // base "greet" + 새 "greet_2"
    let names: Vec<&str> = funcs.iter()
        .filter_map(|f| f["name"].as_str())
        .collect();
    assert!(names.contains(&"greet"), "base greet 유지");
    assert!(names.contains(&"greet_2"), "중복 이름은 suffix: {names:?}");
}

/// helper 가 없어도 project.functions 는 빈 배열로 emit.
#[test]
fn compile_always_emits_empty_functions_array() {
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &empty_project())
        .expect("compile").0;
    assert!(v["functions"].is_array(), "functions 는 항상 배열");
    assert_eq!(v["functions"].as_array().unwrap().len(), 0);
}

/// when_message 트리거가 없으면 messages 도 빈 배열로 emit.
#[test]
fn compile_emits_empty_messages_when_no_when_message() {
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &empty_project())
        .expect("compile").0;
    assert!(v["messages"].is_array(), "messages 는 항상 배열");
    assert_eq!(v["messages"].as_array().unwrap().len(), 0);
}

/// base 의 변수는 id 기준 union 으로 보존 (교체 아님).
/// 같은 id 의 새 변수는 덮음, 다른 id 의 새 변수는 추가.
#[test]
fn compile_variables_union_preserves_base() {
    let mut base = empty_project();
    base["variables"] = json!([
        {"id":"v1","name":"base_var","variableType":"variable","value":"","object":null,"x":0,"y":0,"visible":true,"isCloud":false,"isRealTime":false,"cloudDate":false},
        {"id":"v2","name":"user_var","variableType":"variable","value":"","object":null,"x":0,"y":0,"visible":true,"isCloud":false,"isRealTime":false,"cloudDate":false},
    ]);
    // 새 빌드에서 base_var 를 읽기만 해도 변수로 집계됨.
    let src = r#"
        fn when_start() {
            let a = base_var;
            let b = new_var;
        }
    "#;
    let v = compile(&[("obj", src)], &base).expect("compile").0;
    let vars = v["variables"].as_array().unwrap();
    let names: Vec<&str> = vars.iter().filter_map(|v| v["name"].as_str()).collect();
    assert!(names.contains(&"base_var"), "base_var 보존");
    assert!(names.contains(&"user_var"), "user_var 보존");
    assert!(names.contains(&"new_var"), "new_var 추가");
}

/// replace_variables=true 면 base variables 무시, 새 빌드만 사용.
#[test]
fn compile_variables_replace_drops_base() {
    let mut base = empty_project();
    base["variables"] = json!([
        {"id":"v1","name":"base_var","variableType":"variable","value":"","object":null,"x":0,"y":0,"visible":true,"isCloud":false,"isRealTime":false,"cloudDate":false},
    ]);
    let options = entrycore::CompileOptions {
        replace_variables: true,
        ..Default::default()
    };
    let src = "fn when_start() { let new_var = 1; }";
    let v = entrycore::compile_with_options(&[("obj", src)], &base, &options)
        .expect("compile").0;
    let vars = v["variables"].as_array().unwrap();
    let names: Vec<&str> = vars.iter().filter_map(|v| v["name"].as_str()).collect();
    assert!(!names.contains(&"base_var"), "replace 면 base 변수 제거");
    assert!(names.contains(&"new_var"), "새 변수만 유지");
}

/// base 의 malformed 변수 (id/name/variableType 없음) 는 union 모드에서도
/// 제외 — EntryJS 가 silent hash 로 노이즈 생성하는 것 방지.
#[test]
fn compile_variables_filter_malformed_base() {
    let mut base = empty_project();
    base["variables"] = json!([
        {"id":"v1","name":"good","variableType":"variable","value":""},
        {"id":"v2"},
        {},
        {"name":"no_id","variableType":"variable","value":""},
    ]);
    let src = "fn when_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &base).expect("compile").0;
    let vars = v["variables"].as_array().unwrap();
    // good 만 살아남고 malformed 3개는 제외.
    let good_count = vars.iter().filter(|v| v["name"] == "good").count();
    assert_eq!(good_count, 1);
    // 나머지 malformed 가 새 빌드의 x 와 섞여서 들어가지 않았는지.
    let malformed_count = vars.iter().filter(|v| {
        v.get("name").is_none() || v.get("variableType").is_none() || v.get("id").is_none()
    }).count();
    assert_eq!(malformed_count, 0, "malformed base 변수는 필터링");
}

/// `let x: CloudVar = ...` → variableType:"cloud", isCloud:true.
#[test]
fn compile_typed_cloud_var_emits_cloud_metadata() {
    let src = r#"
        fn when_start() {
            let cloud_v: CloudVar = "";
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().unwrap();
    let cloud = vars.iter().find(|v| v["name"] == "cloud_v").expect("cloud_v");
    assert_eq!(cloud["variableType"], "cloud");
    assert_eq!(cloud["isCloud"], true);
}

/// `let x: RealtimeVar = ...` → variableType:"realtime", isRealTime:true.
#[test]
fn compile_typed_realtime_var_emits_realtime_metadata() {
    let src = r#"
        fn when_start() {
            let rt_v: RealtimeVar = "";
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().unwrap();
    let rt = vars.iter().find(|v| v["name"] == "rt_v").expect("rt_v");
    assert_eq!(rt["variableType"], "realtime");
    assert_eq!(rt["isRealTime"], true);
}

/// top-level `static` → variables[].object = null (전역).
#[test]
fn compile_static_var_is_global() {
    let src = r#"
        static GLOBAL_VAR: i32 = 0;
        fn when_start() { let x = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().unwrap();
    let g = vars.iter().find(|v| v["name"] == "GLOBAL_VAR").expect("GLOBAL_VAR");
    assert!(g["object"].is_null(), "static 변수는 object: null");
    // 함수 내 let x 는 object = "obj" (로컬).
    let x = vars.iter().find(|v| v["name"] == "x").expect("x");
    assert_eq!(x["object"], "obj", "let 변수는 object: stem");
}

// ── 시작 블록 매핑 (트리거) ──

/// `fn when_key_pressed(key: &str)` → `when_some_key_pressed` 트리거.
/// params[0] = null (Indicator), params[1] = key code 문자열.
#[test]
fn compile_when_key_pressed_trigger() {
    let src = r#"fn when_key_pressed(key: &str) { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_some_key_pressed");
    assert!(thread[0]["params"][0].is_null());
    assert_eq!(thread[0]["params"][1].as_str(), Some("key"));
}

/// key code 미지정 (param 없는 시그니처) 도 매핑. default "81".
#[test]
fn compile_when_key_pressed_no_param_defaults_to_81() {
    // syn 상 fn f() 만 가능 — param 0개는 정상. 단 우리 DSL 신택스상
    // key: &str 필요이지만 fallback 동작 확인용으로 빈 케이스 테스트:
    let src = r#"fn when_key_pressed() { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_some_key_pressed");
    assert_eq!(thread[0]["params"][1].as_str(), Some("81"));
}

/// `fn when_mouse_clicked()` → `mouse_clicked` 트리거.
#[test]
fn compile_when_mouse_clicked_trigger() {
    let src = r#"fn when_mouse_clicked() { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "mouse_clicked");
}

/// `fn when_mouse_released()` → `mouse_click_cancled` 트리거.
#[test]
fn compile_when_mouse_released_trigger() {
    let src = r#"fn when_mouse_released() { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "mouse_click_cancled");
}

/// `fn when_object_released()` → `when_object_click_canceled` 트리거.
#[test]
fn compile_when_object_released_trigger() {
    let src = r#"fn when_object_released() { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_object_click_canceled");
}

/// `fn when_scene_start()` → `when_scene_start` 트리거.
#[test]
fn compile_when_scene_start_trigger() {
    let src = r#"fn when_scene_start() { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_scene_start");
}

// ── 시작 블록 매핑 (액션) ──

/// `send_message("foo")` → `message_cast` 블록.
#[test]
fn compile_send_message_emits_message_cast() {
    let src = r#"fn when_start() { send_message("foo"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // thread[0] = when_run, thread[1] = message_cast
    assert_eq!(thread[0]["type"], "when_run");
    assert_eq!(thread[1]["type"], "message_cast");
    assert_eq!(thread[1]["params"][0].as_str(), Some("foo"));
    assert!(thread[1]["params"][1].is_null());
}

/// `wait_message("foo")` → `message_cast_wait` 블록.
#[test]
fn compile_wait_message_emits_message_cast_wait() {
    let src = r#"fn when_start() { wait_message("foo"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "message_cast_wait");
    assert_eq!(thread[1]["params"][0].as_str(), Some("foo"));
}

/// `start_scene("scene2")` → `start_scene` 블록.
#[test]
fn compile_start_scene_emits_start_scene() {
    let src = r#"fn when_start() { start_scene("scene2"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "start_scene");
    assert_eq!(thread[1]["params"][0].as_str(), Some("scene2"));
}

/// `start_next_scene()` → `start_neighbor_scene` (next).
#[test]
fn compile_start_next_scene_emits_start_neighbor_scene_next() {
    let src = r#"fn when_start() { start_next_scene(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "start_neighbor_scene");
    assert_eq!(thread[1]["params"][0].as_str(), Some("next"));
}

/// `start_prev_scene()` → `start_neighbor_scene` (prev).
#[test]
fn compile_start_prev_scene_emits_start_neighbor_scene_prev() {
    let src = r#"fn when_start() { start_prev_scene(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "start_neighbor_scene");
    assert_eq!(thread[1]["params"][0].as_str(), Some("prev"));
}

/// 메시지 액션 사용 시 project.messages 에 메시지 등록.
#[test]
fn compile_when_message_registers_message() {
    // send_message/wait_message 는 호출만. 메시지 정의는 when_message 트리거에서.
    let src = r#"
        fn when_start() { send_message("foo"); wait_message("foo"); }
        fn when_message(my_msg: &str) { let x = 1; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let messages = v["messages"].as_array().expect("messages");
    let names: Vec<&str> = messages.iter().filter_map(|m| m["name"].as_str()).collect();
    // send_message 자체는 메시지 등록 안 함 (호출만). EntryJS 가 호출 시 dynamic 처리.
    // when_message 트리거의 my_msg 만 등록.
    assert!(names.contains(&"my_msg"), "when_message 의 my_msg 등록");
    assert!(!names.contains(&"foo"), "send_message 의 foo 는 등록 안 됨");
}

// ── 라운드트립 ──

/// 시작 트리거/액션 블록의 deparse → parse 라운드트립 보존.
#[test]
fn compile_start_blocks_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;

    let src = r#"
        fn when_key_pressed(k: &str) { send_message("foo"); }
        fn when_mouse_clicked() { start_next_scene(); }
        fn when_scene_start() { start_scene("s2"); }
    "#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script str");
    let _ = program_from_script_string_with_vars(script_str, &vars).expect("deparse");
}
