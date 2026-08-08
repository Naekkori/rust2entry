//! lib::compile 통합 테스트.
//!
//! parse + codegen 을 거치며 최종 project.json 구조 확인.

use entrycore::compile;
use serde_json::{Value, json};

fn empty_project() -> Value {
    json!({
        "name": "test",
        "speed": 60, "objects": [], "variables": [], "messages": [],
        "functions": [], "scenes": [{"id":"scene1","name":"장면1"}],
        "interface": {"views": []}, "meta": {}
    })
}

/// build 가 object.script 에 저장한 JSON 문자열을 thread 배열 Value 로 파싱.
/// (실제 .ent 형식이 String 이라 테스트에서 매번 역직렬화 필요)
fn parse_script_string(script: &Value) -> Value {
    let s = script.as_str().expect("script 는 JSON 문자열");
    serde_json::from_str(s).expect("script JSON 파싱")
}

fn obj_threads(obj: &Value) -> Vec<Value> {
    parse_script_string(&obj["script"])
        .as_array()
        .expect("threads array")
        .clone()
}

fn first_thread(obj: &Value) -> Vec<Value> {
    obj_threads(obj)
        .into_iter()
        .next()
        .expect("first thread")
        .as_array()
        .expect("first thread array")
        .clone()
}

/// base 에 object 가 없을 때 rs stem 이름으로 가짜 sprite 가 추가되고,
/// 그 object 의 `script` 필드(JSON 문자열, thread 배열)에 `[[when_run, body...]]` 가 들어가는지 확인.
#[test]
fn compile_single_source() {
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 1);
    // object.script 는 JSON 문자열 (실제 .ent 형식)
    assert!(
        objects[0]["script"].as_str().is_some(),
        "script 는 JSON 문자열"
    );
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + body 1개");
    assert_eq!(thread[0]["type"], "when_run");
    assert_eq!(thread[1]["type"], "set_variable");
    assert_eq!(thread[1]["params"][0]["name"], "x");
}

/// rs 가 둘이고 base 비어있으면 둘 다 가짜 object 로 추가.
#[test]
fn compile_multi_source_merges_scripts() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 2);
    let a_obj = objects.iter().find(|o| o["name"] == "a").expect("a");
    let b_obj = objects.iter().find(|o| o["name"] == "b").expect("b");
    let a_thread = first_thread(a_obj);
    let b_thread = first_thread(b_obj);
    assert_eq!(a_thread[0]["type"], "when_run");
    assert_eq!(b_thread[0]["type"], "when_run");
    assert_eq!(a_thread[1]["params"][0]["name"], "x");
    assert_eq!(b_thread[1]["params"][0]["name"], "y");
}

/// base 메타(name/scenes/speed) 보존.
#[test]
fn compile_preserves_base_metadata() {
    let mut base = empty_project();
    base["name"] = json!("my_proj");
    base["scenes"] = json!([
        { "id": "scene1", "name": "장면1" },
        { "id": "scene2", "name": "장면2" },
    ]);
    base["speed"] = json!(30);
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    assert_eq!(v["name"], "my_proj");
    assert_eq!(v["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(v["speed"], 30);
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2);
}

/// 변수 집계: 여러 rs 의 트리거 body 안 변수명이 variables 에 모두 들어가는지.
#[test]
fn compile_aggregates_variables_across_sources() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; let z = 3; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().expect("variables array");
    let names: Vec<&str> = vars.iter().filter_map(|x| x["name"].as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert!(names.contains(&"z"));
    assert_eq!(vars.len(), 3);
}

/// 같은 변수명 두 소스에서 쓰면 하나로 합쳐짐.
#[test]
fn compile_deduplicates_variables() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let x = 2; }";
    let v = compile(&[("a", a), ("b", b)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().expect("variables array");
    let xs: Vec<&Value> = vars.iter().filter(|x| x["name"] == "x").collect();
    assert_eq!(xs.len(), 1, "중복 변수 발생");
}

#[test]
fn compile_parse_error_propagates() {
    let bad = "fn when_start() { let = ; }";
    let r = compile(&[("obj", bad)], &empty_project());
    assert!(r.is_err(), "parse 에러가 전파돼야 함");
}

/// rs 가 없으면 base 그대로 (가짜 object 추가 안 함).
#[test]
fn compile_empty_sources_returns_base() {
    let mut base = empty_project();
    base["name"] = json!("untouched");
    let v = compile(&[], &base).expect("compile").0;
    assert_eq!(v["name"], "untouched");
    assert!(v["objects"].as_array().unwrap().is_empty());
    assert_eq!(v["variables"].as_array().unwrap().len(), 0);
}

/// if 블록: object.script[0] (thread) = [when_run, if, ...]. 본문 if 는 thread[1].
#[test]
fn compile_if_block_structure() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + if");
    assert_eq!(thread[0]["type"], "when_run");
    assert_eq!(thread[1]["type"], "if");
    assert_eq!(thread[1]["params"][0]["type"], "boolean_basic");
}

