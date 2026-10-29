//! Checks blueprint files: the structural validator (`ch-blueprint`) and the
//! data-backed check (`ch-engine`: every essence, alloy and liquid emotion
//! exists for one of the blueprint's classes), and no two files with one id.
//!
//! ```text
//! cargo run --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --example check_blueprints -- <file or folder>...
//! ```
//!
//! A folder means its `.toml` files, sorted by name. Prints one
//! `<path>:<line>:<col>: <message>` line per error and one `<path>: ok` line
//! per valid file. Exits 1 on any error, 2 with no argument, 0 otherwise.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    if args.is_empty() {
        eprintln!("usage: check_blueprints <blueprint file or folder>...");
        return ExitCode::from(2);
    }
    let mut failed = false;
    let mut ids: HashMap<String, String> = HashMap::new();
    for arg in &args {
        let files = match ch_engine::toml_files(arg) {
            Ok(files) => files,
            Err(e) => {
                println!("{}:1:1: {e}", arg.display());
                failed = true;
                continue;
            }
        };
        for file in files {
            match ch_engine::check_path(&file) {
                Ok(resolved) => {
                    if let Some(first) = ids.get(resolved.id()) {
                        let span = resolved.blueprint().id.span();
                        let e = resolved.checked.at(
                            span,
                            format!("blueprint id `{}` is also the id of {first}", resolved.id()),
                        );
                        println!("{e}");
                        failed = true;
                    } else {
                        ids.insert(resolved.id().to_string(), resolved.path().to_string());
                        println!("{}: ok", resolved.path());
                    }
                }
                Err(errors) => {
                    failed = true;
                    for e in errors {
                        println!("{e}");
                    }
                }
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
