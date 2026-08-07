//! IR -> Entry DSL 소스 직렬화.
//!
//! 출력은 `entrycore::parse`가 인식하는 Rust-like DSL 형태:
//!   fn when_start() { ... }
//!   let x = 0;
//!   x = x + 1;
//!   if x != 1 { ... } else { ... }
//!   while cond { ... }
//!   for _ in 0..n { ... }

use crate::ir::{BinOp, Expr, FuncRef, Program, Stmt, UnaryOp};
use crate::var::{VarInfo, VarInit, VarMap};
use crate::Result;

/// IR Program -> DSL 소스 문자열.
pub fn emit(program: &Program) -> Result<String> {
    emit_with_var_map(program, &VarMap::new())
}

/// IR Program -> DSL 소스 문자열. VarMap에 등록된 변수를 함수 본문 위에 선언.
///
/// top-level stmt 중 `FuncDef`는 그대로 함수 정의로 출력,
/// 나머지는 `fn when_start() { ... }` 블록으로 묶어 출력 (Entry 트리거 형태).
pub fn emit_with_var_map(program: &Program, vars: &VarMap) -> Result<String> {
    let mut out = String::new();
    let mut trigger_body: Vec<&Stmt> = Vec::new();
    for stmt in &program.stmts {
        match stmt {
            Stmt::FuncDef { .. } => {
                if !trigger_body.is_empty() {
                    emit_trigger_block(&trigger_body, &mut out, vars)?;
                    trigger_body.clear();
                    out.push('\n');
                }
                emit_stmt(stmt, &mut out, 0, vars)?;
                out.push('\n');
            }
            other => trigger_body.push(other),
        }
    }
    if !trigger_body.is_empty() {
        emit_trigger_block(&trigger_body, &mut out, vars)?;
        out.push('\n');
    }
    Ok(out)
}

/// 트리거 블록 (`fn when_start() { ... }`) 출력.
fn emit_trigger_block(stmts: &[&Stmt], out: &mut String, vars: &VarMap) -> Result<()> {
    out.push_str("fn when_start() {\n");
    for v in vars.iter() {
        emit_var_decl(out, 1, v);
    }
    for s in stmts {
        emit_stmt(s, out, 1, vars)?;
    }
    out.push_str("}\n");
    Ok(())
}

fn indent_of(level: usize) -> String {
    "    ".repeat(level)
}

