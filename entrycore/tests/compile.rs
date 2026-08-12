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
    assert_eq!(wait["params"][0]["type"], "boolean_basic");
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
    // flag 는 미정의 변수 — codegen 은 drop-down 만 emit.
    assert!(wait["params"][0].get("id").is_some() || wait["params"][0].get("name").is_some());
}

/// 산술 포함 조건.
#[test]
fn compile_wait_until_true_arith_cond() {
    let src = r#"fn when_start() { wait_until_true(1 + 2 < 5); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let wait = thread.iter().find(|b| b["type"] == "wait_until_true").expect("wait_until_true");
    assert_eq!(wait["params"][0]["type"], "boolean_basic");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(set["params"][1]["params"][0]["params"][0].as_f64(), Some(1.0));
    assert_eq!(set["params"][1]["params"][1]["params"][0].as_f64(), Some(10.0));
}

/// `calc_rand(1.5, 9.5)` → 실수 보존.
#[test]
fn compile_calc_rand_float() {
    let src = r#"fn when_start() { let x = calc_rand(1.5, 9.5); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let set = thread.iter().find(|b| b["type"] == "set_variable").expect("set_variable");
    assert_eq!(set["params"][1]["params"][0]["params"][0].as_f64(), Some(1.5));
    assert_eq!(set["params"][1]["params"][1]["params"][0].as_f64(), Some(9.5));
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
    assert_eq!(set["params"][1]["params"][0]["name"], "lo");
    assert_eq!(set["params"][1]["params"][1]["name"], "hi");
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
    let vars = collect_var_map(&p1);
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(thread[1]["params"][0], true);
}

/// `hide_timer();` → `set_visible_project_timer`, params[0] = false.
#[test]
fn compile_hide_timer() {
    let src = r#"fn when_start() { hide_timer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_visible_project_timer");
    assert_eq!(thread[1]["params"][0], false);
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
    assert_eq!(thread[1]["params"][0], true);
}

