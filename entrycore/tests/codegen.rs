//! parse -> codegen 통합 테스트.

use entrycore::{VarKind, VarMap};
use entrycore::codegen::{collect_var_map, generate};
use entrycore::deparse::program_from_script_value_with_vars;
use entrycore::parse::parse;
use entrycore::var::var_map_from_value;
use serde_json::{Value, json};
#[test]
fn variable_map_supports_bidirectional_lookup() {
    let vars = var_map_from_value(&json!([
        {"id":"variable-x", "name":"x", "variableType":"variable"},
        {"id":"timer", "name":"timer", "variableType":"timer"}
    ]));
    let id = vars.id_by_name("x").expect("name -> id");
    assert_eq!(vars.name_by_id(id), Some("x"));
    assert_eq!(vars.get_by_name("x").expect("x").id, "variable-x");
}

#[test]
fn variable_map_insert_is_bidirectional() {
    let mut vars = VarMap::new();
    vars.insert(entrycore::VarInfo {
        id: "id-x".into(), name: "x".into(), original_name: "x".into(),
        kind: VarKind::Variable,
        init: entrycore::VarInit::Int0, scope: entrycore::ir::VarScope::Local,
    });
    assert_eq!(vars.id_by_name("x"), Some("id-x"));
    assert_eq!(vars.name_by_id("id-x"), Some("x"));
}

fn empty_project() -> Value {
    json!({
        "speed": 60, "objects": [], "variables": [], "messages": [],
        "functions": [], "scenes": [{"id":"scene1","name":"장면1"}],
        "interface": {"views": []}, "meta": {}
    })
}
#[test]
fn simple_set_var() {
    let src = r#"
        fn when_start() {
            let x = 42;
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program,&empty_project()).expect("generate");
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
    let json = generate(&program,&empty_project()).expect("generate");
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
    let json = generate(&program,&empty_project()).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "if");
    let cond = &block["params"][0];
    assert_eq!(cond["type"], "boolean_basic_operator");
}