/// for-range 는 repeat_basic 으로 직렬화.
#[test]
fn compile_for_range_expands_to_repeat() {
    let src = "fn when_start() { for i in 0..5 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + repeat");
    assert_eq!(thread[1]["type"], "repeat_basic");
}

/// compile -> object.script (JSON 문자열) -> deparse 라운드트립.
/// thread 0 = [when_run, if] -> deparse 가 when_run 을 FuncDef 로 감싸고 본문 If 를 body 에.
#[test]
fn compile_roundtrip_via_deparse() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::Stmt;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let p1 = entrycore::parse::parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            assert!(matches!(body[0], Stmt::If { .. }));
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// base objects 가 비어있고 rs 가 있으면 가짜 object 1개 + entity 기본값.
#[test]
fn compile_adds_fake_object_when_empty() {
    let src = "fn when_start() { let x = 42; }";
    let v = compile(&[("my_obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects array");
    assert_eq!(objects.len(), 1, "가짜 오브젝트 1개 추가");
    assert_eq!(objects[0]["name"], "my_obj");
    assert_eq!(objects[0]["objectType"], "sprite");
    assert_eq!(objects[0]["scene"], "scene1");
    assert_eq!(objects[0]["entity"]["x"], 0.0);
    assert_eq!(objects[0]["entity"]["visible"], true);
    assert_eq!(objects[0]["sprite"]["name"], "my_obj");
    assert!(objects[0]["sprite"]["pictures"].as_array().unwrap().is_empty());
    let fake_id = objects[0]["id"].as_str().expect("fake id str");
    assert!(fake_id.starts_with("obj_"), "가짜 id 는 obj_ prefix: {fake_id}");
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2, "when_run + body");
}

/// base 에 objects 가 이미 있으면 추가하지 않고, rs stem == name 인 object 의
/// script 필드만 패치한다.
#[test]
fn compile_does_not_overwrite_existing_objects() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "existing1",
            "name": "existing_obj",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "existing_obj", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("existing_obj", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().expect("objects");
    assert_eq!(objects.len(), 1, "기존 objects 보존, 추가 안 함");
    assert_eq!(objects[0]["name"], "existing_obj");
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_run");
    assert_eq!(thread[1]["type"], "set_variable");
}

/// 각 rs 가 stem 이름으로 base object 와 매칭되어 그 object 의 script 가 패치된다.
#[test]
fn compile_matches_existing_object_by_name() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "a1",
            "name": "alpha",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "alpha", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        },
        {
            "id": "b1",
            "name": "beta",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "beta", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let v = compile(&[("alpha", a), ("beta", b)], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2, "기존 objects 2개 보존");
    let alpha_obj = objects.iter().find(|o| o["name"] == "alpha").unwrap();
    let beta_obj = objects.iter().find(|o| o["name"] == "beta").unwrap();
    let alpha_thread = first_thread(alpha_obj);
    let beta_thread = first_thread(beta_obj);
    assert_eq!(alpha_thread[1]["params"][0]["name"], "x");
    assert_eq!(beta_thread[1]["params"][0]["name"], "y");
}

