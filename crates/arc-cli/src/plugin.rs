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
        let (import, registration, registration_marker) = match *capability {
            "auth-db" => ("use arc_auth_db::DbIdentityPlugin;", ".register_plugin(DbIdentityPlugin::new(std::env::var(\"DATABASE_URL\").unwrap_or_else(|_| \"database/database.sqlite\".into())))", ".register_plugin(DbIdentityPlugin::new("),
            "auth-session" => ("use arc_auth_session::SessionAuthPlugin;", ".register_plugin(SessionAuthPlugin)", ".register_plugin(SessionAuthPlugin)"),
            "auth-jwt" => ("use arc_auth_jwt::JwtAuthPlugin;", ".register_plugin(JwtAuthPlugin)", ".register_plugin(JwtAuthPlugin)"),
            "auth-rbac" => ("use arc_auth_rbac::RbacPlugin;", ".register_plugin(RbacPlugin)", ".register_plugin(RbacPlugin)"),
            _ => unreachable!(),
        };
        if !main.contains(import) {
            main = main.replace(IMPORT, &format!("{IMPORT}\n{import}"));
        }
        if !main.contains(registration_marker) {
            main = main.replace(REGISTER, &format!("        {registration}\n{REGISTER}"));
        }
        let _ = crate_name;
    }
    for marker in [
        ".register_plugin(DbIdentityPlugin::new(",
        ".register_plugin(SessionAuthPlugin)",
        ".register_plugin(JwtAuthPlugin)",
        ".register_plugin(RbacPlugin)",
    ] {
        main = remove_duplicate_registration(main, marker);
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

fn remove_duplicate_registration(mut source: String, marker: &str) -> String {
    let Some(first) = source.find(marker) else {
        return source;
    };
    let mut search_from = first + marker.len();
    while let Some(relative) = source[search_from..].find(marker) {
        let start = search_from + relative;
        let Some(open_relative) = source[start..].find('(') else {
            break;
        };
        let open = start + open_relative;
        let mut depth = 0_usize;
        let mut end = None;
        for (offset, character) in source[open..].char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(open + offset + character.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        source.replace_range(start..end, "");
        search_from = first + marker.len();
    }
    source
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
    if !source.contains("IdleTimeoutMiddleware") {
        source = source.replace(
            "use arc_auth_session::RequireSession;",
            "use arc_auth_session::RequireSession;\nuse arc_web::http::middlewares::idle_timeout_middleware::IdleTimeoutMiddleware;",
        );
    }
    source = source.replace("#[get(\"/admin\")]\n", "");
    source = source.replace(
        "web::scope(\"\").wrap(RequireSession).service(dashboard)",
        "web::scope(\"/admin\").wrap(RequireSession).route(\"\", web::get().to(dashboard))",
    );
    source = source.replace(
        "web::scope(\"/admin\").route(\"\", web::get().to(dashboard))",
        "web::scope(\"/admin\").wrap(RequireSession).route(\"\", web::get().to(dashboard))",
    );
    source = source.replace(
        "cfg.service(home).service(dashboard);",
        "cfg.service(home).service(web::scope(\"/admin\").wrap(RequireSession).route(\"\", web::get().to(dashboard)));",
    );
    source = source.replace(
        ".wrap(RequireSession).route(\"\", web::get().to(dashboard))",
        ".wrap(RequireSession).wrap(IdleTimeoutMiddleware::from_env()).route(\"\", web::get().to(dashboard))",
    );
    fs::write(path, source).map_err(|e| e.to_string())?;

    let layout_path = root.join("resources/views/layouts/admin.html");
    let mut layout = fs::read_to_string(&layout_path)
        .map_err(|_| "browser session auth requires the generated admin layout".to_string())?;
    if !layout.contains("href=\"/admin/profile\"") {
        layout = layout.replace(
            "<!-- arc:capability-navigation -->",
            "<a href=\"/admin/profile\">Profile</a><a href=\"/admin/users\">Users</a><!-- arc:capability-navigation -->",
        );
    }
    fs::write(layout_path, layout).map_err(|e| e.to_string())
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
        add_plugin(root.clone(), "auth-db-session").unwrap();
        add_plugin(root.clone(), "auth-db-session").unwrap();
        let ui = fs::read_to_string(root.join("src/ui.rs")).unwrap();
        assert!(ui.contains("wrap(RequireSession)"));
        assert!(ui.contains("IdleTimeoutMiddleware::from_env()"));
        assert!(ui.contains("scope(\"/admin\")"));
        assert!(!ui.contains("scope(\"\")"));
        assert!(ui.contains("use arc_auth_session::RequireSession;"));
        let admin_layout =
            fs::read_to_string(root.join("resources/views/layouts/admin.html")).unwrap();
        assert!(admin_layout.contains("href=\"/admin/profile\""));
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert_eq!(
            main.matches(".register_plugin(DbIdentityPlugin::new(")
                .count(),
            1
        );
        assert_eq!(
            main.matches(".register_plugin(SessionAuthPlugin)").count(),
            1
        );
        assert_eq!(main.matches(".register_plugin(RbacPlugin)").count(), 1);
        fs::remove_dir_all(destination).unwrap();
    }
}
