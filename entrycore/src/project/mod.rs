//! .ent zip 패키징.

pub mod pack;

use crate::Result;
use std::path::Path;

/// project.json + 스프라이트 -> .ent zip 바이트스트림.
pub fn build(_project: serde_json::Value, _sprites: Option<&Path>) -> Result<Vec<u8>> {
    todo!("네가 구현 - zip 빌드")
}

/// 스프라이트 디렉토리 -> (이름, 데이터) 목록.
pub fn collect_sprites(_dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    todo!("네가 구현 - walkdir/스프라이트 수집")
}
