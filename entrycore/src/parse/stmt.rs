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

/// `x = x + n` / `x = x - n` / `x = x + -n` 패턴에서 rhs delta 부분만 추출.
/// 패턴이면 Some(delta_expr), 아니면 None. lhs 의 var 이름은 sanitize 후 비교.
fn extract_change_variable_delta(lhs_name: &str, rhs: &syn::Expr) -> Option<syn::Expr> {
    let bin = match rhs {
        syn::Expr::Binary(b) => b,
        _ => return None,
    };
    // lhs 가 rhs 의 left 에 등장하는 형태만 인식한다.
    // `n + x` 처럼 rhs 가 뒤집힌 경우는 일반 set_var 로 둔다 (의미가 모호).
    let left_ident = match &*bin.left {
        syn::Expr::Path(p) if p.path.segments.len() == 1 => p.path.segments[0].ident.to_string(),
        _ => return None,
    };
    if left_ident != lhs_name {
        return None;
    }
    match bin.op {
        syn::BinOp::Add(_) => Some((*bin.right).clone()),
        syn::BinOp::Sub(_) => {
            // x - n을 x + -n 으로 통일 — delta 의 부호만 뒤집는다.
            Some(negate_expr((*bin.right).clone()))
        }
        _ => None,
    }
}

/// 표현식의 부호를 뒤집는다. 정수/실수 리터럴은 음수로, 그 외는 UnaryOp(Neg) 로 감싼다.
fn negate_expr(e: syn::Expr) -> syn::Expr {
    match e {
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => *expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(i),
            ..
        }) => {
            let n: i64 = i.base10_parse().unwrap_or(0);
            let neg = syn::LitInt::new(&format!("{}", -n), i.span());
            syn::Expr::Lit(syn::ExprLit {
                attrs: Vec::new(),
                lit: syn::Lit::Int(neg),
            })
        }
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Float(f),
            ..
        }) => {
            let v: f64 = f.base10_parse().unwrap_or(0.0);
            let neg = syn::LitFloat::new(&format!("{}", -v), f.span());
            syn::Expr::Lit(syn::ExprLit {
                attrs: Vec::new(),
                lit: syn::Lit::Float(neg),
            })
        }
        other => syn::Expr::Unary(syn::ExprUnary {
            attrs: Vec::new(),
            op: syn::UnOp::Neg(syn::token::Minus::default()),
            expr: Box::new(other),
        }),
    }
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
                // `var = expr;` → 패턴에 따라 ChangeVariable 또는 SetVar.
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
                    let san = crate::block::sanitize_ident(&name);
                    let rhs = *a.right;
                    // `x = x + n` 또는 `x = x - n` 패턴이면 Entry 의미
                    // `change_variable` 으로 복원. 양방향 왕복 시 `change_variable`
                    // 블록이 정확히 emit 되도록. 그 외 일반 대입은 `set_variable`.
                    if let Some(delta) = extract_change_variable_delta(&san, &rhs) {
                        let value = convert_expr(delta)?;
                        out.push(IrStmt::ChangeVariable {
                            variable: VarRef::new(san),
                            value,
                        });
                    } else {
                        let value = convert_expr(rhs)?;
                        out.push(IrStmt::SetVar(VarRef::new(san), value));
                    }
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
