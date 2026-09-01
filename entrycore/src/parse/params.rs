//함수 시그니처에서 파라미터 이름 + 타입 수집.

use syn::{FnArg, Pat, Signature, Type};

/// Param 타입 — EntryJS function_param_string/boolean 매핑.
/// `StringParam` (default) 또는 `BoolParam`.
fn type_to_param_kind(ty: &Type) -> crate::ir::ParamKind {
    use crate::ir::ParamKind;
    if let Type::Path(tp) = ty
        && let Some(last) = tp.path.segments.last() {
            match last.ident.to_string().as_str() {
                "BoolParam" | "bool" => return ParamKind::Bool,
                // StringParam, &str, &String, String, i32, f64, 그 외 → String (default)
                _ => return ParamKind::String,
            }
        }
    ParamKind::String
}

/// fn f(x: i32, y: BoolParam) -> [(x, String), (y, Bool)]
pub(crate) fn collect_params(sig: &Signature) -> Vec<(String, crate::ir::ParamKind)> {
    sig.inputs
        .iter()
        .filter_map(|arg| match arg {
            // self 무시
            FnArg::Receiver(_) => None,
            // x: T ...
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(pi) => {
                    let name = pi.ident.to_string();
                    let kind = type_to_param_kind(&pat_type.ty);
                    Some((name, kind))
                }
                _ => Some((String::from("_pattern"), crate::ir::ParamKind::String)),
            },
        })
        .collect()
}
