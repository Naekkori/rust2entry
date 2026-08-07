use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

// Rust ↔ Entry .ent 변환기
#[derive(Parser, Debug)]
#[command(
    name = "entryc",
    about = "Rust ↔ Entry .ent 변환기",
    long_about = "Rust 소스를 Entry .ent 로 컴파일하거나 (.rs -> .ent), Entry .ent 를 Rust 소스로 추출 (.ent -> .rs) 한다."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// .ent -> .rs 오브젝트별 추출
    Extract {
        /// 입력 Entry 프로젝트 파일
        #[arg(long, value_name = "FILE")]
        ent: PathBuf,
        /// 출력 폴더 (미지정시 .ent 위치/<프로젝트이름>)
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
    },
    /// .rs -> .ent 빌드
    Build {
        /// 입력 .rs 파일 (1개 이상, 반복 가능, 오브젝트별 1개)
        #[arg(long, value_name = "FILE", required = true)]
        rs: Vec<PathBuf>,
        /// 베이스 .ent (선택). 미지정시 빈 프로젝트에서 시작.
        #[arg(long, value_name = "FILE")]
        ent_template: Option<PathBuf>,
        /// 출력 .ent 경로
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
}
fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// raw JSON 을 줄별로 들여쓰기 + `// ` 접두 붙여서 보기 좋게 변환.
fn format_raw_block(pretty_json: &str) -> String {
    pretty_json
        .lines()
        .map(|line| format!("// {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 에러 메시지를 콜론(:) 기준으로 분할해 단계별 들여쓰기.
/// 예: "unmapped block: entry block type: when_object_click"
///   -> "unmapped block:"
///      "    entry block type:"
///      "        when_object_click"
fn format_error_block(msg: &str) -> String {
    let parts: Vec<&str> = msg.split(": ").collect();
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let indent = "    ".repeat(i);
            let suffix = if i + 1 < parts.len() { ":" } else { "" };
            format!("// {indent}{p}{suffix}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Extract { ent, out } => run_extract(ent, out),
        Cmd::Build { rs, ent_template, out } => run_build(&rs, ent_template.as_deref(), &out),
    }
}

// .ent -> 임시폴더 언팩 -> 오브젝트별 .rs 생성
fn run_extract(ent: PathBuf, out: Option<PathBuf>) -> Result<(), String> {
    // 임시폴더 생성
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).map_err(|e| format!("temp mkdir failed: {e}"))?;
    // 언팩 보장: 작업 후 정리
    let result = (|| -> Result<(), String> {
        extract(&ent, &temp_dir)?;
        let project = load_project(&temp_dir)?;
        let scripts = &project.scripts_value;
        let unmapped = entrycore::deparse::collect_unmapped_blocks(scripts, &project.var_map);
        if !unmapped.is_empty() {
            let summary: Vec<String> = unmapped
                .iter()
                .map(|(t, c)| format!("{t}({c})"))
                .collect();
            eprintln!("unmapped: {}", summary.join(", "));
            eprintln!("hint: 미매핑 블록은 .rs 에 raw JSON 코멘트로 보존됨");
        }
        let out_dir = resolve_out_dir(&ent, out.as_deref(), &project)?;
        fs::create_dir_all(&out_dir).map_err(|e| format!("out mkdir failed: {e}"))?;
        write_object_scripts(&project, &out_dir)?;
        println!("project: {}", project.name);
        println!("out:     {}", out_dir.display());
        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

// .ent (gzip+tar) 을 dir 에 풀기
fn extract(ent: &Path, dir: &Path) -> Result<(), String> {
    let file = fs::File::open(ent).map_err(|e| format!("open {}: {e}", ent.display()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);

    // 한글 경로 안전을 위해 set_preserve_permissions off, set_unpack_xattrs off (기본값)
    for entry in tar.entries().map_err(|e| format!("tar entries: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        let rel = entry.path().map_err(|e| format!("tar path: {e}"))?.into_owned();
        if rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(format!("unsafe path in tar: {}", rel.display()));
        }
        let out_path = dir.join(&rel);

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| format!("mkdir: {e}"))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let mut f = fs::File::create(&out_path).map_err(|e| format!("create: {e}"))?;
        io_copy(&mut entry, &mut f).map_err(|e| format!("write: {e}"))?;
    }

    Ok(())
}

fn io_copy<R: Read, W: Write>(r: &mut R, w: &mut W) -> io::Result<()> {
    let mut buf = [0u8; 8192];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        w.write_all(&buf[..n])?;
    }
    Ok(())
}

fn find_project_json(temp_dir: &Path) -> Result<PathBuf, String> {
    let direct = temp_dir.join("project.json");
    if direct.is_file() {
        return Ok(direct);
    }
    // 한 단계 아래 탐색
    let entries = fs::read_dir(temp_dir).map_err(|e| format!("read dir: {e}"))?;
    for e in entries {
        let e = e.map_err(|e| format!("dir entry: {e}"))?;
        let sub = e.path().join("project.json");
        if sub.is_file() {
            return Ok(sub);
        }
    }
    Err("project.json not found".to_string())
}

// project.json 파싱
#[derive(Debug)]
struct Project {
    name: String,
    objects: Vec<Object>,
    var_map: entrycore::VarMap,
    /// project.json 의 scripts 필드 (extract 시 미매핑 집계용)
    scripts_value: serde_json::Value,
}

#[derive(Debug)]
struct Object {
    name: String,
    script: Option<serde_json::Value>,
    has_script: bool,
}

fn load_project(temp_dir: &Path) -> Result<Project, String> {
    // .ent 안쪽이 temp/<...> 구조일 수 있음. project.json 위치 탐색.
    let path = find_project_json(temp_dir)?;
    let raw = fs::read_to_string(&path).map_err(|e| format!("read project.json: {e}"))?;

    // 원본 Value (variables 전달용) + 구조 파싱
    let raw_value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("project.json parse: {e}"))?;

    #[derive(serde::Deserialize)]
    struct RawProject {
        name: String,
        #[serde(default)]
        objects: Vec<RawObject>,
    }
    #[derive(serde::Deserialize)]
    struct RawObject {
        name: String,
        #[serde(default)]
        script: Option<serde_json::Value>,
    }

    let raw_proj: RawProject =
        serde_json::from_str(&raw).map_err(|e| format!("project.json parse: {e}"))?;

    // 변수 맵 빌드
    let var_map = entrycore::var::var_map_from_value(
        raw_value.get("variables").unwrap_or(&serde_json::Value::Null),
    );

    let mut objects = Vec::with_capacity(raw_proj.objects.len());
    for o in raw_proj.objects {
        let has_script = match &o.script {
            None => false,
            Some(serde_json::Value::Null) => false,
            Some(serde_json::Value::String(s)) => {
                let s = s.trim();
                !(s.is_empty() || s == "[]")
            }
            Some(serde_json::Value::Array(a)) => !a.is_empty(),
            Some(_) => true,
        };

        objects.push(Object {
            name: o.name,
            script: o.script,
            has_script,
        });
    }

    Ok(Project {
        name: raw_proj.name,
        objects,
        var_map,
        scripts_value: raw_value.get("scripts").cloned().unwrap_or(serde_json::Value::Null),
    })
}

fn resolve_out_dir(ent: &Path, out: Option<&Path>, project: &Project) -> Result<PathBuf, String> {
    if let Some(p) = out {
        return Ok(p.to_path_buf());
    }
    let parent = ent
        .parent()
        .ok_or_else(|| "invalid --ent path".to_string())?;
    Ok(parent.join(sanitize_dir_name(&project.name)))
}

// 오브젝트별 <이름>.rs 생성
fn write_object_scripts(project: &Project, out_dir: &Path) -> Result<(), String> {
    for o in &project.objects {
        let file_name = format!("{}.rs", sanitize_filename(&o.name));
        let path = out_dir.join(&file_name);

        let body = if !o.has_script {
            format!(
                "// object: {} (empty script)\nfn when_start() {{\n}}\n",
                o.name
            )
        } else {
            match &o.script {
                Some(serde_json::Value::String(s)) => {
                    // String 케이스: 내부 JSON 을 파싱해서 pretty 로 들여쓰기
                    let pretty = serde_json::from_str::<serde_json::Value>(s)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| s.clone());
                    match entrycore::deparse::program_from_script_string_with_vars(s, &project.var_map) {
                        Ok(program) => match entrycore::decodegen::emit_with_var_map(&program, &project.var_map) {
                            Ok(dsl) => {
                                let header = format!("// object: {}\n", o.name);
                                format!("{header}{dsl}")
                            }
                            Err(e) => {
                                let err_block = format_error_block(&format!("decodegen error: {e}"));
                                format!(
                                    "// object: {}\n{err_block}\n",
                                    o.name
                                )
                            }
                        },
                        Err(e) => {
                            let err_block = format_error_block(&format!("deparse error: {e}"));
                            let raw = format_raw_block(&pretty);
                            format!(
                                "// object: {}\n{err_block}\n// raw (매핑 안 되는 블록 포함):\n{raw}\n",
                                o.name
                            )
                        }
                    }
                }
                Some(v) => {
                    match entrycore::deparse::program_from_script_value_with_vars(v, &project.var_map) {
                        Ok(program) => match entrycore::decodegen::emit_with_var_map(&program, &project.var_map) {
                            Ok(dsl) => {
                                let header = format!("// object: {}\n", o.name);
                                format!("{header}{dsl}")
                            }
                            Err(e) => {
                                let err_block = format_error_block(&format!("decodegen error: {e}"));
                                format!(
                                    "// object: {}\n{err_block}\n",
                                    o.name
                                )
                            }
                        },
                        Err(e) => {
                            let pretty = serde_json::to_string_pretty(v)
                                .unwrap_or_else(|_| v.to_string());
                            let err_block = format_error_block(&format!("deparse error: {e}"));
                            let raw = format_raw_block(&pretty);
                            format!(
                                "// object: {}\n{err_block}\n// raw (매핑 안 되는 블록 포함):\n{raw}\n",
                                o.name
                            )
                        }
                    }
                }
                None => continue,
            }
        };

        fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
        println!("wrote: {}", path.display());
    }
    Ok(())
}

// 파일/폴더명 안전 처리 (한글/특수문자 보존, 경로 금지문자만 치환)
fn sanitize_filename(s: &str) -> String {
    let bad = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let cleaned: String = s
        .chars()
        .map(|c| if bad.contains(&c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed
    }
}

fn sanitize_dir_name(s: &str) -> String {
    let mut n = sanitize_filename(s);
    // 폴더명 trailing space/dot 윈도우 금지
    while n.ends_with(' ') || n.ends_with('.') {
        n.pop();
    }
    if n.is_empty() {
        "_".to_string()
    } else {
        n
    }
}

fn unique_temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    base.join(format!("entryc-{pid}-{nanos}"))
}

// .rs -> IR -> codegen -> project.json 패치 -> tar+gzip (.ent) 패키징
fn run_build(rs_files: &[PathBuf], template: Option<&Path>, out: &Path) -> Result<(), String> {
    if rs_files.is_empty() {
        return Err("no --rs inputs".to_string());
    }

    // base Value 로드 (template 또는 빈 프로젝트)
    let base = match template {
        Some(p) => load_project_value(p)?,
        None => default_empty_project(),
    };

    // .rs 소스 로드 (파일명 stem 을 오브젝트 이름으로 사용)
    let mut sources: Vec<(String, String)> = Vec::with_capacity(rs_files.len());
    for rs in rs_files {
        let src = fs::read_to_string(rs).map_err(|e| format!("read {}: {e}", rs.display()))?;
        let name = rs
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("object")
            .to_string();
        sources.push((name, src));
    }
    let sources_ref: Vec<(&str, &str)> = sources
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();

    // lib::compile 으로 일괄 처리 (parse 합치기 + codegen + base 패치)
    let final_project = entrycore::compile(&sources_ref, &base)
        .map_err(|e| format!("compile: {e}"))?;

    // .ent 패키징
    pack_ent(template, &final_project, out)?;

    println!("out: {}", out.display());
    Ok(())
}

// .ent 언팩 -> project.json -> serde_json::Value (var_map 없이 Value만)
fn load_project_value(ent: &Path) -> Result<serde_json::Value, String> {
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).map_err(|e| format!("temp mkdir: {e}"))?;
    let result = (|| -> Result<serde_json::Value, String> {
        extract(ent, &temp_dir)?;
        let path = find_project_json(&temp_dir)?;
        let raw = fs::read_to_string(&path).map_err(|e| format!("read project.json: {e}"))?;
        serde_json::from_str(&raw).map_err(|e| format!("project.json parse: {e}"))
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

// codegen 테스트와 동일한 빈 프로젝트 기본값
fn default_empty_project() -> serde_json::Value {
    serde_json::json!({
        "name": "rust2entry",
        "speed": 60, "objects": [], "variables": [], "messages": [],
        "functions": [], "scenes": [{"id":"scene1","name":"장면1"}],
        "interface": {"views": []}, "meta": {}
    })
}

// 최종 project.json + 베이스에서 가져온 부수파일(이미지 등) -> tar+gzip -> .ent
fn pack_ent(template: Option<&Path>, project: &serde_json::Value, out: &Path) -> Result<(), String> {
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).map_err(|e| format!("temp mkdir: {e}"))?;
    let result = (|| -> Result<(), String> {
        // 베이스가 있으면 부수파일을 먼저 풀어둠
        if let Some(t) = template {
            extract(t, &temp_dir)?;
        }

        // project.json 덮어쓰기 (베이스가 있으면 그쪽 경로에)
        let pj_path = if template.is_some() {
            // 베이스의 project.json 경로를 재탐색 (하위 폴더 구조 매칭)
            let probe_dir = if temp_dir.join("project.json").is_file() {
                temp_dir.clone()
            } else {
                // 베이스에서 가장 최근에 project.json이 있던 하위 폴더
                let entries = fs::read_dir(&temp_dir).map_err(|e| format!("read_dir: {e}"))?;
                let mut found_sub: Option<PathBuf> = None;
                for e in entries {
                    let e = e.map_err(|e| format!("dir: {e}"))?;
                    let cand = e.path().join("project.json");
                    if cand.is_file() {
                        found_sub = Some(e.path());
                        break;
                    }
                }
                found_sub.unwrap_or_else(|| temp_dir.clone())
            };
            find_project_json(&probe_dir)?
        } else {
            temp_dir.join("project.json")
        };

        if let Some(parent) = pj_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let serialized = serde_json::to_string_pretty(project)
            .map_err(|e| format!("serialize: {e}"))?;
        fs::write(&pj_path, serialized).map_err(|e| format!("write project.json: {e}"))?;

        // tar + gzip
        let out_file = fs::File::create(out)
            .map_err(|e| format!("create {}: {e}", out.display()))?;
        let gz = flate2::write::GzEncoder::new(out_file, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(gz);
        tar_builder
            .append_dir_all(".", &temp_dir)
            .map_err(|e| format!("tar append: {e}"))?;
        tar_builder.finish().map_err(|e| format!("tar finish: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    result
}
