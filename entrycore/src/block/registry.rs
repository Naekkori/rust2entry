//! 블록 매핑 레지스트리 (확장용).

use crate::Result;

#[derive(Debug, Default)]
pub struct BlockRegistry {
    // 네가 채움: stmt -> Block 변환 규칙.
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// IR stmt -> Block.
    pub fn convert(&self, _stmt: &crate::ir::Stmt) -> Result<crate::block::Block> {
        todo!("네가 구현 - stmt -> Block")
    }
}
