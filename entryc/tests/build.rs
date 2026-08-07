//! entryc build E2E 테스트.
//!
//! 빌드 모드 호출 + 결과 .ent 검증 (gzip 매직, project.json 보존).

use std::path::PathBuf;
use std::process::Command;

// 테스트용 임시폴더 (프로세스 ID로 충돌 방지)
fn unique_tmp(label: &str) -> PathBuf {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    base.join(format!("entryc-test-{label}-{pid}-{nanos}"))
}

// 테스트 후 정리용 RAII 가드
struct TmpGuard(PathBuf);
impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// .ent 파일을 dir 에 풀기 (외부 tar 사용). Windows 10+/Git Bash/macOS/Linux 호환.
fn unpack_ent(ent: &PathBuf, dir: &PathBuf) {
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(ent)
        .arg("-C")
        .arg(dir)
        .status()
        .expect("failed to spawn tar");
    assert!(status.success(), "tar unpack failed for {}", ent.display());
}

// entryc 바이너리 경로
fn entryc_bin() -> &'static str {
    env!("CARGO_BIN_EXE_entryc")
}

#[test]
fn build_creates_gzip_ent_file() {
    let dir = unique_tmp("basic");
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = TmpGuard(dir.clone());

    let rs = dir.join("main.rs");
    std::fs::write(&rs, "fn when_start() { let x = 42; }\n").unwrap();
    let ent = dir.join("out.ent");

    let status = Command::new(entryc_bin())
        .args(["build", "--rs"])
        .arg(&rs)
        .arg("--out")
        .arg(&ent)
        .status()
        .expect("spawn entryc build");
    assert!(status.success(), "entryc build failed");

    // gzip 매직바이트 1f 8b 확인
    let bytes = std::fs::read(&ent).expect("read ent");
    assert!(bytes.len() >= 2, "ent too small");
    assert_eq!(bytes[0], 0x1f, "gzip magic[0]");
    assert_eq!(bytes[1], 0x8b, "gzip magic[1]");
}

#[test]
fn build_unpacks_to_valid_project_json() {
    let dir = unique_tmp("unpack");
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = TmpGuard(dir.clone());

    let rs = dir.join("main.rs");
    std::fs::write(
        &rs,
        "fn when_start() { let x = 42; let y = 1 + 2; if 1 < 2 { let z = 1; } }\n",
    )
    .unwrap();
    let ent = dir.join("out.ent");

    let status = Command::new(entryc_bin())
        .args(["build", "--rs"])
        .arg(&rs)
        .arg("--out")
        .arg(&ent)
        .status()
        .expect("spawn entryc build");
    assert!(status.success(), "entryc build failed");

    // 외부 tar 로 unpack
    let unpack_dir = dir.join("unpacked");
    std::fs::create_dir_all(&unpack_dir).unwrap();
    unpack_ent(&ent, &unpack_dir);

    let pj_path = unpack_dir.join("project.json");
    assert!(pj_path.is_file(), "project.json not in unpacked dir");
    let raw = std::fs::read_to_string(&pj_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).expect("project.json parse");

    // name 필드 존재 (default_empty_project 가 설정)
    assert_eq!(v.get("name").and_then(|x| x.as_str()), Some("rust2entry"));

    // scripts 3개: set x, set y(calc), if z
    let scripts = v.get("scripts").and_then(|x| x.as_array()).expect("scripts array");
    assert_eq!(scripts.len(), 3, "expected 3 scripts, got {}", scripts.len());

    // 0번: set_variable x
    assert_eq!(scripts[0].get("type").and_then(|x| x.as_str()), Some("set_variable"));
    // 1번: set_variable y (값은 calc_basic)
    assert_eq!(scripts[1].get("type").and_then(|x| x.as_str()), Some("set_variable"));
    let y_val = &scripts[1]["params"][1];
    assert_eq!(y_val.get("type").and_then(|x| x.as_str()), Some("calc_basic"));
    // 2번: if (조건 boolean_basic, statements 안에 set z)
    assert_eq!(scripts[2].get("type").and_then(|x| x.as_str()), Some("if"));
    let cond = &scripts[2]["params"][0];
    assert_eq!(cond.get("type").and_then(|x| x.as_str()), Some("boolean_basic"));
    let stmts = scripts[2]["statements"][0].as_array().expect("if body");
    assert_eq!(stmts.len(), 1);
    assert_eq!(stmts[0].get("type").and_then(|x| x.as_str()), Some("set_variable"));

    // variables 3개: x, y, z
    let vars = v.get("variables").and_then(|x| x.as_array()).expect("variables array");
    assert_eq!(vars.len(), 3, "expected 3 variables, got {}", vars.len());
    let names: Vec<&str> = vars
        .iter()
        .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert!(names.contains(&"z"));
}

