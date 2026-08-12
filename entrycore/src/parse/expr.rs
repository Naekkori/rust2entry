//! syn::Expr -> IR Expr 변환.

use syn::Expr;

use crate::Error::UnmappedBlock;
use crate::Result;
use crate::ir::{self, Expr as IrExpr};

pub(crate) fn convert_expr(e: Expr) -> Result<IrExpr> {
    // 엔트리는 자바스크립트 기반으로 돌아가고 있다, 사용자가 넣을수있는건
    // Int, Float, String 뿐 그외는 판단블럭 에서 True/Flase 반환
    match e {
        Expr::Lit(lit) => match lit.lit {
            syn::Lit::Int(i) => {
                let n = i
                    .base10_parse()
                    .map_err(|e| UnmappedBlock(format!("int parse: {e}")))?;
                Ok(IrExpr::Int(n))
            }
            syn::Lit::Float(f) => {
                let v = f
                    .base10_parse()
                    .map_err(|e| UnmappedBlock(format!("float parse {e}")))?;
                Ok(IrExpr::Float(v))
            }
            syn::Lit::Str(s) => Ok(IrExpr::Str(s.value())),
            syn::Lit::Bool(b) => Ok(IrExpr::Bool(b.value())),
            _ => Err(UnmappedBlock("lit".into())),
        },
        Expr::Binary(b) => {
            let op = match b.op {
                syn::BinOp::Add(_) => ir::BinOp::Add,
                syn::BinOp::Sub(_) => ir::BinOp::Sub,
                syn::BinOp::Mul(_) => ir::BinOp::Mul,
                syn::BinOp::Div(_) => ir::BinOp::Div,
                syn::BinOp::Rem(_) => ir::BinOp::Mod,
                syn::BinOp::And(_) => ir::BinOp::And,
                syn::BinOp::Or(_) => ir::BinOp::Or,
                syn::BinOp::Eq(_) => ir::BinOp::Eq,
                syn::BinOp::Lt(_) => ir::BinOp::Lt,
                syn::BinOp::Le(_) => ir::BinOp::Le,
                syn::BinOp::Ne(_) => ir::BinOp::Ne,
                syn::BinOp::Ge(_) => ir::BinOp::Ge,
                syn::BinOp::Gt(_) => ir::BinOp::Gt,
                _ => return Err(UnmappedBlock("binop".into())),
            };
            Ok(IrExpr::BinOp(
                op,
                Box::new(convert_expr(*b.left)?),
                Box::new(convert_expr(*b.right)?),
            ))
        }
        Expr::Unary(u) => {
            let op = match u.op {
                syn::UnOp::Not(_) => ir::UnaryOp::Not,
                syn::UnOp::Neg(_) => ir::UnaryOp::Neg,
                _ => return Err(UnmappedBlock("unop".into())),
            };
            Ok(IrExpr::UnaryOp(op, Box::new(convert_expr(*u.expr)?)))
        }
        Expr::Call(c) => {
            let name = match &*c.func {
                Expr::Path(p) => path_to_name(&p.path)?,
                _ => return Err(UnmappedBlock("call func".into())),
            };
            let args = c
                .args
                .into_iter()
                .map(convert_expr)
                .collect::<Result<Vec<_>>>()?;
            Ok(IrExpr::Call(
                ir::FuncRef {
                    name,
                    arity: args.len(),
                },
                args,
            ))
        }
        Expr::Path(p) => {
            let name = path_to_name(&p.path)?;
            Ok(IrExpr::Var(name))
        }
        Expr::Paren(p) => convert_expr(*p.expr),
        Expr::Range(r) => {
            let start = r
                .start
                .as_deref()
                .ok_or_else(|| UnmappedBlock("range start".into()))?;
            let end = r
                .end
                .as_deref()
                .ok_or_else(|| UnmappedBlock("range end".into()))?;

            Ok(IrExpr::Range(
                Box::new(convert_expr(start.clone())?),
                Box::new(convert_expr(end.clone())?),
            ))
        }
        Expr::Reference(r) => convert_expr(*r.expr),
        _ => Err(UnmappedBlock("expr".into())),
    }
}

fn path_to_name(path: &syn::Path) -> Result<String> {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .ok_or_else(|| UnmappedBlock("empty path".into()))
}