fn emit_stmt(
    stmt: &Stmt,
    out: &mut String,
    level: usize,
    vars: &VarMap,
) -> Result<()> {
    let indent = indent_of(level);
    match stmt {
        Stmt::VarDecl(name, expr) => {
            out.push_str(&indent);
            out.push_str("let ");
            out.push_str(name);
            out.push_str(" = ");
            emit_expr(expr, out)?;
            out.push_str(";\n");
        }
        Stmt::SetVar(name, expr) => {
            out.push_str(&indent);
            out.push_str(name);
            out.push_str(" = ");
            emit_expr(expr, out)?;
            out.push_str(";\n");
        }
        Stmt::FuncDef {
            name,
            params,
            body,
        } => {
            out.push_str(&indent);
            out.push_str("fn ");
            out.push_str(name);
            out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(p);
                out.push_str(": i32");
            }
            out.push_str(") {\n");
            for v in vars.iter() {
                emit_var_decl(out, level + 1, v);
            }
            for s in body {
                emit_stmt(s, out, level + 1, vars)?;
            }
            out.push_str(&indent);
            out.push_str("}\n");
        }
        Stmt::Expr(expr) => {
            out.push_str(&indent);
            emit_expr(expr, out)?;
            out.push_str(";\n");
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            out.push_str(&indent);
            out.push_str("if ");
            emit_expr(cond, out)?;
            out.push_str(" {\n");
            for s in then_body {
                emit_stmt(s, out, level + 1, vars)?;
            }
            if else_body.is_empty() {
                out.push_str(&indent);
                out.push_str("}\n");
            } else {
                out.push_str(&indent);
                out.push_str("} else {\n");
                for s in else_body {
                    emit_stmt(s, out, level + 1, vars)?;
                }
                out.push_str(&indent);
                out.push_str("}\n");
            }
        }
        Stmt::While { cond, body } => {
            out.push_str(&indent);
            out.push_str("while ");
            emit_expr(cond, out)?;
            out.push_str(" {\n");
            for s in body {
                emit_stmt(s, out, level + 1, vars)?;
            }
            out.push_str(&indent);
            out.push_str("}\n");
        }
        Stmt::For { var, iter, body } => {
            out.push_str(&indent);
            out.push_str("for ");
            out.push_str(var);
            out.push_str(" in ");
            emit_expr(iter, out)?;
            out.push_str(" {\n");
            for s in body {
                emit_stmt(s, out, level + 1, vars)?;
            }
            out.push_str(&indent);
            out.push_str("}\n");
        }
        Stmt::Repeat { times, body } => {
            out.push_str(&indent);
            out.push_str("for _ in 0..");
            emit_expr(times, out)?;
            out.push_str(" {\n");
            for s in body {
                emit_stmt(s, out, level + 1, vars)?;
            }
            out.push_str(&indent);
            out.push_str("}\n");
        }
        Stmt::Return(expr) => {
            out.push_str(&indent);
            out.push_str("return ");
            emit_expr(expr, out)?;
            out.push_str(";\n");
        }
        Stmt::Break => {
            out.push_str(&indent);
            out.push_str("break;\n");
        }
        Stmt::Continue => {
            out.push_str(&indent);
            out.push_str("continue;\n");
        }
    }
    Ok(())
}

fn emit_var_decl(out: &mut String, level: usize, v: &VarInfo) {
    let indent = indent_of(level);
    out.push_str(&indent);
    out.push_str("let ");
    out.push_str(&v.name);
    out.push_str(" = ");
    match &v.init {
        VarInit::Int0 => out.push_str("0"),
        VarInit::Float0 => out.push_str("0.0"),
        VarInit::EmptyStr => out.push_str("\"\""),
        VarInit::False => out.push_str("false"),
        VarInit::EmptyList => out.push_str("[]"),
    }
    out.push_str(";\n");
}

fn emit_expr(expr: &Expr, out: &mut String) -> Result<()> {
    match expr {
        Expr::Int(n) => {
            out.push_str(&n.to_string());
        }
        Expr::Float(f) => {
            if f.fract() == 0.0 {
                out.push_str(&format!("{}.0", f));
            } else {
                out.push_str(&format!("{}", f));
            }
        }
        Expr::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    other => out.push(other),
                }
            }
            out.push('"');
        }
        Expr::Bool(b) => {
            out.push_str(if *b { "true" } else { "false" });
        }
        Expr::Var(name) => {
            out.push_str(name);
        }
        Expr::BinOp(op, lhs, rhs) => {
            out.push('(');
            emit_expr(lhs, out)?;
            out.push(' ');
            out.push_str(op_str(*op));
            out.push(' ');
            emit_expr(rhs, out)?;
            out.push(')');
        }
        Expr::UnaryOp(op, expr) => {
            out.push_str(un_op_str(*op));
            emit_expr(expr, out)?;
        }
        Expr::Call(fref, args) => {
            emit_call(fref, args, out)?;
        }
        Expr::Func(fref) => {
            out.push_str(&fref.name);
        }
        Expr::Range(start, end) => {
            emit_expr(start, out)?;
            out.push_str("..");
            emit_expr(end, out)?;
        }
    }
    Ok(())
}

fn emit_call(fref: &FuncRef, args: &[Expr], out: &mut String) -> Result<()> {
    out.push_str(&fref.name);
    out.push('(');
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        emit_expr(a, out)?;
    }
    out.push(')');
    Ok(())
}

fn op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::Range => "..",
    }
}

fn un_op_str(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}
