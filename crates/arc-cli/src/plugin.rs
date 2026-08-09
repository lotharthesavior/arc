use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const IMPORT: &str = "// arc:plugin-imports";
const REGISTER: &str = "        // arc:plugin-registrations";

pub fn add_plugin(root: PathBuf, requested: &str) -> Result<(), String> {
    let capabilities: &[&str] = match requested {
        "auth-db-session" => &["auth-db", "auth-session", "auth-rbac"],
        "auth-db-jwt" => &["auth-db", "auth-jwt", "auth-rbac"],
        "auth-db" | "auth-session" | "auth-jwt" | "auth-rbac" => &[requested],
        _ => return Err(format!("unknown plugin `{requested}`")),
    };
    let manifest_path = root.join("Cargo.toml");
    let main_path = root.join("src/main.rs");
    let mut manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let mut main =
        fs::read_to_string(&main_path).map_err(|e| format!("{}: {e}", main_path.display()))?;
    if !main.contains(IMPORT) || !main.contains(REGISTER) {
        return Err("generated application lacks Arc plugin markers".into());
    }
    for capability in capabilities {
        let crate_name = capability.replace('-', "_");
        let package = format!("arc-{capability}");
        if !manifest
            .lines()
            .any(|line| line.starts_with(&format!("{package} =")))
        {
            let dependency = dependency(&package);
            manifest = manifest.replace(
                "[dependencies]\n",
                &format!("[dependencies]\n{dependency}\n"),
            );
        }
        let (import, registration) = match *capability {
            "auth-db" => ("use arc_auth_db::DbIdentityPlugin;", ".register_plugin(DbIdentityPlugin::new(std::env::var(\"DATABASE_URL\").unwrap_or_else(|_| \"database/database.sqlite\".into())))"),
            "auth-session" => ("use arc_auth_session::SessionAuthPlugin;", ".register_plugin(SessionAuthPlugin)"),
            "auth-jwt" => ("use arc_auth_jwt::JwtAuthPlugin;", ".register_plugin(JwtAuthPlugin)"),
            "auth-rbac" => ("use arc_auth_rbac::RbacPlugin;", ".register_plugin(RbacPlugin)"),
            _ => unreachable!(),
        };
        if !main.contains(import) {
            main = main.replace(IMPORT, &format!("{IMPORT}\n{import}"));
        }
        if !main.contains(registration) {
            main = main.replace(REGISTER, &format!("        {registration}\n{REGISTER}"));
        }
        let _ = crate_name;
    }
    fs::write(manifest_path, manifest).map_err(|e| e.to_string())?;
    fs::write(main_path, main).map_err(|e| e.to_string())?;
    if capabilities.contains(&"auth-session") {
        protect_admin_dashboard(&root)?;
    }
    let status = Command::new("cargo")
        .arg("fmt")
        .current_dir(&root)
        .status()
        .map_err(|e| format!("could not run cargo fmt: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("cargo fmt failed after plugin installation".into())
    }
}

fn protect_admin_dashboard(root: &Path) -> Result<(), String> {
    let path = root.join("src/ui.rs");
    let mut source = fs::read_to_string(&path).map_err(|_| {
        "browser session auth requires an application created with `arc new --ui`".to_string()
    })?;
    if !source.contains("use arc_auth_session::RequireSession;") {
        source = source.replace(
            "use actix_web::{get, web, HttpResponse, Responder};",
            "use actix_web::{get, web, HttpResponse, Responder};\nuse arc_auth_session::RequireSession;",
        );
    }
    if !source.contains("wrap(RequireSession)") {
        source = source.replace(
            "cfg.service(home).service(dashboard);",
            "cfg.service(home).service(web::scope(\"\").wrap(RequireSession).service(dashboard));",
        );
    }
    fs::write(path, source).map_err(|e| e.to_string())
}

fn dependency(package: &str) -> String {
    if let Ok(root) = std::env::var("ARC_CLI_TEST_LOCAL_ROOT") {
        format!("{package} = {{ path = \"{root}/crates/{package}\" }}")
    } else {
        format!("{package} = \"{}\"", env!("CARGO_PKG_VERSION"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::{create_project, NewProject};

    #[test]
    fn session_capability_protects_generated_admin_dashboard() {
        let destination =
            std::env::temp_dir().join(format!("arc-plugin-test-{}", std::process::id()));
        let root = create_project(&NewProject {
            name: "protected-ui".into(),
            destination: destination.clone(),
            ui: true,
            api: false,
            git: false,
        })
        .unwrap();
        add_plugin(root.clone(), "auth-session").unwrap();
        let ui = fs::read_to_string(root.join("src/ui.rs")).unwrap();
        assert!(ui.contains("wrap(RequireSession)"));
        assert!(ui.contains("use arc_auth_session::RequireSession;"));
        fs::remove_dir_all(destination).unwrap();
    }
}
