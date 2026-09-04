//! syn::Stmt -> IR Stmt 변환.

use syn::Stmt as SynStmt;

use crate::Error::ParseUnsupported;
use crate::Result;
use crate::ir::{Expr, Stmt as IrStmt, VarRef};
use crate::parse::{convert_block, convert_expr};
use crate::var::VarKind;

/// `let name: T = ...` 의 타입 어노테이션을 VarKind 로 매핑.
fn type_to_kind(ty: &syn::Type) -> Result<Option<VarKind>> {
    let last = match ty {
        syn::Type::Path(tp) => match tp.path.segments.last() {
            Some(s) => &s.ident,
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(Some(match last.to_string().as_str() {
        "CloudVar" | "cloud" => VarKind::Cloud,
        "RealtimeVar" | "RealTimeVar" | "realtime" | "realTime" => VarKind::RealTime,
        other => return Err(ParseUnsupported(format!("unknown var type: {other}"))),
    }))
}

pub(crate) fn convert_stmt(s: SynStmt, out: &mut Vec<IrStmt>) -> Result<()> {
    match s {
        SynStmt::Local(local) => {
            // 변수명 + 타입 추출. `let x: T = ...` 형태면 Pat::Type, 아니면 Pat::Ident.
            let (name, kind) = match &local.pat {
                syn::Pat::Ident(pi) => (pi.ident.to_string(), None),
                syn::Pat::Type(pt) => {
                    let ident = match pt.pat.as_ref() {
                        syn::Pat::Ident(pi) => pi.ident.to_string(),
                        _ => {
                            return Err(ParseUnsupported("destructuring pattern".into()));
                        }
                    };
                    let k = type_to_kind(&pt.ty)?;
                    (ident, k)
                }
                _ => return Err(ParseUnsupported("destructuring pattern".into())),
            };
            // 초기값
            let init = match local.init {
                Some(i) => convert_expr(*i.expr)?,
                None => Expr::Int(0),
            };
            // 함수 내 `let` → Local scope (EntryJS variables[].object = rs stem).
            // top-level 전역 변수는 `static` 키워드로 별도 처리 (parse/mod.rs).
            out.push(IrStmt::VarDecl(
                name,
                init,
                kind,
                crate::ir::VarScope::Local,
            ));
        }
        SynStmt::Expr(expr, _semi) => {
            match expr {
                // `break;` → `Stmt::Break`
                syn::Expr::Break(_) => out.push(IrStmt::Break),
                syn::Expr::Continue(_) => out.push(IrStmt::Continue),
                // `var = expr;` → `Stmt::SetVar`
                syn::Expr::Assign(a) => {
                    let name = match &*a.left {
                        syn::Expr::Path(p) => {
                            p.path
                                .segments
                                .last()
                                .map(|s| s.ident.to_string())
                                .ok_or_else(|| ParseUnsupported("assign left".into()))?
                        }
                        _ => return Err(ParseUnsupported("assign left".into())),
                    };
                    let value = convert_expr(*a.right)?;
                    out.push(IrStmt::SetVar(VarRef::new(crate::block::sanitize_ident(&name)), value));
                }
                syn::Expr::If(e) => {
                    let cond = convert_expr(*e.cond)?;
                    let then_body = convert_block(Some(e.then_branch))?;
                    let else_body = if let Some((_, else_expr)) = e.else_branch {
                        match *else_expr {
                            syn::Expr::Block(b) => convert_block(Some(b.block))?,
                            syn::Expr::If(_) => {
                                // 단일 if
                                let mut v = Vec::new();
                                convert_stmt(syn::Stmt::Expr(*else_expr, None), &mut v)?;
                                v
                            }
                            _ => return Err(ParseUnsupported("else branch".into())),
                        }
                    } else {
                        Vec::new()
                    };
                    out.push(IrStmt::If {
                        cond,
                        then_body,
                        else_body,
                    });
                }
                syn::Expr::While(e) => {
                    let cond = convert_expr(*e.cond)?;
                    let body = convert_block(Some(e.body))?;
                    out.push(IrStmt::While { cond, body });
                }
                // `loop { ... }` (Rust idiomatic 무한 루프) 지원.
                // `Stmt::Loop {  body }` 로 매핑한다
                // (decodegen.rs 가 이를 다시 `loop { ... }` 로 emit 한다).
                syn::Expr::Loop(l) => {
                    let body = convert_block(Some(l.body))?;
                    out.push(IrStmt::Loop { body });
                }
                syn::Expr::ForLoop(f) => {
                    let var = match &*f.pat {
                        syn::Pat::Ident(pi) => crate::block::sanitize_ident(&pi.ident.to_string()),
                        syn::Pat::Wild(_) => "_".to_string(),
                        _ => return Err(ParseUnsupported("for pat".into())),
                    };
                    let iter = convert_expr(*f.expr)?;
                    let body = convert_block(Some(f.body))?;
                    out.push(IrStmt::For { var, iter, body });
                }
                syn::Expr::Return(e) => {
                    let expr = match e.expr {
                        Some(inner) => convert_expr(*inner)?,
                        None => Expr::Int(0),
                    };
                    out.push(IrStmt::Return(expr));
                }
                other => {
                    out.push(IrStmt::Expr(convert_expr(other)?));
                }
            }
        }
        _ => return Err(ParseUnsupported("stmt".into())),
    }
    Ok(())
}
