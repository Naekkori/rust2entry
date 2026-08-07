//! zip 작성 헬퍼.

use std::io::{Cursor, Write};

use zip::{ZipWriter, write::SimpleFileOptions};

use crate::Result;


/// zip 빌더.
pub struct ZipBuilder {
    entries: Vec<(String, Vec<u8>)>
}

impl ZipBuilder {
    pub fn new() -> Self {
        Self{
            entries: Vec::new(),
        }
    }

    /// 파일 추가.
    pub fn add_file(&mut self, _name: &str, _data: &[u8]) -> Result<()> {
        self.entries.push((_name.to_string(),_data.to_vec()));
        Ok(())
    }

    /// 빌드 완성.
    pub fn finish(self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
        
        for (name, data) in self.entries  {
            zip.start_file(&name, opts)?;
            zip.write_all(&data)?;
        }
        zip.finish()?;
        Ok(buf)
    }
}

impl Default for ZipBuilder {
    fn default() -> Self {
        Self::new()
    }
}