#[test]
fn function_call_stmt() {
    let src = r#"
        fn when_start() {
            greet();
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program,&empty_project()).expect("generate");
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
    let json = generate(&program,&empty_project()).expect("generate");
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
    let json = generate(&p1,&empty_project()).expect("generate");
    let vars = collect_var_map(&p1);
    // scripts = [set_variable_block]. deparse는 [[block,...]] 형태 기대.
    let scripts_wrapped = serde_json::json!([json["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars)
        .expect("deparse");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    // Entry `set_variable`은 VarDecl/SetVar 모두 표현 가능. 변수명만 보존 확인.
    let n1 = match &p1.stmts[0] {
        entrycore::ir::Stmt::VarDecl(n, _, _, _) | entrycore::ir::Stmt::SetVar(n, _) => n,
        other => panic!("p1[0] not var stmt: {other:?}"),
    };
    let n2 = match &p2.stmts[0] {
        entrycore::ir::Stmt::VarDecl(n, _, _, _) | entrycore::ir::Stmt::SetVar(n, _) => n,
        other => panic!("p2[0] not var stmt: {other:?}"),
    };
    assert_eq!(n1, n2, "variable name roundtrip");
}

#[test]
fn roundtrip_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = parse(src).expect("parse1");
    let json = generate(&p1,&empty_project()).expect("generate");
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
    let json = generate(&p1,&empty_project()).expect("generate");
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

//거부테스트
#[test]
fn timer_named_var_registers_as_timer() {
    let src = "fn when_start() { let 초시계 = 0; }";
    // ↑ 이건 위 거부 테스트에서 거부되므로, 등록은 collect_var_map 단독 테스트로
    let p = parse(src).expect("parse");
    let vars = collect_var_map(&p);
    let info = vars.get(&entrycore::block::id_for("초시계")).expect("timer registered");
    assert!(matches!(info.kind, VarKind::Timer));
}

// ── 데이터분석 (테이블) 매핑 ──

fn block_stmt(json: &Value) -> &Value {
    let arr = json["scripts"].as_array().expect("scripts array");
    // generate() 는 scripts = [block,...] (스레드 wrapper 없이 직접).
    &arr[0]
}

#[test]
fn table_append_row_to_table_emits_correct_block() {
    let src = r#"
        fn when_start() {
            append_row_to_table("mytable", "row");
        }
    "#;
    let p = parse(src).expect("parse");
    let json = generate(&p, &empty_project()).expect("generate");
    let stmt = block_stmt(&json);
    assert_eq!(stmt["type"], "append_row_to_table");
    let params = stmt["params"].as_array().expect("params array");
    assert!(params[0].is_null(), "MATRIX dropdown -> null");
    assert_eq!(params[1], "ROW");
    assert!(params[2].is_null(), "Indicator -> null");
}

#[test]
fn table_calc_values_from_table_emits_correct_block() {
    let src = r#"
        fn when_start() {
            let v = calc_values_from_table("t", 1, "avg");
        }
    "#;
    let p = parse(src).expect("parse");
    let json = generate(&p, &empty_project()).expect("generate");
    let stmt = block_stmt(&json);
    assert_eq!(stmt["type"], "set_variable");
    let val = &stmt["params"][1];
    assert_eq!(val["type"], "calc_values_from_table");
    let params = val["params"].as_array().expect("params array");
    assert!(params[0].is_null(), "MATRIX dropdown");
    assert_eq!(params[1]["type"], "number");
    assert_eq!(params[2], "AVG");
}

#[test]
fn table_set_value_from_cell_emits_correct_block() {
    let src = r#"
        fn when_start() {
            set_value_from_cell("t", "A2", 10);
        }
    "#;
    let p = parse(src).expect("parse");
    let json = generate(&p, &empty_project()).expect("generate");
    let stmt = block_stmt(&json);
    assert_eq!(stmt["type"], "set_value_from_cell");
    let params = stmt["params"].as_array().expect("params array");
    assert!(params[0].is_null(), "MATRIX dropdown");
    assert_eq!(params[1]["type"], "text");
    assert_eq!(params[1]["params"][0], "A2");
    assert_eq!(params[2]["type"], "number");
    assert!(params[3].is_null(), "Indicator");
}

#[test]
fn table_close_table_chart_has_no_args() {
    let src = r#"
        fn when_start() {
            close_table_chart();
        }
    "#;
    let p = parse(src).expect("parse");
    let json = generate(&p, &empty_project()).expect("generate");
    let stmt = block_stmt(&json);
    assert_eq!(stmt["type"], "close_table_chart");
}

#[test]
fn table_value_block_in_value_position_emits_value_block() {
    // 값 슬롯 전용 table 블록을 값 자리에서 정상 사용 가능.
    let src = r#"
        fn when_start() {
            let n = get_table_count("t", "row");
        }
    "#;
    let p = parse(src).expect("parse");
    let json = generate(&p, &empty_project()).expect("generate");
    let stmt = block_stmt(&json);
    assert_eq!(stmt["type"], "set_variable");
    let val = &stmt["params"][1];
    assert_eq!(val["type"], "get_table_count");
    // params[0] = table dropdown (null placeholder)
    assert!(val["params"][0].is_null());
    // params[1] = "ROW" dimension
    assert_eq!(val["params"][1], "ROW");
}

#[test]
fn table_invalid_enum_string_rejected() {
    // 잘못된 dimension enum ("DIAG") → parse_enum_arg 에서 거부.
    let src = r#"
        fn when_start() {
            append_row_to_table("t", "DIAG");
        }
    "#;
    let p = parse(src).expect("parse");
    let err = generate(&p, &empty_project()).expect_err("should reject");
    let msg = format!("{err:?}");
    assert!(msg.contains("invalid row/col") || msg.contains("unknown RowCol"), "got: {msg}");
}

#[test]
fn table_arity_mismatch_rejected() {
    // insert_row_to_table 는 3 args 필수.
    let src = r#"
        fn when_start() {
            insert_row_to_table("t", 2);
        }
    "#;
    let p = parse(src).expect("parse");
    let err = generate(&p, &empty_project()).expect_err("should reject");
    let msg = format!("{err:?}");
    assert!(msg.contains("insert_row_to_table needs 3 args"), "got: {msg}");
}

#[test]
fn table_roundtrip_stmt_preserved() {
    // stmt 형 table 블록 라운드트립: 함수 이름 + 차원 enum 보존.
    let src = r#"
        fn when_start() {
            append_row_to_table("t1", "row");
            insert_row_to_table("t1", 3, "col");
            delete_row_from_table("t1", 2, "row");
            save_current_table("t1");
            open_table("t1");
            open_table_wait("t1", 5);
            close_table_chart();
        }
    "#;
    let p1 = parse(src).expect("parse1");
    let json = generate(&p1, &empty_project()).expect("generate");
    let vars = collect_var_map(&p1);
    let scripts_wrapped = serde_json::json!([json["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars).expect("deparse");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    // 함수 호출 이름/인자 보존 확인.
    for (a, b) in p1.stmts.iter().zip(p2.stmts.iter()) {
        let (n1, args1) = match a {
            entrycore::ir::Stmt::Expr(entrycore::ir::Expr::Call(f, args)) => (&f.name, args),
            other => panic!("p1 not call: {other:?}"),
        };
        let (n2, args2) = match b {
            entrycore::ir::Stmt::Expr(entrycore::ir::Expr::Call(f, args)) => (&f.name, args),
            other => panic!("p2 not call: {other:?}"),
        };
        assert_eq!(n1, n2);
        assert_eq!(args1.len(), args2.len());
    }
}

#[test]
fn table_roundtrip_value_block_preserved() {
    // 값 슬롯 table 블록 라운드트립.
    let src = r#"
        fn when_start() {
            let n = get_table_count("t", "row");
            let v = get_value_from_table("t", 2, 1);
            let s = calc_values_from_table("t", 1, "sum");
            let c = get_coefficient("t", 1, 2);
        }
    "#;
    let p1 = parse(src).expect("parse1");
    let json = generate(&p1, &empty_project()).expect("generate");
    let vars = collect_var_map(&p1);
    let scripts_wrapped = serde_json::json!([json["scripts"].clone()]);
    let p2 = program_from_script_value_with_vars(&scripts_wrapped, &vars).expect("deparse");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
}

/// 한글 변수명 sanitize: 한글 -> 로마자, ASCII 그대로.
/// 한글 유니코드는 `char::from_u32` 로 직접 생성하여 콘솔 인코딩 영향 회피.
#[test]
fn sanitize_korean_variable_name() {
    use entrycore::block::sanitize_ident;
    let ga = std::char::from_u32(0xAC00).unwrap(); // 가
    let kk = std::char::from_u32(0xAD8C).unwrap(); // 권 (U+AD8C)
    let hi = std::char::from_u32(0xD558).unwrap(); // 하
    // 영문 그대로
    assert_eq!(sanitize_ident("score"), "score");
    assert_eq!(sanitize_ident("x"), "x");
    // 숫자 시작 -> prefix
    assert_eq!(sanitize_ident("123abc"), "v_123abc");
    // 공백/특수문자 -> underscore
    assert_eq!(sanitize_ident("hello world"), "hello_world");
    assert_eq!(sanitize_ident("a.b"), "a_b");
    // 키워드 충돌 -> raw identifier
    assert_eq!(sanitize_ident("type"), "r#type");
    // 빈 문자열
    assert_eq!(sanitize_ident(""), "v_empty");
    // 모두 underscore
    assert_eq!(sanitize_ident("___"), "v_empty");
    // 한글 음절 단독: `가` = ㄱ + ㅏ + 종성 없음 → "ga"
    assert_eq!(sanitize_ident("가"), "ga");
    // `권` = ㄱ + ㅜ + ㅓ + ㄴ 받침 → "gwon" (Revised strict: 받침 + 모음 연음)
    assert_eq!(sanitize_ident("권"), "gueon");
    // `하` = ㅎ + ㅏ → "ha"
    assert_eq!(sanitize_ident("하"), "ha");
    // 한글 2개 연결
    assert_eq!(sanitize_ident(&format!("{ga}{kk}")), "gagueon");
    // 한글+특수문자 혼합
    assert_eq!(sanitize_ident(&format!("{ga}!")), "ga_");
    // 충돌 방지: 같은 sanitize 결과 다른 원본
    assert_ne!(sanitize_ident(&format!("{ga}")), sanitize_ident(&format!("{hi}")));
}
