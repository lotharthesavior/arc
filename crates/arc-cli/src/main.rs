use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

mod scaffold;

use scaffold::{create_project, NewProject};

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut args = args.peekable();
    match args.next().as_deref() {
        Some("new") => {
            let name = args
                .next()
                .ok_or_else(|| "usage: arc new <name> [--ui] [--no-git]".to_string())?;
            let mut ui = false;
            let mut git = true;

            for argument in args {
                match argument.as_str() {
                    "--ui" => ui = true,
                    "--no-git" => git = false,
                    "--help" | "-h" => {
                        print_new_help();
                        return Ok(());
                    }
                    unknown => return Err(format!("unknown option `{unknown}`")),
                }
            }

            let project = NewProject {
                name,
                destination: PathBuf::from("."),
                ui,
                git,
            };
            let path = create_project(&project)?;
            println!();
            println!("Created {} at {}", project.name, path.display());
            println!();
            println!("  cd {}", project.name);
            println!("  make setup");
            println!("  make dev");
            Ok(())
        }
        Some("--version" | "-V") => {
            println!("arc {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("--help" | "-h") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!(
            "unknown command `{command}`; run `arc --help` for usage"
        )),
    }
}

fn print_help() {
    println!(
        "Arc application generator

Usage:
  arc new <name> [--ui] [--no-git]

Options:
  --ui       Add Tera views and browser assets
  --no-git   Do not initialize a Git repository
  -h, --help Show help
  -V, --version Show version"
    );
}

fn print_new_help() {
    println!(
        "Create a self-contained Arc application

Usage:
  arc new <name> [--ui] [--no-git]"
    );
}
