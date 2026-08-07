//! IR -> Block -> project.json 변환.

pub mod schema;

use crate::Result;
use crate::block::{from_stmt, to_value};
use crate::ir::Program;
use serde_json::Value;

/// IR Program -> Entry project.json.
pub fn generate(program: &Program) -> Result<Value> {
    // IR stmt들을 Block으로 변환한 뒤 to_value() 호출.
    // project.json 최상위 구조는 schema::Project 참고.
    let blocks: Result<Vec<_>> = program.stmts.iter().map(from_stmt).collect();
    let scripts = blocks?.into_iter().map(|b| to_value(&b)).collect::<Result<Vec<_>>>()?;

    let project = serde_json::json!({
        "speed": 60,
        "objects": [],
        "variables": [],
        "messages": [],
        "functions": [],
        "scenes": [{"id": "scene1", "name": "장면1"}],
        "interface": { "views": [] },
        "meta": {
            "last_modified": "2026-01-01T00:00:00.000Z", //에
            "created_at": "2026-01-01T00:00:00.000Z",
            "version": "0.1.0"
        },
        "scripts": scripts,
    });
    Ok(project)
}
