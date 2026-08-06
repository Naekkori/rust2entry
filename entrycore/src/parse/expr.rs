//! syn::Expr -> IR Expr 변환.

use syn::Expr;

use crate::Error::UnmappedBlock;
use crate::ir::Expr as IrExpr;
use crate::{Error, Result};

pub(crate) fn convert_expr(e: Expr) -> Result<IrExpr> {
    Err(UnmappedBlock("expr".into()))
    // TODO: Expr::Lit / Binary / Unary / Path / Call / Paren / Block 분기
}
