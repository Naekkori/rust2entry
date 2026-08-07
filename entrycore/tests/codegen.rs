//! parse -> codegen 통합 테스트.

use entrycore::codegen::{collect_var_map, generate};
use entrycore::deparse::program_from_script_value_with_vars;
use entrycore::parse::parse;

#[test]
fn simple_set_var() {
    let src = r#"
        fn when_start() {
            let x = 42;
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let scripts = json.get("scripts").expect("scripts");
    let arr = scripts.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let block = &arr[0];
    assert_eq!(block.get("type").and_then(|v| v.as_str()), Some("set_variable"));
    let params = block.get("params").and_then(|v| v.as_array()).expect("params");
    assert_eq!(params[0].get("name").and_then(|v| v.as_str()), Some("x"));
    assert!(params[1].get("type").is_some(), "value param not null");
}

#[test]
fn arithmetic_block() {
    let src = r#"
        fn when_start() {
            let y = 1 + 2;
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "set_variable");
    // value는 calc_block (sub-block)
    let value = &block["params"][1];
    assert_eq!(value["type"], "calc_basic");
}

#[test]
fn if_block() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let x = 1;
            }
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "if");
    let cond = &block["params"][0];
    assert_eq!(cond["type"], "boolean_basic");
}

#[test]
fn function_call_stmt() {
    let src = r#"
        fn when_start() {
            greet();
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "function_call");
}

#[test]
fn for_range_expands_to_repeat() {
    let src = r#"
        fn when_start() {
            for i in 0..5 {
                let x = 1;
            }
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    // for i in 0..5 -> repeat_basic(5 - 0)
    assert_eq!(block["type"], "repeat_basic");
    let times = &block["params"][0];
    assert_eq!(times["type"], "calc_basic");
    // 슬롯: [lhs, op, rhs]
    assert_eq!(times["params"][1], "-");
    // 본문 thread: [set_variable i 0, set_variable x 1, change_variable i 1]
    let thread = block["statements"][0].as_array().expect("thread array");
    assert_eq!(thread.len(), 3);
    assert_eq!(thread[0]["type"], "set_variable");
    assert_eq!(thread[0]["params"][0]["name"], "i");
    assert_eq!(thread[1]["type"], "set_variable");
    assert_eq!(thread[2]["type"], "change_variable");
    assert_eq!(thread[2]["params"][0]["name"], "i");
}

/// 라운드트립: parse -> codegen -> deparse -> IR 구조 확인.
/// 단순한 set/if만 검증. for-range는 의미 보존(반복)만 확인.
#[test]
fn roundtrip_simple_set() {
    let src = "fn when_start() { let x = 42; }";
    let p1 = parse(src).expect("parse1");
    let json = generate(&p1).expect("generate");
    let vars = collect_var_map(&p1);
    // scripts = [set_variable_block]. deparse는 [[block,...]] 형태 기대.
    let scripts_wrapped = serde_json::json!([json["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars)
        .expect("deparse");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    // Entry `set_variable`은 VarDecl/SetVar 모두 표현 가능. 변수명만 보존 확인.
    let n1 = match &p1.stmts[0] {
        entrycore::ir::Stmt::VarDecl(n, _) | entrycore::ir::Stmt::SetVar(n, _) => n,
        other => panic!("p1[0] not var stmt: {other:?}"),
    };
    let n2 = match &p2.stmts[0] {
        entrycore::ir::Stmt::VarDecl(n, _) | entrycore::ir::Stmt::SetVar(n, _) => n,
        other => panic!("p2[0] not var stmt: {other:?}"),
    };
    assert_eq!(n1, n2, "variable name roundtrip");
}

#[test]
fn roundtrip_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = parse(src).expect("parse1");
    let json = generate(&p1).expect("generate");
    let vars = collect_var_map(&p1);
    let scripts_wrapped = serde_json::json!([json["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars)
        .expect("deparse");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    match (&p1.stmts[0], &p2.stmts[0]) {
        (
            entrycore::ir::Stmt::If { then_body: tb1, else_body: eb1, .. },
            entrycore::ir::Stmt::If { then_body: tb2, else_body: eb2, .. },
        ) => {
            assert_eq!(tb1.len(), tb2.len());
            assert_eq!(eb1.len(), eb2.len());
        }
        _ => panic!("roundtrip if mismatch"),
    }
}

/// for-range의 codegen 결과를 다시 IR로 풀었을 때
/// `Stmt::Repeat { times: BinOp(Sub, b, a), body: [SetVar(i,a), ..., ChangeVar(i,1)] }` 인지 확인.
#[test]
fn for_range_roundtrip_is_repeat() {
    let src = "fn when_start() { for i in 0..5 { let x = 1; } }";
    let p1 = parse(src).expect("parse1");
    let json = generate(&p1).expect("generate");
    let vars = collect_var_map(&p1);
    let scripts_wrapped = serde_json::json!([json["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars)
        .expect("deparse");
    // 첫 stmt는 Repeat.
    match &p2.stmts[0] {
        entrycore::ir::Stmt::Repeat { times, body } => {
            // times = 5 - 0
            match times {
                entrycore::ir::Expr::BinOp(entrycore::ir::BinOp::Sub, _, _) => {}
                other => panic!("expected BinOp::Sub, got {other:?}"),
            }
            // body 길이: [SetVar i, SetVar x, ChangeVar i]
            assert_eq!(body.len(), 3);
            assert!(matches!(&body[0], entrycore::ir::Stmt::SetVar(n, _) if n == "i"));
            assert!(matches!(&body[2], entrycore::ir::Stmt::SetVar(n, _) if n == "i"));
        }
        other => panic!("expected Repeat, got {other:?}"),
    }
}
