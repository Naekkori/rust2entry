//! IR -> Block -> project.json 변환.

pub mod schema;

use crate::Result;
use crate::block::{from_stmt, to_value};
use crate::ir::{Expr, Program, Stmt};
use crate::var::{VarInfo, VarInit, VarKind, VarMap};
use serde_json::{Value, json};

/// IR Program -> Entry project.json.
///
/// ## Deprecated for new code
/// 이 함수는 `from_stmt`/`to_value` 의 `UnmappedBlock` 에러를 catch 하지 않고
/// 그대로 propagate 한다. 새 코드에서는 `crate::compile_with_options` 를
/// 사용하면 unmapped 블록이 `(Value, Vec<String>)` 의 두 번째 반환에 누적되어
/// build 시 eprintln 으로 경고할 수 있다. `generate` 는 extract 라운드트립
/// 테스트, codegen 단위 테스트 등 low-level 용도로만 남겨둔다.
#[allow(dead_code)]
pub fn generate(program: &Program, original: &Value) -> Result<Value> {
    // IR stmt들을 Block으로 변환한 뒤 to_value() 호출.
    // project.json 최상위 구조는 schema::Project 참고.
    let blocks: Result<Vec<_>> = program.stmts.iter().map(from_stmt).collect();
    let scripts = blocks?.into_iter().map(|b| to_value(&b)).collect::<Result<Vec<_>>>()?;
    let vars = collect_var_map(program);
    let vars_arr: Vec<Value> = vars.iter().map(|v| {
        json!({
            "id":v.id,
            "name": v.name,
            "variableType": match v.kind{
                VarKind::Variable => "variable",
                VarKind::Answer => "answer",
                VarKind::Timer => "timer",
                VarKind::List => "list",
                VarKind::Cloud => "cloud",
                VarKind::RealTime => "realtime",
                VarKind::Unknown => "variable",
            },
            "value":match v.init {
                VarInit::Int0 => json!(0),
                VarInit::Float0 => json!(0.0),
                VarInit::EmptyStr => json!(""),
                VarInit::False => json!(false),
                VarInit::EmptyList => json!([]),
            }
        })
    }).collect();
    let mut project = original.clone();
    project["scripts"] = json!(scripts);
    // variables 는 base 의 기존 항목을 보존하고, 같은 id 의 새 항목은 덮어쓰고,
    // 없는 id 는 추가한다 (union by id). 이렇게 하지 않으면 --ent-template 으로
    // 빌드할 때 base 의 변수가 사라지는 회귀가 발생한다.
    let base_vars = project
        .get("variables")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut merged_vars: Vec<Value> = base_vars;
    for v in &vars_arr {
        let new_id = v.get("id").and_then(|x| x.as_str());
        if let Some(new_id) = new_id {
            if let Some(existing) = merged_vars.iter_mut().find(|e| {
                e.get("id").and_then(|x| x.as_str()) == Some(new_id)
            }) {
                *existing = v.clone();
                continue;
            }
        }
        merged_vars.push(v.clone());
    }
    project["variables"] = json!(merged_vars);
    Ok(project)
}

/// IR Program에서 사용하는 변수 이름을 모아 VarMap 생성.
/// `block::id_for`와 동일한 id 생성 규칙을 사용 (djb2 해시).
/// 라운드트립 테스트에서 codegen 결과를 deparse에 넘길 때 사용.
///
/// `let x: CloudVar = ...` 같이 VarDecl 이 명시적 kind 를 가지면
/// `block::kind_for(name)` (이름 기반) 보다 우선한다.
///
/// `static x = ...` 같이 VarDecl 이 Global scope 를 가지면 VarInfo.scope 도
/// Global 로 설정 — EntryJS variables[].object = null.
pub fn collect_var_map(program: &Program) -> VarMap {
    use crate::var::VarScope;
    let mut map = VarMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut explicit_kinds: std::collections::HashMap<String, VarKind> =
        std::collections::HashMap::new();
    let mut scopes: std::collections::HashMap<String, VarScope> =
        std::collections::HashMap::new();
    collect_vars_program(program, &mut names);
    collect_explicit_kinds_program(program, &mut explicit_kinds);
    collect_scopes_program(program, &mut scopes);
    for name in names {
        let id = crate::block::id_for(&name);
        let kind = explicit_kinds
            .get(&name)
            .cloned()
            .unwrap_or_else(|| crate::block::kind_for(&name));
        // scope: Global scope VarDecl 이 있으면 우선, 없으면 Local (default).
        let scope = scopes
            .get(&name)
            .copied()
            .unwrap_or(VarScope::Local);
        map.insert(VarInfo {
            id,
            name: name.clone(),
            kind: kind.clone(),
            init: match kind {
                VarKind::List => VarInit::EmptyList,
                VarKind::Variable => VarInit::EmptyStr,
                VarKind::Timer => VarInit::Float0,
                VarKind::Answer => VarInit::EmptyStr,
                VarKind::Cloud => VarInit::EmptyStr,
                VarKind::RealTime => VarInit::EmptyStr,
                VarKind::Unknown => VarInit::EmptyStr,
            },
            scope,
        });
    }
    map
}

