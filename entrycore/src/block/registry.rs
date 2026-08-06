use crate::{Result, block::Block};

/// Rust 표현 -> Entry 블록 매핑 레지스트리.
/// 키: 캐노니컬 표현 문자열 (예: "if_stmt", "while_stmt", "var_decl").
pub type BlockKey = String;

#[derive(Debug, Default)]
pub struct BlockRegistry {
    // 네가 채움: 키 -> 변환 함수 매핑.
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 새 변환 등록.
    pub fn register<F>(&mut self, _key: BlockKey, _convert: F) -> Result<()>
    where
        F: Fn(&crate::ir::Stmt) -> Result<Block> + 'static,
    {
        todo!("네가 구현 - map insert")
    }

    /// IR 명령문 -> Entry 블록.
    pub fn convert(&self, _stmt: &crate::ir::Stmt) -> Result<Block> {
        todo!("네가 구현 - stmt 분기 매칭")
    }

    /// 디버그용: 등록된 키 목록.
    pub fn keys(&self) -> Vec<&BlockKey> {
        todo!("네가 구현 - map.keys collect")
    }
}