#[test]
fn build_with_template_preserves_project_metadata() {
    let dir = unique_tmp("template");
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = TmpGuard(dir.clone());

    // 템플릿 .ent 빌드 (name: template_proj, scenes 2개)
    let tmpl_rs = dir.join("tmpl_main.rs");
    std::fs::write(&tmpl_rs, "fn when_start() { let a = 1; }\n").unwrap();
    let tmpl_ent = dir.join("template.ent");
    let status = Command::new(entryc_bin())
        .args(["build", "--rs"])
        .arg(&tmpl_rs)
        .arg("--out")
        .arg(&tmpl_ent)
        .status()
        .expect("spawn build template");
    assert!(status.success());

    // 템플릿의 project.json 에 scenes 추가 (수동 패치) 후 다시 .ent 빌드
    let unpack_dir = dir.join("tmpl_unpack");
    std::fs::create_dir_all(&unpack_dir).unwrap();
    unpack_ent(&tmpl_ent, &unpack_dir);
    let pj_path = unpack_dir.join("project.json");
    let raw = std::fs::read_to_string(&pj_path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["name"] = serde_json::json!("template_proj");
    v["scenes"] = serde_json::json!([
        { "id": "scene1", "name": "장면1" },
        { "id": "scene2", "name": "장면2" },
    ]);
    std::fs::write(&pj_path, serde_json::to_string_pretty(&v).unwrap()).unwrap();

    // 수정된 project.json 포함해 .ent 재생성
    let tmpl_ent2 = dir.join("template2.ent");
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(&tmpl_ent2)
        .arg("-C")
        .arg(&unpack_dir)
        .arg(".")
        .status()
        .expect("tar pack");
    assert!(status.success());

    // template 기반으로 실제 빌드
    let rs = dir.join("main.rs");
    std::fs::write(&rs, "fn when_start() { let b = 2; }\n").unwrap();
    let ent = dir.join("out.ent");
    let status = Command::new(entryc_bin())
        .args(["build", "--rs"])
        .arg(&rs)
        .arg("--ent-template")
        .arg(&tmpl_ent2)
        .arg("--out")
        .arg(&ent)
        .status()
        .expect("spawn build with template");
    assert!(status.success(), "entryc build with template failed");

    // unpack 후 검증
    let out_unpack = dir.join("out_unpack");
    std::fs::create_dir_all(&out_unpack).unwrap();
    unpack_ent(&ent, &out_unpack);
    let out_pj = out_unpack.join("project.json");
    let raw = std::fs::read_to_string(&out_pj).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();

    // 템플릿 메타 보존
    assert_eq!(v.get("name").and_then(|x| x.as_str()), Some("template_proj"));
    let scenes = v.get("scenes").and_then(|x| x.as_array()).expect("scenes");
    assert_eq!(scenes.len(), 2, "template scenes lost");

    // 새 scripts/variables 패치됨
    let scripts = v.get("scripts").and_then(|x| x.as_array()).expect("scripts");
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].get("type").and_then(|x| x.as_str()), Some("set_variable"));
    let vars = v.get("variables").and_then(|x| x.as_array()).expect("variables");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].get("name").and_then(|x| x.as_str()), Some("b"));
}

#[test]
fn build_fails_with_no_rs_inputs() {
    let dir = unique_tmp("noinput");
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = TmpGuard(dir.clone());
    let ent = dir.join("out.ent");

    let status = Command::new(entryc_bin())
        .args(["build", "--out"])
        .arg(&ent)
        .status()
        .expect("spawn entryc build");
    assert!(!status.success(), "expected failure when --rs missing");
}

/// build -> extract 라운드트립 (단순):
/// build 가 만든 .ent 에 objects 1개가 있고,
/// extract 가 그 오브젝트 이름의 .rs 파일을 생성하는지 확인.
/// (script 내용 보존은 Entry 형식 확인 후 별도 작업.)
#[test]
fn build_extract_roundtrip_creates_object_file() {
    let dir = unique_tmp("roundtrip");
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = TmpGuard(dir.clone());

    // 빌드 입력: 오브젝트 이름이 파일 stem 이 됨
    let rs = dir.join("my_sprite.rs");
    std::fs::write(
        &rs,
        "fn when_start() { let x = 42; let y = 1 + 2; }\n",
    )
    .unwrap();
    let ent = dir.join("out.ent");
    let status = Command::new(entryc_bin())
        .args(["build", "--rs"])
        .arg(&rs)
        .arg("--out")
        .arg(&ent)
        .status()
        .expect("spawn build");
    assert!(status.success(), "build failed");

    // unpack 해서 objects 확인
    let unpack = dir.join("unpack");
    std::fs::create_dir_all(&unpack).unwrap();
    unpack_ent(&ent, &unpack);
    let pj_path = unpack.join("project.json");
    let raw = std::fs::read_to_string(&pj_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).expect("project.json parse");
    let objects = v.get("objects").and_then(|x| x.as_array()).expect("objects");
    assert_eq!(objects.len(), 1, "가짜 오브젝트 1개");
    assert_eq!(objects[0].get("name").and_then(|x| x.as_str()), Some("my_sprite"));

    // extract: out_dir 에 my_sprite.rs 생성되는지
    let out_dir = dir.join("extracted");
    std::fs::create_dir_all(&out_dir).unwrap();
    let status = Command::new(entryc_bin())
        .args(["extract", "--ent"])
        .arg(&ent)
        .arg("--out")
        .arg(&out_dir)
        .status()
        .expect("spawn extract");
    assert!(status.success(), "extract failed");

    let rs_out = out_dir.join("my_sprite.rs");
    assert!(rs_out.is_file(), "my_sprite.rs 가 생성돼야 함 (objects 1개)");
}
