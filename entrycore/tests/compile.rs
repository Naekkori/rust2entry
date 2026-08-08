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

#[test]
fn compile_single_source() {
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let scripts = v["scripts"].as_array().expect("scripts array");
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0]["type"], "set_variable");
    // 변수명 보존
    assert_eq!(scripts[0]["params"][0]["name"], "x");
}

#[test]
fn compile_multi_source_merges_scripts() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile");
    let scripts = v["scripts"].as_array().expect("scripts array");
    assert_eq!(scripts.len(), 2);
    let names: Vec<&str> = scripts
        .iter()
        .filter_map(|s| s["params"][0]["name"].as_str())
        .collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
}

#[test]
fn compile_preserves_base_metadata() {
    let mut base = empty_project();
    base["name"] = json!("my_proj");
    base["scenes"] = json!([
        { "id": "scene1", "name": "장면1" },
        { "id": "scene2", "name": "장면2" },
    ]);
    base["speed"] = json!(30);
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &base).expect("compile");
    // base 메타 보존
    assert_eq!(v["name"], "my_proj");
    assert_eq!(v["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(v["speed"], 30);
    // scripts 는 패치됨
    assert_eq!(v["scripts"].as_array().unwrap().len(), 1);
}

#[test]
fn compile_aggregates_variables_across_sources() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; let z = 3; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile");
    let vars = v["variables"].as_array().expect("variables array");
    let names: Vec<&str> = vars.iter().filter_map(|x| x["name"].as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert!(names.contains(&"z"));
    assert_eq!(vars.len(), 3);
}

#[test]
fn compile_deduplicates_variables() {
    // 같은 변수명을 두 소스에서 쓰면 하나로 합쳐져야 함
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let x = 2; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile");
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

#[test]
fn compile_empty_sources_returns_base() {
    let mut base = empty_project();
    base["name"] = json!("untouched");
    let v = compile(&[], &base).expect("compile");
    // scripts/variables 비어있음 (0개 stmts)
    assert_eq!(v["scripts"].as_array().unwrap().len(), 0);
    assert_eq!(v["variables"].as_array().unwrap().len(), 0);
    // base 메타는 보존
    assert_eq!(v["name"], "untouched");
}

#[test]
fn compile_if_block_structure() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let scripts = v["scripts"].as_array().expect("scripts");
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0]["type"], "if");
    assert_eq!(scripts[0]["params"][0]["type"], "boolean_basic");
}

#[test]
fn compile_for_range_expands_to_repeat() {
    let src = "fn when_start() { for i in 0..5 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let scripts = v["scripts"].as_array().expect("scripts");
    assert_eq!(scripts[0]["type"], "repeat_basic");
}

#[test]
fn compile_roundtrip_via_deparse() {
    // compile 결과 -> deparse -> IR 구조 보존
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_value_with_vars;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let vars = collect_var_map(&p1);
    let scripts_wrapped = json!([v["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars).expect("deparse");
    assert!(matches!(p2.stmts[0], entrycore::ir::Stmt::If { .. }));
}

#[test]
fn compile_adds_fake_object_when_empty() {
    // extract 라운드트립용: objects 가 비어있으면 가짜 오브젝트 1개 추가
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("my_obj", src)], &empty_project()).expect("compile");
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 1, "가짜 오브젝트 1개 추가");
    assert_eq!(objects[0]["name"], "my_obj");
    assert_eq!(objects[0]["objectType"], "sprite");
    assert_eq!(objects[0]["scene"], "scene1");
    // entity 기본값 확인
    assert_eq!(objects[0]["entity"]["x"], 0.0);
    assert_eq!(objects[0]["entity"]["visible"], true);
    // sprite 메타도 보존
    assert_eq!(objects[0]["sprite"]["name"], "my_obj");
    assert!(objects[0]["sprite"]["pictures"].as_array().unwrap().is_empty());
}

#[test]
fn compile_does_not_overwrite_existing_objects() {
    // base 에 objects 가 이미 있으면 추가하지 않음
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
    let v = compile(&[("new_obj", "fn when_start() { let x = 1; }")], &base).expect("compile");
    let objects = v["objects"].as_array().expect("objects");
    assert_eq!(objects.len(), 1, "기존 objects 보존, 추가 안 함");
    assert_eq!(objects[0]["name"], "existing_obj");
}

/// if-else 블록: compile -> codegen 결과가 if_else 타입이고
/// statements[0]=then / statements[1]=else 로 분기되는지 확인.
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
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let scripts = v["scripts"].as_array().expect("scripts");
    assert_eq!(scripts.len(), 1);
    let block = &scripts[0];
    assert_eq!(block["type"], "if_else");
    // 조건 슬롯
    assert_eq!(block["params"][0]["type"], "boolean_basic");
    // then / else 두 thread
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 2, "if_else 는 then/else 2개 thread");
    // then 본문: set_variable x
    let then_first = &stmts[0][0];
    assert_eq!(then_first["type"], "set_variable");
    assert_eq!(then_first["params"][0]["name"], "x");
    // else 본문: set_variable y
    let else_first = &stmts[1][0];
    assert_eq!(else_first["type"], "set_variable");
    assert_eq!(else_first["params"][0]["name"], "y");
}

/// else 없으면 if_else 가 아닌 if 로 직렬화 (Entry 의 if 블록 형식).
#[test]
fn compile_if_without_else_stays_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let block = &v["scripts"][0];
    assert_eq!(block["type"], "if");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 1, "if 는 then 1개 thread");
}

/// if-else -> compile -> deparse 라운드트립 보존.
#[test]
fn compile_if_else_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_value_with_vars;
    use entrycore::ir::Stmt;
    use entrycore::parse::parse;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } else { let y = 2; } }";
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile");
    let vars = collect_var_map(&p1);
    let scripts_wrapped = json!([v["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::If { then_body, else_body, .. } => {
            assert_eq!(then_body.len(), 1);
            assert_eq!(else_body.len(), 1);
            // codegen -> deparse 라운드트립은 VarDecl/SetVar 양쪽 모두 가능 (Entry 가 구분 안 함)
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
        other => panic!("expected If, got {other:?}"),
    }
}
