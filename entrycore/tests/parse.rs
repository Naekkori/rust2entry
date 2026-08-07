//! parse::parse() 통합 테스트.

use entrycore::ir::{Expr, Stmt};
use entrycore::parse::parse;

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
