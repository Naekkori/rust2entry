//! Rust 소스 파싱 -> IR.

mod block;
mod expr;
mod params;
mod stmt;

use syn::Item;

use crate::Error::UnmappedBlock;
use crate::ir::{Program, Stmt as IrStmt};
use crate::{Error, Result};

pub(crate) use block::convert_block;
pub(crate) use expr::convert_expr;
pub(crate) use params::collect_params;
pub(crate) use stmt::convert_stmt;

/// 트리거 함수 한 개의 정보. Entry object.script 의 thread 1개에 대응.
/// `name` 은 함수명 ("when_start" 등), `body` 는 본문 stmt 시퀀스.
#[derive(Debug, Clone)]
pub struct TriggerDef {
    pub name: String,
    /// when_message 함수의 params[0] 가 메시지 이름. 그 외는 빈 벡터.
    pub params: Vec<String>,
    pub body: Vec<IrStmt>,
}

/// Rust 소스 문자열을 IR Program으로 변환.
pub fn parse(_source: &str) -> Result<Program> {
    let file: syn::File = syn::parse_str(_source).map_err(map_syn_err)?;
    let mut stmts = Vec::new();
    for item in file.items {
        convert_item(item, &mut stmts, None)?;
    }
    Ok(Program { stmts })
}

/// Rust 소스 문자열을 (IR Program, 트리거 함수 목록) 으로 변환.
///
/// `parse` 와 다른 점: `when_*` 함수의 body 가 stmts 에 평탄화되지 않고
/// 별도 `TriggerDef` 로 분리된다. build 시 Entry object.script 의 thread 별
/// 그룹화에 사용. deparse 라운드트립(extract)과의 호환을 위해 `parse` 는
/// 평탄화 동작을 유지한다.
pub fn parse_with_triggers(_source: &str) -> Result<(Program, Vec<TriggerDef>)> {
    let file: syn::File = syn::parse_str(_source).map_err(map_syn_err)?;
    let mut stmts = Vec::new();
    let mut triggers: Vec<TriggerDef> = Vec::new();
    for item in file.items {
        convert_item(item, &mut stmts, Some(&mut triggers))?;
    }
    Ok((Program { stmts }, triggers))
}

fn convert_item(
    item: Item,
    out: &mut Vec<IrStmt>,
    triggers: Option<&mut Vec<TriggerDef>>,
) -> Result<()> {
    match item {
        Item::Fn(f) => {
            let name = f.sig.ident.to_string();
            let is_trigger = is_trigger_name(&name);
            if is_trigger {
                if let Some(t) = triggers {
                    let mut body = Vec::new();
                    for s in &f.block.stmts {
                        convert_stmt(s.clone(), &mut body)?;
                    }
                    let params = collect_params(&f.sig);
                    t.push(TriggerDef { name, params, body });
                } else {
                    // parse 모드: 평탄화 (기존 동작 보존)
                    for s in &f.block.stmts {
                        convert_stmt(s.clone(), out)?;
                    }
                }
            } else {
                let params = collect_params(&f.sig);
                let body = convert_block(Some((*f.block).clone()))?;
                out.push(IrStmt::FuncDef { name, params, body });
            }
            Ok(())
        }
        Item::Static(_) | Item::Const(_) => Err(UnmappedBlock("static/const".into())),
        _ => Err(UnmappedBlock("item".into())),
    }
}

/// Entry 가 시작점으로 인식하는 트리거 함수 이름 판별.
fn is_trigger_name(name: &str) -> bool {
    name.starts_with("when_")
}

/// syn 파싱 에러 -> 컴파일러 에러.
fn map_syn_err(e: syn::Error) -> Error {
    Error::Parse(e.to_string())
}
