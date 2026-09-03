//! Entry 변수 표현. deparse/decodegen 양쪽에서 공용.

use std::collections::HashMap;

pub use crate::ir::VarScope;

/// 변수 한 개의 정보.
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// project.json `variables[*].id`
    pub id: String,
    /// Rust DSL 에서 쓸 sanitize 이름 (한글/특수문자 변수명을 raw identifier 등으로 변환).
    pub name: String,
    /// EntryJS 원본 이름 (`variables[*].name` 그대로, 빌드 시 보존).
    pub original_name: String,
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

    /// 같은 name 가진 기존 항목이 있으면 그 항목을 `info` 로 교체한다.
    /// 없으면 `insert` 와 동일하게 추가. base 변수 id 보존용.
    pub fn replace(&mut self, name: &str, info: VarInfo) {
        if let Some(old_id) = self.names.get(name).cloned() {
            self.inner.remove(&old_id);
        }
        self.insert(info);
    }

    pub fn get(&self, id: &str) -> Option<&VarInfo> {
        self.inner.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&VarInfo> {
        self.names.get(name).and_then(|id| self.inner.get(id))
    }

    /// sanitize 이름 (DSL 식별자) 으로 lookup.
    pub fn id_by_name(&self, name: &str) -> Option<&str> {
        let s = crate::block::sanitize_ident(name);
        self.names.get(&s).map(String::as_str)
    }

    /// EntryJS 원본 이름 (`variables[*].name` 그대로) 반환.
    /// 빌드 시 EntryJS variables[*] 이름 보존에 사용.
    pub fn name_by_id(&self, id: &str) -> Option<&str> {
        self.get(id).map(|info| info.original_name.as_str())
    }

    /// DSL 식별자 이름 (= sanitize 이름) 반환.
    pub fn dsl_name_by_id(&self, id: &str) -> Option<&str> {
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
///
/// `name` (DSL 식별자) 은 sanitize 해서 Rust 정합성 보존 (raw identifier 회피,
/// 충돌 시 hash suffix). `original_name` 은 EntryJS native 변수명 그대로
/// (한글/공백 포함 가능) 보존해서 EntryJS variable list 의 name 과 일치시키고
/// socket 연결에 사용한다.
pub fn var_map_from_value(v: &serde_json::Value) -> VarMap {
    let mut map = VarMap::new();
    let Some(arr) = v.as_array() else {
        return map;
    };
    let mut used_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in arr {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let id = match obj.get("id").and_then(serde_json::Value::as_str) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let original = obj
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(&id)
            .to_string();
        // DSL 식별자로 sanitize (raw identifier 처리) — 충돌 시 hash suffix.
        let base_name = crate::block::sanitize_ident(&original);
        let name = if used_names.contains(&base_name) {
            // 충돌 시 짧은 해시 suffix 로 unique 보장.
            let suffix = {
                let mut h: u64 = 5381;
                for b in original.bytes() {
                    h = h.wrapping_mul(33).wrapping_add(b as u64);
                }
                format!("_{:x}", h & 0xFFF)
            };
            let mut candidate = format!("{base_name}{suffix}");
            let mut n = 0;
            while used_names.contains(&candidate) {
                n += 1;
                candidate = format!("{base_name}{suffix}_{n}");
            }
            candidate
        } else {
            base_name
        };
        used_names.insert(name.clone());
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
        let scope = if obj.get("object").is_some_and(serde_json::Value::is_null) {
            VarScope::Global
        } else {
            VarScope::Local
        };
        map.insert(VarInfo {
            id,
            name,
            original_name: original,
            kind,
            init,
            scope,
        });
    }
    map
}
