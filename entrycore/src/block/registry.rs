//! 블록 매핑 레지스트리 (확장용).
//!
//! 이 모듈은 두 가지를 담당한다:
//! 1. IR stmt -> Entry `Block` 변환 규칙의 등록 지점 (`convert`).
//! 2. **Tier-0 스키마 검증 ATS** (`validate_schema` / `validate_json`).
//!
//! 스키마 검증은 entryjs 의 전체 블럭 정의(기본 · 인공지능 · 확장 · 하드웨어)를
//! JSON 덤프로 받아, 블럭 하나하나에 테스트를 쓰는 대신 "전 블럭을 순회하는
//! 제네릭 패스" 하나로 구조 불변식을 확인한다. 블럭 수와 무관하게 비용이
//! 일정하므로 수천 개의 하드웨어 블럭을 개별 ATS 없이 커버할 수 있다.

use crate::Result;
use serde::Deserialize;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// 스키마 덤프 (entryjs `dump_schema.js` 산출물의 역직렬화 타입)
// ─────────────────────────────────────────────────────────────────────────────

/// 스키마 덤프 루트. `{ generated, source, total, groupCount, groups: [...] }`.
#[derive(Debug, Deserialize)]
pub struct SchemaDump {
    #[serde(default)]
    pub generated: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub total: usize,
    #[serde(rename = "groupCount", default)]
    pub group_count: usize,
    #[serde(default)]
    pub groups: Vec<SchemaGroup>,
}

/// 하나의 원천 파일(= 그룹)과 그 안의 블럭들.
#[derive(Debug, Deserialize)]
pub struct SchemaGroup {
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub count: usize,
    pub blocks: HashMap<String, BlockSchema>,
}

/// 개별 블럭의 스키마 필드 (entryjs block 정의에서 추출한 부분집합).
#[derive(Debug, Deserialize)]
pub struct BlockSchema {
    #[serde(default)]
    pub skeleton: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(rename = "outerLine", default)]
    pub outer_line: Option<String>,
    #[serde(rename = "def_type", default)]
    pub def_type: Option<String>,
    #[serde(default)]
    pub has_func: bool,
    #[serde(default)]
    pub class: Option<String>,
    /// 각 param 의 type. `None` 항목은 타입이 없는(비정상) param.
    #[serde(default)]
    pub params: Option<Vec<Option<String>>>,
    #[serde(rename = "paramCount", default)]
    pub param_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// 검증 결과 타입
// ─────────────────────────────────────────────────────────────────────────────

/// 검증 체크 식별자 (Violation.check 에 들어가는 값).
pub const CHECK_SKELETON_PRESENT: &str = "skeleton_present";
pub const CHECK_DEF_TYPE_MATCH: &str = "def_type_match";
pub const CHECK_PARAMS_CONSISTENT: &str = "params_consistent";
pub const CHECK_PARAMS_TYPED: &str = "params_typed";
pub const CHECK_FUNC_EXPECTED: &str = "func_expected";

/// 하나의 스키마 불변식 위반.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub group: String,
    pub file: String,
    pub block: String,
    pub check: &'static str,
    pub detail: String,
}

/// 전체 블럭에 대한 검증 결과.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SchemaReport {
    pub total_blocks: usize,
    pub violations: Vec<Violation>,
}

impl SchemaReport {
    /// 특정 체크의 위반 수.
    pub fn count_by_check(&self, check: &str) -> usize {
        self.violations.iter().filter(|v| v.check == check).count()
    }

    /// 체크별 위반 수 (정렬된 목록).
    pub fn counts_by_check(&self) -> Vec<(&'static str, usize)> {
        use std::collections::BTreeSet;
        let checks: BTreeSet<&'static str> =
            self.violations.iter().map(|v| v.check).collect();
        checks
            .iter()
            .map(|c| (*c, self.count_by_check(c)))
            .collect()
    }

    /// 검증 통과 여부 (위반 0건).
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// func 를 기대하지 않는(순수 UI/라벨) skeleton 들.
/// 실행형 skeleton(`basic`/`basic_*_field`/`basic_event` 등)은 func 를 가져야
/// 하므로 이 목록에 없으면 func 부재를 위반으로 본다.
fn is_non_executable_skeleton(s: &str) -> bool {
    matches!(s, "basic_text" | "basic_button")
}

// ─────────────────────────────────────────────────────────────────────────────
// BlockRegistry + 스키마 검증
// ─────────────────────────────────────────────────────────────────────────────

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

    /// 스키마 덤프를 순회하며 구조 불변식을 검증한다.
    ///
    /// 단일 제네릭 패스로 전 블럭을 검사하므로, 블럭이 수천 개여도 비용이
    /// 블럭 수에 비례할 뿐 개별 테스트 작성 비용은 없다.
    pub fn validate_schema(&self, dump: &SchemaDump) -> SchemaReport {
        let mut report = SchemaReport::default();
        for g in &dump.groups {
            for (id, b) in &g.blocks {
                report.total_blocks += 1;

                // 1) skeleton_present — skeleton 은 존재해야 하고 비어있지 않아야.
                let skeleton_ok = b
                    .skeleton
                    .as_deref()
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);
                if !skeleton_ok {
                    report.violations.push(Violation {
                        group: g.group.clone(),
                        file: g.file.clone(),
                        block: id.clone(),
                        check: CHECK_SKELETON_PRESENT,
                        detail: format!("skeleton missing or empty: {:?}", b.skeleton),
                    });
                }

                // 2) def_type_match — def.type 은 블럭 id 와 일치해야.
                if let Some(dt) = &b.def_type {
                    if dt != id {
                        report.violations.push(Violation {
                            group: g.group.clone(),
                            file: g.file.clone(),
                            block: id.clone(),
                            check: CHECK_DEF_TYPE_MATCH,
                            detail: format!("def.type='{dt}' != block id '{id}'"),
                        });
                    }
                }

                // 3) params_consistent — params 배열 길이는 paramCount 와 일치해야.
                // 4) params_typed — 각 param 은 타입을 가져야.
                if let Some(params) = &b.params {
                    if params.len() != b.param_count {
                        report.violations.push(Violation {
                            group: g.group.clone(),
                            file: g.file.clone(),
                            block: id.clone(),
                            check: CHECK_PARAMS_CONSISTENT,
                            detail: format!(
                                "params len {} != paramCount {}",
                                params.len(),
                                b.param_count
                            ),
                        });
                    }
                    for (i, t) in params.iter().enumerate() {
                        if t.is_none() {
                            report.violations.push(Violation {
                                group: g.group.clone(),
                                file: g.file.clone(),
                                block: id.clone(),
                                check: CHECK_PARAMS_TYPED,
                                detail: format!("param[{i}] has no type"),
                            });
                        }
                    }
                }

                // 5) func_expected — 실행형 skeleton 은 func 를 가져야.
                if let Some(s) = &b.skeleton {
                    if !is_non_executable_skeleton(s) && !b.has_func {
                        report.violations.push(Violation {
                            group: g.group.clone(),
                            file: g.file.clone(),
                            block: id.clone(),
                            check: CHECK_FUNC_EXPECTED,
                            detail: format!("executable skeleton '{s}' missing func"),
                        });
                    }
                }
            }
        }
        report
    }

    /// 스키마 덤프 JSON 문자열을 파싱해 검증한다.
    pub fn validate_json(&self, json: &str) -> Result<SchemaReport> {
        let dump: SchemaDump = serde_json::from_str(json)?;
        Ok(self.validate_schema(&dump))
    }
}
