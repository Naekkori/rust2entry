//! EntryJS 자산 / 오브젝트 name↔ID 매핑 테스트.
//!
//! EntryJS Runtime 은 `Entry.container.getEntity(id)` 로 sprite 를 lookup 하므로
//! dropdown 슬롯 (`spritesWithMouse`, `spritesWithSelf`) 값은 반드시 sprite 의
//! stable id 여야 한다. `AssetMap` 가 이 name↔id 양방향 매핑을 담당한다.

use entrycore::AssetMap;
use serde_json::{Value, json};

/// base project + sprite 1개 ("Sprite1", id="obj_sprite1").
fn base_with_sprite() -> Value {
    json!({
        "name": "test",
        "speed": 60,
        "objects": [
            {
                "id": "obj_sprite1",
                "name": "Sprite1",
                "script": "[]",
                "objectType": "sprite",
                "rotateMethod": "free",
                "scene": "scene1",
                "sprite": {"pictures": [], "sounds": []},
                "text": "Sprite1",
                "lock": false,
                "entity": {}
            }
        ],
        "variables": [],
        "messages": [],
        "functions": [],
        "scenes": [{"id": "scene1", "name": "장면1"}],
        "interface": {"views": []},
        "meta": {}
    })
}

#[test]
fn assetmap_object_id_by_name_lookup() {
    let base = base_with_sprite();
    let assets = AssetMap::from_project_value(&base);
    assert_eq!(assets.object_id_by_name("Sprite1"), Some("obj_sprite1"));
    // case-sensitive — EntryJS 가 name 을 그대로 emit 하므로 소문자 변형은 매핑 안 됨.
    assert_eq!(assets.object_id_by_name("sprite1"), None);
    assert_eq!(assets.object_id_by_name("NotInProject"), None);
}

#[test]
fn debug_print_assets() {
    let assets = AssetMap::from_project_value(&base_with_sprite());
    eprintln!("{:#?}", assets);
    assert_eq!(assets.object_id_by_name("Sprite1"), Some("obj_sprite1"));
}

#[test]
fn assetmap_object_name_by_id_lookup() {
    let assets = AssetMap::from_project_value(&base_with_sprite());
    assert_eq!(assets.object_name_by_id("obj_sprite1"), Some("Sprite1"));
    assert_eq!(assets.object_name_by_id("unknown_id"), None);
}

#[test]
fn assetmap_passthrough_keywords() {
    // 'mouse' / 'self' 는 EntryJS Runtime 의 reserved 키워드 — 매핑 없이 통과.
    let assets = AssetMap::from_project_value(&base_with_sprite());
    assert_eq!(assets.object_id_by_name("mouse"), Some("mouse"));
    assert_eq!(assets.object_id_by_name("self"), Some("self"));
    assert_eq!(assets.object_name_by_id("mouse"), Some("mouse"));
    assert_eq!(assets.object_name_by_id("self"), Some("self"));
}

#[test]
fn assetmap_empty_project() {
    // objects 가 비어 있으면 lookup 도 빈 결과.
    let assets = AssetMap::from_project_value(&json!({
        "objects": []
    }));
    assert_eq!(assets.object_id_by_name("Sprite1"), None);
    assert_eq!(assets.object_name_by_id("obj_sprite1"), None);
    // 단 mouse / self 는 통과.
    assert_eq!(assets.object_id_by_name("mouse"), Some("mouse"));
}
