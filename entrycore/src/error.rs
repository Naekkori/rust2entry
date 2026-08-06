use thiserror::Error;

/// 컴파일러 최상위 에러 타입.
#[derive(Debug, Error)]
pub enum Error {
    /// Rust 소스 파싱 실패.
    #[error("parse error: {0}")]
    Parse(String),

    /// 의미 분석 실패.
    #[error("semantic error: {0}")]
    Semantic(String),

    /// 코드 생성 실패.
    #[error("codegen error: {0}")]
    Codegen(String),

    /// 직렬화 실패.
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    /// .ent 패키징 실패.
    #[error("pack error: {0}")]
    Pack(String),

    /// IO 실패.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// zip 실패.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// 매핑되지 않은 블록.
    #[error("unmapped block: {0}")]
    UnmappedBlock(String),
}

/// 컴파일러 결과 타입 별칭.
pub type Result<T> = std::result::Result<T, Error>;