/// VarDecl 의 explicit kind 를 수집. SetVar 는 이름만 쓰므로 무시.
pub(crate) fn collect_explicit_kinds_program(
    p: &Program,
    out: &mut std::collections::HashMap<String, VarKind>,
) {
    for s in &p.stmts {
        collect_explicit_kinds_stmt(s, out);
    }
}

fn collect_explicit_kinds_stmt(
    s: &Stmt,
    out: &mut std::collections::HashMap<String, VarKind>,
) {
    match s {
        Stmt::VarDecl(name, _, Some(kind), _) => {
            out.insert(name.clone(), kind.clone());
        }
        Stmt::VarDecl(_, _, None, _) => {}
        Stmt::SetVar(_, _) => {}
        Stmt::Expr(e) => {
            collect_explicit_kinds_expr(e, out);
        }
        Stmt::If { cond, then_body, else_body } => {
            collect_explicit_kinds_expr(cond, out);
            for s in then_body {
                collect_explicit_kinds_stmt(s, out);
            }
            for s in else_body {
                collect_explicit_kinds_stmt(s, out);
            }
        }
        Stmt::While { cond, body } => {
            collect_explicit_kinds_expr(cond, out);
            for s in body {
                collect_explicit_kinds_stmt(s, out);
            }
        }
        Stmt::For { body, .. } => {
            for s in body {
                collect_explicit_kinds_stmt(s, out);
            }
        }
        Stmt::Repeat { times, body } => {
            collect_explicit_kinds_expr(times, out);
            for s in body {
                collect_explicit_kinds_stmt(s, out);
            }
        }
        Stmt::FuncDef { body, .. } => {
            for s in body {
                collect_explicit_kinds_stmt(s, out);
            }
        }
        Stmt::Return(_) | Stmt::Break | Stmt::Continue => {}
    }
}

fn collect_explicit_kinds_expr(e: &Expr, _out: &mut std::collections::HashMap<String, VarKind>) {
    // Expr 안에는 새 VarDecl 가 없으므로 noop.
    let _ = e;
}

/// VarDecl 의 scope 를 수집. 같은 이름이 여러 번 등장해도 마지막 것 유지
/// (실제로는 let 한 번 + static 한 번 같은 중복은 발생하지 않음).
pub(crate) fn collect_scopes_program(
    p: &Program,
    out: &mut std::collections::HashMap<String, crate::var::VarScope>,
) {
    for s in &p.stmts {
        collect_scopes_stmt(s, out);
    }
}

fn collect_scopes_stmt(
    s: &Stmt,
    out: &mut std::collections::HashMap<String, crate::var::VarScope>,
) {
    match s {
        Stmt::VarDecl(name, _, _, scope) => {
            out.insert(name.clone(), *scope);
        }
        Stmt::SetVar(_, _) => {}
        Stmt::Expr(_) => {}
        Stmt::If { then_body, else_body, .. } => {
            for s in then_body {
                collect_scopes_stmt(s, out);
            }
            for s in else_body {
                collect_scopes_stmt(s, out);
            }
        }
        Stmt::While { body, .. } | Stmt::Repeat { body, .. } => {
            for s in body {
                collect_scopes_stmt(s, out);
            }
        }
        Stmt::For { body, .. } => {
            for s in body {
                collect_scopes_stmt(s, out);
            }
        }
        Stmt::FuncDef { body, .. } => {
            for s in body {
                collect_scopes_stmt(s, out);
            }
        }
        Stmt::Return(_) | Stmt::Break | Stmt::Continue => {}
    }
}

pub(crate) fn collect_vars_program(p: &Program, out: &mut Vec<String>) {
    for s in &p.stmts {
        collect_vars_stmt(s, out);
    }
}

pub(crate) fn collect_vars_stmt(s: &Stmt, out: &mut Vec<String>) {
    match s {
        Stmt::VarDecl(n, e, _, _) | Stmt::SetVar(n, e) => {
            push_unique(out, n);
            collect_vars_expr(e, out);
        }
        Stmt::FuncDef { params, body, .. } => {
            for (p, _) in params {
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

pub(crate) fn collect_vars_expr(e: &Expr, out: &mut Vec<String>) {
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
