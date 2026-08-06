//! Entry project.json 스키마 (참고용).
//!
//! `codegen::generate`가 직접 `serde_json::json!{}` 매크로로 출력하므로
//! 이 struct는 실제 직렬화에 쓰이지 않고, 형식 명세 역할.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Project {
    pub speed: f64,
    pub objects: Vec<Sprite>,
    pub variables: Vec<Variable>,
    pub messages: Vec<Message>,
    pub functions: Vec<Function>,
    pub scenes: Vec<Scene>,
    pub interface: Interface,
    pub meta: Meta,
}

#[derive(Debug, Serialize)]
pub struct Sprite {
    pub id: String,
    pub name: String,
    pub object_type: String,
    pub scene: String,
    pub script: Vec<Thread>,
    pub sprite: SpriteInfo,
    pub entity: Entity,
    pub selected_picture_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpriteInfo {
    pub name: String,
    pub pictures: Vec<Picture>,
    pub sounds: Vec<Sound>,
}

#[derive(Debug, Serialize)]
pub struct Picture {
    pub id: String,
    pub name: String,
    pub fileurl: String,
    pub dimension: Dimension,
}

#[derive(Debug, Serialize)]
pub struct Dimension {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Serialize)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub fileurl: String,
    pub duration: f64,
}

#[derive(Debug, Serialize)]
pub struct Entity {
    pub rotation: f64,
    pub direction: f64,
    pub x: f64,
    pub y: f64,
    pub reg_x: f64,
    pub reg_y: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
}

#[derive(Debug, Serialize)]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub variable_type: String,
    pub value: String,
    pub object: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Function {
    pub id: String,
    pub name: String,
    pub content: Vec<Thread>,
    pub param: Vec<Param>,
}

#[derive(Debug, Serialize)]
pub struct Param {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Interface {
    pub views: Vec<View>,
}

#[derive(Debug, Serialize)]
pub struct View {
    pub id: String,
    pub name: String,
    pub location: Vec<i64>,
    pub size: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub last_modified: String,
    pub created_at: String,
    pub version: String,
}

/// 스레드 = 블록의 선형 배열.
#[derive(Debug, Serialize)]
pub struct Thread {
    pub blocks: Vec<serde_json::Value>,
}
