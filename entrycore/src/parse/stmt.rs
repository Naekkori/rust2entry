//! syn::Stmt -> IR Stmt 변환.

use syn::Stmt as SynStmt;

use crate::Error::UnmappedBlock;
use crate::Result;
use crate::ir::{Expr, Stmt as IrStmt};
use crate::parse::convert_expr;

pub(crate) fn convert_stmt(s: SynStmt, out: &mut Vec<IrStmt>) -> Result<()> {
    match s {
        SynStmt::Local(local) => {
            // 변수명 추출
            let name = match &local.pat {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                _ => return Err(UnmappedBlock("destructuring pattern".into())),
            };
            // 초기값
            let init = match local.init {
                Some(i) => convert_expr(*i.expr)?,
                None => Expr::Int(0),
            };
            out.push(IrStmt::VarDecl(name, init));
        }
        SynStmt::Expr(expr, _semi) => {
            out.push(IrStmt::Expr(convert_expr(expr)?));
        }
        _ => return Err(UnmappedBlock("stmt".into())),
    }
    Ok(())
}
