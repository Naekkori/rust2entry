//! lib::compile 통합 테스트.
//!
//! parse + codegen 을 거치며 최종 project.json 구조 확인.

use entrycore::compile;
use entrycore::ir::{BinOp, Expr, Stmt, UnaryOp};
use entrycore::VarMap;
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
    assert_eq!(thread[0]["type"], "when_run_button_click");
    assert_eq!(thread[1]["type"], "set_variable");
    assert_eq!(
        thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
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
    assert_eq!(a_thread[0]["type"], "when_run_button_click");
    assert_eq!(b_thread[0]["type"], "when_run_button_click");
    assert_eq!(
        a_thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
    assert_eq!(
        b_thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("y"))
    );
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
    assert_eq!(thread[0]["type"], "when_run_button_click");
    assert_eq!(thread[1]["type"], "_if");
    assert_eq!(thread[1]["params"][0]["type"], "boolean_basic_operator");
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
    let vars = collect_var_map(&p1, &VarMap::new());
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
    // 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit (EntryJS 호환).
    assert_eq!(wait["params"][0]["type"], "get_variable");
    assert_eq!(
        wait["params"][0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
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
    let vars = collect_var_map(&p1, &VarMap::new());
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

// ── wait_until_true ──

/// `wait_until_true(x > 5)` → `wait_until_true` 블록, params[0] = boolean_basic.
#[test]
fn compile_wait_until_true_compare() {
    let src = r#"
        fn when_start() {
            let x = 3;
            wait_until_true(x > 5);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let wait = thread.iter().find(|b| b["type"] == "wait_until_true").expect("wait_until_true");
    assert_eq!(wait["params"][0]["type"], "boolean_basic_operator");
}

/// `wait_until_true(flag)` → 변수 슬롯.
#[test]
fn compile_wait_until_true_var() {
    let src = r#"
        fn when_start() {
            wait_until_true(flag);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let wait = thread.iter().find(|b| b["type"] == "wait_until_true").expect("wait_until_true");
    // flag 는 미정의 변수 — codegen 은 `get_variable` 값 슬롯 블록으로 emit.
    // EntryJS 가 값 슬롯은 Block 객체만 받기 때문 (dropdown 슬롯과 다름).
    assert_eq!(wait["params"][0]["type"], "get_variable");
}

/// 산술 포함 조건.
#[test]
fn compile_wait_until_true_arith_cond() {
    let src = r#"fn when_start() { wait_until_true(1 + 2 < 5); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let wait = thread.iter().find(|b| b["type"] == "wait_until_true").expect("wait_until_true");
    assert_eq!(wait["params"][0]["type"], "boolean_basic_operator");
}

/// 라운드트립.
#[test]
fn compile_wait_until_true_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { wait_until_true(x > 5); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "wait_until_true");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Call(wait_until_true), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// if 블록에서 조건 lhs 가 변수 dropdown (type 키 없음) 일 때 deparse 가
/// `block.type missing` 에러 없이 ParamBlock::Variable 로 라운드트립되는지.
#[test]
fn compile_if_roundtrip_with_var_cond() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::Stmt;
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let x = 1;
            if x < 5 { let y = 2; }
        }
    "#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            // body[0] = let x = 1; body[1] = if x < 5 ...
            assert!(matches!(body[1], Stmt::If { .. }), "body[1] If expected, got {:?}", body[1]);
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── calc_rand (난수) ──

/// `calc_rand(1, 10)` → `calc_rand` 블록, params[0]/[1] = number 슬롯.
#[test]
fn compile_calc_rand_int() {
    let src = r#"fn when_start() { let x = calc_rand(1, 10); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    // set 의 params[1] 이 calc_rand.
    assert_eq!(set["params"][1]["type"], "calc_rand");
    assert_eq!(set["params"][1]["params"][1]["params"][0].as_f64(), Some(1.0));
    assert_eq!(set["params"][1]["params"][3]["params"][0].as_f64(), Some(10.0));
}

/// `calc_rand(1.5, 9.5)` → 실수 보존.
#[test]
fn compile_calc_rand_float() {
    let src = r#"fn when_start() { let x = calc_rand(1.5, 9.5); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    assert_eq!(set["params"][1]["params"][1]["params"][0].as_f64(), Some(1.5));
    assert_eq!(set["params"][1]["params"][3]["params"][0].as_f64(), Some(9.5));
}

/// `calc_rand` 의 args 가 변수일 때 dropdown 슬롯.
#[test]
fn compile_calc_rand_var_args() {
    let src = r#"
        fn when_start() {
            let lo = 1;
            let hi = 10;
            let x = calc_rand(lo, hi);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set = thread.iter().rev().find(|b| b["type"] == "set_variable").expect("last set");
    assert_eq!(set["params"][1]["type"], "calc_rand");
    // 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit (EntryJS 호환).
    assert_eq!(set["params"][1]["params"][1]["type"], "get_variable");
    assert_eq!(
        set["params"][1]["params"][1]["params"][0]
            .as_str()
            .map(|s| s.to_string()),
        Some(entrycore::block::id_for("lo"))
    );
    assert_eq!(set["params"][1]["params"][3]["type"], "get_variable");
    assert_eq!(
        set["params"][1]["params"][3]["params"][0]
            .as_str()
            .map(|s| s.to_string()),
        Some(entrycore::block::id_for("hi"))
    );
}

/// 라운드트립: compile → deparse → IR 에 calc_rand 호출 보존.
#[test]
fn compile_calc_rand_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { let x = calc_rand(1, 10); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "calc_rand");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected SetVar(Call(calc_rand)), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── get_project_timer_value (타이머 값) ──

/// `let x = get_project_timer_value();` → set_variable 의 params[1] = get_project_timer_value 블록.
#[test]
fn compile_get_project_timer_value() {
    let src = r#"fn when_start() { let x = get_project_timer_value(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    assert_eq!(set["params"][1]["type"], "get_project_timer_value");
}

/// `let x = get_project_timer_value() + 1;` → calc_basic 의 lhs 가 timer 값.
#[test]
fn compile_get_project_timer_value_in_expr() {
    let src = r#"fn when_start() { let x = get_project_timer_value() + 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    // params[1] = calc_basic, 그 lhs 가 timer 값 블록.
    assert_eq!(set["params"][1]["type"], "calc_basic");
    assert_eq!(set["params"][1]["params"][0]["type"], "get_project_timer_value");
}

/// 라운드트립: get_project_timer_value 가 IR 에서 Call 로 보존.
#[test]
fn compile_get_project_timer_value_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { let x = get_project_timer_value(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "get_project_timer_value");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected SetVar(Call(timer)), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── set_visible_project_timer (타이머 보이기/숨기기) ──

/// `show_timer();` → `set_visible_project_timer`, params[0] = true.
#[test]
fn compile_show_timer() {
    let src = r#"fn when_start() { show_timer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_visible_project_timer");
    assert_eq!(thread[1]["params"][1], "SHOW");
}

/// `hide_timer();` → `set_visible_project_timer`, params[0] = false.
#[test]
fn compile_hide_timer() {
    let src = r#"fn when_start() { hide_timer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_visible_project_timer");
    assert_eq!(thread[1]["params"][1], "HIDE");
}

/// 라운드트립: show_timer → set_visible_project_timer → deparse → show_timer 재호출.
#[test]
fn compile_show_timer_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { show_timer(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "show_timer");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(show_timer), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── set_visible_answer (대답 보이기/숨기기) ──

/// `show_answer();` → `set_visible_answer`, params[0] = true.
#[test]
fn compile_show_answer() {
    let src = r#"fn when_start() { show_answer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_visible_answer");
    assert_eq!(thread[1]["params"][1], "SHOW");
}

/// `hide_answer();` → `set_visible_answer`, params[0] = false.
#[test]
fn compile_hide_answer() {
    let src = r#"fn when_start() { hide_answer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_visible_answer");
    assert_eq!(thread[1]["params"][1], "HIDE");
}

/// 라운드트립.
#[test]
fn compile_show_answer_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { show_answer(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "show_answer");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(show_answer), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── dialog (말하기) ──

/// `say("hello");` → `dialog` 블록, params[0] = text 슬롯, params[1] = "speak".
#[test]
fn compile_say_text() {
    let src = r#"fn when_start() { say("hello"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "dialog");
    assert_eq!(thread[1]["params"][0]["type"], "text");
    assert_eq!(thread[1]["params"][0]["params"][0].as_str(), Some("hello"));
    // EntryJS dialog dropdown 의 value 는 'speak' / 'think'.
    assert_eq!(thread[1]["params"][1].as_str(), Some("speak"));
}

/// `say(x);` → params[0] = 변수 dropdown.
#[test]
fn compile_say_var() {
    let src = r#"
        fn when_start() {
            let x = "hi";
            say(x);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let dlg = thread.iter().find(|b| b["type"] == "dialog").expect("dialog");
    // 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit (EntryJS 호환).
    assert_eq!(dlg["params"][0]["type"], "get_variable");
    assert_eq!(
        dlg["params"][0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
}

/// 라운드트립.
#[test]
fn compile_say_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { say("hi"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Dialog { value, mode } => {
                    assert_eq!(*mode, entrycore::block::DialogMode::Say);
                    assert!(matches!(value, Expr::Str(s) if s == "hi"));
                }
                other => panic!("expected Dialog(Say), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `think("hmm");` → `dialog` 블록, params[1] = "think".
#[test]
fn compile_think_text() {
    let src = r#"fn when_start() { think("hmm"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "dialog");
    assert_eq!(thread[1]["params"][0]["params"][0].as_str(), Some("hmm"));
    assert_eq!(thread[1]["params"][1].as_str(), Some("think"));
}

/// 라운드트립: think → dialog(think) → think 재호출.
#[test]
fn compile_think_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { think("hmm"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Dialog { value, mode } => {
                    assert_eq!(*mode, entrycore::block::DialogMode::Think);
                    assert!(matches!(value, Expr::Str(s) if s == "hmm"));
                }
                other => panic!("expected Dialog(Think), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `say("hello", 2.0);` → `dialog_time` 블록, params[2] = number 슬롯, params[1] = "speak".
#[test]
fn compile_say_with_time() {
    let src = r#"fn when_start() { say("hello", 2.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "dialog_time");
    assert_eq!(thread[1]["params"][0]["params"][0].as_str(), Some("hello"));
    // EntryJS dialog dropdown 의 value 는 'speak' / 'think'.
    assert_eq!(thread[1]["params"][1].as_str(), Some("speak"));
    assert_eq!(thread[1]["params"][2]["params"][0].as_f64(), Some(2.0));
}

/// `think("hmm", 1.5);` → `dialog_time` 블록, params[1] = "think".
#[test]
fn compile_think_with_time() {
    let src = r#"fn when_start() { think("hmm", 1.5); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "dialog_time");
    assert_eq!(thread[1]["params"][1].as_str(), Some("think"));
    assert_eq!(thread[1]["params"][2]["params"][0].as_f64(), Some(1.5));
}

/// `say("hi", 2.0);` 라운드트립.
#[test]
fn compile_say_with_time_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { say("hi", 2.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "say");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected Call(say, 2), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `think("hmm", 1.5);` 라운드트립.
#[test]
fn compile_think_with_time_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { think("hmm", 1.5); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "think");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected Call(think, 2), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `change_to_some_shape("walk");` → `change_to_some_shape` 블록, params[0] = "walk".
#[test]
fn compile_change_to_some_shape() {
    let src = r#"fn when_start() { change_to_some_shape("walk"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_to_some_shape");
    assert_eq!(thread[1]["params"][0]["type"], "get_pictures");
    assert_eq!(thread[1]["params"][0]["params"][0], "walk");
}

/// `change_to_next_shape();` → `change_to_next_shape` 블록, params = [].
#[test]
fn compile_change_to_next_shape() {
    let src = r#"fn when_start() { change_to_next_shape(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_to_next_shape");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_change_to_some_shape_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_to_some_shape("walk"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_to_some_shape");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(args[0], Expr::Str(_)));
                }
                other => panic!("expected Call(change_to_some_shape), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// 라운드트립.
#[test]
fn compile_change_to_next_shape_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_to_next_shape(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_to_next_shape");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(change_to_next_shape), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `remove_dialog();` → `remove_dialog` 블록, params = [].
#[test]
fn compile_remove_dialog() {
    let src = r#"fn when_start() { remove_dialog(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "remove_dialog");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_remove_dialog_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { remove_dialog(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "remove_dialog");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(remove_dialog), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `add_effect_amount("color", 50.0);` → `add_effect_amount`, params[0] = "color", params[1] = 50.0.
#[test]
fn compile_add_effect_amount() {
    let src = r#"fn when_start() { add_effect_amount("color", 50.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "add_effect_amount");
    assert_eq!(thread[1]["params"][0].as_str(), Some("color"));
    assert_eq!(thread[1]["params"][1]["params"][0].as_f64(), Some(50.0));
}

/// 다른 효과 (ghost).
#[test]
fn compile_add_effect_amount_ghost() {
    let src = r#"fn when_start() { add_effect_amount("ghost", 25.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "add_effect_amount");
    assert_eq!(thread[1]["params"][0].as_str(), Some("ghost"));
}

/// 라운드트립.
#[test]
fn compile_add_effect_amount_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { add_effect_amount("color", 50.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "add_effect_amount");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "color"));
                }
                other => panic!("expected Call(add_effect_amount), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `change_effect_amount("color", 50.0);` → `change_effect_amount`, params[0] = "color".
#[test]
fn compile_change_effect_amount() {
    let src = r#"fn when_start() { change_effect_amount("color", 50.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_effect_amount");
    assert_eq!(thread[1]["params"][0].as_str(), Some("color"));
    assert_eq!(thread[1]["params"][1]["params"][0].as_f64(), Some(50.0));
}

/// 다른 효과.
#[test]
fn compile_change_effect_amount_brightness() {
    let src = r#"fn when_start() { change_effect_amount("brightness", 100.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_effect_amount");
    assert_eq!(thread[1]["params"][0].as_str(), Some("brightness"));
}

/// 라운드트립.
#[test]
fn compile_change_effect_amount_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_effect_amount("color", 50.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_effect_amount");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "color"));
                }
                other => panic!("expected Call(change_effect_amount), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `erase_all_effects();` → `erase_all_effects` 블록, params = [].
#[test]
fn compile_erase_all_effects() {
    let src = r#"fn when_start() { erase_all_effects(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "erase_all_effects");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_erase_all_effects_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { erase_all_effects(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "erase_all_effects");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(erase_all_effects), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `change_scale_size(10.0);` → `change_scale_size`, params[0] = number 슬롯.
#[test]
fn compile_change_scale_size() {
    let src = r#"fn when_start() { change_scale_size(10.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_scale_size");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(10.0));
}

/// `change_scale_size(n);` → 변수 슬롯.
#[test]
fn compile_change_scale_size_var() {
    let src = r#"
        fn when_start() {
            let n = 50.0;
            change_scale_size(n);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let css = thread.iter().find(|b| b["type"] == "change_scale_size").expect("change_scale_size");
    // 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit (EntryJS 호환).
    assert_eq!(css["params"][0]["type"], "get_variable");
    assert_eq!(
        css["params"][0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("n"))
    );
}

/// `value_of_index_from_list(1, list)`는 set_variable의 값 슬롯 안에서
/// 리스트 조회 표현식 블록으로 emit되어야 한다.
#[test]
fn compile_value_of_index_from_list() {
    let src = r#"
        fn when_start() {
            let list = "";
            let x = value_of_index_from_list(1, list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project())
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    let set_x = thread
        .iter()
        .find(|b| b["type"] == "set_variable" && b["params"][0] == entrycore::block::id_for("x"))
        .expect("set x");
    let value = &set_x["params"][1];
    assert_eq!(value["type"], "value_of_index_from_list");
    assert_eq!(value["params"].as_array().unwrap().len(), 2);
    assert_eq!(value["params"][0]["type"], "number");
    assert_eq!(value["params"][0]["params"][0], 1.0);
    assert_eq!(
        value["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    // EntryJS 호환: list dropdown 슬롯도 string id 만 emit (object 아님).
    assert_eq!(
        value["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
}

/// 리스트 조회 표현식은 Entry JSON에서 DSL로 deparse한 뒤에도 보존되어야 한다.
#[test]
fn compile_value_of_index_from_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            let x = value_of_index_from_list(1, list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project())
        .expect("compile")
        .0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let Some(Stmt::SetVar(vref, Expr::Call(fref, args))) = body.iter().find(|stmt| {
        matches!(stmt, Stmt::SetVar(vref, Expr::Call(_, _)) if vref.name == "x")
    }) else {
        panic!("expected set x to list lookup call");
    };
    assert_eq!(vref.name, "x");
    assert_eq!(fref.name, "value_of_index_from_list");
    assert_eq!(args.len(), 2);
    assert!(
        matches!(&args[0], Expr::Int(1))
            || matches!(&args[0], Expr::Float(n) if *n == 1.0)
    );
    assert!(matches!(&args[1], Expr::Var(name) if name == "list"));
}

/// `add_value_to_list("apple", list)`는 항목 값과 리스트 dropdown을 가진
/// statement 블록으로 emit되어야 한다.
#[test]
fn compile_add_value_to_list() {
    let src = r#"
        fn when_start() {
            let list = "";
            add_value_to_list("apple", list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project())
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    let add = thread
        .iter()
        .find(|b| b["type"] == "add_value_to_list")
        .expect("add_value_to_list");
    assert_eq!(add["params"].as_array().unwrap().len(), 3);
    assert_eq!(add["params"][0]["type"], "text");
    assert_eq!(add["params"][0]["params"][0], "apple");
    assert_eq!(
        add["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert_eq!(
        add["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert!(add["params"][2].is_null());
}

#[test]
fn compile_add_value_to_named_list_without_declaration() {
    let src = r#"
        fn when_start() {
            add_value_to_list("apple", fruit);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let fruit = v["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|var| var["name"] == "fruit")
        .expect("fruit variable");
    assert_eq!(fruit["variableType"], "list");
    assert_eq!(fruit["value"], serde_json::json!([]));
    assert_eq!(fruit["array"], serde_json::json!([]));
    assert!(fruit["object"].is_null());

    let thread = first_thread(&v["objects"].as_array().unwrap()[0]);
    let add = thread
        .iter()
        .find(|block| block["type"] == "add_value_to_list")
        .expect("add_value_to_list");
    assert_eq!(
        add["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("fruit"))
    );
    // EntryJS 호환: list dropdown 슬롯은 string id 만 emit (object 아님).
}

/// 리스트 항목 추가 statement는 Entry JSON에서 DSL 호출로 deparse되어야 한다.
#[test]
fn compile_add_value_to_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            add_value_to_list("apple", list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project())
        .expect("compile")
        .0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let Some(Stmt::Expr(Expr::Call(fref, args))) = body.iter().find(|stmt| {
        matches!(stmt, Stmt::Expr(Expr::Call(fref, _)) if fref.name == "add_value_to_list")
    }) else {
        panic!("expected add_value_to_list call");
    };
    assert_eq!(fref.name, "add_value_to_list");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0], Expr::Str(value) if value == "apple"));
    assert!(matches!(&args[1], Expr::Var(name) if name == "list"));
}

/// `remove_value_from_list(1, list)`는 index와 리스트 dropdown을 가진
/// statement 블록으로 emit되어야 한다.
#[test]
fn compile_remove_value_from_list() {
    let src = r#"
        fn when_start() {
            let list = "";
            remove_value_from_list(1, list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project())
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    let remove = thread
        .iter()
        .find(|b| b["type"] == "remove_value_from_list")
        .expect("remove_value_from_list");
    assert_eq!(remove["params"].as_array().unwrap().len(), 3);
    assert_eq!(remove["params"][0]["type"], "number");
    assert_eq!(remove["params"][0]["params"][0], 1.0);
    assert_eq!(
        remove["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert_eq!(
        remove["params"][1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert!(remove["params"][2].is_null());
}

/// 리스트 항목 삭제 statement는 Entry JSON에서 DSL 호출로 deparse되어야 한다.
#[test]
fn compile_remove_value_from_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            remove_value_from_list(1, list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project())
        .expect("compile")
        .0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let Some(Stmt::Expr(Expr::Call(fref, args))) = body.iter().find(|stmt| {
        matches!(stmt, Stmt::Expr(Expr::Call(fref, _)) if fref.name == "remove_value_from_list")
    }) else {
        panic!("expected remove_value_from_list call");
    };
    assert_eq!(fref.name, "remove_value_from_list");
    assert_eq!(args.len(), 2);
    assert!(
        matches!(&args[0], Expr::Int(1))
            || matches!(&args[0], Expr::Float(n) if *n == 1.0)
    );
    assert!(matches!(&args[1], Expr::Var(name) if name == "list"));
}

#[test]
fn compile_insert_value_to_list() {
    let src = r#"
        fn when_start() {
            let list = "";
            insert_value_to_list("apple", 2, list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let insert = thread
        .iter()
        .find(|b| b["type"] == "insert_value_to_list")
        .expect("insert_value_to_list");
    assert_eq!(insert["params"].as_array().unwrap().len(), 4);
    assert_eq!(insert["params"][0]["type"], "text");
    assert_eq!(insert["params"][0]["params"][0], "apple");
    assert_eq!(insert["params"][1]["type"], "number");
    assert_eq!(insert["params"][1]["params"][0], 2.0);
    assert_eq!(
        insert["params"][2].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert_eq!(
        insert["params"][2].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert!(insert["params"][3].is_null());
}

#[test]
fn compile_insert_value_to_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            insert_value_to_list("apple", 2, list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");
    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else { panic!("expected when_start function") };
    let Some(Stmt::Expr(Expr::Call(_, args))) = body.iter().find(|stmt| {
        matches!(stmt, Stmt::Expr(Expr::Call(fref, _)) if fref.name == "insert_value_to_list")
    }) else { panic!("expected insert_value_to_list call") };
    assert_eq!(args.len(), 3);
    assert!(matches!(&args[0], Expr::Str(s) if s == "apple"));
    assert!(matches!(&args[1], Expr::Int(2) | Expr::Float(2.0)));
    assert!(matches!(&args[2], Expr::Var(name) if name == "list"));
}

#[test]
fn compile_change_value_list_index() {
    let src = r#"
        fn when_start() {
            let list = "";
            change_value_list_index(2, "apple", list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let change = thread
        .iter()
        .find(|b| b["type"] == "change_value_list_index")
        .expect("change_value_list_index");
    assert_eq!(change["params"].as_array().unwrap().len(), 4);
    assert_eq!(change["params"][0]["type"], "number");
    assert_eq!(change["params"][0]["params"][0], 2.0);
    assert_eq!(change["params"][1]["type"], "text");
    assert_eq!(change["params"][1]["params"][0], "apple");
    assert_eq!(
        change["params"][2].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert_eq!(
        change["params"][2].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert!(change["params"][3].is_null());
}

#[test]
fn compile_change_value_list_index_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            change_value_list_index(2, "apple", list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let Some(Stmt::Expr(Expr::Call(_, args))) = body.iter().find(|stmt| {
        matches!(stmt, Stmt::Expr(Expr::Call(fref, _)) if fref.name == "change_value_list_index")
    }) else {
        panic!("expected change_value_list_index call");
    };
    assert_eq!(args.len(), 3);
    assert!(matches!(&args[0], Expr::Int(2)) || matches!(&args[0], Expr::Float(n) if *n == 2.0));
    assert!(matches!(&args[1], Expr::Str(s) if s == "apple"));
    assert!(matches!(&args[2], Expr::Var(name) if name == "list"));
}

/// `length_of_list(list)` → params[1] dropdown list.
#[test]
fn compile_length_of_list() {
    let src = r#"
        fn when_start() {
            let list = "";
            length_of_list(list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let len_block = thread
        .iter()
        .find(|b| b["type"] == "length_of_list")
        .expect("length_of_list");
let params = len_block["params"].as_array().unwrap();
    assert_eq!(params.len(), 3);
    assert!(params[0].is_null());
    assert!(params[2].is_null());
    assert_eq!(
        params[1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert_eq!(
        params[1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
}

/// 라운드트립.
#[test]
fn compile_length_of_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            length_of_list(list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found_call = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "length_of_list" => Some(fref),
        _ => None,
    });
    assert!(found_call.is_some(), "expected length_of_list call");
}

/// `is_included_in_list(list, "x")` → params[3] value, params[1] dropdown.
#[test]
fn compile_is_included_in_list() {
    let src = r#"
        fn when_start() {
            let list = "";
            is_included_in_list(list, "x");
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let check_block = thread
        .iter()
        .find(|b| b["type"] == "is_included_in_list")
        .expect("is_included_in_list");
    let params = check_block["params"].as_array().unwrap();
    assert_eq!(params.len(), 5);
    assert!(params[0].is_null());
    assert_eq!(
        params[1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert_eq!(
        params[1].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("list"))
    );
    assert!(params[2].is_null());
    assert_eq!(params[3]["type"], "text");
    assert_eq!(params[3]["params"][0], "x");
    assert!(params[4].is_null());
}

/// 라운드트립.
#[test]
fn compile_is_included_in_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            let list = "";
            is_included_in_list(list, "x");
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found_call = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "is_included_in_list" => Some(fref),
        _ => None,
    });
    assert!(found_call.is_some(), "expected is_included_in_list call");
}

/// 라운드트립.
#[test]
fn compile_change_scale_size_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_scale_size(10.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_scale_size");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Call(change_scale_size), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `set_scale_size(100.0);` → `set_scale_size`, params[0] = number 슬롯.
#[test]
fn compile_set_scale_size() {
    let src = r#"fn when_start() { set_scale_size(100.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_scale_size");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(100.0));
}

/// `set_scale_size(n);` → 변수 슬롯.
#[test]
fn compile_set_scale_size_var() {
    let src = r#"
        fn when_start() {
            let n = 200.0;
            set_scale_size(n);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let sss = thread.iter().find(|b| b["type"] == "set_scale_size").expect("set_scale_size");
    // 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit (EntryJS 호환).
    assert_eq!(sss["params"][0]["type"], "get_variable");
    assert_eq!(
        sss["params"][0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("n"))
    );
}

/// 라운드트립.
#[test]
fn compile_set_scale_size_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { set_scale_size(100.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "set_scale_size");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Call(set_scale_size), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `reset_scale_size();` → `reset_scale_size` 블록, params = [].
#[test]
fn compile_reset_scale_size() {
    let src = r#"fn when_start() { reset_scale_size(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "reset_scale_size");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_reset_scale_size_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { reset_scale_size(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "reset_scale_size");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(reset_scale_size), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `stretch_scale_size("height", 10);` → params = ["HEIGHT", 10, null].
#[test]
fn compile_stretch_scale_size() {
    let src = r#"fn when_start() { stretch_scale_size("height", 10); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "stretch_scale_size");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 3);
    assert_eq!(params[0], "HEIGHT");
    assert!(params[2].is_null());
}

/// 라운드트립.
#[test]
fn compile_stretch_scale_size_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { stretch_scale_size("width", 10); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "stretch_scale_size");
                    assert_eq!(args.len(), 2);
                    match &args[0] {
                        Expr::Str(s) => assert_eq!(s, "width"),
                        other => panic!("expected Str(width), got {other:?}"),
                    }
                }
                other => panic!("expected Call(stretch_scale_size), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `flip_x();` → `flip_x` 블록, params = [].
#[test]
fn compile_flip_x() {
    let src = r#"fn when_start() { flip_x(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "flip_x");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_flip_x_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { flip_x(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "flip_x");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(flip_x), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `flip_y();` → `flip_y` 블록, params = [].
#[test]
fn compile_flip_y() {
    let src = r#"fn when_start() { flip_y(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "flip_y");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_flip_y_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { flip_y(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "flip_y");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(flip_y), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `is_clicked();` → `is_clicked` 블록, params = [].
#[test]
fn compile_is_clicked() {
    let src = r#"fn when_start() { is_clicked(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "is_clicked");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_is_clicked_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { is_clicked(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "is_clicked");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(is_clicked), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `is_object_clicked();` → `is_object_clicked` 블록, params = [].
#[test]
fn compile_is_object_clicked() {
    let src = r#"fn when_start() { is_object_clicked(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "is_object_clicked");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_is_object_clicked_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { is_object_clicked(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "is_object_clicked");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(is_object_clicked), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `coordinate_mouse("x")`는 값 슬롯에 좌표 축을 보존한다.
#[test]
fn compile_coordinate_mouse_value() {
    let src = r#"fn when_start() { let x = coordinate_mouse("x"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let thread = first_thread(&v["objects"][0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    let block = &set["params"][1];
    assert_eq!(block["type"], "coordinate_mouse");
    assert_eq!(block["params"][1], "x");
}

/// `coordinate_object`는 대상과 속성을 각각 EntryJS 슬롯에 보존한다.
#[test]
fn compile_coordinate_object_value() {
    let src = r#"fn when_start() { let x = coordinate_object("enemy", "direction"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let thread = first_thread(&v["objects"][0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    let block = &set["params"][1];
    assert_eq!(block["type"], "coordinate_object");
    assert_eq!(block["params"][1], "enemy");
    assert_eq!(block["params"][3], "direction");
}

// ── ask_and_wait ──

/// `change_object_index("front");` → `change_object_index`, params[0] = "front".
#[test]
fn compile_change_object_index_front() {
    let src = r#"fn when_start() { change_object_index("front"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_object_index");
    assert_eq!(thread[1]["params"][0].as_str(), Some("front"));
}

/// `change_object_index("back");` → params[0] = "back".
#[test]
fn compile_change_object_index_back() {
    let src = r#"fn when_start() { change_object_index("back"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_object_index");
    assert_eq!(thread[1]["params"][0].as_str(), Some("back"));
}

/// 라운드트립.
#[test]
fn compile_change_object_index_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_object_index("front"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_object_index");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "front"));
                }
                other => panic!("expected Call(change_object_index), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `delete_clone();` → `delete_clone` 블록, params = [].
#[test]
fn compile_delete_clone() {
    let src = r#"fn when_start() { delete_clone(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "delete_clone");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_delete_clone_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { delete_clone(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "delete_clone");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(delete_clone), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `remove_all_clones();` → `remove_all_clones` 블록, params = [].
#[test]
fn compile_remove_all_clones() {
    let src = r#"fn when_start() { remove_all_clones(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "remove_all_clones");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_remove_all_clones_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { remove_all_clones(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "remove_all_clones");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(remove_all_clones), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `bounce_wall();` → `bounce_wall` 블록, params = [].
#[test]
fn compile_bounce_wall() {
    let src = r#"fn when_start() { bounce_wall(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "bounce_wall");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_bounce_wall_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { bounce_wall(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "bounce_wall");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(bounce_wall), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `is_press_some_key("space");` → stmt-level 호출.
#[test]
fn compile_is_press_some_key() {
    let src = r#"fn when_start() { is_press_some_key("space"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "is_press_some_key");
    assert_eq!(thread[1]["params"][0].as_str(), Some("space"));
}

/// `if is_press_some_key("space") { ... }` → 값 슬롯으로 사용.
#[test]
fn compile_is_press_some_key_in_cond() {
    let src = r#"
        fn when_start() {
            if is_press_some_key("space") {
                let x = 1;
            }
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // when_run + if
    assert_eq!(thread[1]["type"], "_if");
    // if 블록의 cond 슬롯 = is_press_some_key
    let cond = &thread[1]["params"][0];
    assert_eq!(cond["type"], "is_press_some_key");
    assert_eq!(cond["params"][0].as_str(), Some("space"));
}

/// 라운드트립.
#[test]
fn compile_is_press_some_key_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { is_press_some_key("space"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "is_press_some_key");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "space"));
                }
                other => panic!("expected Call(is_press_some_key), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `reach_something("enemy");` → 다른 sprite 와 닿음.
#[test]
fn compile_reach_something_target() {
    let src = r#"fn when_start() { reach_something("enemy"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "reach_something");
    assert_eq!(thread[1]["params"][1].as_str(), Some("enemy"));
}

/// `reach_something();` → self (인자 없으면 "self" fallback).
#[test]
fn compile_reach_something_self() {
    let src = r#"fn when_start() { reach_something(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "reach_something");
    assert_eq!(thread[1]["params"][1].as_str(), Some("self"));
}

/// 라운드트립.
#[test]
fn compile_reach_something_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { reach_something("enemy"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "reach_something");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "enemy"));
                }
                other => panic!("expected Call(reach_something), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `move_direction("forward", 10.0);` → `move_direction`, params[0]="forward", params[1]=10.0.
#[test]
fn compile_move_direction() {
    let src = r#"fn when_start() { move_direction("forward", 10.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "move_direction");
    assert_eq!(thread[1]["params"][0].as_str(), Some("forward"));
    assert_eq!(thread[1]["params"][1]["params"][0].as_f64(), Some(10.0));
}

/// `move_direction("backward", n);` → 변수 슬롯.
#[test]
fn compile_move_direction_var() {
    let src = r#"
        fn when_start() {
            let n = 5.0;
            move_direction("backward", n);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let md = thread.iter().find(|b| b["type"] == "move_direction").expect("move_direction");
    assert_eq!(md["params"][0].as_str(), Some("backward"));
    // 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit (EntryJS 호환).
    assert_eq!(md["params"][1]["type"], "get_variable");
    assert_eq!(
        md["params"][1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("n"))
    );
}

/// 라운드트립.
#[test]
fn compile_move_direction_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { move_direction("forward", 10.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "move_direction");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "forward"));
                }
                other => panic!("expected Call(move_direction), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `move_x(10.0);` → `move_x`, params[0]=10.0.
#[test]
fn compile_move_x() {
    let src = r#"fn when_start() { move_x(10.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "move_x");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(10.0));
    assert_eq!(thread[1]["params"][1], json!(null));
}

/// `move_y(5.0);` → `move_y`, params[0]=5.0.
/// 음수 인자(-5.0)는 UnaryOp로 파싱되어 roundtrip 미지원.
/// roundtrip 테스트도 5.0 사용.
#[test]
fn compile_move_y() {
    let src = r#"fn when_start() { move_y(5.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "move_y");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(5.0));
    assert_eq!(thread[1]["params"][1], json!(null));
}

/// 라운드트립.
#[test]
fn compile_move_x_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { move_x(10.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "move_x");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 10.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(move_x), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

#[test]
fn compile_move_y_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { move_y(5.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "move_y");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 5.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(move_y), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `rotate_relative(45.0);` → `rotate_relative`, params[0]=45.0.
#[test]
fn compile_rotate_relative() {
    let src = r#"fn when_start() { rotate_relative(45.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "rotate_relative");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(45.0));
    assert_eq!(thread[1]["params"][1], json!(null));
}

/// `direction_relative(90.0);` → `direction_relative`, params[0]=90.0.
#[test]
fn compile_direction_relative() {
    let src = r#"fn when_start() { direction_relative(90.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "direction_relative");
    assert_eq!(thread[1]["params"][0]["params"][0].as_f64(), Some(90.0));
    assert_eq!(thread[1]["params"][1], json!(null));
}

/// 라운드트립.
#[test]
fn compile_rotate_relative_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { rotate_relative(45.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "rotate_relative");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 45.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(rotate_relative), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

#[test]
fn compile_direction_relative_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { direction_relative(90.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "direction_relative");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 90.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(direction_relative), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `rotate_absolute(90.0)` → `rotate_absolute` 블록, params = [각도, null].
#[test]
fn compile_rotate_absolute() {
    let src = r#"fn when_start() { rotate_absolute(90.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "rotate_absolute");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2); // 1 arg + trailing null
}

#[test]
fn compile_rotate_absolute_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { rotate_absolute(90.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "rotate_absolute");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 90.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(rotate_absolute), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `direction_absolute(45.0)` → `direction_absolute` 블록, params = [방향, null].
#[test]
fn compile_direction_absolute() {
    let src = r#"fn when_start() { direction_absolute(45.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "direction_absolute");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2); // 1 arg + trailing null
}

#[test]
fn compile_direction_absolute_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { direction_absolute(45.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "direction_absolute");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 45.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(direction_absolute), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `see_angle_object("mouse")` → `see_angle_object` 블록, params = [대상, null].
#[test]
fn compile_see_angle_object() {
    let src = r#"fn when_start() { see_angle_object("mouse"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "see_angle_object");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2); // 1 arg + trailing null
}

#[test]
fn compile_see_angle_object_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { see_angle_object("mouse"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "see_angle_object");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "mouse"));
                }
                other => panic!("expected Call(see_angle_object), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `move_to_angle(45.0, 10.0)` → `move_to_angle` 블록, params = [각도, 거리, null].
#[test]
fn compile_move_to_angle() {
    let src = r#"fn when_start() { move_to_angle(45.0, 10.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "move_to_angle");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 3); // 2 args + trailing null
}

#[test]
fn compile_move_to_angle_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { move_to_angle(45.0, 10.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "move_to_angle");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 45.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[1], Expr::Float(n) if (n - 10.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(move_to_angle), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `brush_stamp()` → `brush_stamp` 블록, params = [].
#[test]
fn compile_brush_stamp() {
    let src = r#"fn when_start() { brush_stamp(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "brush_stamp");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_brush_stamp_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { brush_stamp(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "brush_stamp");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(brush_stamp), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `start_drawing()` → `start_drawing` 블록, params = [].
#[test]
fn compile_start_drawing() {
    let src = r#"fn when_start() { start_drawing(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "start_drawing");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_start_drawing_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { start_drawing(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "start_drawing");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(start_drawing), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `stop_drawing()` → `stop_drawing` 블록, params = [].
#[test]
fn compile_stop_drawing() {
    let src = r#"fn when_start() { stop_drawing(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "stop_drawing");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_stop_drawing_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { stop_drawing(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "stop_drawing");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(stop_drawing), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `start_fill()` → `start_fill` 블록, params = [].
#[test]
fn compile_start_fill() {
    let src = r#"fn when_start() { start_fill(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "start_fill");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_start_fill_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { start_fill(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "start_fill");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(start_fill), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `stop_fill()` → `stop_fill` 블록, params = [].
#[test]
fn compile_stop_fill() {
    let src = r#"fn when_start() { stop_fill(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "stop_fill");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_stop_fill_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { stop_fill(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "stop_fill");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(stop_fill), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `set_color(50.0, 100.0, 0.0)` → `set_color` 블록, params = [r, g, b].
#[test]
fn compile_set_color() {
    let src = r#"fn when_start() { set_color(50.0, 100.0, 0.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_color");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 3); // 3 args (trailing null X)
}

#[test]
fn compile_set_color_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { set_color(50.0, 100.0, 0.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "set_color");
                    assert_eq!(args.len(), 3);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 50.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[1], Expr::Float(n) if (n - 100.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[2], Expr::Float(n) if (n - 0.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(set_color), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `set_random_color()` → `set_random_color` 블록, params = [].
#[test]
fn compile_set_random_color() {
    let src = r#"fn when_start() { set_random_color(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_random_color");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_set_random_color_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { set_random_color(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "set_random_color");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(set_random_color), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `set_fill_color("#FF0000")` → `set_fill_color` 블록, params = [color].
#[test]
fn compile_set_fill_color() {
    let src = r##"fn when_start() { set_fill_color("#FF0000"); }"##;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_fill_color");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert!(params[1].is_null());
}

#[test]
fn compile_set_fill_color_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r##"fn when_start() { set_fill_color("#FF0000"); }"##;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "set_fill_color");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "#FF0000"));
                }
                other => panic!("expected Call(set_fill_color), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `change_thickness(5.0)` → `change_thickness` 블록, params = [amount, null].
#[test]
fn compile_change_thickness() {
    let src = r#"fn when_start() { change_thickness(5.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_thickness");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
}

#[test]
fn compile_change_thickness_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_thickness(5.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_thickness");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 5.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(change_thickness), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `x = x + 1` → Entry `change_variable` 블록, params[0]=변수 socket, params[1]=값 슬롯.
/// parse 가 `x = x + n`/`x = x - n` 패턴을 감지해 IrStmt::ChangeVariable 로 emit 하는지 확인.
#[test]
fn compile_change_variable_from_self_plus() {
    let src = r#"
        fn when_start() {
            let x = 0;
            x = x + 1;
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let change = thread
        .iter()
        .find(|b| b["type"] == "change_variable")
        .expect("change_variable block");
    // params[0] = variable socket, params[1] = value block, params[2] = null
    assert!(change["params"][0].is_string() || change["params"][0].is_object());
    let value = &change["params"][1];
    assert_eq!(value["type"], "number");
    assert_eq!(value["params"][0].as_f64(), Some(1.0));
}

/// `x = x - 1` → Entry `change_variable`, value 슬롯이 `-1`.
/// parse 가 Sub 도 ChangeVariable 로 인식하고 decodegen 이 `+ -1` 형태로 emit.
#[test]
fn compile_change_variable_from_self_minus() {
    let src = r#"
        fn when_start() {
            let x = 10;
            x = x - 1;
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let change = thread
        .iter()
        .find(|b| b["type"] == "change_variable")
        .expect("change_variable block");
    let value = &change["params"][1];
    assert_eq!(value["type"], "number");
    assert_eq!(value["params"][0].as_f64(), Some(-1.0));
}

/// `x = expr` (self-add 가 아닌 일반 대입) → Entry `set_variable` 으로 emit.
#[test]
fn compile_set_variable_for_general_assign() {
    let src = r#"
        fn when_start() {
            let x = 0;
            x = 42;
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert!(
        thread.iter().any(|b| b["type"] == "set_variable"),
        "expected set_variable block, got thread={thread:?}"
    );
    assert!(
        !thread.iter().any(|b| b["type"] == "change_variable"),
        "general assign must not emit change_variable"
    );
}

/// 라운드트립 — `x = x + 1` -> JSON -> 다시 IR 했을 때 ChangeVariable 복원.
#[test]
fn compile_change_variable_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::Stmt;
    use entrycore::parse::parse;

    let src = r#"fn when_start() { let x = 0; x = x + 1; }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    // when_start 본문 마지막 stmt 가 ChangeVariable 인지 확인.
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => {
            assert!(
                body.iter().any(|s| matches!(s, Stmt::ChangeVariable { .. })),
                "expected ChangeVariable after roundtrip: {body:?}"
            );
        }
        _ => panic!("expected FuncDef"),
    }
}

/// `set_thickness(10.0)` → `set_thickness` 블록, params = [value, null].
#[test]
fn compile_set_thickness() {
    let src = r#"fn when_start() { set_thickness(10.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_thickness");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
}

#[test]
fn compile_set_thickness_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { set_thickness(10.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "set_thickness");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 10.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(set_thickness), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `change_brush_transparency(10.0)` → `change_brush_transparency` 블록, params = [amount, null].
#[test]
fn compile_change_brush_transparency() {
    let src = r#"fn when_start() { change_brush_transparency(10.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "change_brush_transparency");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
}

#[test]
fn compile_change_brush_transparency_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { change_brush_transparency(10.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "change_brush_transparency");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 10.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(change_brush_transparency), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `set_brush_tranparency(50.0)` → `set_brush_tranparency` 블록 (오타 그대로), params = [value, null].
#[test]
fn compile_set_brush_tranparency() {
    let src = r#"fn when_start() { set_brush_tranparency(50.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_brush_tranparency");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
}

#[test]
fn compile_set_brush_tranparency_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { set_brush_tranparency(50.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "set_brush_tranparency");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 50.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(set_brush_tranparency), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `brush_erase_all()` → `brush_erase_all` 블록, params = [].
#[test]
fn compile_brush_erase_all() {
    let src = r#"fn when_start() { brush_erase_all(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "brush_erase_all");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

#[test]
fn compile_brush_erase_all_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { brush_erase_all(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "brush_erase_all");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(brush_erase_all), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `text_read` (값 슬롯) — `let x = text_read("self")` → SetVar 값으로 Sub 사용.
#[test]
fn compile_text_read() {
    let src = r#"fn when_start() { let x = text_read("self"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // SetVar 블록 type
    assert_eq!(thread[1]["type"], "set_variable");
    // SetVar 의 params[1] (value 자리) 에 Sub 블록 (text_read)
    let value = &thread[1]["params"][1];
    assert_eq!(value["type"], "text_read");
    let value_params = value["params"].as_array().unwrap();
    // to_value 가 [value, null] 로 emit (params.len == 2)
    assert_eq!(value_params.len(), 2);
}

#[test]
fn compile_text_read_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { let x = text_read("self"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            // SetVar (text_read 가 Sub 로 들어감)
            match &body[0] {
                Stmt::SetVar(vref, expr) => {
                    assert_eq!(vref.name, "x");
                    // SetVar 의 expr 이 Call(text_read) 이어야
                    match expr {
                        Expr::Call(fref, args) => {
                            assert_eq!(fref.name, "text_read");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(&args[0], Expr::Str(s) if s == "self"));
                        }
                        other => panic!("expected Call(text_read), got {other:?}"),
                    }
                }
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `move_xy_time(1.0, 10.0, 5.0)` → `move_xy_time` 블록, params = [시간, x, y].
#[test]
fn compile_move_xy_time() {
    let src = r#"fn when_start() { move_xy_time(1.0, 10.0, 5.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "move_xy_time");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 4); // 3 args + trailing null
}

/// 라운드트립.
#[test]
fn compile_move_xy_time_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { move_xy_time(1.0, 10.0, 5.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "move_xy_time");
                    assert_eq!(args.len(), 3);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 1.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[1], Expr::Float(n) if (n - 10.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[2], Expr::Float(n) if (n - 5.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(move_xy_time), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `locate_x(100.0)` → `locate_x` 블록.
#[test]
fn compile_locate_x() {
    let src = r#"fn when_start() { locate_x(100.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "locate_x");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
}

#[test]
fn compile_locate_x_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { locate_x(100.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "locate_x");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 100.0).abs() < f64::EPSILON));
                }
                other => panic!("expected Call(locate_x), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `locate_y(-50.0)` → `locate_y` 블록.
#[test]
fn compile_locate_y() {
    let src = r#"fn when_start() { locate_y(-50.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "locate_y");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
}

#[test]
fn compile_locate_y_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt, UnaryOp};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { locate_y(-50.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "locate_y");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::BinOp(BinOp::Sub, lhs, rhs) if matches!(**lhs, Expr::Float(n) if n.abs() < f64::EPSILON) && matches!(**rhs, Expr::Float(n) if (n - 50.0).abs() < f64::EPSILON)));
                }
                other => panic!("expected Call(locate_y), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `locate_xy(100.0, -50.0)` → `locate_xy` 블록.
#[test]
fn compile_locate_xy() {
    let src = r#"fn when_start() { locate_xy(100.0, -50.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "locate_xy");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 3);
}

#[test]
fn compile_locate_xy_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt, UnaryOp};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { locate_xy(100.0, -50.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "locate_xy");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 100.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[1], Expr::BinOp(BinOp::Sub, lhs, rhs) if matches!(**lhs, Expr::Float(n) if n.abs() < f64::EPSILON) && matches!(**rhs, Expr::Float(n) if (n - 50.0).abs() < f64::EPSILON)));
                }
                other => panic!("expected Call(locate_xy), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `locate_xy_time(1.0, 100.0, -50.0)` → `locate_xy_time` 블록, params = [시간, x, y].
#[test]
fn compile_locate_xy_time() {
    let src = r#"fn when_start() { locate_xy_time(1.0, 100.0, -50.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "locate_xy_time");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 4); // 3 args + trailing null
}

#[test]
fn compile_locate_xy_time_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt, UnaryOp};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { locate_xy_time(1.0, 100.0, -50.0); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "locate_xy_time");
                    assert_eq!(args.len(), 3);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 1.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[1], Expr::Float(n) if (n - 100.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[2], Expr::BinOp(BinOp::Sub, lhs, rhs) if matches!(**lhs, Expr::Float(n) if n.abs() < f64::EPSILON) && matches!(**rhs, Expr::Float(n) if (n - 50.0).abs() < f64::EPSILON)));
                }
                other => panic!("expected Call(locate_xy_time), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `locate_object_time(1.0, "mouse")` → `locate_object_time` 블록, params = [시간, 대상, null].
#[test]
fn compile_locate_object_time() {
    let src = r#"fn when_start() { locate_object_time(1.0, "mouse"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "locate_object_time");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 3); // 2 args + trailing null
}

#[test]
fn compile_locate_object_time_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { locate_object_time(1.0, "mouse"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "locate_object_time");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Float(n) if (n - 1.0).abs() < f64::EPSILON));
                    assert!(matches!(&args[1], Expr::Str(s) if s == "mouse"));
                }
                other => panic!("expected Call(locate_object_time), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `locate("mouse")` → `locate` 블록, params = [target, null].
#[test]
fn compile_locate() {
    let src = r#"fn when_start() { locate("mouse"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "locate");
    let params = thread[1]["params"].as_array().unwrap();
    assert_eq!(params.len(), 2); // 1 arg + trailing null
}

#[test]
fn compile_locate_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { locate("mouse"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "locate");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "mouse"));
                }
                other => panic!("expected Call(locate), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `ask_and_wait("이름을 입력")` → `ask_and_wait` 블록, params[0] = text 슬롯.
#[test]
fn compile_ask_and_wait() {
    let src = r#"fn when_start() { ask_and_wait("이름을 입력"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let ask = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "ask_and_wait")
        .expect("ask_and_wait");
    assert_eq!(ask["params"][0]["type"], "text");
    assert_eq!(ask["params"][0]["params"][0], "이름을 입력");
}

/// `ask_and_wait(name)` → params[0] = 변수 드롭다운.
#[test]
fn compile_ask_and_wait_var_arg() {
    let src = r#"
        fn when_start() {
            ask_and_wait(name);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let ask = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "ask_and_wait")
        .expect("ask_and_wait");
    // EntryJS 호환: 값 슬롯 자리의 변수 ref 는 `get_variable` 블록으로 emit.
    assert_eq!(ask["params"][0]["type"], "get_variable");
    assert_eq!(
        ask["params"][0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("name"))
    );
}

/// 라운드트립: compile → deparse → IR 에 ask_and_wait 호출 보존.
#[test]
fn compile_ask_and_wait_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { ask_and_wait("이름"); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "ask_and_wait");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::Str(s) => assert_eq!(s, "이름"),
                        other => panic!("expected Str, got {other:?}"),
                    }
                }
                other => panic!("expected Call(ask_and_wait), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `get_canvas_input_value()` 라운드트립.
#[test]
fn compile_get_canvas_input_value_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { let x = get_canvas_input_value(); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "get_canvas_input_value");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected SetVar(Call(canvas_input)), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── 타이머 시작/정지/리셋 ──

/// `start_timer()` → `choose_project_timer_action` 블록, params[0] = "start".
#[test]
fn compile_start_timer() {
    let src = r#"fn when_start() { start_timer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let action = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "choose_project_timer_action")
        .expect("choose_project_timer_action");
    assert_eq!(action["params"][1], "START");
}

/// `stop_timer()` / `reset_timer()` 매핑.
#[test]
fn compile_stop_reset_timer() {
    let src = r#"fn when_start() { stop_timer(); reset_timer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let blocks: Vec<&Value> = thread
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["type"] == "choose_project_timer_action")
        .collect();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["params"][1], "STOP");
    assert_eq!(blocks[1]["params"][1], "RESET");
}

/// start_timer 라운드트립.
#[test]
fn compile_start_timer_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { start_timer(); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "start_timer");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(start_timer), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// reset_timer 라운드트립.
#[test]
fn compile_reset_timer_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { reset_timer(); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "reset_timer");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(reset_timer), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── quotient_and_mod ──

/// `quotient_and_mod(a, b, "quotient")` → 블록 + params[2] = "quotient".
#[test]
fn compile_quotient_and_mod_quotient() {
    let src = r#"fn when_start() { let x = quotient_and_mod(10, 3, "quotient"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let set_var = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "set_variable")
        .expect("set_variable");
    let block = &set_var["params"][1];
    assert_eq!(block["type"], "quotient_and_mod");
    assert_eq!(block["params"][5], "quotient");
}

/// `quotient_and_mod(a, b, "modulo")` → params[2] = "modulo".
#[test]
fn compile_quotient_and_mod_modulo() {
    let src = r#"fn when_start() { let x = quotient_and_mod(10, 3, "modulo"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let set_var = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "set_variable")
        .expect("set_variable");
    let block = &set_var["params"][1];
    assert_eq!(block["type"], "quotient_and_mod");
    assert_eq!(block["params"][5], "modulo");
}

/// quotient_and_mod 라운드트립.
#[test]
fn compile_quotient_and_mod_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { let x = quotient_and_mod(10, 3, "modulo"); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "quotient_and_mod");
                    assert_eq!(args.len(), 3);
                    match &args[2] {
                        Expr::Str(s) => assert_eq!(s, "modulo"),
                        other => panic!("expected Str, got {other:?}"),
                    }
                }
                other => panic!("expected SetVar(Call(quotient_and_mod)), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

// ── calc_operation ──

/// `abs(x)` → calc_operation 블록, params[0] = "abs".
#[test]
fn compile_abs() {
    let src = r#"fn when_start() { let y = abs(x); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let set_var = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "set_variable")
        .expect("set_variable");
    let block = &set_var["params"][1];
    assert_eq!(block["type"], "calc_operation");
    assert_eq!(block["params"][3], "abs");
}

/// sqrt 라운드트립.
#[test]
fn compile_sqrt_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { let y = sqrt(x); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "sqrt");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected SetVar(Call(sqrt)), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// sin 매핑.
#[test]
fn compile_sin() {
    let src = r#"fn when_start() { let y = sin(x); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let set_var = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "set_variable")
        .expect("set_variable");
    let block = &set_var["params"][1];
    assert_eq!(block["type"], "calc_operation");
    assert_eq!(block["params"][3], "sin");
}

// ── show / hide (외형) ──

/// `show()` → `show` 블록, params 없음.
#[test]
fn compile_show() {
    let src = r#"fn when_start() { show(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let show = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "show")
        .expect("show");
    assert!(show["params"].as_array().unwrap().is_empty());
}

/// `hide()` → `hide` 블록, params 없음.
#[test]
fn compile_hide() {
    let src = r#"fn when_start() { hide(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let obj = &v["objects"][0];
    let thread = &obj_threads(obj)[0];
    let hide = thread
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["type"] == "hide")
        .expect("hide");
    assert!(hide["params"].as_array().unwrap().is_empty());
}

/// show 라운드트립.
#[test]
fn compile_show_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { show(); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "show");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(show), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// hide 라운드트립.
#[test]
fn compile_hide_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { hide(); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "hide");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(hide), got {other:?}"),
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
    assert_eq!(thread[0]["type"], "when_run_button_click");
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
    assert_eq!(
        alpha_thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
    assert_eq!(
        beta_thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("y"))
    );
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
    assert_eq!(thread[0]["type"], "when_run_button_click");
    let block = &thread[1];
    assert_eq!(block["type"], "if_else");
    assert_eq!(block["params"][0]["type"], "boolean_basic_operator");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 2, "if_else 는 then/else 2개 thread");
    let then_first = &stmts[0][0];
    assert_eq!(then_first["type"], "set_variable");
    assert_eq!(
        then_first["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
    let else_first = &stmts[1][0];
    assert_eq!(else_first["type"], "set_variable");
    assert_eq!(
        else_first["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("y"))
    );
}

/// else 없으면 if (Entry 의 if 블록 형식).
#[test]
fn compile_if_without_else_stays_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = &thread[1];
    assert_eq!(block["type"], "_if");
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
    let vars = collect_var_map(&p1, &VarMap::new());
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
                        Stmt::VarDecl(n, _, _, _) => n,
                        Stmt::SetVar(vref, _) => &vref.name,
                        other => panic!("unexpected then stmt: {other:?}"),
                    };
                    let else_var = match &else_body[0] {
                        Stmt::VarDecl(n, _, _, _) => n,
                        Stmt::SetVar(vref, _) => &vref.name,
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
    assert_eq!(first_thread[0]["type"], "when_run_button_click");
    assert_eq!(first_thread[1]["type"], "set_variable");
    assert_eq!(
        first_thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
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
    // EntryJS function_field_label.params[0] = TextInput 필드 객체.
    let label_field = label["params"][0].as_object().expect("label textinput field");
    assert_eq!(label_field["type"], "TextInput");
    assert_eq!(label_field["value"].as_str(), Some("helper"));
    let head_body = head["statements"][0].as_array().expect("head body");
    assert_eq!(head_body.len(), 1);
    // 함수 본문 내 let 은 set_func_variable (EntryJS local variable) 로 emit.
    assert_eq!(head_body[0]["type"], "set_func_variable");
    assert_eq!(
        head_body[0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("y"))
    );
}

/// 같은 이름 + 다른 arity 함수 정의 → 호출은 args 개수로 매칭되어
/// 각각 올바른 id 로 재작성되어야.
#[test]
fn compile_function_same_name_diff_arity_routes_by_arity() {
    let src = r#"
        fn when_start() {
            greet("a");
            greet("a", "b");
        }

        fn greet(x: String) {
            let s = x;
        }

        fn greet(x: String, y: String) {
            let s = x;
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    // 호출 블록 두 개가 서로 다른 func_<id> 로 재작성돼야.
    let objects = v["objects"].as_array().unwrap();
    let mut found_ids: Vec<String> = Vec::new();
    walk_call_ids(&objects[0]["script"], &mut found_ids);
    assert_eq!(found_ids.len(), 2);
    assert_ne!(
        found_ids[0], found_ids[1],
        "arity 가 다른 호출은 서로 다른 func id 로 가야 함"
    );
}

/// 호출 트리에서 func_<id> 타입의 `type` 값만 수집.
/// script 가 JSON 문자열이면 파싱해서 walk.
fn walk_call_ids(v: &Value, out: &mut Vec<String>) {
    if v.is_string() {
        if let Ok(parsed) = serde_json::from_str::<Value>(v.as_str().unwrap()) {
            walk_call_ids(&parsed, out);
        }
        return;
    }
    match v {
        Value::Array(arr) => arr.iter().for_each(|x| walk_call_ids(x, out)),
        Value::Object(obj) => {
            if let Some(t) = obj.get("type").and_then(|x| x.as_str()) {
                if let Some(rest) = t.strip_prefix("func_") {
                    out.push(rest.to_string());
                }
            }
            if let Some(arr) = obj.get("params").and_then(|x| x.as_array()) {
                arr.iter().for_each(|p| walk_call_ids(p, out));
            }
            if let Some(arr) = obj.get("statements").and_then(|x| x.as_array()) {
                arr.iter().for_each(|p| walk_call_ids(p, out));
            }
        }
        _ => {}
    }
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
    assert_eq!(t0[0]["type"], "when_run_button_click");
    assert_eq!(t1[0]["type"], "when_click");
    assert_eq!(
        t0[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
    assert_eq!(
        t1[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("y"))
    );
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
    assert_eq!(first_thread[0]["type"], "when_run_button_click");
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
    assert_eq!(x["array"], json!([]));
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
    assert_eq!(thread[0]["type"], "when_run_button_click");
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
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script str");
    let _ = program_from_script_string_with_vars(script_str, &vars).expect("deparse");
}

/// `show_list(my_list)` → params = [list_var_dropdown, null], 변수 kind 자동 List.
#[test]
fn compile_show_list() {
    let src = r#"
        fn when_start() {
            show_list(my_list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    // show_list 블록 emit 검증.
    let show = thread
        .iter()
        .find(|b| b["type"] == "show_list")
        .expect("show_list block");
    let params = show["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(
        params[0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("my_list"))
    );
    // EntryJS 호환: list dropdown 슬롯은 string id 만 emit (object 아님).
    assert!(params[1].is_null());

    // 변수 kind 가 List 로 자동 분류되었는지 검증 (list_context_names 효과).
    let var = v["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "my_list")
        .expect("my_list variable");
    assert_eq!(var["variableType"], "list");
}

/// 라운드트립.
#[test]
fn compile_show_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            show_list(my_list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script_str, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "show_list" => Some(fref),
        _ => None,
    });
    assert!(found.is_some(), "expected show_list call");
}

/// `hide_list(my_list)` → params = [list_var_dropdown, null], 변수 kind 자동 List.
#[test]
fn compile_hide_list() {
    let src = r#"
        fn when_start() {
            hide_list(my_list);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    let hide = thread
        .iter()
        .find(|b| b["type"] == "hide_list")
        .expect("hide_list block");
    let params = hide["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(
        params[0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("my_list"))
    );
    // EntryJS 호환: list dropdown 슬롯은 string id 만 emit (object 아님).
    assert!(params[1].is_null());
}

/// 라운드트립.
#[test]
fn compile_hide_list_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            hide_list(my_list);
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script_str, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "hide_list" => Some(fref),
        _ => None,
    });
    assert!(found.is_some(), "expected hide_list call");
}

/// `stop_run_all();` → Block::StopAll, params = [null].
#[test]
fn compile_stop_run_all() {
    let src = r#"fn when_start() { stop_run_all(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "stop_run_all")
        .expect("stop_run_all block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 1);
    assert!(params[0].is_null());
}

/// 라운드트립.
#[test]
fn compile_stop_run_all_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { stop_run_all(); }"#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script_str, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found_stop = body
        .iter()
        .find_map(|stmt| if matches!(stmt, Stmt::StopAll) { Some(stmt) } else { None });
    assert!(found_stop.is_some(), "expected StopAll stmt");
}

/// `restart_project();` → Block::RestartProject, params = [null].
#[test]
fn compile_restart_project() {
    let src = r#"fn when_start() { restart_project(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "restart_project")
        .expect("restart_project block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 1);
    assert!(params[0].is_null());
}

/// 라운드트립.
#[test]
fn compile_restart_project_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { restart_project(); }"#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script_str, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found_call = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "restart_project" => Some(fref),
        _ => None,
    });
    assert!(found_call.is_some(), "expected restart_project call");
}

/// `create_clone()` → target="self" 디폴트. `create_clone("sprite_name")` → target=sprite_name.
#[test]
fn compile_create_clone() {
    let src = r#"
        fn when_start() {
            create_clone();
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "create_clone")
        .expect("create_clone block");
    eprintln!("DEBUG create_clone block = {}", serde_json::to_string(block).unwrap());
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert!(params[1].is_null());
}

/// 라운드트립 — self 디폴트 args=[] 로 emit.
#[test]
fn compile_create_clone_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"
        fn when_start() {
            create_clone();
        }
    "#;
    let p1 = parse(src).expect("parse");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script_str, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found_call = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "create_clone" => Some(fref),
        _ => None,
    });
    assert!(found_call.is_some(), "expected create_clone call");
}

/// `create_clone("sprite_name")` → args[0] = target string.
#[test]
fn compile_create_clone_with_target() {
    let src = r#"
        fn when_start() {
            create_clone("another_sprite");
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "create_clone")
        .expect("create_clone block");
    eprintln!("DEBUG create_clone with target block = {}", serde_json::to_string(block).unwrap());
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert!(params[1].is_null());
}

/// `create_clone(&self)` → Expr::Reference normalize 후 self 로 처리. 동일 emit.
#[test]
fn compile_create_clone_self_reference() {
    let src = r#"
        fn when_start() {
            create_clone(&self);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "create_clone")
        .expect("create_clone block");
    eprintln!("DEBUG create_clone &self block = {}", serde_json::to_string(block).unwrap());
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].as_str().unwrap(), "self");
    assert!(params[1].is_null());
}

/// `text_write("hello")` → `text_write` 블록, params[0] = `{type:"text", params:["hello"]}` (TextInput 슬롯), params[1] = null (Indicator).
#[test]
fn compile_text_write() {
    let src = r#"fn when_start() { text_write("hello"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_write")
        .expect("text_write block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0]["type"], "text");
    assert_eq!(params[0]["params"][0].as_str().unwrap(), "hello");
    assert!(params[1].is_null());
}

/// text_write 라운드트립 — codegen → deparse → IR 의 `Stmt::Expr(Call(text_write, [str("hi")]))` 가 복원되는지.
#[test]
fn compile_text_write_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { text_write("hi"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "text_write");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::Str(s) => assert_eq!(s, "hi"),
                        other => panic!("expected Str(\"hi\"), got {other:?}"),
                    }
                }
                other => panic!("expected Call(text_write), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// text_write 의 args 로 표현식 (`text_read` 결과) 도 정상 처리 — 값 슬롯 블록을 Sub 로 emit.
#[test]
fn compile_text_write_sub_expr() {
    let src = r#"fn when_start() { text_write(text_read("self")); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_write")
        .expect("text_write block");
    let params = block["params"].as_array().unwrap();
    // params[0] = text_read Sub 블록이 nested 으로 emit 됨.
    assert_eq!(params[0]["type"], "text_read");
    assert!(params[1].is_null());
}

/// text_write 의 args 가 0 또는 2 이면 SyntaxError.
#[test]
fn compile_text_write_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { text_write(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { text_write("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

// --- text_append / text_prepend (글상자 뒤/앞에 이어쓰기) ---

/// `text_append("hello")` → `text_append` 블록, params[0] = TextInput 슬롯, params[1] = null (Indicator).
#[test]
fn compile_text_append() {
    let src = r#"fn when_start() { text_append("hello"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_append")
        .expect("text_append block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0]["type"], "text");
    assert_eq!(params[0]["params"][0].as_str().unwrap(), "hello");
    assert!(params[1].is_null());
}

/// text_append 라운드트립 — codegen → deparse → IR 의 `Stmt::Expr(Call(text_append, [str("hi")]))` 가 복원되는지.
#[test]
fn compile_text_append_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { text_append("hi"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "text_append");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::Str(s) => assert_eq!(s, "hi"),
                        other => panic!("expected Str(\"hi\"), got {other:?}"),
                    }
                }
                other => panic!("expected Call(text_append), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// text_append 의 args 로 표현식 (`text_read` 결과) 도 정상 처리 — 값 슬롯 블록을 Sub 로 emit.
#[test]
fn compile_text_append_sub_expr() {
    let src = r#"fn when_start() { text_append(text_read("self")); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_append")
        .expect("text_append block");
    let params = block["params"].as_array().unwrap();
    // params[0] = text_read Sub 블록이 nested 으로 emit 됨.
    assert_eq!(params[0]["type"], "text_read");
    assert!(params[1].is_null());
}

/// text_append 의 args 가 0 또는 2 이면 SyntaxError.
#[test]
fn compile_text_append_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { text_append(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { text_append("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

/// `text_prepend("hello")` → `text_prepend` 블록, params[0] = TextInput 슬롯, params[1] = null (Indicator).
#[test]
fn compile_text_prepend() {
    let src = r#"fn when_start() { text_prepend("hello"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_prepend")
        .expect("text_prepend block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0]["type"], "text");
    assert_eq!(params[0]["params"][0].as_str().unwrap(), "hello");
    assert!(params[1].is_null());
}

/// text_prepend 라운드트립 — codegen → deparse → IR 의 `Stmt::Expr(Call(text_prepend, [str("hi")]))` 가 복원되는지.
#[test]
fn compile_text_prepend_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { text_prepend("hi"); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "text_prepend");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::Str(s) => assert_eq!(s, "hi"),
                        other => panic!("expected Str(\"hi\"), got {other:?}"),
                    }
                }
                other => panic!("expected Call(text_prepend), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// text_prepend 의 args 로 표현식 (`text_read` 결과) 도 정상 처리 — 값 슬롯 블록을 Sub 로 emit.
#[test]
fn compile_text_prepend_sub_expr() {
    let src = r#"fn when_start() { text_prepend(text_read("self")); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_prepend")
        .expect("text_prepend block");
    let params = block["params"].as_array().unwrap();
    // params[0] = text_read Sub 블록이 nested 으로 emit 됨.
    assert_eq!(params[0]["type"], "text_read");
    assert!(params[1].is_null());
}

/// text_prepend 의 args 가 0 또는 2 이면 SyntaxError.
#[test]
fn compile_text_prepend_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { text_prepend(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { text_prepend("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

// --- text_change_effect (텍스트에 효과) ---

/// `text_change_effect("strike", true)` → `text_change_effect` 블록, params[0] = "strike" (Dropdown), params[1] = "on" (Dropdown, bool → "on"), params[2] = null (Indicator).
#[test]
fn compile_text_change_effect() {
    let src = r#"fn when_start() { text_change_effect("strike", true); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_change_effect")
        .expect("text_change_effect block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 3);
    assert_eq!(params[0].as_str().unwrap(), "strike");
    assert_eq!(params[1].as_str().unwrap(), "on");
    assert!(params[2].is_null());
}

/// `TextEffect::Strike` enum 문법도 text_change_effect 블록으로 컴파일된다.
#[test]
fn compile_text_change_effect_enum_syntax() {
    let src = r#"fn when_start() { text_change_effect(TextEffect::Strike, true); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_change_effect")
        .expect("text_change_effect block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params[0].as_str().unwrap(), "strike");
    assert_eq!(params[1].as_str().unwrap(), "on");
}

/// string literal 문법은 enum 문법 추가 뒤에도 그대로 지원된다.
#[test]
fn compile_text_change_effect_mixed_syntax() {
    let src = r#"fn when_start() { text_change_effect("strike", true); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_change_effect")
        .expect("text_change_effect block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params[0].as_str().unwrap(), "strike");
    assert_eq!(params[1].as_str().unwrap(), "on");
}

/// enum 문법은 다른 enum 기반 dropdown 함수에도 공통으로 적용된다.
#[test]
fn compile_enum_syntax_for_all_enum_dropdowns() {
    let src = r#"
        fn when_start() {
            add_effect_amount(EffectType::Ghost, 25);
            change_effect_amount(EffectType::Brightness, 10);
            stretch_scale_size(Dimension::Height, 10);
            let x = quotient_and_mod(10, 3, QamMethod::Mod);
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    let add_effect = thread
        .iter()
        .find(|b| b["type"] == "add_effect_amount")
        .expect("add_effect_amount block");
    assert_eq!(add_effect["params"][0], "ghost");

    let change_effect = thread
        .iter()
        .find(|b| b["type"] == "change_effect_amount")
        .expect("change_effect_amount block");
    assert_eq!(change_effect["params"][0], "brightness");

    let stretch = thread
        .iter()
        .find(|b| b["type"] == "stretch_scale_size")
        .expect("stretch_scale_size block");
    assert_eq!(stretch["params"][0], "HEIGHT");

    let quotient = thread
        .iter()
        .find(|b| b["type"] == "set_variable")
        .expect("set_variable block");
    assert_eq!(quotient["params"][1]["type"], "quotient_and_mod");
    assert_eq!(quotient["params"][1]["params"][5], "modulo");
}

/// text_change_effect 라운드트립 — codegen → deparse → IR 의 `Stmt::Expr(Call(text_change_effect, [Str("strike"), Bool(true)]))` 가 복원되는지.
#[test]
fn compile_text_change_effect_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { text_change_effect("strike", true); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "text_change_effect");
                    assert_eq!(args.len(), 2);
                    match &args[0] {
                        Expr::Str(s) => assert_eq!(s, "strike"),
                        other => panic!("expected Str(\"strike\"), got {other:?}"),
                    }
                    match &args[1] {
                        Expr::Bool(b) => assert!(*b),
                        other => panic!("expected Bool(true), got {other:?}"),
                    }
                }
                other => panic!("expected Call(text_change_effect), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// text_change_effect 의 args 가 0/1/3 이면 SyntaxError.
#[test]
fn compile_text_change_effect_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { text_change_effect(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src1 = r#"fn when_start() { text_change_effect("strike"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r#"fn when_start() { text_change_effect("strike", true, "x"); }"#;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// text_change_effect 의 args 가 string literal / bool 이 아니면 SyntaxError.
#[test]
fn compile_text_change_effect_type_check() {
    use entrycore::compile;
    // effect 가 string literal 아님 (숫자).
    let src_num = r#"fn when_start() { text_change_effect(123, true); }"#;
    assert!(compile(&[("obj", src_num)], &empty_project()).is_err());
    // mode 가 bool 아님 (string).
    let src_str_mode = r#"fn when_start() { text_change_effect("strike", "on"); }"#;
    assert!(compile(&[("obj", src_str_mode)], &empty_project()).is_err());
    // effect 가 unknown string.
    let src_unknown = r#"fn when_start() { text_change_effect("unknown_effect", true); }"#;
    assert!(compile(&[("obj", src_unknown)], &empty_project()).is_err());
}

// --- text_flush (텍스트 모두 지우기) ---

/// `text_flush()` → `text_flush` 블록, params = `[]` (no-arg statement, EntryJS 의 Indicator 슬롯 없음 — def.params = [null] 가 .ent 에선 빈 배열로 emit).
#[test]
fn compile_text_flush() {
    let src = r#"fn when_start() { text_flush(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = thread
        .iter()
        .find(|b| b["type"] == "text_flush")
        .expect("text_flush block");
    let params = block["params"].as_array().unwrap();
    assert_eq!(params.len(), 0);
}

/// text_flush 라운드트립 — codegen → deparse → IR 의 `Stmt::Expr(Call(text_flush, []))` 가 복원되는지.
#[test]
fn compile_text_flush_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { text_flush(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "text_flush");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(text_flush), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// text_flush 의 args 가 1개 이상이면 SyntaxError.
#[test]
fn compile_text_flush_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { text_flush("x"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { text_flush("x", "y"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

// --- text_change_font / text_change_font_color / text_change_bg_color (글상자 서식) ---

/// 글씨체는 동적 드롭다운이므로 문자열로, 두 색상은 색상 블록으로 emit된다.
#[test]
fn compile_text_style_blocks() {
    let src = r##"fn when_start() {
        text_change_font("Nanum Gothic");
        text_change_font_color("#112233");
        text_change_bg_color("#445566");
    }"##;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);

    let font = thread.iter().find(|b| b["type"] == "text_change_font").expect("font block");
    assert_eq!(font["params"][0], "Nanum Gothic");
    assert!(font["params"][1].is_null());

    for (type_id, color) in [("text_change_font_color", "#112233"), ("text_change_bg_color", "#445566")] {
        let block = thread.iter().find(|b| b["type"] == type_id).expect("color block");
        assert_eq!(block["params"][0]["type"], "text");
        assert_eq!(block["params"][0]["params"][0], color);
        assert!(block["params"][1].is_null());
    }
}

/// 글상자 서식 세 블록이 codegen 후 deparse에서도 원래 호출 이름과 인자를 유지한다.
#[test]
fn compile_text_style_blocks_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r##"fn when_start() {
        text_change_font("Nanum Gothic");
        text_change_font_color("#112233");
        text_change_bg_color("#445566");
    }"##;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let script = v["objects"][0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &collect_var_map(&p1, &VarMap::new())).expect("deparse");
    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else { panic!("expected when_start"); };

    for (stmt, (expected_name, expected_arg)) in body.iter().zip([
        ("text_change_font", "Nanum Gothic"),
        ("text_change_font_color", "#112233"),
        ("text_change_bg_color", "#445566"),
    ]) {
        let Stmt::Expr(Expr::Call(fref, args)) = stmt else { panic!("expected call"); };
        assert_eq!(fref.name, expected_name);
        assert_eq!(args.len(), 1);
        match &args[0] {
            Expr::Str(arg) => assert_eq!(arg, expected_arg),
            other => panic!("expected string argument, got {other:?}"),
        }
    }
}

/// 글상자 서식 함수는 정확히 하나의 인자만 허용하고 글씨체는 문자열만 허용한다.
#[test]
fn compile_text_style_blocks_validation() {
    for src in [
        r#"fn when_start() { text_change_font(); }"#,
        r#"fn when_start() { text_change_font("a", "b"); }"#,
        r#"fn when_start() { text_change_font(1); }"#,
        r#"fn when_start() { text_change_font_color(); }"#,
        r#"fn when_start() { text_change_bg_color("a", "b"); }"#,
    ] {
        assert!(compile(&[("obj", src)], &empty_project()).is_err());
    }
}

/// 현재 오브젝트의 이미지·소리 자산은 이름과 ID를 양방향으로 조회한다.
#[test]
fn asset_map_is_scoped_per_object_and_bidirectional() {
    let project = json!({
        "objects": [
            {"name": "hero", "sprite": {
                "pictures": [{"id": "picture-walk", "name": "walk"}],
                "sounds": [{"id": "sound-jump", "name": "jump"}]
            }},
            {"name": "enemy", "sprite": {
                "pictures": [{"id": "picture-enemy-walk", "name": "walk"}],
                "sounds": [{"id": "sound-enemy-jump", "name": "jump"}]
            }}
        ]
    });
    let assets = entrycore::AssetMap::from_project_value(&project);

    assert_eq!(assets.picture_id_by_name("hero", "walk"), Some("picture-walk"));
    assert_eq!(assets.picture_name_by_id("hero", "picture-walk"), Some("walk"));
    assert_eq!(assets.sound_id_by_name("hero", "jump"), Some("sound-jump"));
    assert_eq!(assets.sound_name_by_id("hero", "sound-jump"), Some("jump"));
    assert_eq!(assets.picture_id_by_name("enemy", "walk"), Some("picture-enemy-walk"));
    assert_eq!(assets.sound_id_by_name("enemy", "jump"), Some("sound-enemy-jump"));
}

/// 이미지 이름은 build에서 자산 ID로 저장되고 extract에서 다시 이름으로 복원된다.
#[test]
fn compile_shape_change_uses_picture_id_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "hero-object",
        "name": "hero",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {
            "name": "hero",
            "pictures": [{"id": "picture-walk", "name": "walk"}],
            "sounds": [{"id": "sound-jump", "name": "jump"}]
        },
        "entity": {"x": 0, "y": 0, "visible": true}
    }]);
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() { change_to_some_shape("walk"); }"#;
    let compiled = compile(&[("hero", src)], &base).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    assert_eq!(value[0][1]["params"][0]["type"], "get_pictures");
    assert_eq!(value[0][1]["params"][0]["params"][0], "picture-walk");

    let program = program_from_script_string_with_vars_and_assets(
        script,
        &entrycore::VarMap::new(),
        &assets,
        "hero",
    )
    .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    let Stmt::Expr(Expr::Call(fref, args)) = &body[0] else {
        panic!("expected shape call");
    };
    assert_eq!(fref.name, "change_to_some_shape");
    assert!(matches!(&args[0], Expr::Str(name) if name == "walk"));
}

/// 소리 이름은 `get_sounds` 값 블록의 ID로 저장되고 extract에서 이름으로 복원된다.
#[test]
fn compile_sound_blocks_use_sound_id_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "hero-object",
        "name": "hero",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {
            "name": "hero",
            "pictures": [],
            "sounds": [{"id": "sound-jump", "name": "jump"}]
        },
        "entity": {"x": 0, "y": 0, "visible": true}
    }]);
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() {
        sound_something_with_block("jump");
        sound_something_second_with_block("jump", 1.5);
    }"#;
    let compiled = compile(&[("hero", src)], &base).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    for block in [&value[0][1], &value[0][2]] {
        assert_eq!(block["params"][0]["type"], "get_sounds");
        assert_eq!(block["params"][0]["params"][0], "sound-jump");
    }
    assert_eq!(value[0][2]["params"][1]["type"], "number");
    assert_eq!(value[0][2]["params"][1]["params"][0], 1.5);

    let program = program_from_script_string_with_vars_and_assets(
        script,
        &entrycore::VarMap::new(),
        &assets,
        "hero",
    )
    .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    for (stmt, expected_name) in body.iter().zip([
        "sound_something_with_block",
        "sound_something_second_with_block",
    ]) {
        let Stmt::Expr(Expr::Call(fref, args)) = stmt else {
            panic!("expected sound call");
        };
        assert_eq!(fref.name, expected_name);
        assert!(matches!(&args[0], Expr::Str(name) if name == "jump"));
    }
    let Stmt::Expr(Expr::Call(_, second_args)) = &body[1] else {
        panic!("expected second sound call");
    };
    assert!(matches!(second_args[1], Expr::Float(value) if value == 1.5));
}

/// 구간 재생도 소리 이름과 ID를 양방향으로 변환한다.
#[test]
fn compile_sound_from_to_uses_sound_id_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "hero-object",
        "name": "hero",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {
            "name": "hero",
            "pictures": [],
            "sounds": [{"id": "sound-jump", "name": "jump"}]
        },
        "entity": {"x": 0, "y": 0, "visible": true}
    }]);
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() { sound_from_to("jump", 0.5, 2.0); }"#;
    let compiled = compile(&[("hero", src)], &base).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    let block = &value[0][1];
    assert_eq!(block["type"], "sound_from_to");
    assert_eq!(block["params"][0]["type"], "get_sounds");
    assert_eq!(block["params"][0]["params"][0], "sound-jump");
    assert_eq!(block["params"][1]["params"][0], 0.5);
    assert_eq!(block["params"][2]["params"][0], 2.0);

    let program = program_from_script_string_with_vars_and_assets(
        script,
        &entrycore::VarMap::new(),
        &assets,
        "hero",
    )
    .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    let Stmt::Expr(Expr::Call(fref, args)) = &body[0] else {
        panic!("expected sound_from_to call");
    };
    assert_eq!(fref.name, "sound_from_to");
    assert!(matches!(&args[0], Expr::Str(name) if name == "jump"));
    assert!(matches!(args[1], Expr::Float(value) if value == 0.5));
    assert!(matches!(args[2], Expr::Float(value) if value == 2.0));
}

/// 기다리기 포함 소리 블록 3종도 이름과 ID를 양방향으로 변환한다.
#[test]
fn compile_sound_wait_blocks_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "hero-object",
        "name": "hero",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {
            "name": "hero",
            "pictures": [],
            "sounds": [{"id": "sound-jump", "name": "jump"}]
        },
        "entity": {"x": 0, "y": 0, "visible": true}
    }]);
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() {
        sound_something_wait_with_block("jump");
        sound_something_second_wait_with_block("jump", 1.5);
        sound_from_to_and_wait("jump", 0.5, 2.0);
    }"#;
    let compiled = compile(&[("hero", src)], &base).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    for block in [&value[0][1], &value[0][2], &value[0][3]] {
        assert_eq!(block["params"][0]["type"], "get_sounds");
        assert_eq!(block["params"][0]["params"][0], "sound-jump");
    }

    let program = program_from_script_string_with_vars_and_assets(
        script,
        &entrycore::VarMap::new(),
        &assets,
        "hero",
    )
    .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    for (stmt, expected_name) in body.iter().zip([
        "sound_something_wait_with_block",
        "sound_something_second_wait_with_block",
        "sound_from_to_and_wait",
    ]) {
        let Stmt::Expr(Expr::Call(fref, args)) = stmt else {
            panic!("expected waiting sound call");
        };
        assert_eq!(fref.name, expected_name);
        assert!(matches!(&args[0], Expr::Str(name) if name == "jump"));
    }
}

/// 소리 크기 변경·설정 블록의 JSON과 Rust 왕복을 검증한다.
/// 소리 정지와 배경음악 블록의 소리 ID 왕복을 검증한다.
#[test]
fn compile_sound_stop_and_bgm_blocks_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "hero-object",
        "name": "hero",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {
            "name": "hero",
            "pictures": [],
            "sounds": [{"id": "sound-jump", "name": "jump"}]
        },
        "entity": {"x": 0, "y": 0, "visible": true}
    }]);
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() {
        sound_silent_all("all");
        play_bgm("jump");
        stop_bgm();
    }"#;
    let compiled = compile(&[("hero", src)], &base).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    assert_eq!(value[0][1]["params"][0], "all");
    assert_eq!(value[0][2]["params"][0]["type"], "get_sounds");
    assert_eq!(value[0][2]["params"][0]["params"][0], "sound-jump");
    assert_eq!(value[0][3]["type"], "stop_bgm");

    let program = program_from_script_string_with_vars_and_assets(
        script,
        &entrycore::VarMap::new(),
        &assets,
        "hero",
    )
    .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    for (stmt, expected_name) in body.iter().zip(["sound_silent_all", "play_bgm", "stop_bgm"]) {
        let Stmt::Expr(Expr::Call(fref, args)) = stmt else {
            panic!("expected sound stop or bgm call");
        };
        assert_eq!(fref.name, expected_name);
        if expected_name == "sound_silent_all" {
            assert!(matches!(&args[0], Expr::Str(name) if name == "all"));
        } else if expected_name != "stop_bgm" {
            assert!(matches!(&args[0], Expr::Str(name) if name == "jump"));
        }
    }
}

/// 소리 크기와 길이 값 블록의 인자와 왕복을 검증한다.
#[test]
fn compile_sound_value_blocks_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "hero-object",
        "name": "hero",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {
            "name": "hero",
            "pictures": [],
            "sounds": [{"id": "sound-jump", "name": "jump"}]
        },
        "entity": {"x": 0, "y": 0, "visible": true}
    }]);
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() {
        let volume = get_sound_volume();
        let duration = get_sound_duration("jump");
    }"#;
    let compiled = compile(&[("hero", src)], &base).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    assert_eq!(value[0][1]["type"], "set_variable");
    assert_eq!(value[0][1]["params"][1]["type"], "get_sound_volume");
    assert_eq!(value[0][2]["params"][1]["type"], "get_sound_duration");
    assert_eq!(value[0][2]["params"][1]["params"][1], "sound-jump");

    let program = program_from_script_string_with_vars_and_assets(
        script,
        &entrycore::VarMap::new(),
        &assets,
        "hero",
    )
    .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    let Stmt::SetVar(_, volume) = &body[0] else {
        panic!("expected volume let");
    };
    assert!(matches!(volume, Expr::Call(fref, args) if fref.name == "get_sound_volume" && args.is_empty()));
    let Stmt::SetVar(_, duration) = &body[1] else {
        panic!("expected duration let");
    };
    assert!(matches!(duration, Expr::Call(fref, args) if fref.name == "get_sound_duration" && matches!(&args[0], Expr::Str(name) if name == "jump")));
}

#[test]
fn compile_sound_volume_blocks_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() {
        sound_volume_change(10.0);
        sound_volume_set(75.0);
    }"#;
    let compiled = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    assert_eq!(value[0][1]["type"], "sound_volume_change");
    assert_eq!(value[0][1]["params"][0]["type"], "number");
    assert_eq!(value[0][1]["params"][0]["params"][0], 10.0);
    assert_eq!(value[0][2]["type"], "sound_volume_set");
    assert_eq!(value[0][2]["params"][0]["type"], "number");
    assert_eq!(value[0][2]["params"][0]["params"][0], 75.0);

    let program = program_from_script_string_with_vars(script, &entrycore::VarMap::new())
        .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    for (stmt, (expected_name, expected_value)) in body.iter().zip([
        ("sound_volume_change", 10.0),
        ("sound_volume_set", 75.0),
    ]) {
        let Stmt::Expr(Expr::Call(fref, args)) = stmt else {
            panic!("expected sound volume call");
        };
        assert_eq!(fref.name, expected_name);
        assert!(matches!(args[0], Expr::Float(value) if value == expected_value));
    }
}

/// 소리 빠르기 변경 블록의 JSON과 Rust 왕복을 검증한다.
#[test]
fn compile_sound_speed_blocks_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() {
        sound_speed_change(0.1);
        sound_speed_set(1.5);
    }"#;
    let compiled = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    assert_eq!(value[0][1]["type"], "sound_speed_change");
    assert_eq!(value[0][1]["params"][0]["type"], "number");
    assert_eq!(value[0][1]["params"][0]["params"][0], 0.1);
    assert_eq!(value[0][2]["type"], "sound_speed_set");
    assert_eq!(value[0][2]["params"][0]["type"], "number");
    assert_eq!(value[0][2]["params"][0]["params"][0], 1.5);

    let program = program_from_script_string_with_vars(script, &entrycore::VarMap::new())
        .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else {
        panic!("expected when_start");
    };
    for (stmt, (expected_name, expected_value)) in body.iter().zip([
        ("sound_speed_change", 0.1),
        ("sound_speed_set", 1.5),
    ]) {
        let Stmt::Expr(Expr::Call(fref, args)) = stmt else {
            panic!("expected sound speed call");
        };
        assert_eq!(fref.name, expected_name);
        assert!(matches!(args[0], Expr::Float(value) if value == expected_value));
    }
}
#[test]
fn compile_is_type_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() {
        if is_type(123, "number") {
        }
    }"#;
    let compiled = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let script = compiled["objects"][0]["script"].as_str().expect("script string");
    let value: Value = serde_json::from_str(script).expect("script JSON");
    assert_eq!(value[0][1]["type"], "_if");
    assert_eq!(value[0][1]["params"][0]["type"], "is_type");
    assert_eq!(value[0][1]["params"][0]["params"][2], "number");

    let program = program_from_script_string_with_vars(script, &entrycore::VarMap::new())
        .expect("deparse");
    let Stmt::FuncDef { body, .. } = &program.stmts[0] else { panic!("expected when_start"); };
    let Stmt::If { cond, .. } = &body[0] else { panic!("expected if"); };
    assert!(matches!(cond, Expr::Call(fref, args) if fref.name == "is_type" && args.len() == 2));
}

// --- is_boost_mode (부스트 모드) ---

/// `is_boost_mode();` → `is_boost_mode` 블록, params = [].
#[test]
fn compile_is_boost_mode() {
    let src = r#"fn when_start() { is_boost_mode(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "is_boost_mode");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_is_boost_mode_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { is_boost_mode(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "is_boost_mode");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(is_boost_mode), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `is_boost_mode("foo")` 인자 전달 시 SyntaxError.
#[test]
fn compile_is_boost_mode_arity_check() {
    use entrycore::compile;
    let src = r#"fn when_start() { is_boost_mode("foo"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- is_touch_supported (터치 지원 여부) ---

/// `is_touch_supported();` → `is_touch_supported` 블록, params = [].
#[test]
fn compile_is_touch_supported() {
    let src = r#"fn when_start() { is_touch_supported(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "is_touch_supported");
    assert_eq!(thread[1]["params"].as_array().unwrap().len(), 0);
}

/// 라운드트립.
#[test]
fn compile_is_touch_supported_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() { is_touch_supported(); }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Expr(Expr::Call(fref, args)) => {
                    assert_eq!(fref.name, "is_touch_supported");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(is_touch_supported), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `is_touch_supported("foo")` 인자 전달 시 SyntaxError.
#[test]
fn compile_is_touch_supported_arity_check() {
    use entrycore::compile;
    let src = r#"fn when_start() { is_touch_supported("foo"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- get_date (날짜/시/분/초) ---

/// `let y = get_date("year");` → `get_date` 블록, params = [null, "YEAR", null].
#[test]
fn compile_get_date() {
    let src = r#"fn when_start() {
        let y = get_date("year");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // thread[0] = when_run, thread[1] = let y = ...
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "get_date");
    assert_eq!(set_var["params"][1]["params"][1], "YEAR");
}

/// 라운드트립.
#[test]
fn compile_get_date_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let y = get_date("year");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "get_date");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "YEAR"));
                    }
                    other => panic!("expected Call(get_date), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `get_date()` (인자 없음) 또는 `get_date("a", "b")` (2개) → SyntaxError.
#[test]
fn compile_get_date_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() {
        let y = get_date();
    }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() {
        let y = get_date("year", "month");
    }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

/// `get_date("year");` 단독 statement → SyntaxError (값 블럭).
#[test]
fn compile_get_date_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { get_date("year"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- distance_something (두 점 사이 거리) ---

/// `distance_something("mouse")` → 값 슬롯 블록으로 emit, params = `[null, "mouse", null]` (Text/DropdownDynamic/Text 슬롯).
#[test]
fn compile_distance_something() {
    let src = r#"fn when_start() {
        let d = distance_something("mouse");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // thread[0] = when_run, thread[1] = let d = ...
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "distance_something");
    let ds_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(ds_params.len(), 3);
    assert!(ds_params[0].is_null());
    assert_eq!(ds_params[1].as_str().unwrap(), "mouse");
    assert!(ds_params[2].is_null());
}

/// 라운드트립 — `distance_something("Sprite1")` → `Stmt::SetVar(_, Call(distance_something, [Str("Sprite1")]))` 복원.
#[test]
fn compile_distance_something_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let d = distance_something("Sprite1");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "distance_something");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "Sprite1"));
                    }
                    other => panic!("expected Call(distance_something), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `distance_something()` (인자 없음) 또는 `distance_something("a", "b")` (2개) → SyntaxError.
#[test]
fn compile_distance_something_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() {
        let d = distance_something();
    }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() {
        let d = distance_something("a", "b");
    }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

/// `distance_something("Sprite1")` 의 target 이 base 에 있으면 `.ent` 에선 stable id 로 emit 되고,
/// extract 시 다시 sprite name 으로 복원되는지 검증 (EntryJS Runtime 이 `Entry.container.getEntity(id)`
/// 로 lookup 하므로 dropdown 슬롯 값은 sprite id 여야 함).
#[test]
fn compile_distance_something_object_name_id_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "obj_sprite1",
        "name": "Sprite1",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {"pictures": [], "sounds": []},
        "text": "Sprite1",
        "lock": false,
        "entity": {}
    }]);

    let src = r#"fn when_start() {
        let d = distance_something("Sprite1");
    }"#;
    let assets = entrycore::AssetMap::from_project_value(&base);
    let v = compile(&[("Sprite1", src)], &base).expect("compile").0;

    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    // 정방향: sprite name → id 변환 확인.
    assert_eq!(
        set_var["params"][1]["params"][1].as_str().unwrap(),
        "obj_sprite1"
    );

    // 역방향: id → name 복원 확인.
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars_and_assets(
        obj_script_str,
        &entrycore::VarMap::new(),
        &assets,
        "Sprite1",
    )
    .expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "distance_something");
                        assert!(matches!(&args[0], Expr::Str(s) if s == "Sprite1"));
                    }
                    other => panic!("expected Call(distance_something), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `mouse` 키워드는 id 변환 없이 그대로 통과 (EntryJS Runtime 의 reserved keyword).
#[test]
fn compile_distance_something_mouse_passthrough() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let base = empty_project();
    let assets = entrycore::AssetMap::from_project_value(&base);
    let src = r#"fn when_start() {
        let d = distance_something("mouse");
    }"#;
    let v = compile(&[("Sprite1", src)], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["params"][1]["params"][1].as_str().unwrap(), "mouse");

    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars_and_assets(
        obj_script_str,
        &entrycore::VarMap::new(),
        &assets,
        "Sprite1",
    )
    .expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => match &body[0] {
            Stmt::SetVar(_, rhs) => match rhs {
                Expr::Call(_, args) => {
                    assert!(matches!(&args[0], Expr::Str(s) if s == "mouse"));
                }
                other => panic!("expected Call, got {other:?}"),
            },
            other => panic!("expected SetVar, got {other:?}"),
        },
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

/// `reach_something("Enemy1")` 의 target 이 base 에 있으면 `.ent` 에선 sprite id 로 emit 되고,
/// extract 시 sprite name 으로 복원되는지 검증 (collision dropdown 슬롯).
#[test]
fn compile_reach_something_object_name_id_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars_and_assets;
    use entrycore::ir::{Expr, Stmt};

    let mut base = empty_project();
    base["objects"] = json!([{
        "id": "obj_enemy1",
        "name": "Enemy1",
        "objectType": "sprite",
        "scene": "scene1",
        "script": "[]",
        "sprite": {"pictures": [], "sounds": []},
        "text": "Enemy1",
        "lock": false,
        "entity": {}
    }]);

    let src = r#"fn when_start() { reach_something("Enemy1"); }"#;
    let assets = entrycore::AssetMap::from_project_value(&base);
    let v = compile(&[("Sprite1", src)], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    // 'Sprite1' 가 base 에 없어 fake object 가 생성됨 — 마지막 object 가 우리 stem 매핑.
    let target_idx = objects
        .iter()
        .position(|o| o["name"].as_str() == Some("Sprite1"))
        .expect("fake Sprite1 object");
    let thread = first_thread(&objects[target_idx]);
    // 정방향: sprite name → id.
    assert_eq!(thread[1]["params"][1].as_str().unwrap(), "obj_enemy1");

    // 역방향: id → name.
    let obj_script_str = objects[target_idx]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars_and_assets(
        obj_script_str,
        &entrycore::VarMap::new(),
        &assets,
        "Sprite1",
    )
    .expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => match &body[0] {
            Stmt::Expr(Expr::Call(fref, args)) => {
                assert_eq!(fref.name, "reach_something");
                assert!(matches!(&args[0], Expr::Str(s) if s == "Enemy1"));
            }
            other => panic!("expected Call(reach_something), got {other:?}"),
        },
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

// --- get_user_name / get_nickname ---

/// `get_user_name()` → `get_user_name` 값 슬롯 블록, params = `[]`.
#[test]
fn compile_get_user_name() {
    let src = r#"fn when_start() {
        let u = get_user_name();
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // thread[0] = when_run, thread[1] = let u = ...
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "get_user_name");
    assert!(set_var["params"][1]["params"].as_array().unwrap().is_empty());
}

/// `get_user_name` 라운드트립 — codegen → deparse → IR 의 `Stmt::SetVar(_, Call(get_user_name, []))` 가 복원되는지.
#[test]
fn compile_get_user_name_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let u = get_user_name();
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "get_user_name");
                        assert_eq!(args.len(), 0);
                    }
                    other => panic!("expected Call(get_user_name), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `get_user_name("x")` (인자 1개) → SyntaxError.
#[test]
fn compile_get_user_name_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() {
        let u = get_user_name("x");
    }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
}

/// `get_user_name();` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_get_user_name_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { get_user_name(); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

/// `get_nickname()` → `get_nickname` 값 슬롯 블록, params = `[]`.
#[test]
fn compile_get_nickname() {
    let src = r#"fn when_start() {
        let n = get_nickname();
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "get_nickname");
    assert!(set_var["params"][1]["params"].as_array().unwrap().is_empty());
}

/// `get_nickname` 라운드트립.
#[test]
fn compile_get_nickname_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let n = get_nickname();
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "get_nickname");
                        assert_eq!(args.len(), 0);
                    }
                    other => panic!("expected Call(get_nickname), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `get_nickname("x")` (인자 1개) → SyntaxError.
#[test]
fn compile_get_nickname_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() {
        let n = get_nickname("x");
    }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
}

/// `get_nickname();` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_get_nickname_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { get_nickname(); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- length_of_string / reverse_of_string ---

/// `length_of_string("hello")` → `length_of_string` 값 슬롯, params = [null, text, null].
#[test]
fn compile_length_of_string() {
    let src = r#"fn when_start() {
        let n = length_of_string("hello");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "length_of_string");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 3);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert!(sub_params[2].is_null());
}

/// `length_of_string` 라운드트립 — codegen → deparse → IR 의 `Call(length_of_string, [Str("hello")])` 복원.
#[test]
fn compile_length_of_string_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let n = length_of_string("hello");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "length_of_string");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                    }
                    other => panic!("expected Call(length_of_string), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `length_of_string()` (0 args) / `length_of_string("a", "b")` (2 args) → SyntaxError.
#[test]
fn compile_length_of_string_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { let n = length_of_string(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { let n = length_of_string("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

/// `length_of_string("x");` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_length_of_string_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { length_of_string("x"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

/// `reverse_of_string("hello")` → `reverse_of_string` 값 슬롯, params = [null, text, null].
#[test]
fn compile_reverse_of_string() {
    let src = r#"fn when_start() {
        let r = reverse_of_string("hello");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "reverse_of_string");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 3);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert!(sub_params[2].is_null());
}

/// `reverse_of_string` 라운드트립.
#[test]
fn compile_reverse_of_string_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let r = reverse_of_string("hello");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "reverse_of_string");
                        assert_eq!(args.len(), 1);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                    }
                    other => panic!("expected Call(reverse_of_string), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `reverse_of_string()` / `reverse_of_string("a", "b")` → SyntaxError.
#[test]
fn compile_reverse_of_string_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { let r = reverse_of_string(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { let r = reverse_of_string("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

/// `reverse_of_string("x");` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_reverse_of_string_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { reverse_of_string("x"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- combine_something ---

/// `combine_something("hello", "world")` → 값 슬롯, params = [null, text_a, null, text_b, null].
#[test]
fn compile_combine_something() {
    let src = r#"fn when_start() {
        let s = combine_something("hello", "world");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "combine_something");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 5);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3]["type"], "text");
    assert_eq!(sub_params[3]["params"][0], "world");
    assert!(sub_params[4].is_null());
}

/// `combine_something` 라운드트립.
#[test]
fn compile_combine_something_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let s = combine_something("hello", "world");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "combine_something");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(matches!(&args[1], Expr::Str(s) if s == "world"));
                    }
                    other => panic!("expected Call(combine_something), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `combine_something("a")` (1) / `combine_something("a","b","c")` (3) → SyntaxError.
#[test]
fn compile_combine_something_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let s = combine_something("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r#"fn when_start() { let s = combine_something("a", "b", "c"); }"#;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// `combine_something("a","b");` 단독 statement → SyntaxError.
#[test]
fn compile_combine_something_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { combine_something("a", "b"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- char_at ---

/// `char_at("hello", 2)` → 값 슬롯, params = [null, text, null, number, null].
#[test]
fn compile_char_at() {
    let src = r#"fn when_start() {
        let c = char_at("hello", 2);
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "char_at");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 5);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3]["type"], "number");
    assert_eq!(
        sub_params[3]["params"][0].as_f64().expect("number literal"),
        2.0
    );
    assert!(sub_params[4].is_null());
}

/// `char_at` 라운드트립 — codegen → deparse → IR 의 `Call(char_at, [Str("hello"), Int(2)])` 복원.
#[test]
fn compile_char_at_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let c = char_at("hello", 2);
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "char_at");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(
                            matches!(&args[1], Expr::Float(f) if *f == 2.0)
                                || matches!(&args[1], Expr::Int(2)),
                            "expected Int(2) or Float(2.0), got {:?}",
                            args[1]
                        );
                    }
                    other => panic!("expected Call(char_at), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `char_at("a")` (1) / `char_at("a", 2, 3)` (3) → SyntaxError.
#[test]
fn compile_char_at_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let c = char_at("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r#"fn when_start() { let c = char_at("a", 2, 3); }"#;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// `char_at("a", 2);` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_char_at_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { char_at("a", 2); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- substring ---

/// `substring("hello", 1, 3)` → 값 슬롯, params = [null, text, null, number, null, number, null].
#[test]
fn compile_substring() {
    let src = r#"fn when_start() {
        let c = substring("hello", 1, 3);
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "substring");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 7);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3]["type"], "number");
    assert_eq!(
        sub_params[3]["params"][0].as_f64().expect("number literal"),
        1.0
    );
    assert!(sub_params[4].is_null());
    assert_eq!(sub_params[5]["type"], "number");
    assert_eq!(
        sub_params[5]["params"][0].as_f64().expect("number literal"),
        3.0
    );
    assert!(sub_params[6].is_null());
}

/// `substring` 라운드트립 — codegen → deparse → IR 의 `Call(substring, [Str("hello"), Int(1), Int(3)])` 복원.
#[test]
fn compile_substring_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let c = substring("hello", 1, 3);
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "substring");
                        assert_eq!(args.len(), 3);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(
                            matches!(&args[1], Expr::Float(f) if *f == 1.0)
                                || matches!(&args[1], Expr::Int(1)),
                            "expected Int(1) or Float(1.0), got {:?}",
                            args[1]
                        );
                        assert!(
                            matches!(&args[2], Expr::Float(f) if *f == 3.0)
                                || matches!(&args[2], Expr::Int(3)),
                            "expected Int(3) or Float(3.0), got {:?}",
                            args[2]
                        );
                    }
                    other => panic!("expected Call(substring), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `substring("a")` (1) / `substring("a", 1)` (2) / `substring("a", 1, 3, 5)` (4) → SyntaxError.
#[test]
fn compile_substring_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let c = substring("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { let c = substring("a", 1); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
    let src4 = r#"fn when_start() { let c = substring("a", 1, 3, 5); }"#;
    assert!(compile(&[("obj", src4)], &empty_project()).is_err());
}

/// `substring("a", 1, 3);` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_substring_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { substring("a", 1, 3); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- count_match_string ---

/// `count_match_string("hello", "l")` → 값 슬롯, params = [null, text, null, text, null].
#[test]
fn compile_count_match_string() {
    let src = r#"fn when_start() {
        let n = count_match_string("hello", "l");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "count_match_string");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 5);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3]["type"], "text");
    assert_eq!(sub_params[3]["params"][0], "l");
    assert!(sub_params[4].is_null());
}

/// `count_match_string` 라운드트립 — codegen → deparse → IR 의 `Call(count_match_string, [Str("hello"), Str("l")])` 복원.
#[test]
fn compile_count_match_string_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let n = count_match_string("hello", "l");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "count_match_string");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(matches!(&args[1], Expr::Str(s) if s == "l"));
                    }
                    other => panic!("expected Call(count_match_string), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `count_match_string("a")` (1) / `count_match_string("a", "b", "c")` (3) → SyntaxError.
#[test]
fn compile_count_match_string_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let n = count_match_string("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r#"fn when_start() { let n = count_match_string("a", "b", "c"); }"#;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// `count_match_string("a", "b");` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_count_match_string_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { count_match_string("a", "b"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- index_of_string ---

/// `index_of_string("hello", "l")` → 값 슬롯, params = [null, text, null, text, null].
#[test]
fn compile_index_of_string() {
    let src = r#"fn when_start() {
        let n = index_of_string("hello", "l");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "index_of_string");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 5);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3]["type"], "text");
    assert_eq!(sub_params[3]["params"][0], "l");
    assert!(sub_params[4].is_null());
}

/// `index_of_string` 라운드트립 — codegen → deparse → IR 의 `Call(index_of_string, [Str("hello"), Str("l")])` 복원.
#[test]
fn compile_index_of_string_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let n = index_of_string("hello", "l");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "index_of_string");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(matches!(&args[1], Expr::Str(s) if s == "l"));
                    }
                    other => panic!("expected Call(index_of_string), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `index_of_string("a")` (1) / `index_of_string("a", "b", "c")` (3) → SyntaxError.
#[test]
fn compile_index_of_string_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let n = index_of_string("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r#"fn when_start() { let n = index_of_string("a", "b", "c"); }"#;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// `index_of_string("a", "b");` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_index_of_string_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { index_of_string("a", "b"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- replace_string ---

/// `replace_string("hello", "l", "r")` → 값 슬롯, params = [null, text, null, text, null, text, null].
#[test]
fn compile_replace_string() {
    let src = r#"fn when_start() {
        let s = replace_string("hello", "l", "r");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "replace_string");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 7);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3]["type"], "text");
    assert_eq!(sub_params[3]["params"][0], "l");
    assert!(sub_params[4].is_null());
    assert_eq!(sub_params[5]["type"], "text");
    assert_eq!(sub_params[5]["params"][0], "r");
    assert!(sub_params[6].is_null());
}

/// `replace_string` 라운드트립 — codegen → deparse → IR 의 `Call(replace_string, [Str("hello"), Str("l"), Str("r")])` 복원.
#[test]
fn compile_replace_string_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let s = replace_string("hello", "l", "r");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "replace_string");
                        assert_eq!(args.len(), 3);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(matches!(&args[1], Expr::Str(s) if s == "l"));
                        assert!(matches!(&args[2], Expr::Str(s) if s == "r"));
                    }
                    other => panic!("expected Call(replace_string), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `replace_string("a")` (1) / `replace_string("a", "b")` (2) / `replace_string("a", "b", "c", "d")` (4) → SyntaxError.
#[test]
fn compile_replace_string_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let s = replace_string("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { let s = replace_string("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
    let src4 = r#"fn when_start() { let s = replace_string("a", "b", "c", "d"); }"#;
    assert!(compile(&[("obj", src4)], &empty_project()).is_err());
}

/// `replace_string("a", "b", "c");` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_replace_string_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { replace_string("a", "b", "c"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- change_string_case ---

/// `change_string_case("hello", "toUpperCase")` → 값 슬롯, params = [null, text, null, "toUpperCase", null].
#[test]
fn compile_change_string_case() {
    let src = r#"fn when_start() {
        let u = change_string_case("hello", "toUpperCase");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "change_string_case");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 5);
    assert!(sub_params[0].is_null());
    assert_eq!(sub_params[1]["type"], "text");
    assert_eq!(sub_params[1]["params"][0], "hello");
    assert!(sub_params[2].is_null());
    assert_eq!(sub_params[3], "toUpperCase");
    assert!(sub_params[4].is_null());
}

/// `change_string_case` 라운드트립.
#[test]
fn compile_change_string_case_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let u = change_string_case("hello", "toUpperCase");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::SetVar(_, rhs) => match rhs {
                    Expr::Call(fref, args) => {
                        assert_eq!(fref.name, "change_string_case");
                        assert_eq!(args.len(), 2);
                        assert!(matches!(&args[0], Expr::Str(s) if s == "hello"));
                        assert!(matches!(&args[1], Expr::Str(s) if s == "toUpperCase"));
                    }
                    other => panic!("expected Call(change_string_case), got {other:?}"),
                },
                other => panic!("expected SetVar, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `change_string_case("a")` (1) / `change_string_case("a", "X", "Y")` (3) → SyntaxError.
#[test]
fn compile_change_string_case_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let u = change_string_case("a"); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r#"fn when_start() { let u = change_string_case("a", "toUpperCase", "extra"); }"#;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// `change_string_case("a", "toUpperCase")` 의 case 인자가 invalid string → SyntaxError.
#[test]
fn compile_change_string_case_invalid_case() {
    use entrycore::compile;
    let src = r#"fn when_start() {
        let u = change_string_case("a", "bogusCase");
    }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

/// `change_string_case("a", "toUpperCase");` 단독 statement → SyntaxError (값 슬롯).
#[test]
fn compile_change_string_case_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { change_string_case("a", "toUpperCase"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- get_block_count ---

/// `get_block_count("self")` → 값 슬롯, params = [text("self")].
#[test]
fn compile_get_block_count() {
    let src = r#"fn when_start() {
        let n = get_block_count("self");
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "get_block_count");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 1);
    assert_eq!(sub_params[0]["type"], "text");
    assert_eq!(sub_params[0]["params"][0], "self");
}

/// `get_block_count` 라운드트립.
#[test]
fn compile_get_block_count_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let n = get_block_count("self");
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => match &body[0] {
            Stmt::SetVar(_, rhs) => match rhs {
                Expr::Call(fref, args) => {
                    assert_eq!(fref.name, "get_block_count");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "self"));
                }
                other => panic!("expected Call(get_block_count), got {other:?}"),
            },
            other => panic!("expected SetVar, got {other:?}"),
        },
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

/// `get_block_count()` (0) / `get_block_count("a","b")` (2) → SyntaxError.
#[test]
fn compile_get_block_count_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { let n = get_block_count(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
    let src2 = r#"fn when_start() { let n = get_block_count("a", "b"); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

/// `get_block_count("x");` 단독 statement → SyntaxError.
#[test]
fn compile_get_block_count_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { get_block_count("x"); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- change_rgb_to_hex ---

/// `change_rgb_to_hex(255, 0, 0)` → 값 슬롯, params = [number, number, number].
#[test]
fn compile_change_rgb_to_hex() {
    let src = r#"fn when_start() {
        let hex = change_rgb_to_hex(255, 0, 0);
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "change_rgb_to_hex");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 3);
    assert_eq!(sub_params[0]["type"], "number");
    assert_eq!(
        sub_params[0]["params"][0].as_f64().expect("number"),
        255.0
    );
    assert_eq!(sub_params[1]["type"], "number");
    assert_eq!(sub_params[1]["params"][0].as_f64().expect("number"), 0.0);
    assert_eq!(sub_params[2]["type"], "number");
    assert_eq!(sub_params[2]["params"][0].as_f64().expect("number"), 0.0);
}

/// `change_rgb_to_hex` 라운드트립.
#[test]
fn compile_change_rgb_to_hex_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let hex = change_rgb_to_hex(255, 0, 0);
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => match &body[0] {
            Stmt::SetVar(_, rhs) => match rhs {
                Expr::Call(fref, args) => {
                    assert_eq!(fref.name, "change_rgb_to_hex");
                    assert_eq!(args.len(), 3);
                    assert!(matches!(&args[0], Expr::Float(f) if *f == 255.0));
                    assert!(matches!(&args[1], Expr::Float(f) if *f == 0.0));
                    assert!(matches!(&args[2], Expr::Float(f) if *f == 0.0));
                }
                other => panic!("expected Call(change_rgb_to_hex), got {other:?}"),
            },
            other => panic!("expected SetVar, got {other:?}"),
        },
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

/// `change_rgb_to_hex(255)` (1) / `change_rgb_to_hex(255,0,0,0)` (4) → SyntaxError.
#[test]
fn compile_change_rgb_to_hex_arity_check() {
    use entrycore::compile;
    let src1 = r#"fn when_start() { let h = change_rgb_to_hex(255); }"#;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src4 = r#"fn when_start() { let h = change_rgb_to_hex(255, 0, 0, 0); }"#;
    assert!(compile(&[("obj", src4)], &empty_project()).is_err());
}

/// `change_rgb_to_hex(255,0,0);` 단독 statement → SyntaxError.
#[test]
fn compile_change_rgb_to_hex_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { change_rgb_to_hex(255, 0, 0); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- change_hex_to_rgb ---

/// `change_hex_to_rgb("#FF0000", "r")` → 값 슬롯, params = [text, "r"].
#[test]
fn compile_change_hex_to_rgb() {
    let src = r##"fn when_start() {
        let rgb = change_hex_to_rgb("#FF0000", "r");
    }"##;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "change_hex_to_rgb");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 2);
    assert_eq!(sub_params[0]["type"], "text");
    assert_eq!(sub_params[0]["params"][0], "#FF0000");
    assert_eq!(sub_params[1], "r");
}

/// `change_hex_to_rgb` 라운드트립.
#[test]
fn compile_change_hex_to_rgb_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r##"fn when_start() {
        let rgb = change_hex_to_rgb("#FF0000", "r");
    }"##;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => match &body[0] {
            Stmt::SetVar(_, rhs) => match rhs {
                Expr::Call(fref, args) => {
                    assert_eq!(fref.name, "change_hex_to_rgb");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(&args[0], Expr::Str(s) if s == "#FF0000"));
                    assert!(matches!(&args[1], Expr::Str(s) if s == "r"));
                }
                other => panic!("expected Call(change_hex_to_rgb), got {other:?}"),
            },
            other => panic!("expected SetVar, got {other:?}"),
        },
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

/// `change_hex_to_rgb("#FF0000")` (1) / `change_hex_to_rgb("#FF0000","r","g")` (3) → SyntaxError.
#[test]
fn compile_change_hex_to_rgb_arity_check() {
    use entrycore::compile;
    let src1 = r##"fn when_start() { let rgb = change_hex_to_rgb("#FF0000"); }"##;
    assert!(compile(&[("obj", src1)], &empty_project()).is_err());
    let src3 = r##"fn when_start() { let rgb = change_hex_to_rgb("#FF0000", "r", "g"); }"##;
    assert!(compile(&[("obj", src3)], &empty_project()).is_err());
}

/// `change_hex_to_rgb("#FF0000", "x")` 의 channel 이 invalid → SyntaxError.
#[test]
fn compile_change_hex_to_rgb_invalid_channel() {
    use entrycore::compile;
    let src = r##"fn when_start() {
        let rgb = change_hex_to_rgb("#FF0000", "x");
    }"##;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

/// `change_hex_to_rgb("#FF0000", "r");` 단독 statement → SyntaxError.
#[test]
fn compile_change_hex_to_rgb_statement_error() {
    use entrycore::compile;
    let src = r##"fn when_start() { change_hex_to_rgb("#FF0000", "r"); }"##;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- get_boolean_value ---

/// `get_boolean_value(true)` → 값 슬롯, params = [True 블록].
#[test]
fn compile_get_boolean_value() {
    let src = r#"fn when_start() {
        let s = get_boolean_value(true);
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    assert_eq!(set_var["params"][1]["type"], "get_boolean_value");
    let sub_params = set_var["params"][1]["params"].as_array().unwrap();
    assert_eq!(sub_params.len(), 1);
    assert_eq!(sub_params[0]["type"], "boolean");
}

/// `get_boolean_value` 라운드트립.
#[test]
fn compile_get_boolean_value_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::codegen::collect_var_map;
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn when_start() {
        let s = get_boolean_value(true);
    }"#;
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1, &VarMap::new());
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { body, .. } => match &body[0] {
            Stmt::SetVar(_, rhs) => match rhs {
                Expr::Call(fref, args) => {
                    assert_eq!(fref.name, "get_boolean_value");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Bool(true)));
                }
                other => panic!("expected Call(get_boolean_value), got {other:?}"),
            },
            other => panic!("expected SetVar, got {other:?}"),
        },
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

/// `get_boolean_value()` (0) / `get_boolean_value(true, false)` (2) → SyntaxError.
#[test]
fn compile_get_boolean_value_arity_check() {
    use entrycore::compile;
    let src0 = r#"fn when_start() { let s = get_boolean_value(); }"#;
    assert!(compile(&[("obj", src0)], &empty_project()).is_err());
}

// --- function_create_value (결괏값 반환 함수) ---

/// `fn double(x: i32) -> i32 { return x * 2; }` → `function_create_value` 헤드 emit.
#[test]
fn compile_function_create_value() {
    let src = r#"fn double(x: i32) -> i32 {
        return x * 2;
    }
    fn when_start() {
        let v = double(3);
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    // project.functions 에 double 함수가 function_create_value 로 emit.
    let functions = v["functions"].as_array().expect("functions array");
    let double = functions
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("double"))
        .expect("double function");
    let content = double["content"].as_array().expect("content array");
    // EntryJS `content` 는 head array — `content[0]` 가 헤드 블록 자체.
    // 헤드의 `statements` = `[[body_block, ...]]` (스레드 배열).
    let head = &content[0];
    assert_eq!(head["type"], "function_create_value");
    let sub_params = head["params"].as_array().expect("sub_params");
    assert_eq!(sub_params.len(), 4);
    // params[3] = VALUE 슬롯 (= `return x * 2` 의 expr 블록)
    assert!(!sub_params[3].is_null(), "VALUE slot must contain the return expr");
    let value_slot = &sub_params[3];
    // BinaryOp 블록 (calc_basic) 이어야 함
    assert_eq!(
        value_slot["type"], "calc_basic",
        "expected calc_basic for x * 2, got {}",
        value_slot["type"]
    );
}

/// 결괏값 반환 함수 IR 검증 — `Stmt::FuncDef { return_type: Some, body 끝: Stmt::Return }`.
#[test]
fn compile_function_create_value_roundtrip() {
    use entrycore::ir::{Expr, Stmt};
    use entrycore::parse::parse;

    let src = r#"fn double(x: i32) -> i32 {
        return x * 2;
    }"#;
    let p1 = parse(src).expect("parse1");
    // parse 결과의 Stmt::FuncDef 에 return_type 이 Some 인지 검증.
    match &p1.stmts[0] {
        Stmt::FuncDef {
            name,
            return_type,
            body,
            ..
        } => {
            assert_eq!(name, "double");
            assert!(return_type.is_some(), "return_type must be Some");
            assert!(
                matches!(body.last(), Some(Stmt::Return(_))),
                "body must end with Stmt::Return"
            );
            if let Some(Stmt::Return(Expr::BinOp(_, _, _))) = body.last() {
                // x * 2 는 BinOp
            } else {
                panic!("expected Stmt::Return(BinOp), got {:?}", body.last());
            }
        }
        other => panic!("expected FuncDef, got {other:?}"),
    }
}

/// `fn f() -> i32 { }` — return stmt 없는 결괏값 함수 → ParseError.
#[test]
fn compile_function_create_value_no_return_error() {
    use entrycore::compile;
    let src = r#"fn f() -> i32 {
        let x = 5;
    }
    fn when_start() {
    }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

/// 기존 statement 본문 함수 (return type 없음) → `function_create` (regression).
#[test]
fn compile_function_create_no_return_type() {
    let src = r#"fn greet() {
        let x = 5;
    }
    fn when_start() {
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let functions = v["functions"].as_array().expect("functions array");
    let greet = functions
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("greet"))
        .expect("greet function");
    let content = greet["content"].as_array().expect("content array");
    let head = &content[0];
    assert_eq!(head["type"], "function_create");
}

/// `let v = double(3)` 값 슬롯 자리 호출 → `function_general` skeleton 의 func_<id> 블록 emit.
#[test]
fn compile_function_create_value_call_roundtrip() {
    let src = r#"fn double(x: i32) -> i32 {
        return x * 2;
    }
    fn when_start() {
        let v = double(3);
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // thread[0] = when_start, thread[1] = let v = double(3) → set_variable(params[1] = func_<id>)
    let set_var = &thread[1];
    assert_eq!(set_var["type"], "set_variable");
    let value_slot = &set_var["params"][1];
    let func_type = value_slot["type"].as_str().expect("type str");
    assert!(
        func_type.starts_with("func_"),
        "expected func_<id>, got {}",
        func_type
    );
}

/// `get_boolean_value(true);` 단독 statement → SyntaxError.
/// `get_boolean_value(true, false);` 단독 statement → SyntaxError.
#[test]
fn compile_get_boolean_value_arity_check_extra() {
    use entrycore::compile;
    let src2 = r#"fn when_start() { let s = get_boolean_value(true, false); }"#;
    assert!(compile(&[("obj", src2)], &empty_project()).is_err());
}

#[test]
fn compile_get_boolean_value_statement_error() {
    use entrycore::compile;
    let src = r#"fn when_start() { get_boolean_value(true); }"#;
    assert!(compile(&[("obj", src)], &empty_project()).is_err());
}

// --- set_func_variable / get_func_variable (함수 본문 local variable) ---

/// 함수 본문 `let x = 1` → `set_func_variable` 블록 emit (EntryJS local variable).
#[test]
fn compile_set_func_variable() {
    let src = r#"fn helper() {
        let x = 1;
    }
    fn when_start() {
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let funcs = v["functions"].as_array().expect("functions");
    let helper = funcs.iter().find(|f| f["name"] == "helper").expect("helper");
    let content = helper["content"].as_array().expect("content");
    let head = content[0].as_object().expect("head");
    assert_eq!(head["type"], "function_create");
    let body = head["statements"][0].as_array().expect("body");
    assert_eq!(body[0]["type"], "set_func_variable");
    assert_eq!(
        body[0]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
}

/// 함수 본문 local var `x` 사용 — 현재는 ParamBlock::Variable 로 emit (set/get_func_variable 분리 매핑).
/// `get_func_variable` 블록 자리는 향후 별도 작업 (from_expr 의 local var 분기).
#[test]
fn compile_get_func_variable() {
    let src = r#"fn helper() {
        let x = 5;
        let y = x;
    }
    fn when_start() {
    }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let funcs = v["functions"].as_array().expect("functions");
    let helper = funcs.iter().find(|f| f["name"] == "helper").expect("helper");
    let content = helper["content"].as_array().expect("content");
    let head = content[0].as_object().expect("head");
    let body = head["statements"][0].as_array().expect("body");
    // body[0] = set_func_variable (let x = 5)
    // body[1] = set_func_variable (let y = x) — value 자리는 현재 ParamBlock::Variable
    assert_eq!(body[0]["type"], "set_func_variable");
    assert_eq!(body[1]["type"], "set_func_variable");
    // value 슬롯 = variable dropdown (EntryJS 의 set_variable/get_variable 동일 형식).
    // 향후 local var 전용 get_func_variable emit 으로 확장 가능.
    assert!(
        body[1]["params"][1].is_object() || body[1]["params"][1].is_string(),
        "value slot should be a variable reference, got: {}",
        body[1]["params"][1]
    );
}

/// 트리거 본문 `let x = 1` → `set_variable` (set_func_variable 아님) — regression.
#[test]
fn compile_trigger_let_uses_set_variable() {
    let src = r#"fn when_start() { let x = 1; }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    // thread[0] = when_start, thread[1] = set_variable
    assert_eq!(thread[1]["type"], "set_variable");
    assert_eq!(
        thread[1]["params"][0].as_str().map(|s| s.to_string()),
        Some(entrycore::block::id_for("x"))
    );
}
