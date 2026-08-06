//! syn::Block -> IR Vec<Stmt> 변환.

use syn::Block;

use crate::ir::Stmt as IrStmt;
use crate::parse::convert_stmt;
use crate::Result;

pub(crate) fn convert_block(block: Option<Block>) -> Result<Vec<IrStmt>> {
    let mut out = Vec::new();
    if let Some(b) = block {
        for s in b.stmts {
            convert_stmt(s, &mut out)?;
        }
    }
    Ok(out)
}
