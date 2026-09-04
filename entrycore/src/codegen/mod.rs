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
    let scripts = blocks?
        .into_iter()
        .map(|b| to_value(&b))
        .collect::<Result<Vec<_>>>()?;
    let vars = collect_var_map(program, &VarMap::new());
    let vars_arr: Vec<Value> = vars
        .iter()
        .map(|v| {
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
        })
        .collect();
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
        if let Some(new_id) = new_id
            && let Some(existing) = merged_vars
                .iter_mut()
                .find(|e| e.get("id").and_then(|x| x.as_str()) == Some(new_id))
            {
                *existing = v.clone();
                continue;
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
///
/// `base` 는 base .ent 에서 추출한 VarMap. 같은 name 의 변수가 base 에 있으면
/// 우리 codegen 의 id 를 base 의 id 로 맞춰 script 안의 variable param id 가
/// EntryJS variable list 와 일치하도록 한다 (socket 연결 복원).
pub fn collect_var_map(program: &Program, base: &VarMap) -> VarMap {
    let analysis = analyze_variables(program);
    let mut map = VarMap::new();
    let names = analysis.names;
    let mut used_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for name in names {
        let id = crate::block::id_for(&name);
        let kind = analysis
            .kinds
            .get(&name)
            .cloned()
            .unwrap_or(VarKind::Variable);
        // scope: Global scope VarDecl 이 있으면 우선, 없으면 Local (default).
        let scope = analysis
            .scopes
            .get(&name)
            .copied()
            .unwrap_or(crate::var::VarScope::Local);
        // sanitize + 충돌 시 suffix.
        let base_name = crate::block::sanitize_ident(&name);
        let final_name = if used_names.contains(&base_name) {
            let suffix = {
                let mut h: u64 = 5381;
                for b in name.bytes() {
                    h = h.wrapping_mul(33).wrapping_add(b as u64);
                }
                format!("_{:x}", h & 0xFFF)
            };
            let mut candidate = format!("{base_name}{suffix}");
            let mut n = 0;
            while used_names.contains(&candidate) {
                n += 1;
                candidate = format!("{base_name}{suffix}_{n}");
            }
            candidate
        } else {
            base_name
        };
        used_names.insert(final_name.clone());
        // 같은 name 이 base 에 있으면 base 의 id 를 그대로 사용해서 EntryJS
        // variable list 의 id 와 일치시킨다 (socket 연결 복원). base 에 없으면
        // 우리 hash 로 신규 발급.
        let final_id = base
            .iter()
            .find(|v| v.name == final_name || v.original_name == name)
            .map(|v| v.id.clone())
            .unwrap_or(id);
        map.insert(VarInfo {
            id: final_id,
            name: final_name,
            original_name: name.clone(),
            kind: kind,
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
    // base 에만 있는 변수도 유지 (codegen 결과에 없는 base 변수 보존).
    for v in base.iter() {
        if !map.iter().any(|m| m.name == v.name) {
            map.insert(v.clone());
        }
    }
    map
}

struct VariableAnalysis {
    names: Vec<String>,
    explicit_kinds: std::collections::HashMap<String, VarKind>,
    scopes: std::collections::HashMap<String, crate::var::VarScope>,
    list_context_names: std::collections::HashSet<String>,
    kinds: std::collections::HashMap<String, VarKind>,
}

fn analyze_variables(program: &Program) -> VariableAnalysis {
    let mut analysis = VariableAnalysis {
        names: Vec::new(),
        explicit_kinds: std::collections::HashMap::new(),
        scopes: std::collections::HashMap::new(),
        list_context_names: std::collections::HashSet::new(),
        kinds: std::collections::HashMap::new(),
    };
    analyze_statements(program.stmts.as_slice(), &mut analysis);
    for name in &analysis.names {
        let kind = analysis
            .explicit_kinds
            .get(name)
            .cloned()
            .or_else(|| {
                analysis
                    .list_context_names
                    .contains(name)
                    .then_some(VarKind::List)
            })
            .unwrap_or_else(|| crate::block::kind_for(name));
        analysis.kinds.insert(name.clone(), kind);
    }
    analysis
}

fn analyze_statements(stmts: &[Stmt], out: &mut VariableAnalysis) {
    for stmt in stmts {
        match stmt {
            Stmt::VarDecl(name, expr, kind, scope) => {
                push_unique(&mut out.names, name);
                if let Some(kind) = kind {
                    out.explicit_kinds.insert(name.clone(), *kind);
                }
                out.scopes.insert(name.clone(), *scope);
                analyze_expr(expr, out);
            }
            Stmt::SetVar(vref, expr) => {
                push_unique(&mut out.names, &vref.name);
                analyze_expr(expr, out);
            }
            Stmt::ChangeVariable { variable, value } => {
                push_unique(&mut out.names, &variable.name);
                analyze_expr(value, out);
            }
            Stmt::Expr(expr) | Stmt::Return(expr) => analyze_expr(expr, out),
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                analyze_expr(cond, out);
                analyze_statements(then_body, out);
                analyze_statements(else_body, out);
            }
            Stmt::While { cond, body } | Stmt::Repeat { times: cond, body } => {
                analyze_expr(cond, out);
                analyze_statements(body, out);
            }
            Stmt::For { var, iter, body } => {
                push_unique(&mut out.names, var);
                analyze_expr(iter, out);
                analyze_statements(body, out);
            }
            Stmt::Loop { body } => {
                analyze_statements(body, out);
            }
            Stmt::Dialog { value, .. } => {
                analyze_expr(value, out);
            }
            Stmt::FuncDef { params, body, .. } => {
                for (param, _) in params {
                    push_unique(&mut out.names, param);
                }
                analyze_statements(body, out);
            }
            Stmt::Break | Stmt::Continue | Stmt::StopAll => {}
        }
    }
}

fn analyze_expr(expr: &Expr, out: &mut VariableAnalysis) {
    match expr {
        Expr::Var(name) => push_unique(&mut out.names, name),
        Expr::Call(func, args) => {
            let list_index = match func.name.as_str() {
                "value_of_index_from_list" | "add_value_to_list" | "remove_value_from_list" => {
                    Some(1)
                }
                "insert_value_to_list" | "change_value_list_index" => Some(2),
                "length_of_list" => Some(0),
                "is_included_in_list" => Some(0),
                "show_list" => Some(0),
                "hide_list" => Some(0),
                _ => None,
            };
            if let Some(index) = list_index
                && let Some(Expr::Var(name)) = args.get(index) {
                    out.list_context_names.insert(name.clone());
                }
            for arg in args {
                analyze_expr(arg, out);
            }
        }
        Expr::BinOp(_, lhs, rhs) | Expr::Range(lhs, rhs) => {
            analyze_expr(lhs, out);
            analyze_expr(rhs, out);
        }
        Expr::UnaryOp(_, inner) => analyze_expr(inner, out),
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::Str(_)
        | Expr::Bool(_)
        | Expr::Path(_)
        | Expr::Func(_) => {}
    }
}

/// VarDecl 의 explicit kind 를 수집. SetVar 는 이름만 쓰므로 무시.
/// VarDecl 의 scope 를 수집. 같은 이름이 여러 번 등장해도 마지막 것 유지
/// (실제로는 let 한 번 + static 한 번 같은 중복은 발생하지 않음).
pub(crate) fn collect_vars_program(p: &Program, out: &mut Vec<String>) {
    for s in &p.stmts {
        collect_vars_stmt(s, out);
    }
}

pub(crate) fn collect_vars_stmt(s: &Stmt, out: &mut Vec<String>) {
    match s {
        Stmt::VarDecl(n, e, _, _) => {
            push_unique(out, n);
            collect_vars_expr(e, out);
        }
        Stmt::SetVar(vref, e) => {
            push_unique(out, &vref.name);
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
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
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
        Stmt::Loop { body } => {
            for s in body {
                collect_vars_stmt(s, out);
            }
        }
        Stmt::Dialog { value, .. } => collect_vars_expr(value, out),
        Stmt::ChangeVariable { variable, value } => {
            push_unique(out, &variable.name);
            collect_vars_expr(value, out);
        }
        Stmt::Return(e) => collect_vars_expr(e, out),
        Stmt::Break | Stmt::Continue | Stmt::StopAll => {}
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
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::Str(_)
        | Expr::Bool(_)
        | Expr::Path(_)
        | Expr::Func(_) => {}
    }
}

fn push_unique(out: &mut Vec<String>, n: &str) {
    if !out.iter().any(|x| x == n) {
        out.push(n.to_string());
    }
}
