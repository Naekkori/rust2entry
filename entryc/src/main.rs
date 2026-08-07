use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

// zip 언팩, project.json 파싱, 오브젝트별 <name>.rs 생성
#[derive(Parser, Debug)]
#[command(
    name = "entryc",
    about = "Entry .ent extractor",
    long_about = "Entry .ent (gzip+tar) 프로젝트를 임시폴더에 언팩하고 project.json 을 읽어 오브젝트별 <name>.rs 를 생성한다."
)]
struct Cli {
    /// Entry 프로젝트 파일 (.ent)
    #[arg(long, value_name = "FILE")]
    ent: PathBuf,

    /// 출력 폴더 (미지정시 .ent 위치/<프로젝트이름>)
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    // 임시폴더 생성
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).map_err(|e| format!("temp mkdir failed: {e}"))?;

    // 언팩 보장: 작업 후 정리
    let result = (|| -> Result<(), String> {
        extract(&cli.ent, &temp_dir)?;
        let project = load_project(&temp_dir)?;
        let out_dir = resolve_out_dir(&cli, &project)?;
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
    })
}

fn resolve_out_dir(cli: &Cli, project: &Project) -> Result<PathBuf, String> {
    if let Some(p) = &cli.out {
        return Ok(p.clone());
    }
    let parent = cli
        .ent
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
                    match entrycore::deparse::program_from_script_string_with_vars(s, &project.var_map) {
                        Ok(program) => match entrycore::decodegen::emit_with_var_map(&program, &project.var_map) {
                            Ok(dsl) => {
                                let header = format!("// object: {}\n", o.name);
                                format!("{header}{dsl}")
                            }
                            Err(e) => format!(
                                "// object: {}\n// decodegen error: {e}\n",
                                o.name
                            ),
                        },
                        Err(e) => format!(
                            "// object: {}\n// deparse error: {e}\n// raw:\n{s}\n",
                            o.name
                        ),
                    }
                }
                Some(v) => {
                    match entrycore::deparse::program_from_script_value_with_vars(v, &project.var_map) {
                        Ok(program) => match entrycore::decodegen::emit_with_var_map(&program, &project.var_map) {
                            Ok(dsl) => {
                                let header = format!("// object: {}\n", o.name);
                                format!("{header}{dsl}")
                            }
                            Err(e) => format!(
                                "// object: {}\n// decodegen error: {e}\n",
                                o.name
                            ),
                        },
                        Err(e) => {
                            let pretty = serde_json::to_string_pretty(v)
                                .unwrap_or_else(|_| v.to_string());
                            format!(
                                "// object: {}\n// deparse error: {e}\n// raw:\n{pretty}\n",
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
