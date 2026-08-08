//! parse::parse() 통합 테스트.

use entrycore::codegen::generate;
use entrycore::decodegen;
use entrycore::ir::{Expr, Stmt};
use entrycore::parse::parse;
use serde_json::{Value, json};

/// codegen 테스트용 빈 project.json 베이스.
fn empty_project() -> Value {
    json!({
        "name": "test",
        "speed": 60, "objects": [], "variables": [], "messages": [],
        "functions": [], "scenes": [{"id":"scene1","name":"장면1"}],
        "interface": {"views": []}, "meta": {}
    })
}

#[test]
fn when_start_int_literal() {
    let src = r#"
        fn when_start() {
            let x = 42;
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(
        &program.stmts[0],
        Stmt::VarDecl(name, Expr::Int(42)) if name == "x"
    ));
}

#[test]
fn when_start_arith() {
    let src = r#"
        fn when_start() {
            let x = 1 + 2 * 3;
        }
    "#;

    let program = parse(src).expect("parse ok");

    let Stmt::VarDecl(_, expr) = &program.stmts[0] else {
        panic!("expected VarDecl");
    };
    assert!(matches!(
        expr,
        Expr::BinOp(entrycore::ir::BinOp::Add, _, _)
    ));
}

#[test]
fn when_start_if_else() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let a = 1;
            } else {
                let b = 2;
            }
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(
        &program.stmts[0],
        Stmt::If { cond: Expr::BinOp(entrycore::ir::BinOp::Lt, _, _), then_body, else_body }
            if !then_body.is_empty() && !else_body.is_empty()
    ));
}

#[test]
fn when_start_while() {
    let src = r#"
        fn when_start() {
            while true {
                let x = 0;
            }
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert!(matches!(
        &program.stmts[0],
        Stmt::While { cond: Expr::Bool(true), body } if !body.is_empty()
    ));
}

#[test]
fn func_def_collected() {
    let src = r#"
        fn when_start() {
            let x = 0;
        }

        fn greet(name: String) {
            let s = name;
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::VarDecl(_, _)));
    assert!(matches!(
        &program.stmts[1],
        Stmt::FuncDef { name, params, .. } if name == "greet" && params == &["name".to_string()]
    ));
}

#[test]
fn when_click_also_top_level() {
    let src = r#"
        fn when_click() {
            let x = 1;
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(&program.stmts[0], Stmt::VarDecl(_, _)));
}

#[test]
fn main_becomes_funcdef() {
    let src = r#"
        fn main() {
            let x = 1;
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert!(matches!(
        &program.stmts[0],
        Stmt::FuncDef { name, .. } if name == "main"
    ));
}

#[test]
fn string_literal() {
    let src = r#"
        fn when_start() {
            let s = "hello";
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert!(matches!(
        &program.stmts[0],
        Stmt::VarDecl(_, Expr::Str(s)) if s == "hello"
    ));
}
#[test]
fn when_start_if_else_full() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let a = 1;
            } else {
                let b = 2;
            }
        }
    "#;
    let program = parse(src).expect("parse");
    assert_eq!(program.stmts.len(), 1);
    match &program.stmts[0] {
        Stmt::If { then_body, else_body, .. } => {
            assert_eq!(then_body.len(), 1);
            assert_eq!(else_body.len(), 1);
            // then: VarDecl a = 1
            assert!(matches!(&then_body[0], Stmt::VarDecl(n, _) if n == "a"));
            // else: VarDecl b = 2
            assert!(matches!(&else_body[0], Stmt::VarDecl(n, _) if n == "b"));
        }
        _ => panic!("expected If with else"),
    }
}

#[test]
fn when_start_elif_chain() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let a = 1;
            } else if 3 < 4 {
                let b = 2;
            } else {
                let c = 3;
            }
        }
    "#;
    let program = parse(src).expect("parse");
    // else if 는 Stmt::If {} 가 else_body[0] 에 들어감 (재귀)
    match &program.stmts[0] {
        Stmt::If { else_body, .. } => {
            assert_eq!(else_body.len(), 1);
            match &else_body[0] {
                Stmt::If { then_body, else_body: inner_else, .. } => {
                    assert_eq!(then_body.len(), 1);
                    assert_eq!(inner_else.len(), 1);
                }
                _ => panic!("elif not nested If"),
            }
        }
        _ => panic!("expected If"),
    }
}

#[test]
fn if_else_block() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let x = 1;
            } else {
                let y = 2;
            }
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program, &empty_project()).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "if_else");
    // params[0] = 조건 (boolean_basic)
    assert_eq!(block["params"][0]["type"], "boolean_basic");
    // statements[0] = then, statements[1] = else
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 2);
    assert_eq!(stmts[0][0]["type"], "set_variable"); // then: let x = 1
    assert_eq!(stmts[1][0]["type"], "set_variable"); // else: let y = 2
}

