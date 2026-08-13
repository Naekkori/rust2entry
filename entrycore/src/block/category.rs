use serde::{Deserialize, Serialize};

/// Entry 블록 카테고리.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Category {
    /// 일반 동작 (흐름 아닌 부수효과).
    General,
    /// 흐름 (반복, 조건, 함수).
    Flow,
    /// 판정 / 비교.
    Judgment,
    /// 산술 / 수 연산.
    Calc,
    /// 문자열.
    String,
    /// 변수.
    Variable,
    /// 자료구조 (리스트 등).
    Data,
    /// 이벤트 시작.
    Start,
    /// 외형 / 스프라이트.
    Looks,
    /// 소리.
    Sound,
    /// 펜.
    Pen,
    /// 정의 (함수).
    Define,
    /// 하드웨어 (소스맵 기반 동적 블럭).
    Hardware,
    /// 미분류.
    #[default]
    Unknown,
}

