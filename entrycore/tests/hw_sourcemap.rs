//! 하드웨어 소스맵(`hw_sourcemap.json`) 파싱·검증 통합 테스트.
//!
//! 하드웨어 블럭은 수천 개라 하나하나 ATS 를 쓸 수 없으므로, `tool/` CLI 가
//! entryjs 에서 하드웨어 장치별 블럭 스키마를 뽑아낸 `hw_sourcemap.json` 을
//! `parse_hw_sourcemap` 으로 파싱하고, 장치별 블럭을 기존 `validate_schema`
//! 경로(`validate_hw_sourcemap`)로 검증한다. 이 테스트는 그 파이프라인이
//! 실제로 유효한 소스맵에서 작동함을 고정(freeze)한다.

use entrycore::block::registry::{BlockRegistry, HwSourcemap, CHECK_DEF_TYPE_MATCH};

fn load_fixture() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hw_sourcemap.json");
    std::fs::read_to_string(path).expect("hw_sourcemap.json fixture exists")
}

fn parse() -> HwSourcemap {
    let registry = BlockRegistry::new();
    registry
        .parse_hw_sourcemap(&load_fixture())
        .expect("hw sourcemap parses")
}

#[test]
fn parses_hw_sourcemap_counts() {
    let map = parse();

    // tool/ CLI 가 뽑아낸 메타데이터.
    assert_eq!(map.device_count(), 201, "장치 수");
    assert_eq!(map.block_total(), 5531, "메타데이터 총 블럭 수");
    assert_eq!(map.loaded(), 201, "로드 성공 장치 수");
    assert_eq!(map.failed(), 0, "로드 실패 장치 수 (로더 보강 후 0)");

    // devices[].blocks 를 실제 순회해 센 합계가 메타데이터와 일치.
    assert_eq!(map.block_count(), 5531, "devices 블럭 합계 == blockTotal");
}

#[test]
fn schema_validation_covers_all_hardware_blocks() {
    let registry = BlockRegistry::new();
    let map = parse();

    // 전 하드웨어 블럭이 단일 제네릭 패스로 스키마 검증된다.
    let report = registry.validate_hw_sourcemap(&map);
    assert_eq!(report.total_blocks, 5531, "검증된 블럭 수");

    // 하드웨어에도 실제 스키마 버그가 존재한다 (빈 skeleton, def.type 불일치,
    // param 타입 누락, func 부재). ATS 가 이를 감지해야 한다.
    // 현재 fixture 스냅샷 기준 위반 구성:
    //   skeleton_present 73, def_type_match 3, params_typed 11, func_expected 6
    assert_eq!(report.violations.len(), 93, "하드웨어 블럭 스키마 위반 수 (스냅샷)");
    assert_eq!(report.count_by_check("params_consistent"), 0, "모든 블럭 params 길이 == paramCount");
}

#[test]
fn detects_real_hardware_def_type_bugs() {
    let registry = BlockRegistry::new();
    let map = parse();
    let report = registry.validate_hw_sourcemap(&map);

    // 하드웨어 블럭의 def.type 불일치 버그를 감지한다.
    let def_mismatches: Vec<&str> = report
        .violations
        .iter()
        .filter(|v| v.check == CHECK_DEF_TYPE_MATCH)
        .map(|v| v.block.as_str())
        .collect();
    assert_eq!(def_mismatches.len(), 3, "def.type 불일치 3건");
    assert!(def_mismatches.contains(&"coconut_tmp_senser"));
    assert!(def_mismatches.contains(&"dalgona_step_rotate3"));
    assert!(def_mismatches.contains(&"edumaker_tone_value"));
}

#[test]
fn base_modules_without_getblocks_stay_at_zero() {
    // getBlocks 가 없는 순수 base/헬퍼 모듈은 blockCount:0 으로 남는다
    // (로드 실패가 아니라 정상 — 자식 장치가 블럭을 물려받는다).
    let map = parse();
    let zero: Vec<&str> = map
        .devices
        .iter()
        .filter(|d| d.blocks.is_empty())
        .map(|d| d.name.as_str())
        .collect();
    assert!(
        zero.contains(&"block_byrobot_base"),
        "base 모듈은 블럭이 없어야 함: {zero:?}"
    );
    assert!(zero.contains(&"block_telliot_Base"));
    assert!(zero.contains(&"block_roborobo_base"));
}

#[test]
fn hardware_blocks_have_consistent_params() {
    let registry = BlockRegistry::new();
    let map = parse();
    let report = registry.validate_hw_sourcemap(&map);

    // params 길이 == paramCount 는 모든 하드웨어 블럭에서 유지.
    assert_eq!(
        report.count_by_check("params_consistent"),
        0,
        "모든 하드웨어 블럭 params 길이 == paramCount"
    );
}