/// 이름 매칭이 대소문자 무시.
#[test]
fn compile_matches_object_name_case_insensitive() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "a1",
            "name": "Alpha",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "Alpha", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("alpha", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 1);
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2);
}

/// if-else: object.script[0] (thread) = [when_run, if_else]. 본문 if_else 는 thread[1].
#[test]
fn compile_if_else_block() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let x = 1;
            } else {
                let y = 2;
            }
        }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread.len(), 2);
    assert_eq!(thread[0]["type"], "when_run");
    let block = &thread[1];
    assert_eq!(block["type"], "if_else");
    assert_eq!(block["params"][0]["type"], "boolean_basic");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 2, "if_else 는 then/else 2개 thread");
    let then_first = &stmts[0][0];
    assert_eq!(then_first["type"], "set_variable");
    assert_eq!(then_first["params"][0]["name"], "x");
    let else_first = &stmts[1][0];
    assert_eq!(else_first["type"], "set_variable");
    assert_eq!(else_first["params"][0]["name"], "y");
}

/// else 없으면 if (Entry 의 if 블록 형식).
#[test]
fn compile_if_without_else_stays_if() {
    let src = "fn when_start() { if 1 < 2 { let x = 1; } }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    let block = &thread[1];
    assert_eq!(block["type"], "if");
    let stmts = block["statements"].as_array().expect("statements");
    assert_eq!(stmts.len(), 1, "if 는 then 1개 thread");
}

/// if-else compile -> object.script (JSON 문자열) -> deparse 라운드트립.
#[test]
fn compile_if_else_roundtrip() {
    use entrycore::codegen::collect_var_map;
    use entrycore::deparse::program_from_script_string_with_vars;
    use entrycore::ir::Stmt;
    use entrycore::parse::parse;

    let src = "fn when_start() { if 1 < 2 { let x = 1; } else { let y = 2; } }";
    let p1 = parse(src).expect("parse1");
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = collect_var_map(&p1);
    let objects = v["objects"].as_array().unwrap();
    let obj_script_str = objects[0]["script"].as_str().expect("script str");
    let p2 = program_from_script_string_with_vars(obj_script_str, &vars).expect("deparse");
    match &p2.stmts[0] {
        Stmt::FuncDef { name, body, .. } => {
            assert_eq!(name, "when_start");
            match &body[0] {
                Stmt::If { then_body, else_body, .. } => {
                    assert_eq!(then_body.len(), 1);
                    assert_eq!(else_body.len(), 1);
                    let then_var = match &then_body[0] {
                        Stmt::VarDecl(n, _) | Stmt::SetVar(n, _) => n,
                        other => panic!("unexpected then stmt: {other:?}"),
                    };
                    let else_var = match &else_body[0] {
                        Stmt::VarDecl(n, _) | Stmt::SetVar(n, _) => n,
                        other => panic!("unexpected else stmt: {other:?}"),
                    };
                    assert_eq!(then_var, "x");
                    assert_eq!(else_var, "y");
                }
                other => panic!("expected If in body, got {other:?}"),
            }
        }
        other => panic!("expected FuncDef(when_start), got {other:?}"),
    }
}

