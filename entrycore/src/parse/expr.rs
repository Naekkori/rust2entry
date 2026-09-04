//! syn::Expr -> IR Expr 변환.

use syn::Expr;
use syn::spanned::Spanned;

use crate::Error::ParseUnsupported;
use crate::Result;
use crate::ir::{self, Expr as IrExpr};
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

/// 소스의 `// @hwraw {...}` 주석에서 추출한 하드웨어 블럭 raw JSON 큐.
/// decodegen 이 post-order 로 emit 한 순서대로, convert_expr 가 하드웨어 Call 을
/// 만날 때마다 pop 해 `FuncRef.raw` 에 담는다.
static RAW_QUEUE: OnceLock<Mutex<VecDeque<serde_json::Value>>> = OnceLock::new();
fn raw_queue() -> &'static Mutex<VecDeque<serde_json::Value>> {
    RAW_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// 소스 텍스트에서 `// @hwraw {json}` 주석을 순서대로 큐에 넣는다.
pub(crate) fn prepare_raw_map(source: &str) {
    let mut q = raw_queue().lock().unwrap();
    q.clear();
    for line in source.lines() {
        if let Some(pos) = line.find("// @hwraw ") {
            let rest = line[pos + "// @hwraw ".len()..].trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(rest) {
                q.push_back(v);
            }
        }
    }
}

fn pop_raw() -> Option<serde_json::Value> {
    raw_queue().lock().unwrap().pop_front()
}

pub(crate) fn convert_expr(e: Expr) -> Result<IrExpr> {
    let line = e.span().start().line;
    // 엔트리는 자바스크립트 기반으로 돌아가고 있다, 사용자가 넣을수있는건
    // Int, Float, String 뿐 그외는 판단블럭 에서 True/Flase 반환
    match e {
        Expr::Lit(lit) => match lit.lit {
            syn::Lit::Int(i) => {
                let n = i
                    .base10_parse()
                    .map_err(|e| ParseUnsupported(format!("int parse: {e}")))?;
                Ok(IrExpr::Int(n))
            }
            syn::Lit::Float(f) => {
                let v = f
                    .base10_parse()
                    .map_err(|e| ParseUnsupported(format!("float parse {e}")))?;
                Ok(IrExpr::Float(v))
            }
            syn::Lit::Str(s) => Ok(IrExpr::Str(s.value())),
            syn::Lit::Bool(b) => Ok(IrExpr::Bool(b.value())),
            _ => Err(ParseUnsupported("lit".into())),
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
                _ => return Err(ParseUnsupported("binop".into())),
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
                _ => return Err(ParseUnsupported("unop".into())),
            };
            Ok(IrExpr::UnaryOp(op, Box::new(convert_expr(*u.expr)?)))
        }
        Expr::Call(c) => {
            let name = match &*c.func {
                Expr::Path(p) => path_to_name(&p.path)?,
                _ => return Err(ParseUnsupported("call func".into())),
            };
            let args = c
                .args
                .into_iter()
                .map(convert_expr)
                .collect::<Result<Vec<_>>>()?;
            // 하드웨어 블럭 호출이면 소스의 `// @hwraw {...}` 주석(raw)을 큐에서 pop.
            let raw = if crate::block::registry::is_hw_block(&name) {
                pop_raw()
            } else {
                None
            };
            Ok(IrExpr::Call(
                ir::FuncRef {
                    name,
                    arity: args.len(),
                    raw,
                },
                args,
            ))
        }
        Expr::Path(p) => {
            let segments = path_to_segments(&p.path)?;
            if segments.len() == 1 {
                Ok(IrExpr::Var(segments.into_iter().next().unwrap()))
            } else {
                Ok(IrExpr::Path(segments))
            }
        }
        Expr::Paren(p) => convert_expr(*p.expr),
        Expr::Range(r) => {
            let start = r
                .start
                .as_deref()
                .ok_or_else(|| ParseUnsupported("range start".into()))?;
            let end = r
                .end
                .as_deref()
                .ok_or_else(|| ParseUnsupported("range end".into()))?;

            Ok(IrExpr::Range(
                Box::new(convert_expr(start.clone())?),
                Box::new(convert_expr(end.clone())?),
            ))
        }
        Expr::Reference(r) => convert_expr(*r.expr),
        Expr::Assign(_) => Err(ParseUnsupported(
            "expr at assign; convert_stmt로 처리".into(),
        )),
        _ => Err(ParseUnsupported(format!("expr at line {line}"))),
    }
}

fn path_to_name(path: &syn::Path) -> Result<String> {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .ok_or_else(|| ParseUnsupported("empty path".into()))
}

fn path_to_segments(path: &syn::Path) -> Result<Vec<String>> {
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if segments.is_empty() {
        return Err(ParseUnsupported("empty path".into()));
    }
    Ok(segments)
}
