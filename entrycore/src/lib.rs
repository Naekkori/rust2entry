//! 코어 라이브러리: Rust 소스 -> IR / IR -> Entry 블록 직렬화.
//! + Entry 블록 -> IR 역변환.

pub mod block;
pub mod codegen;
pub mod decodegen;
pub mod deparse;
pub mod error;
pub mod ir;
pub mod parse;
pub mod var;

pub use error::{Error, Result};
pub use var::{VarInfo, VarInit, VarKind, VarMap};
use serde_json::{Value, json};

use crate::ir::{Program, Stmt};

/// 여러 .rs 소스 + base project.json -> 최종 project.json.
///
/// ## 동작
/// 1. 각 .rs 소스를 `parse::parse` -> IR Program.
/// 2. 모든 Program 의 stmts 를 순서대로 합쳐 단일 Program.
/// 3. `codegen::generate(&merged, base)` 호출 -> scripts/variables 패치.
/// 4. base 의 `objects` 가 비어있고 rs_sources 가 있으면, 첫 소스 이름을 가진
///    가짜 오브젝트 1개를 추가 (extract 라운드트립용).
///
/// ## 주의
/// - `objects` 패치는 extract 와의 대칭을 위한 최소 형태. 실제 EntryJS 임포트용
///   스프라이트 메타(이미지 등)는 별도 작업에서 처리.
pub fn compile(rs_sources: &[(&str, &str)], base: &Value) -> Result<Value> {
    let mut merged_stmts: Vec<Stmt> = Vec::new();
    for (_name, src) in rs_sources  {
        let program = parse::parse(src)?;
        merged_stmts.extend(program.stmts);
    }
    let merged_program = Program {stmts:merged_stmts};
    let mut project = codegen::generate(&merged_program, base)?;

    // objects 비어있고 소스가 있으면 가짜 오브젝트 1개 추가 (extract 라운드트립)
    let objects_empty = project
        .get("objects")
        .and_then(|v| v.as_array())
        .map(|a| a.is_empty())
        .unwrap_or(true);
    if objects_empty && !rs_sources.is_empty() {
        let (name, _) = rs_sources[0];
        project["objects"] = json!([{
            "id": "obj1",
            "name": name,
            "script": Value::Null,
            "objectType": "sprite",
            "scene": "scene1",
            "selectedPictureId": Value::Null,
            "sprite": {
                "name": name,
                "pictures": [],
                "sounds": []
            },
            "entity": {
                "rotation": 0.0,
                "direction": 90.0,
                "x": 0.0,
                "y": 0.0,
                "regX": 0.0,
                "regY": 0.0,
                "scaleX": 1.0,
                "scaleY": 1.0,
                "width": 0.0,
                "height": 0.0,
                "visible": true
            }
        }]);
    }

    Ok(project)
}