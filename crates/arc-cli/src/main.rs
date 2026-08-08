use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

mod resource;
mod scaffold;

use resource::{create_resource, NewResource};
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
        Some("generate") => {
            let kind = args.next().ok_or_else(|| {
                "usage: arc generate resource <name> (alias: aggregate)".to_string()
            })?;
            if kind != "resource" && kind != "aggregate" {
                return Err(format!(
                    "unknown generator `{kind}`; expected `resource` or `aggregate`"
                ));
            }
            let name = args.next().ok_or_else(|| {
                "usage: arc generate resource <name> [--api] [--ui] (alias: aggregate)".to_string()
            })?;
            let mut api = false;
            let mut ui = false;
            for argument in args {
                match argument.as_str() {
                    "--api" => api = true,
                    "--ui" => ui = true,
                    "--help" | "-h" => {
                        print_generate_help();
                        return Ok(());
                    }
                    _ => return Err(format!("unknown option `{argument}`")),
                }
            }

            let resource = NewResource {
                name,
                root: PathBuf::from("."),
                api,
                ui,
            };
            let created = create_resource(&resource)?;
            println!();
            println!("Generated {} resource:", created.type_name);
            for path in created.files {
                println!("  {}", path.display());
            }
            println!();
            if let Some(api_path) = created.api_path {
                println!("Run `make migrate`, then use the CRUD API at {api_path}.");
            } else {
                println!(
                    "Run `make migrate` and add routes that dispatch {}Command.",
                    created.type_name
                );
            }
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
  arc generate resource <name> [--api] [--ui]
  arc generate aggregate <name> [--api] [--ui]

Options:
  --ui       Add Tera views and browser assets
  --no-git   Do not initialize a Git repository
  -h, --help Show help
  -V, --version Show version"
    );
}

fn print_generate_help() {
    println!(
        "Generate and register an event-sourced aggregate resource

Usage:
  arc generate resource <name> [--api] [--ui]
  arc generate aggregate <name> [--api] [--ui]

Options:
  --api  Add JWT-protected JSON CRUD endpoints
  --ui   Add session-protected browser CRUD pages (requires `arc new --ui`)

Run this command from the root of an application created by `arc new`."
    );
}

fn print_new_help() {
    println!(
        "Create a self-contained Arc application

Usage:
  arc new <name> [--ui] [--no-git]"
    );
}
