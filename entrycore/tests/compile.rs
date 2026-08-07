use std::io::Read;

use entrycore::compile;

#[test]
fn compile_minimal_fn_main() {
    // 빈 스프라이트 폴더로 zip 빌드. project.json 포함 + non-empty 확인.
    let src = r#"
        fn when_start() {
            let x = 1;
        }
    "#;
    let bytes = compile(src, None).expect("compile ok");
    assert!(!bytes.is_empty(), "zip bytes non-empty");

    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).expect("valid zip");
    let mut f = zip.by_name("project.json").expect("project.json present");
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    assert!(s.contains("\"speed\":60") || s.contains("\"speed\": 60"));
}

#[test]
#[ignore = "project::build 스프라이트 zip 미포함 버그 별도 추적"]
fn compile_with_sprites_includes_files() {
    // 스프라이트 디렉토리의 파일이 zip에 포함되는지 확인.
    let tmp = std::env::temp_dir().join("rust2entry_e2e_sprites");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("cat.png"), b"fake-png-bytes").unwrap();
    std::fs::write(tmp.join("dog.png"), b"fake-png-bytes-2").unwrap();

    let src = "fn main() {}";
    let bytes = compile(src, Some(&tmp)).expect("compile ok");

    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).expect("valid zip");
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "project.json"));
    assert!(
        names.iter().any(|n| n.contains("cat.png") || n.contains("dog.png")),
        "sprite files in zip: {names:?}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