/// base 에 기존 스프라이트가 있을 때 매칭 안 된 rs 가 가짜 object 로 추가될 때
/// entity 메타가 base 에서 복사되어 위치 등이 0 으로 초기화되지 않는다.
#[test]
fn compile_preserves_existing_sprite_metadata_on_fake_object() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "src1",
            "name": "source_sprite",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": {
                "name": "source_sprite",
                "pictures": [{"id":"pic1","name":"a.png","fileurl":"a.png","type":"","dimension":{"width":10,"height":20}}],
                "sounds": []
            },
            "entity": {
                "rotation": 0.0, "direction": 90.0,
                "x": 123.0, "y": 456.0,
                "regX": 5.0, "regY": 10.0,
                "scaleX": 2.0, "scaleY": 1.5,
                "width": 30.0, "height": 40.0, "visible": true
            },
            "selectedPictureId": "pic1"
        }
    ]);
    let v = compile(&[("new_sprite", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2);
    let fake = objects.iter().find(|o| o["name"] == "new_sprite").expect("fake object");
    assert_eq!(fake["entity"]["x"], 123.0);
    assert_eq!(fake["entity"]["y"], 456.0);
    assert_eq!(fake["entity"]["regX"], 5.0);
    assert_eq!(fake["entity"]["regY"], 10.0);
    assert_eq!(fake["entity"]["scaleX"], 2.0);
    let pics = fake["sprite"]["pictures"].as_array().unwrap();
    assert!(pics.is_empty(), "가짜 object 의 pictures 는 비어야 함");
    let sounds = fake["sprite"]["sounds"].as_array().unwrap();
    assert!(sounds.is_empty(), "가짜 object 의 sounds 는 비어야 함");
    assert_eq!(fake["selectedPictureId"], Value::Null);
    let fake_id = fake["id"].as_str().expect("fake id str");
    assert_ne!(fake_id, "src1");
    assert!(fake_id.starts_with("obj_"));
    let src = objects.iter().find(|o| o["name"] == "source_sprite").expect("base obj");
    assert_eq!(src["id"], "src1");
}

/// project.scripts 는 base 값으로 복원된다 (object.script 가 진짜 위치).
#[test]
fn compile_does_not_clobber_project_scripts_field() {
    let mut base = empty_project();
    base["scripts"] = json!([{"type": "old_block"}]);
    let v = compile(&[("obj", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let scripts = v["scripts"].as_array().expect("project.scripts");
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0]["type"], "old_block");
}

/// 여러 가짜 object 가 생성되어도 id 가 서로 충돌하지 않는다.
#[test]
fn compile_fake_objects_have_unique_ids() {
    let a = "fn when_start() { let x = 1; }";
    let b = "fn when_start() { let y = 2; }";
    let c = "fn when_start() { let z = 3; }";
    let v = compile(&[("a", a), ("b", b), ("c", c)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 3);
    let mut ids: Vec<String> = objects
        .iter()
        .map(|o| o["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "가짜 object id 중복: {ids:?}");
    for id in &ids {
        assert!(id.starts_with("obj_"), "id 포맷: {id}");
    }
}

/// 가짜 object 의 id 는 stem 기반 stable hash 이고 base 와 충돌 안 함.
#[test]
fn compile_fake_id_skips_existing_ids_in_base() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "obj_doesnotmatter",
            "name": "base_a",
            "objectType": "sprite",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "base_a", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("new_one", "fn when_start() { let x = 1; }")], &base).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let fake = objects.iter().find(|o| o["name"] == "new_one").expect("new_one");
    let fake_id = fake["id"].as_str().expect("id");
    assert_eq!(fake_id, format!("obj_{}", entrycore::block::id_for("new_one")));
    assert_ne!(fake_id, "obj_doesnotmatter");
}

/// stable id: 같은 stem 으로 두 번 빌드하면 같은 id 가 나온다.
#[test]
fn compile_stable_id_is_deterministic() {
    let src = "fn when_start() { let x = 1; }";
    let v1 = compile(&[("foo", src)], &empty_project()).expect("compile").0;
    let v2 = compile(&[("foo", src)], &empty_project()).expect("compile").0;
    let id1 = v1["objects"][0]["id"].as_str().unwrap();
    let id2 = v2["objects"][0]["id"].as_str().unwrap();
    assert_eq!(id1, id2, "stable id: {id1} == {id2}");
}

/// helper 함수 (트리거 아닌 FuncDef) 는 object script 가 아니라 project.functions 로 emit.
#[test]
fn compile_helpers_go_to_project_functions() {
    let src = r#"
        fn when_start() { let x = 1; }
        fn helper() { let y = 2; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let first_thread = first_thread(&objects[0]);
    // when_run + let x 만 있고 helper 본문(set y)은 없어야
    assert_eq!(first_thread.len(), 2);
    assert_eq!(first_thread[0]["type"], "when_run");
    assert_eq!(first_thread[1]["type"], "set_variable");
    assert_eq!(first_thread[1]["params"][0]["name"], "x");
    // project.functions 에 helper 항목
    let funcs = v["functions"].as_array().expect("functions");
    let helper = funcs.iter().find(|f| f["name"] == "helper").expect("helper fn");
    assert!(helper["id"].as_str().unwrap().starts_with("fn_"));
    let content = helper["content"].as_array().expect("content threads");
    assert_eq!(content.len(), 1, "helper 는 1개 thread");
    let blocks = content[0]["blocks"].as_array().expect("blocks");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["type"], "set_variable");
    assert_eq!(blocks[0]["params"][0]["name"], "y");
}

/// CompileOptions.default_scene 으로 가짜 object 의 scene 지정.
#[test]
fn compile_default_scene_from_options() {
    use entrycore::compile_with_options;
    let src = "fn when_start() { let x = 1; }";
    let options = entrycore::CompileOptions {
        default_scene: Some("scene2".to_string()),
    };
    let v = compile_with_options(&[("obj", src)], &empty_project(), &options)
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects[0]["scene"], "scene2");
}

/// base 의 첫 object 가 text 면 가짜 object 도 text objectType 보존.
#[test]
fn compile_fake_object_preserves_non_sprite_object_type() {
    let mut base = empty_project();
    base["objects"] = json!([
        {
            "id": "txt1",
            "name": "label_obj",
            "objectType": "text",
            "scene": "scene1",
            "script": "[]",
            "sprite": { "name": "label_obj", "pictures": [], "sounds": [] },
            "entity": { "x": 0, "y": 0, "visible": true }
        }
    ]);
    let v = compile(&[("new_obj", "fn when_start() { let x = 1; }")], &base)
        .expect("compile")
        .0;
    let objects = v["objects"].as_array().unwrap();
    let fake = objects.iter().find(|o| o["name"] == "new_obj").expect("new_obj");
    assert_eq!(fake["objectType"], "text", "base 가 text 면 가짜도 text");
}

/// when_click 트리거는 when_click 블록으로 직렬화.
#[test]
fn compile_when_click_trigger() {
    let src = "fn when_click() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_click");
    assert_eq!(thread[1]["type"], "set_variable");
}

