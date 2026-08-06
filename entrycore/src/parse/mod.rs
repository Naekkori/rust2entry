//! Rust 소스 파싱 -> IR.

mod block;
mod expr;
mod params;
mod stmt;

use syn::Item;

use crate::Error::UnmappedBlock;
use crate::ir::Program;
use crate::{Error, Result};

pub(crate) use block::convert_block;
pub(crate) use expr::convert_expr;
pub(crate) use params::collect_params;
pub(crate) use stmt::convert_stmt;

/// Rust 소스 문자열을 IR Program으로 변환.
pub fn parse(_source: &str) -> Result<Program> {
    let file: syn::File = syn::parse_str(_source).map_err(map_syn_err)?;
    let mut stmts = Vec::new();
    for item in file.items {
        convert_item(item, &mut stmts)?;
    }
    Ok(Program { stmts })
}

fn convert_item(item: Item, out: &mut Vec<crate::ir::Stmt>) -> Result<()> {
    match item {
        Item::Fn(f) => {
            let name = f.sig.ident.to_string();
            match name.as_str() {
                "when_start" => {
                    for s in &f.block.stmts {
                        convert_stmt(s.clone(), out)?;
                    }
                }
                "when_click" => {
                    for s in &f.block.stmts {
                        convert_stmt(s.clone(), out)?;
                    }
                }
                name if name.starts_with("when_") => {
                    for s in &f.block.stmts {
                        convert_stmt(s.clone(), out)?;
                    }
                }
                _ => {
                    let params = collect_params(&f.sig);
                    let body = convert_block(Some((*f.block).clone()))?;
                    out.push(crate::ir::Stmt::FuncDef { name, params, body });
                }
            }
            Ok(())
        }
        Item::Static(_) | Item::Const(_) => Err(UnmappedBlock("static/const".into())),
        _ => Err(UnmappedBlock("item".into())),
    }
}

/// syn 파싱 에러 -> 컴파일러 에러.
fn map_syn_err(e: syn::Error) -> Error {
    Error::Parse(e.to_string())
}