#[test]
fn if_without_else_stays_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let program = parse(src).expect("parse");
    let json = generate(&program, &empty_project()).expect("generate");
    let block = &json["scripts"][0];
    // else 없으면 if (if_else 아님)
    assert_eq!(block["type"], "if");
}

#[test]
fn negative_number() {
    let src = r#"
        fn when_start() {
            let x = -5;
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert!(matches!(
        &program.stmts[0],
        Stmt::VarDecl(_, Expr::UnaryOp(entrycore::ir::UnaryOp::Neg, _))
    ));
}

#[test]
fn function_call() {
    let src = r#"
        fn when_start() {
            greet("world");
        }
    "#;

    let program = parse(src).expect("parse ok");

    assert!(matches!(
        &program.stmts[0],
        Stmt::Expr(Expr::Call(_func, args)) if args.len() == 1
    ));
}

#[test]
fn for_range_to_ir() {
    let src = r#"
        fn when_start() {
            for i in 0..3 {
                let x = i;
            }
        }
    "#;
    let program = parse(src).expect("parse ok");
    let Stmt::For { var, iter, body } = &program.stmts[0] else {
        panic!("expected For");
    };
    assert_eq!(var, "i");
    assert!(matches!(iter, Expr::Range(_, _)));
    assert!(matches!(body[0], Stmt::VarDecl(_, _)));
}

/// DSL 라운드트립: parse(src) -> IR -> decodegen -> dsl -> parse(dsl) -> IR' 구조 동일.
#[test]
fn dsl_roundtrip_simple() {
    let src = "fn when_start() { let x = 42; }";
    let p1 = parse(src).expect("parse1");
    let dsl = decodegen::emit(&p1).expect("emit");
    let p2 = parse(&dsl).expect("parse2");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    // 둘 다 단일 변수 선언 (VarDecl 또는 SetVar, Entry 의미 동일)
    let n1 = match &p1.stmts[0] {
        Stmt::VarDecl(n, _) | Stmt::SetVar(n, _) => n.clone(),
        other => panic!("p1[0] unexpected: {other:?}"),
    };
    let n2 = match &p2.stmts[0] {
        Stmt::VarDecl(n, _) | Stmt::SetVar(n, _) => n.clone(),
        other => panic!("p2[0] unexpected: {other:?}"),
    };
    assert_eq!(n1, n2, "variable name roundtrip: dsl={dsl}");
}

#[test]
fn dsl_roundtrip_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = parse(src).expect("parse1");
    let dsl = decodegen::emit(&p1).expect("emit");
    let p2 = parse(&dsl).expect("parse2");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    let tb1 = match &p1.stmts[0] {
        Stmt::If { then_body, .. } => then_body.len(),
        _ => panic!("p1[0] not If"),
    };
    let tb2 = match &p2.stmts[0] {
        Stmt::If { then_body, .. } => then_body.len(),
        _ => panic!("p2[0] not If"),
    };
    assert_eq!(tb1, tb2, "if then_body roundtrip: dsl={dsl}");
}

#[test]
fn dsl_roundtrip_for() {
    let src = "fn when_start() { for i in 0..5 { let x = 1; } }";
    let p1 = parse(src).expect("parse1");
    let dsl = decodegen::emit(&p1).expect("emit");
    let p2 = parse(&dsl).expect("parse2");
    assert_eq!(p1.stmts.len(), p2.stmts.len());
    // p1[0] = For { var: i, iter: Range(0, 5), body: [VarDecl x] }
    // p2[0]는 emit이 `for _ in 0..5`로 만들고 decodegen 재parse가 `Stmt::For { var: _, iter: Range(0, 5), body: [...] }`
    // 또는 decodegen이 본문에 `set x = 1`을 emit하면 p2도 For with body=[SetVar x].
    let (var1, body1) = match &p1.stmts[0] {
        Stmt::For { var, body, .. } => (var.clone(), body.len()),
        _ => panic!("p1[0] not For"),
    };
    let (var2, body2) = match &p2.stmts[0] {
        Stmt::For { var, body, .. } => (var.clone(), body.len()),
        other => panic!("p2[0] not For: {other:?}"),
    };
    assert_eq!(var1, var2);
    assert_eq!(body1, body2, "for body length roundtrip: dsl={dsl}");
}

#[test]
fn dsl_roundtrip_while() {
    let src = "fn when_start() { while true { let x = 0; } }";
    let p1 = parse(src).expect("parse1");
    let dsl = decodegen::emit(&p1).expect("emit");
    let p2 = parse(&dsl).expect("parse2");
    let cond1 = match &p1.stmts[0] {
        Stmt::While { cond, .. } => format!("{cond:?}"),
        _ => panic!("p1[0] not While"),
    };
    let cond2 = match &p2.stmts[0] {
        Stmt::While { cond, .. } => format!("{cond:?}"),
        _ => panic!("p2[0] not While"),
    };
    assert_eq!(cond1, cond2, "while cond roundtrip: dsl={dsl}");
}