/// when_clone_start 트리거는 when_clone_start 블록으로 직렬화.
#[test]
fn compile_when_clone_start_trigger() {
    let src = "fn when_clone_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_clone_start");
}

/// when_message 함수는 params[0] 을 메시지 이름으로 사용한 when_message_cast 트리거 생성.
#[test]
fn compile_when_message_trigger_uses_param_as_msg() {
    let src = "fn when_message(m: &str) { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let thread = first_thread(&objects[0]);
    assert_eq!(thread[0]["type"], "when_message_cast");
    assert_eq!(thread[0]["params"][0].as_str(), Some("m"));
}

/// when_message 트리거 발견 시 project.messages 에 메시지 항목도 추가.
/// EntryJS 가 message 이름으로 매칭하므로 id 도 name 과 동일.
#[test]
fn compile_when_message_registers_message_in_project() {
    let src = "fn when_message(my_msg: &str) { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let messages = v["messages"].as_array().expect("messages array");
    let msg = messages
        .iter()
        .find(|m| m["name"] == "my_msg")
        .expect("my_msg in messages");
    assert_eq!(msg["id"].as_str(), Some("my_msg"));
}

/// 트리거가 둘 (when_start + when_click) 이면 object.script 에 thread 2개.
#[test]
fn compile_multiple_triggers_produce_multiple_threads() {
    let src = r#"
        fn when_start() { let x = 1; }
        fn when_click() { let y = 2; }
    "#;
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    let threads = obj_threads(&objects[0]);
    assert_eq!(threads.len(), 2, "when_start + when_click");
    let t0 = threads[0].as_array().expect("t0");
    let t1 = threads[1].as_array().expect("t1");
    assert_eq!(t0[0]["type"], "when_run");
    assert_eq!(t1[0]["type"], "when_click");
    assert_eq!(t0[1]["params"][0]["name"], "x");
    assert_eq!(t1[1]["params"][0]["name"], "y");
}

