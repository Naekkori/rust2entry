//! zip 작성 헬퍼.

use crate::Result;

/// zip 빌더.
pub struct ZipBuilder {
    buf: Vec<u8>,
}

impl ZipBuilder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// 파일 추가.
    pub fn add_file(&mut self, _name: &str, _data: &[u8]) -> Result<()> {
        todo!("네가 구현 - ZipWriter로 파일 추가")
    }

    /// 빌드 완성.
    pub fn finish(self) -> Result<Vec<u8>> {
        Ok(self.buf)
    }
}

impl Default for ZipBuilder {
    fn default() -> Self {
        Self::new()
    }
}
