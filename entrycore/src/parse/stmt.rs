//! syn::Stmt -> IR Stmt 변환.

use syn::Stmt as SynStmt;

use crate::Error::UnmappedBlock;
use crate::Result;
use crate::ir::{Expr, Stmt as IrStmt};
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
        other => return Err(UnmappedBlock(format!("unknown var type: {other}"))),
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
                            return Err(UnmappedBlock("destructuring pattern".into()));
                        }
                    };
                    let k = type_to_kind(&pt.ty)?;
                    (ident, k)
                }
                _ => return Err(UnmappedBlock("destructuring pattern".into())),
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
                //엔트리 IF 는 두가지 종류가 있다.
                //IF
                //   [만약 <참> 이 라면]
                //    상태
                //   [블럭 끝]
                //IF ELSE
                //   [만약 <참> 이 라면]
                //    상태1
                //   [아니면]
                //    상태2
                //   [블럭 끝]
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
                            },
                            _ => return Err(UnmappedBlock("else branch".into())),
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
                syn::Expr::ForLoop(f)=>{
                    let var = match &*f.pat {
                        syn::Pat::Ident(pi)=>pi.ident.to_string(),
                        _=> return Err(UnmappedBlock("for pat".into()))
                    };
                    let iter = convert_expr(*f.expr)?;
                    let body = convert_block(Some(f.body))?;
                    out.push(IrStmt::For { var, iter, body });
                }
                other =>{
                    out.push(IrStmt::Expr(convert_expr(other)?));
                }
            }
        }
        _ => return Err(UnmappedBlock("stmt".into())),
    }
    Ok(())
}
