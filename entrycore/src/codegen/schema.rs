//! Entry project.json 스키마.

use serde::Serialize;
use serde_json::Value;

/// Entry project.json 최상위 구조.
#[derive(Debug, Serialize)]
pub struct Project {
    pub name: String,
    pub objects: Vec<Sprite>,
    pub variables: Vec<Variable>,
    pub messages: Vec<Value>,
    pub functions: Vec<Function>,
    pub scenes: Vec<Scene>,
    pub speed: f64,
    pub interface: Interface,
    pub meta: Meta,
}

/// 스프라이트/오브젝트.
#[derive(Debug, Serialize)]
pub struct Sprite {
    pub id: String,
    pub name: String,
    pub object_type: String,
    pub sprite: SpriteInfo,
    pub selected_sound: Value,
    pub scene: String,
    pub script: Value,
}

/// 스프라이트 외형 정보.
#[derive(Debug, Serialize)]
pub struct SpriteInfo {
    pub name: String,
    pub category: String,
    pub pictures: Vec<Picture>,
}

/// 스프라이트 그림 한 장.
#[derive(Debug, Serialize)]
pub struct Picture {
    pub id: String,
    pub name: String,
    pub file_url: String,
    pub dimension: Dimension,
}

/// 그림 크기.
#[derive(Debug, Serialize)]
pub struct Dimension {
    pub width: i64,
    pub height: i64,
}

/// 변수.
#[derive(Debug, Serialize)]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub variable_type: String,
    pub value: Value,
    pub object: Value,
}

/// 함수.
#[derive(Debug, Serialize)]
pub struct Function {
    pub id: String,
    pub name: String,
    pub content: Value,
    pub param: Vec<Param>,
}

/// 함수 파라미터.
#[derive(Debug, Serialize)]
pub struct Param {
    pub name: String,
    pub value: Value,
}

/// 신 (장면).
#[derive(Debug, Serialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
}

/// 인터페이스 정보.
#[derive(Debug, Serialize)]
pub struct Interface {
    pub views: Vec<View>,
}

/// 뷰.
#[derive(Debug, Serialize)]
pub struct View {
    pub id: String,
    pub name: String,
    pub location: Vec<i64>,
    pub size: Vec<i64>,
}

/// 메타.
#[derive(Debug, Serialize)]
pub struct Meta {
    pub last_modified: String,
    pub created_at: String,
    pub version: String,
}
