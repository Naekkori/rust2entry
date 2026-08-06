use std::path::PathBuf;

use entrycore::Result;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<()> {
    let opts = parse_args(args)?;
    println!("source: {:?}", opts.source);
    println!("sprites: {:?}", opts.sprites);
    println!("output: {:?}", opts.output);

    let source = std::fs::read_to_string(&opts.source)?;
    let bytes = entrycore::compile(&source, opts.sprites.as_deref())?;
    std::fs::write(&opts.output, bytes)?;
    println!("written: {:?}", opts.output);
    Ok(())
}

#[derive(Debug)]
struct Options {
    source: PathBuf,
    sprites: Option<PathBuf>,
    output: PathBuf,
}

fn parse_args(args: &[String]) -> Result<Options> {
    let mut source: Option<PathBuf> = None;
    let mut sprites: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-s" | "--source" => {
                source = Some(PathBuf::from(args.get(i + 1).cloned().ok_or_else(|| {
                    entrycore::Error::Parse("missing --source value".into())
                })?));
                i += 2;
            }
            "-r" | "--sprites" => {
                sprites = Some(PathBuf::from(args.get(i + 1).cloned().ok_or_else(|| {
                    entrycore::Error::Parse("missing --sprites value".into())
                })?));
                i += 2;
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(args.get(i + 1).cloned().ok_or_else(|| {
                    entrycore::Error::Parse("missing --output value".into())
                })?));
                i += 2;
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other if !other.starts_with('-') => {
                source = Some(PathBuf::from(other));
                i += 1;
            }
            _ => {
                return Err(entrycore::Error::Parse(format!(
                    "unknown arg: {}",
                    args[i]
                )));
            }
        }
    }

    Ok(Options {
        source: source.ok_or_else(|| entrycore::Error::Parse("source required".into()))?,
        sprites,
        output: output.unwrap_or_else(|| PathBuf::from("output.ent")),
    })
}

fn print_help() {
    println!("entryc - Rust to Entry .ent compiler");
    println!();
    println!("USAGE:");
    println!("    entryc <source.rs>            compile source, write output.ent");
    println!("    entryc -s <src.rs> -r <sprites> -o <out.ent>");
    println!();
    println!("OPTIONS:");
    println!("    -s, --source <PATH>     Rust source file");
    println!("    -r, --sprites <DIR>     sprite folder");
    println!("    -o, --output <PATH>     output .ent file (default: output.ent)");
    println!("    -h, --help              show this help");
}
