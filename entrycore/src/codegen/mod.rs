//! IR -> Block -> project.json 변환.

pub mod schema;

use crate::Result;
use crate::block::{from_stmt, to_value};
use crate::ir::{Expr, Program, Stmt};
use crate::var::{VarInfo, VarInit, VarKind, VarMap};
use serde_json::Value;

/// IR Program -> Entry project.json.
pub fn generate(program: &Program) -> Result<Value> {
    // IR stmt들을 Block으로 변환한 뒤 to_value() 호출.
    // project.json 최상위 구조는 schema::Project 참고.
    let blocks: Result<Vec<_>> = program.stmts.iter().map(from_stmt).collect();
    let scripts = blocks?.into_iter().map(|b| to_value(&b)).collect::<Result<Vec<_>>>()?;

    let project = serde_json::json!({
        "speed": 60,
        "objects": [],
        "variables": [],
        "messages": [],
        "functions": [],
        "scenes": [{"id": "scene1", "name": "장면1"}],
        "interface": { "views": [] },
        "meta": {
            "last_modified": "2026-01-01T00:00:00.000Z", //에
            "created_at": "2026-01-01T00:00:00.000Z",
            "version": "0.1.0"
        },
        "scripts": scripts,
    });
    Ok(project)
}

/// IR Program에서 사용하는 변수 이름을 모아 VarMap 생성.
/// `block::id_for`와 동일한 id 생성 규칙을 사용 (djb2 해시).
/// 라운드트립 테스트에서 codegen 결과를 deparse에 넘길 때 사용.
pub fn collect_var_map(program: &Program) -> VarMap {
    let mut map = VarMap::new();
    let mut names: Vec<String> = Vec::new();
    collect_vars_program(program, &mut names);
    for name in names {
        let id = crate::block::id_for(&name);
        let kind = crate::block::kind_for(&name);
        map.insert(VarInfo {
            id,
            name: name.clone(),
            kind: kind.clone(),
            init: VarInit::Int0,
        });
    }
    map
}

fn collect_vars_program(p: &Program, out: &mut Vec<String>) {
    for s in &p.stmts {
        collect_vars_stmt(s, out);
    }
}

fn collect_vars_stmt(s: &Stmt, out: &mut Vec<String>) {
    match s {
        Stmt::VarDecl(n, e) | Stmt::SetVar(n, e) => {
            push_unique(out, n);
            collect_vars_expr(e, out);
        }
        Stmt::FuncDef { params, body, .. } => {
            for p in params {
                push_unique(out, p);
            }
            for s in body {
                collect_vars_stmt(s, out);
            }
        }
        Stmt::Expr(e) => collect_vars_expr(e, out),
        Stmt::If { cond, then_body, else_body } => {
            collect_vars_expr(cond, out);
            for s in then_body {
                collect_vars_stmt(s, out);
            }
            for s in else_body {
                collect_vars_stmt(s, out);
            }
        }
        Stmt::While { cond, body } => {
            collect_vars_expr(cond, out);
            for s in body {
                collect_vars_stmt(s, out);
            }
        }
        Stmt::For { var, iter, body } => {
            push_unique(out, var);
            collect_vars_expr(iter, out);
            for s in body {
                collect_vars_stmt(s, out);
            }
        }
        Stmt::Repeat { times, body } => {
            collect_vars_expr(times, out);
            for s in body {
                collect_vars_stmt(s, out);
            }
        }
        Stmt::Return(e) => collect_vars_expr(e, out),
        Stmt::Break | Stmt::Continue => {}
    }
}

fn collect_vars_expr(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Var(n) => push_unique(out, n),
        Expr::BinOp(_, l, r) => {
            collect_vars_expr(l, out);
            collect_vars_expr(r, out);
        }
        Expr::UnaryOp(_, e) => collect_vars_expr(e, out),
        Expr::Call(_, args) => {
            for a in args {
                collect_vars_expr(a, out);
            }
        }
        Expr::Range(l, r) => {
            collect_vars_expr(l, out);
            collect_vars_expr(r, out);
        }
        Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Func(_) => {}
    }
}

fn push_unique(out: &mut Vec<String>, n: &str) {
    if !out.iter().any(|x| x == n) {
        out.push(n.to_string());
    }
}