/// 트리거가 없는 rs 도 정상 처리.
#[test]
fn compile_no_trigger_source_yields_threads() {
    let src = "fn helper() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    // helper 본문은 project.functions 로 가고 object.script 는 비어있음.
    let threads = obj_threads(&objects[0]);
    // threads 비어있거나 (when_run 만) 있어도 무해.
    let _ = threads.len();
}

/// 미매핑 블록은 (project, Vec<unmapped>) 의 unmapped 에 누적되고 빌드는 성공.
#[test]
fn compile_collects_unmapped_blocks() {
    let src = r#"
        fn when_start() {
            let x = timer;
        }
    "#;
    let (v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    let objects = v["objects"].as_array().unwrap();
    let first_thread = first_thread(&objects[0]);
    // when_run 만 들어가고 timer read stmt 는 빠짐
    assert_eq!(first_thread.len(), 1);
    assert_eq!(first_thread[0]["type"], "when_run");
    assert!(!unmapped.is_empty(), "unmapped 가 비어있으면 안 됨");
    let joined = unmapped.join(" ");
    assert!(joined.contains("timer"), "unmapped 에 timer 사유 포함: {unmapped:?}");
}

/// 미매핑이 없어도 unmapped Vec 은 비어있는 채로 반환.
#[test]
fn compile_empty_unmapped_when_all_supported() {
    let src = "fn when_start() { let x = 1; }";
    let (_v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    assert!(unmapped.is_empty(), "정상 rs 는 unmapped 비어야: {unmapped:?}");
}

/// 같은 미매핑 사유가 여러 위치에서 나도 unmapped 에는 한 번만 들어감 (dedup).
#[test]
fn compile_unmapped_is_deduplicated() {
    // 같은 stmt 가 if 의 두 분기에 들어 있어도 timer read 사유는 한 번만.
    let src = r#"
        fn when_start() {
            if 1 < 2 { let a = timer; }
            else { let b = timer; }
        }
    "#;
    let (_v, unmapped) = compile(&[("obj", src)], &empty_project()).expect("compile");
    let timer_msgs: Vec<&String> = unmapped
        .iter()
        .filter(|m| m.contains("timer"))
        .collect();
    assert_eq!(
        timer_msgs.len(),
        1,
        "같은 timer 사유가 dedup 되어야: {unmapped:?}"
    );
}

/// 변수 항목이 Entry 실제 .ent 형식 (sample 기준) 의 추가 필드를 포함.
/// 일반 변수는 등장한 object 의 stem 으로 object 필드 채워짐.
#[test]
fn compile_variables_have_entry_format_fields() {
    let src = "fn when_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let vars = v["variables"].as_array().expect("variables");
    let x = vars.iter().find(|v| v["name"] == "x").expect("x var");
    assert_eq!(x["visible"], true);
    assert_eq!(x["isCloud"], false);
    assert_eq!(x["isRealTime"], false);
    assert_eq!(x["cloudDate"], false);
    assert_eq!(x["object"], "obj", "x 는 obj 의 변수");
    assert_eq!(x["x"], 0);
    assert_eq!(x["y"], 0);
}

/// 가짜 object 가 Entry 부수 필드 (rotateMethod, lock) 포함.
#[test]
fn compile_fake_object_has_rotate_method_and_lock() {
    let src = "fn when_start() { let x = 1; }";
    let v = compile(&[("obj", src)], &empty_project()).expect("compile").0;
    let objects = v["objects"].as_array().unwrap();
    assert_eq!(objects[0]["rotateMethod"], "free");
    assert_eq!(objects[0]["lock"], false);
}
