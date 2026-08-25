//! Entry 오브젝트별 이미지·소리 자산의 이름/ID 양방향 조회.

use std::collections::HashMap;

use serde_json::Value;

/// 현재 오브젝트 문맥에서 사용하는 이미지·소리 자산 조회표.
#[derive(Debug, Default, Clone)]
pub struct AssetMap {
    objects: HashMap<String, ObjectAssets>,
}

#[derive(Debug, Default, Clone)]
struct ObjectAssets {
    pictures: NameIdMap,
    sounds: NameIdMap,
}

#[derive(Debug, Default, Clone)]
struct NameIdMap {
    by_name: HashMap<String, String>,
    by_id: HashMap<String, String>,
}

impl AssetMap {
    /// project.json `objects[].sprite.pictures/sounds`에서 자산 목록을 읽는다.
    pub fn from_project_value(project: &Value) -> Self {
        let mut map = Self::default();
        let Some(objects) = project.get("objects").and_then(Value::as_array) else {
            return map;
        };
        for object in objects {
            let Some(name) = object.get("name").and_then(Value::as_str) else {
                continue;
            };
            let mut assets = ObjectAssets::default();
            if let Some(sprite) = object.get("sprite") {
                assets.pictures = NameIdMap::from_entries(sprite.get("pictures"));
                assets.sounds = NameIdMap::from_entries(sprite.get("sounds"));
            }
            map.objects.insert(name.to_lowercase(), assets);
        }
        map
    }

    /// 이미지 이름을 Entry 자산 ID로 변환한다.
    pub fn picture_id_by_name(&self, object: &str, name: &str) -> Option<&str> {
        self.objects
            .get(&object.to_lowercase())?
            .pictures
            .by_name
            .get(name)
            .map(String::as_str)
    }

    /// Entry 이미지 ID를 사람이 읽는 이름으로 변환한다.
    pub fn picture_name_by_id(&self, object: &str, id: &str) -> Option<&str> {
        self.objects
            .get(&object.to_lowercase())?
            .pictures
            .by_id
            .get(id)
            .map(String::as_str)
    }

    /// 소리 이름을 Entry 자산 ID로 변환한다.
    pub fn sound_id_by_name(&self, object: &str, name: &str) -> Option<&str> {
        self.objects
            .get(&object.to_lowercase())?
            .sounds
            .by_name
            .get(name)
            .map(String::as_str)
    }

    /// Entry 소리 ID를 사람이 읽는 이름으로 변환한다.
    pub fn sound_name_by_id(&self, object: &str, id: &str) -> Option<&str> {
        self.objects
            .get(&object.to_lowercase())?
            .sounds
            .by_id
            .get(id)
            .map(String::as_str)
    }
}

impl NameIdMap {
    fn from_entries(value: Option<&Value>) -> Self {
        let mut map = Self::default();
        let Some(entries) = value.and_then(Value::as_array) else {
            return map;
        };
        for entry in entries {
            let (Some(id), Some(name)) = (
                entry.get("id").and_then(Value::as_str),
                entry.get("name").and_then(Value::as_str),
            ) else {
                continue;
            };
            map.by_name.insert(name.to_string(), id.to_string());
            map.by_id.insert(id.to_string(), name.to_string());
        }
        map
    }
}
