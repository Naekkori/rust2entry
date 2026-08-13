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
use std::sync::{Mutex, OnceLock};

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
#[derive(Debug, Clone, Deserialize)]
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

/// 하드웨어 소스맵 덤프 (`hw_sourcemap.json` 산출물) 루트.
///
/// `{ generated, source, deviceCount, blockTotal, loaded, failed, devices: [...] }`.
/// 각 장치의 `blocks` 는 `SchemaGroup.blocks` 와 동일한 `BlockSchema` 형식이므로
/// 기존 `validate_schema` 경로를 그대로 재사용해 장치별 블럭을 검증할 수 있다.
#[derive(Debug, Deserialize)]
pub struct HwSourcemap {
    #[serde(default)]
    pub generated: String,
    #[serde(default)]
    pub source: String,
    #[serde(rename = "deviceCount", default)]
    pub device_count: usize,
    #[serde(rename = "blockTotal", default)]
    pub block_total: usize,
    #[serde(default)]
    pub loaded: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub devices: Vec<HwDevice>,
}

/// 하나의 하드웨어 장치(= 원천 파일)와 그 안의 블럭들.
#[derive(Debug, Deserialize)]
pub struct HwDevice {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub file: String,
    #[serde(rename = "blockCount", default)]
    pub block_count: usize,
    /// 장치별 블럭. `SchemaGroup.blocks` 와 동일한 `BlockSchema` 부분집합.
    pub blocks: HashMap<String, BlockSchema>,
}

impl HwSourcemap {
    /// 장치 수 (`devices` 배열 길이).
    pub fn device_count(&self) -> usize {
        self.device_count
    }

    /// 소스맵 메타데이터의 총 블럭 수 (`blockTotal`).
    pub fn block_total(&self) -> usize {
        self.block_total
    }

    /// 로드 성공 블럭 수 (`loaded`).
    pub fn loaded(&self) -> usize {
        self.loaded
    }

    /// 로드 실패 블럭 수 (`failed`).
    pub fn failed(&self) -> usize {
        self.failed
    }

    /// 실제 `devices[].blocks` 를 모두 순회해 센 블럭 수 합계.
    /// 메타데이터(`blockTotal`)와 달리 파싱된 실제 블럭 수다.
    pub fn block_count(&self) -> usize {
        self.devices.iter().map(|d| d.blocks.len()).sum()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 전역 하드웨어 블럭 인덱스 (build/extract 파이프라인 주입용)
// ─────────────────────────────────────────────────────────────────────────────

/// 전역 하드웨어 블럭 인덱스: type_id -> BlockSchema.
/// CLI(`entryc`)가 run_build / run_extract 시작 시 `set_hw_index` 로 설정한다.
/// 정방향 `from_stmt`/`from_expr` 와 역방향 `block_from_value` 가 이 인덱스로
/// 하드웨어 블럭을 인식해 `Block::Raw` 로 처리한다.
static HW_INDEX: OnceLock<Mutex<Option<HashMap<String, BlockSchema>>>> = OnceLock::new();

fn hw_index_lock() -> &'static Mutex<Option<HashMap<String, BlockSchema>>> {
    HW_INDEX.get_or_init(|| Mutex::new(None))
}

/// 하드웨어 소스맵에서 type_id -> BlockSchema flat 인덱스를 세팅한다.
/// (모든 장치의 `blocks` 를 평탄화.)
pub fn set_hw_index(map: &HwSourcemap) {
    let mut g = hw_index_lock().lock().unwrap();
    *g = Some(flatten_hw_index(map));
}

/// 전역 인덱스를 비운다 (테스트 정리용).
pub fn clear_hw_index() {
    *hw_index_lock().lock().unwrap() = None;
}

/// id 가 하드웨어 블럭 type_id 인지 (인덱스에 있으면 true).
pub fn is_hw_block(id: &str) -> bool {
    hw_schema(id).is_some()
}

/// 하드웨어 블럭 스키마 조회 (인덱스에 있으면 해당 BlockSchema 클론).
pub fn hw_schema(id: &str) -> Option<BlockSchema> {
    hw_index_lock()
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|m| m.get(id))
        .cloned()
}

fn flatten_hw_index(map: &HwSourcemap) -> HashMap<String, BlockSchema> {
    let mut out = HashMap::new();
    for d in &map.devices {
        for (id, b) in &d.blocks {
            out.insert(id.clone(), b.clone());
        }
    }
    out
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

    /// 하드웨어 소스맵 JSON 문자열을 파싱한다.
    pub fn parse_hw_sourcemap(&self, json: &str) -> Result<HwSourcemap> {
        Ok(serde_json::from_str(json)?)
    }

    /// 하드웨어 소스맵의 장치별 블럭을 **기존 `validate_schema` 경로**로 순회해
    /// `SchemaReport` 를 낸다. 장치 하나를 `SchemaGroup`(name/file/blocks) 으로
    /// 투영해 검증 로직을 완전히 재사용하므로, 장치 블럭도 기본·AI·확장 블럭과
    /// 동일한 구조 불변식을 적용받는다.
    pub fn validate_hw_sourcemap(&self, map: &HwSourcemap) -> SchemaReport {
        // 장치를 SchemaGroup 으로 투영: group=name, file=file, count=blockCount.
        let dump = SchemaDump {
            generated: map.generated.clone(),
            source: map.source.clone(),
            total: map.block_total,
            group_count: map.device_count,
            groups: map
                .devices
                .iter()
                .map(|d| SchemaGroup {
                    group: d.name.clone(),
                    file: d.file.clone(),
                    count: d.block_count,
                    blocks: d.blocks.clone(),
                })
                .collect(),
        };
        self.validate_schema(&dump)
    }

    /// 하드웨어 소스맵 JSON 문자열을 파싱한 뒤 `validate_hw_sourcemap` 로 검증.
    pub fn validate_hw_sourcemap_json(&self, json: &str) -> Result<SchemaReport> {
        let map = self.parse_hw_sourcemap(json)?;
        Ok(self.validate_hw_sourcemap(&map))
    }
}
