//! Entry 변수 표현. deparse/decodegen 양쪽에서 공용.

use std::collections::HashMap;

pub use crate::ir::VarScope;

/// 변수 한 개의 정보.
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// project.json `variables[*].id`
    pub id: String,
    /// 사용자 노출 이름 (project.json `variables[*].name`). 없을 수 있음.
    pub name: String,
    /// 변수 종류: 일반 / 타이머 / 대답 / 리스트 / 클라우드 / 실시간.
    pub kind: VarKind,
    /// 초기값 (있다면).
    pub init: VarInit,
    /// 변수 scope (Local/Global). EntryJS `variables[*].object` 필드 결정.
    pub scope: VarScope,
}

/// 변수 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    /// 일반 변수.
    Variable,
    /// 타이머.
    Timer,
    /// 대답 변수.
    Answer,
    /// 리스트.
    List,
    /// 클라우드 변수.
    Cloud,
    /// 실시간 변수.
    RealTime,
    /// 알 수 없음.
    Unknown,
}

/// 변수 초기값 표현.
#[derive(Debug, Clone)]
pub enum VarInit {
    Int0,
    Float0,
    EmptyStr,
    False,
    EmptyList,
}

/// ID/name -> VarInfo lookup.
#[derive(Debug, Default, Clone)]
pub struct VarMap {
    inner: HashMap<String, VarInfo>,
    names: HashMap<String, String>,
}

impl VarMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, info: VarInfo) {
        let name = info.name.clone();
        let id = info.id.clone();
        self.names.insert(name, id.clone());
        self.inner.insert(id, info);
    }

    pub fn get(&self, id: &str) -> Option<&VarInfo> {
        self.inner.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&VarInfo> {
        self.names.get(name).and_then(|id| self.inner.get(id))
    }

    pub fn id_by_name(&self, name: &str) -> Option<&str> {
        self.names.get(name).map(String::as_str)
    }

    pub fn name_by_id(&self, id: &str) -> Option<&str> {
        self.get(id).map(|info| info.name.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = &VarInfo> {
        self.inner.values()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// project.json `variables` 배열(serde_json::Value)로부터 VarMap 생성.
///
/// 각 원소: `{id, name, variableType, value, ...}`
pub fn var_map_from_value(v: &serde_json::Value) -> VarMap {
    let mut map = VarMap::new();
    let Some(arr) = v.as_array() else {
        return map;
    };
    for entry in arr {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let id = match obj.get("id").and_then(serde_json::Value::as_str) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let name = obj
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(&id)
            .to_string();
        let kind = match obj
            .get("variableType")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("variable")
        {
            "timer" => VarKind::Timer,
            "answer" => VarKind::Answer,
            "list" => VarKind::List,
            "cloud" => VarKind::Cloud,
            "realtime" | "realTime" | "real_time" | "isRealTime" => VarKind::RealTime,
            "variable" => VarKind::Variable,
            _ => VarKind::Unknown,
        };
        let init = match (kind, obj.get("value")) {
            (VarKind::Variable, Some(serde_json::Value::Number(n))) => {
                if n.as_i64().is_some() {
                    VarInit::Int0
                } else if n.as_f64().is_some() {
                    VarInit::Float0
                } else {
                    VarInit::Int0
                }
            }
            (VarKind::Timer, _) | (VarKind::Answer, _) => VarInit::Int0,
            (VarKind::List, _) => VarInit::EmptyList,
            _ => VarInit::Int0,
        };
        // Entry의 object가 null이면 프로젝트 전역 변수다. 이 정보를
        // 역변환에서 보존해야 다시 컴파일해도 오브젝트 변수로 바뀌지 않는다.
        let scope = if obj.get("object").map_or(false, serde_json::Value::is_null) {
            VarScope::Global
        } else {
            VarScope::Local
        };
        map.insert(VarInfo {
            id,
            name,
            kind,
            init,
            scope,
        });
    }
    map
}
