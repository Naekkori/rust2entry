//! 하드웨어 블럭 정방향·역방향·라운드트립 통합 테스트.
//!
//! 소스맵(`hw_sourcemap.json`)을 전역 인덱스에 설정한 뒤:
//! - **정방향**: Rust `block_id(args)` 호출 → `Block::Raw` → .ent 블럭 (type 보존).
//! - **역방향**: .ent 하드웨어 블럭 → `block_id(args)` Rust 호출 + `// @hwraw` raw 보존.
//! - **라운드트립**: .ent → .rs → .ent 가 원본 하드웨어 블럭을 손실 없이 재생성.

use entrycore::block::registry::{BlockRegistry, set_hw_index};
use entrycore::block::{self, Block};

fn set_index() {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/hw_sourcemap.json"
    ))
    .expect("fixture");
    let map = BlockRegistry::new()
        .parse_hw_sourcemap(&json)
        .expect("parse");
    set_hw_index(&map);
}

/// 정방향: `pyocoding_serial_set("COM1")` 호출이 .ent 하드웨어 블럭으로.
#[test]
fn forward_emits_hardware_block() {
    set_index();
    let src = "fn when_start() { pyocoding_serial_set(\"COM1\"); }";
    let prog = entrycore::parse::parse(src).expect("parse");
    let stmt = prog.stmts.first().expect("stmt");
    let blk = block::from_stmt(stmt).expect("from_stmt");
    let v = block::to_value(&blk).expect("to_value");
    assert_eq!(
        v["type"].as_str(),
        Some("pyocoding_serial_set"),
        "하드웨어 블럭 type 보존 (정방향)"
    );
    assert!(
        !matches!(blk, Block::FuncCall { .. }),
        "FuncCall 이 아니라 Raw 여야"
    );
}

/// 역방향: .ent 하드웨어 블럭이 Rust 호출 + @hwraw 주석으로 추출된다.
#[test]
fn reverse_deparses_hardware_block() {
    set_index();
    let script = serde_json::json!([[
        { "type": "when_run_button_click", "params": [] },
        { "type": "pyocoding_serial_set", "params": [{ "type": "text", "params": ["COM1"] }] },
        { "type": "pyocoding_get_analog_value", "params": [{ "type": "number", "params": [1.0] }] },
    ]]);
    let prog = entrycore::deparse::program_from_script_value(&script).expect("deparse");
    let out = entrycore::decodegen::emit(&prog).expect("emit");
    assert!(
        out.contains("pyocoding_serial_set(\"COM1\")"),
        "출력: {out}"
    );
    assert!(out.contains("pyocoding_get_analog_value"), "출력: {out}");
    assert!(out.contains("@hwraw"), "raw 보존 주석 필요: {out}");
}

/// 라운드트립: .ent 하드웨어 블럭 → .rs → 다시 .ent 가 원본 블럭을 손실 없이 재생성.
#[test]
fn hardware_roundtrip_is_lossless() {
    set_index();
    let original = serde_json::json!([[
        { "type": "when_run_button_click", "params": [] },
        { "type": "pyocoding_serial_set", "params": [{ "type": "text", "params": ["COM1"] }] },
    ]]);

    // 1) 역방향: .ent → .rs (decodegen::emit 은 `fn when_start() {...}` 구조 + @hwraw 주석 산출)
    let prog = entrycore::deparse::program_from_script_value(&original).expect("deparse");
    let rs = entrycore::decodegen::emit(&prog).expect("emit");
    assert!(rs.contains("@hwraw"), ".rs 에 raw 보존 주석 필요: {rs}");

    // 2) 정방향: 그 .rs 를 그대로 파싱 → 블럭 재생성 (하드웨어 블럭만 비교)
    let prog2 = entrycore::parse::parse(&rs).expect("parse");
    let mut rebuilt: Vec<serde_json::Value> = Vec::new();
    for stmt in &prog2.stmts {
        let blk = block::from_stmt(stmt).expect("from_stmt");
        rebuilt.push(block::to_value(&blk).expect("to_value"));
    }
    // when_run 트리거는 parse 가 평탄화해 빠지므로, 하드웨어 블럭 자체만 비교한다.
    let orig_hw: Vec<serde_json::Value> = original[0]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["type"] != "when_run_button_click")
        .cloned()
        .collect();
    assert_eq!(
        rebuilt, orig_hw,
        "하드웨어 블럭이 손실 없이 재생성되어야 함\n재빌드: {rebuilt:?}\n원본  : {orig_hw:?}"
    );
}
