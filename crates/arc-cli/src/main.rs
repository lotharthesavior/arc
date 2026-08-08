use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

mod plugin;
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
                .ok_or_else(|| "usage: arc new <name> [--ui] [--api] [--no-git]".to_string())?;
            let mut ui = false;
            let mut api = false;
            let mut git = true;

            for argument in args {
                match argument.as_str() {
                    "--ui" => ui = true,
                    "--api" => api = true,
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
                api,
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
            let mut api_auth = None;
            let mut ui_auth = None;
            let mut roles = Vec::new();
            while let Some(argument) = args.next() {
                match argument.as_str() {
                    "--api" => api = true,
                    "--ui" => ui = true,
                    "--api-auth" => {
                        api_auth = Some(args.next().ok_or("--api-auth requires jwt or none")?)
                    }
                    "--ui-auth" => {
                        ui_auth = Some(args.next().ok_or("--ui-auth requires session or none")?)
                    }
                    "--roles" => {
                        roles = args
                            .next()
                            .ok_or("--roles requires a comma-separated value")?
                            .split(',')
                            .map(str::to_owned)
                            .collect()
                    }
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
                api_auth,
                ui_auth,
                roles,
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
        Some("plugin") => {
            if args.next().as_deref() != Some("add") {
                return Err("usage: arc plugin add <auth-db|auth-session|auth-jwt|auth-rbac|auth-db-session|auth-db-jwt>".into());
            }
            let name = args.next().ok_or("plugin name is required")?;
            plugin::add_plugin(PathBuf::from("."), &name)?;
            println!("Installed {name}.");
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
  arc new <name> [--ui] [--api] [--no-git]
  arc plugin add <name>
  arc generate resource <name> [--api] [--ui] [--api-auth jwt] [--ui-auth session] [--roles admin,user]
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
  --api  Add public JSON CRUD endpoints
  --ui   Add public browser CRUD pages (requires `arc new --ui`)
  --api-auth jwt      Explicitly protect this API resource
  --ui-auth session   Explicitly protect this browser resource
  --roles LIST        Require any listed role (requires auth)

Run this command from the root of an application created by `arc new`."
    );
}

fn print_new_help() {
    println!(
        "Create a self-contained Arc application

Usage:
  arc new <name> [--ui] [--api] [--no-git]"
    );
}
