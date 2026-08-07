//! .ent zip 패키징.

pub mod pack;
pub mod sprite;

use crate::Result;
use std::path::Path;

pub use sprite::{PictureEntry, PictureMeta, SoundEntry, SoundMeta, SpriteEntry};

/// 스프라이트 디렉토리 -> SpriteEntry 목록.
pub fn collect_sprites(_dir: &Path) -> Result<Vec<SpriteEntry>> {
    sprite::collect_sprites(_dir)
}

/// project.json + 스프라이트 -> .ent zip 바이트스트림.
pub fn build(_project: serde_json::Value, _sprites: Option<&Path>) -> Result<Vec<u8>> {
    todo!("네가 구현 - zip 빌드")
}