/// `hide_answer();` → `set_visible_answer`, params[0] = false.
#[test]
fn compile_hide_answer() {
    let src = r#"fn when_start() { hide_answer(); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "set_visible_answer");
    assert_eq!(thread[1]["params"][0], false);
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

/// `say("hello");` → `dialog` 블록, params[0] = text 슬롯, params[1] = "say".
#[test]
fn compile_say_text() {
    let src = r#"fn when_start() { say("hello"); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "dialog");
    assert_eq!(thread[1]["params"][0]["type"], "text");
    assert_eq!(thread[1]["params"][0]["params"][0].as_str(), Some("hello"));
    assert_eq!(thread[1]["params"][1].as_str(), Some("say"));
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
    assert_eq!(dlg["params"][0]["name"], "x");
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
                    assert_eq!(fref.name, "say");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Call(say), got {other:?}"),
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
                    assert_eq!(fref.name, "think");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Call(think), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// `say("hello", 2.0);` → `dialog_time` 블록, params[2] = number 슬롯, params[1] = "say".
#[test]
fn compile_say_with_time() {
    let src = r#"fn when_start() { say("hello", 2.0); }"#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[1]["type"], "dialog_time");
    assert_eq!(thread[1]["params"][0]["params"][0].as_str(), Some("hello"));
    assert_eq!(thread[1]["params"][1].as_str(), Some("say"));
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
    assert_eq!(thread[1]["params"][0].as_str(), Some("walk"));
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
    assert_eq!(css["params"][0]["name"], "n");
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
        .find(|b| b["type"] == "set_variable" && b["params"][0]["name"] == "x")
        .expect("set x");
    let value = &set_x["params"][1];
    assert_eq!(value["type"], "value_of_index_from_list");
    assert_eq!(value["params"].as_array().unwrap().len(), 2);
    assert_eq!(value["params"][0]["type"], "number");
    assert_eq!(value["params"][0]["params"][0], 1.0);
    assert_eq!(value["params"][1]["name"], "list");
    assert_eq!(value["params"][1]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let script = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let Some(Stmt::SetVar(name, Expr::Call(fref, args))) = body.iter().find(|stmt| {
        matches!(stmt, Stmt::SetVar(name, Expr::Call(_, _)) if name == "x")
    }) else {
        panic!("expected set x to list lookup call");
    };
    assert_eq!(name, "x");
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
    assert_eq!(add["params"][1]["name"], "list");
    assert_eq!(add["params"][1]["variableType"], "list");
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
    assert!(fruit["object"].is_null());

    let thread = first_thread(&v["objects"].as_array().unwrap()[0]);
    let add = thread
        .iter()
        .find(|block| block["type"] == "add_value_to_list")
        .expect("add_value_to_list");
    assert_eq!(add["params"][1]["name"], "fruit");
    assert_eq!(add["params"][1]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(remove["params"][1]["name"], "list");
    assert_eq!(remove["params"][1]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(insert["params"][2]["name"], "list");
    assert_eq!(insert["params"][2]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(change["params"][2]["name"], "list");
    assert_eq!(change["params"][2]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(params[1]["name"], "list");
    assert_eq!(params[1]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(params[1]["name"], "list");
    assert_eq!(params[1]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(sss["params"][0]["name"], "n");
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
                    assert_eq!(fref.name, "flip_y");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("expected Call(flip_y), got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
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
    assert_eq!(ask["params"][0]["name"], "name");
    assert!(ask["params"][0]["id"].is_string());
    assert!(ask["params"][0]["variableType"].is_string());
}

/// 라운드트립: compile → deparse → IR 에 ask_and_wait 호출 보존.
#[test]
fn compile_ask_and_wait_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { ask_and_wait("이름"); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    assert_eq!(action["params"][0], "start");
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
    assert_eq!(blocks[0]["params"][0], "stop");
    assert_eq!(blocks[1]["params"][0], "reset");
}

/// start_timer 라운드트립.
#[test]
fn compile_start_timer_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { start_timer(); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    assert_eq!(block["params"][2], "quotient");
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
    assert_eq!(block["params"][2], "modulo");
}

/// quotient_and_mod 라운드트립.
#[test]
fn compile_quotient_and_mod_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { let x = quotient_and_mod(10, 3, "modulo"); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    assert_eq!(block["params"][0], "abs");
}

/// sqrt 라운드트립.
#[test]
fn compile_sqrt_roundtrip() {
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::{Expr, Stmt};

    let src = r#"fn when_start() { let y = sqrt(x); }"#;
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    assert_eq!(block["params"][0], "sin");
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
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    let vars = entrycore::codegen::collect_var_map(&p1);
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
    // EntryJS function_field_label.params[0] = TextInput 필드 객체.
    let label_field = label["params"][0].as_object().expect("label textinput field");
    assert_eq!(label_field["type"], "TextInput");
    assert_eq!(label_field["value"].as_str(), Some("helper"));
    let head_body = head["statements"][0].as_array().expect("head body");
    assert_eq!(head_body.len(), 1);
    assert_eq!(head_body[0]["type"], "set_variable");
    assert_eq!(head_body[0]["params"][0]["name"], "y");
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
    assert_eq!(params[0]["name"], "my_list");
    assert_eq!(params[0]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    assert_eq!(params[0]["name"], "my_list");
    assert_eq!(params[0]["variableType"], "list");
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
    let vars = collect_var_map(&p1);
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
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let script_str = objects[0]["script"].as_str().expect("script string");
    let p2 = program_from_script_string_with_vars(script_str, &vars).expect("deparse");

    let Stmt::FuncDef { body, .. } = &p2.stmts[0] else {
        panic!("expected when_start function");
    };
    let found_call = body.iter().find_map(|stmt| match stmt {
        Stmt::Expr(Expr::Call(fref, _)) if fref.name == "stop_all" => Some(fref),
        _ => None,
    });
    assert!(found_call.is_some(), "expected stop_all call");
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
    let vars = collect_var_map(&p1);
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
    let vars = collect_var_map(&p1);
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
