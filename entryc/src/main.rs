use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    let rest = &args[2..];

    match cmd {
        "extract" => extract_cmd(rest),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    }
}

/// extract <input.ent> [-o <out_dir>]
fn extract_cmd(args: &[String]) -> Result<(), String> {
    let opts = parse_extract_args(args)?;
    println!("input:  {:?}", opts.input);
    println!("output: {:?}", opts.output);
    // TODO: zip 언팩, project.json 파싱, 오브젝트별 <name>.rs 생성
    Ok(())
}

#[derive(Debug)]
struct ExtractOptions {
    input: PathBuf,
    output: Option<PathBuf>,
}

fn parse_extract_args(args: &[String]) -> Result<ExtractOptions, String> {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                output = Some(PathBuf::from(args.get(i + 1)
                    .cloned()
                    .ok_or_else(|| "missing --output value".to_string())?));
                i += 2;
            }
            other if !other.starts_with('-') => {
                input = Some(PathBuf::from(other));
                i += 1;
            }
            _ => return Err(format!("unknown arg: {}", args[i])),
        }
    }

    Ok(ExtractOptions {
        input: input.ok_or_else(|| "input.ent required".to_string())?,
        output,
    })
}

fn print_help() {
    println!("entryc - Entry .ent extractor");
    println!();
    println!("USAGE:");
    println!("    entryc extract <input.ent> [-o <out_dir>]");
    println!("    entryc help");
}
