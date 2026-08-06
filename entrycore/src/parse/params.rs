//함수 시그니처에서 파라미터 이름 수집.

use syn::{FnArg, Pat, Signature};

/// fn f(x: i32, y: String) -> ["x", "y"]
pub(crate) fn collect_params(sig: &Signature) -> Vec<String> {
    sig.inputs
        .iter()
        .filter_map(|arg| match arg {
            // self 무시
            FnArg::Receiver(_) => None,
            // x:i32, y:String ...
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(pi) => Some(pi.ident.to_string()),
                _ => Some(String::from("_pattern")),
            },
        })
        .collect()
}
