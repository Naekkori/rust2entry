//! Tier-0 스키마 검증 ATS 통합 테스트.
//!
//! entryjs 전체 블럭 스키마 덤프(기본·AI·확장·하드웨어 포함, `fixtures/blocks-schema.json`)
//! 를 로드해 단일 제네릭 패스로 검증한다. 이 테스트는 "수천 개 블럭을 개별 ATS
//! 없이 하나의 순회로 커버"하는 접근이 실제로 entryjs 의 스키마 버그를 감지하는지
//! 고정(freeze)한다.

use entrycore::block::registry::{BlockRegistry, CHECK_DEF_TYPE_MATCH};

fn load_dump() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/blocks-schema.json");
    std::fs::read_to_string(path).expect("blocks-schema.json fixture exists")
}

#[test]
fn validates_every_block_in_one_pass() {
    let registry = BlockRegistry::new();
    let report = registry.validate_json(&load_dump()).expect("dump parses");

    // 하드웨어 포함 전체 블럭이 한 패스로 검증된다.
    assert_eq!(report.total_blocks, 4281, "모든 블럭(기본+AI+확장+하드웨어)을 순회");
}

#[test]
fn params_are_consistent() {
    let registry = BlockRegistry::new();
    let report = registry.validate_json(&load_dump()).expect("dump parses");

    // paramCount 와 params 배열 길이는 모든 블럭에서 일치해야 한다.
    assert_eq!(
        report.count_by_check("params_consistent"),
        0,
        "모든 블럭의 params 길이 == paramCount"
    );
}

#[test]
fn detects_real_entryjs_def_type_bugs() {
    let registry = BlockRegistry::new();
    let report = registry.validate_json(&load_dump()).expect("dump parses");

    // def.type != 블럭 id 인 케이스는 entryjs 의 실제 스키마 버그다.
    // ATS 가 이 두 건을 정확히 감지하는지 고정한다.
    let def_mismatches: Vec<&str> = report
        .violations
        .iter()
        .filter(|v| v.check == CHECK_DEF_TYPE_MATCH)
        .map(|v| v.block.as_str())
        .collect();
    assert_eq!(
        def_mismatches.len(),
        2,
        "def.type 불일치는 정확히 2건이어야 함: {def_mismatches:?}"
    );
    assert!(def_mismatches.contains(&"coconut_tmp_senser"));
    assert!(def_mismatches.contains(&"edumaker_tone_value"));
}

#[test]
fn executable_skeletons_must_have_func() {
    let registry = BlockRegistry::new();
    let report = registry.validate_json(&load_dump()).expect("dump parses");

    // 실행형 skeleton 은 func 를 가져야 한다. 순수 UI(basic_text/basic_button) 제외.
    let no_func = report.count_by_check("func_expected");
    assert!(no_func > 0, "func 부재 케이스가 존재해야 함 (entryjs 실제 데이터)");
}
