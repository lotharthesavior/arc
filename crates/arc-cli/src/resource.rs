use std::fs;
use std::path::{Path, PathBuf};

const DOMAIN_MARKER: &str = "// arc:domain-modules";
const IMPORT_MARKER: &str = "// arc:resource-imports";
const REGISTRATION_MARKER: &str = "        // arc:resource-registrations";
const API_ROUTE_MARKER: &str = "    // arc:api-routes";

pub struct NewResource {
    pub name: String,
    pub root: PathBuf,
    pub api: bool,
}

#[derive(Debug)]
pub struct CreatedResource {
    pub type_name: String,
    pub files: Vec<PathBuf>,
    pub api_path: Option<String>,
}

struct Names {
    type_name: String,
    module: String,
    constant: String,
    view: String,
}

struct GeneratedFile {
    relative: PathBuf,
    contents: String,
}

pub fn create_resource(resource: &NewResource) -> Result<CreatedResource, String> {
    let names = parse_name(&resource.name)?;
    validate_project(&resource.root)?;

    let domain_path = resource.root.join("src/domain.rs");
    let main_path = resource.root.join("src/main.rs");
    let domain = read_domain(&domain_path)?;
    let main = read_main(&main_path)?;

    let migration_version = next_migration_version(&resource.root.join("migrations"))?;
    let files = generated_files(&names, migration_version, resource.api);
    let collisions = files
        .iter()
        .map(|file| resource.root.join(&file.relative))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    if !collisions.is_empty() {
        return Err(format!(
            "Arc will not overwrite existing resource files: {}",
            collisions
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let module_line = format!("pub mod {};", names.module);
    if domain.lines().any(|line| line.trim() == module_line) {
        return Err(format!(
            "resource module `{}` is already registered",
            names.module
        ));
    }

    let import = format!(
        "use crate::domain::{}::aggregate::{}Aggregate;\nuse crate::domain::{}::projector::{{{}Projector, {}_VIEW}};",
        names.module, names.type_name, names.module, names.type_name, names.constant
    );
    let registration = format!(
        "        .register_aggregate::<{}Aggregate>()\n        .register_projector({}Projector, {}_VIEW)",
        names.type_name, names.type_name, names.constant
    );
    let updated_domain = domain.replace(DOMAIN_MARKER, &format!("{DOMAIN_MARKER}\n{module_line}"));
    let updated_main = main
        .replace(IMPORT_MARKER, &format!("{IMPORT_MARKER}\n{import}"))
        .replace(
            REGISTRATION_MARKER,
            &format!("{registration}\n{REGISTRATION_MARKER}"),
        );

    for file in &files {
        let path = resource.root.join(&file.relative);
        let parent = path.parent().expect("generated files always have a parent");
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create `{}`: {error}", parent.display()))?;
        fs::write(&path, &file.contents)
            .map_err(|error| format!("could not write `{}`: {error}", path.display()))?;
    }
    fs::write(&domain_path, updated_domain)
        .map_err(|error| format!("could not update `{}`: {error}", domain_path.display()))?;
    fs::write(&main_path, updated_main)
        .map_err(|error| format!("could not update `{}`: {error}", main_path.display()))?;
    if resource.api {
        register_api_route(&resource.root, &names)?;
    }

    Ok(CreatedResource {
        type_name: names.type_name,
        files: files.into_iter().map(|file| file.relative).collect(),
        api_path: resource.api.then(|| format!("/api/{}", names.view)),
    })
}

fn register_api_route(root: &Path, names: &Names) -> Result<(), String> {
    let path = root.join("src/routes.rs");
    let mut contents = read_file(&path)?;
    if !contents.contains(API_ROUTE_MARKER) {
        let closing = contents.rfind('}').ok_or_else(|| {
            format!(
                "`{}` has no route configuration closing brace",
                path.display()
            )
        })?;
        contents.insert_str(closing, &format!("    {API_ROUTE_MARKER}\n"));
    }
    let registration = format!("    crate::domain::{}::api::config(cfg);", names.module);
    if contents
        .lines()
        .any(|line| line.trim() == registration.trim())
    {
        return Err(format!(
            "resource API `{}` is already registered",
            names.module
        ));
    }
    contents = contents.replace(
        API_ROUTE_MARKER,
        &format!("{registration}\n{API_ROUTE_MARKER}"),
    );
    fs::write(&path, contents)
        .map_err(|error| format!("could not update `{}`: {error}", path.display()))
}

fn validate_project(root: &Path) -> Result<(), String> {
    for relative in ["Cargo.toml", "src/main.rs", "src/domain.rs", "migrations"] {
        let path = root.join(relative);
        if !path.exists() {
            return Err(format!(
                "`{}` is not an Arc application root (missing `{relative}`)",
                root.display()
            ));
        }
    }
    Ok(())
}

fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("could not read `{}`: {error}", path.display()))
}

fn read_domain(path: &Path) -> Result<String, String> {
    let contents = read_file(path)?;
    if contents.contains(DOMAIN_MARKER) {
        return Ok(contents);
    }
    Ok(format!("{contents}\n\n{DOMAIN_MARKER}\n"))
}

fn read_main(path: &Path) -> Result<String, String> {
    let mut contents = read_file(path)?;
    if !contents.contains(IMPORT_MARKER) {
        let anchor = "use crate::domain::AppAggregate;";
        if !contents.contains(anchor) {
            return Err(format!(
                "`{}` has no Arc resource import marker or generated AppAggregate import",
                path.display()
            ));
        }
        contents = contents.replace(anchor, &format!("{anchor}\n{IMPORT_MARKER}"));
    }
    if !contents.contains(REGISTRATION_MARKER) {
        let anchor = "        .register_aggregate::<AppAggregate>()";
        if !contents.contains(anchor) {
            return Err(format!(
                "`{}` has no Arc resource registration marker or generated AppAggregate registration",
                path.display()
            ));
        }
        contents = contents.replace(anchor, &format!("{anchor}\n{REGISTRATION_MARKER}"));
    }
    Ok(contents)
}

fn parse_name(input: &str) -> Result<Names, String> {
    if input.is_empty()
        || !input
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(
            "resource name must contain only ASCII letters, numbers, hyphens, or underscores"
                .to_string(),
        );
    }

    let mut words = Vec::<String>::new();
    let mut current = String::new();
    for character in input.chars() {
        if character == '-' || character == '_' {
            if !current.is_empty() {
                words.push(current);
                current = String::new();
            }
        } else if character.is_ascii_uppercase()
            && current
                .chars()
                .last()
                .is_some_and(|last| last.is_ascii_lowercase())
        {
            words.push(current);
            current = character.to_ascii_lowercase().to_string();
        } else {
            current.push(character.to_ascii_lowercase());
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty()
        || !words[0]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
    {
        return Err("resource name must start with an ASCII letter".to_string());
    }

    let module = words.join("_");
    if is_rust_keyword(&module) {
        return Err(format!(
            "resource name `{input}` is a reserved Rust keyword"
        ));
    }
    let type_name = words
        .iter()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect::<String>();

    Ok(Names {
        type_name,
        constant: pluralize(&module).to_ascii_uppercase(),
        view: pluralize(&module),
        module,
    })
}

fn pluralize(name: &str) -> String {
    if name.ends_with('y')
        && name
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|character| !matches!(character, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        format!("{}ies", &name[..name.len() - 1])
    } else if name.ends_with('s')
        || name.ends_with('x')
        || name.ends_with('z')
        || name.ends_with("ch")
        || name.ends_with("sh")
    {
        format!("{name}es")
    } else {
        format!("{name}s")
    }
}

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

fn next_migration_version(migrations: &Path) -> Result<String, String> {
    let entries = fs::read_dir(migrations)
        .map_err(|error| format!("could not read `{}`: {error}", migrations.display()))?;
    let max = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| name.split('_').next()?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    Ok(format!("{:014}", max + 1))
}

fn generated_files(names: &Names, migration_version: String, api: bool) -> Vec<GeneratedFile> {
    let base = PathBuf::from(format!("src/domain/{}", names.module));
    let migration = PathBuf::from(format!(
        "migrations/{}_{}_view",
        migration_version, names.view
    ));
    let replacements = |template: &str| {
        template
            .replace("{{Type}}", &names.type_name)
            .replace("{{module}}", &names.module)
            .replace("{{view}}", &names.view)
            .replace("{{CONSTANT}}", &names.constant)
            .replace("{{api-module}}", if api { "pub mod api;" } else { "" })
    };
    let mut templates = vec![
        (
            base.join("mod.rs"),
            include_str!("../templates/resource/mod.rs.tpl"),
        ),
        (
            base.join("commands.rs"),
            include_str!("../templates/resource/commands.rs.tpl"),
        ),
        (
            base.join("events.rs"),
            include_str!("../templates/resource/events.rs.tpl"),
        ),
        (
            base.join("aggregate.rs"),
            include_str!("../templates/resource/aggregate.rs.tpl"),
        ),
        (
            base.join("projector.rs"),
            include_str!("../templates/resource/projector.rs.tpl"),
        ),
        (
            migration.join("up.sql"),
            include_str!("../templates/resource/up.sql.tpl"),
        ),
        (
            migration.join("down.sql"),
            include_str!("../templates/resource/down.sql.tpl"),
        ),
    ];
    if api {
        templates.push((
            base.join("api.rs"),
            include_str!("../templates/resource/api.rs.tpl"),
        ));
    }
    templates
        .into_iter()
        .map(|(relative, template)| GeneratedFile {
            relative,
            contents: replacements(template),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::{create_project, NewProject};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("arc-resource-test-{}-{nonce}", std::process::id()))
    }

    fn project(destination: &Path) -> PathBuf {
        create_project(&NewProject {
            name: "catalog".to_string(),
            destination: destination.to_path_buf(),
            ui: false,
            git: false,
        })
        .unwrap()
    }

    #[test]
    fn generates_and_registers_complete_resource() {
        let destination = temp_root();
        let root = project(&destination);
        let created = create_resource(&NewResource {
            name: "OrderItem".to_string(),
            root: root.clone(),
            api: true,
        })
        .unwrap();

        assert_eq!(created.type_name, "OrderItem");
        assert_eq!(created.files.len(), 8);
        assert!(root.join("src/domain/order_item/aggregate.rs").is_file());
        assert!(root.join("src/domain/order_item/api.rs").is_file());
        assert!(root
            .join("migrations/00000000000001_order_items_view/up.sql")
            .is_file());
        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(main.contains("register_aggregate::<OrderItemAggregate>()"));
        assert!(main.contains("register_projector(OrderItemProjector, ORDER_ITEMS_VIEW)"));
        let domain = fs::read_to_string(root.join("src/domain.rs")).unwrap();
        assert!(domain.contains("pub mod order_item;"));
        let routes = fs::read_to_string(root.join("src/routes.rs")).unwrap();
        assert!(routes.contains("crate::domain::order_item::api::config(cfg);"));
        fs::remove_dir_all(destination).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_an_existing_resource() {
        let destination = temp_root();
        let root = project(&destination);
        let resource = NewResource {
            name: "Product".to_string(),
            root: root.clone(),
            api: false,
        };
        create_resource(&resource).unwrap();
        let before = fs::read_to_string(root.join("src/main.rs")).unwrap();
        assert!(create_resource(&resource)
            .unwrap_err()
            .contains("will not overwrite"));
        assert_eq!(
            before,
            fs::read_to_string(root.join("src/main.rs")).unwrap()
        );
        fs::remove_dir_all(destination).unwrap();
    }

    #[test]
    fn upgrades_a_pre_marker_generated_application() {
        let destination = temp_root();
        let root = project(&destination);
        let domain_path = root.join("src/domain.rs");
        let main_path = root.join("src/main.rs");
        let routes_path = root.join("src/routes.rs");
        fs::write(
            &domain_path,
            fs::read_to_string(&domain_path)
                .unwrap()
                .replace(DOMAIN_MARKER, ""),
        )
        .unwrap();
        fs::write(
            &routes_path,
            fs::read_to_string(&routes_path)
                .unwrap()
                .replace(API_ROUTE_MARKER, ""),
        )
        .unwrap();
        fs::write(
            &main_path,
            fs::read_to_string(&main_path)
                .unwrap()
                .replace(IMPORT_MARKER, "")
                .replace(REGISTRATION_MARKER, ""),
        )
        .unwrap();

        create_resource(&NewResource {
            name: "Product".to_string(),
            root: root.clone(),
            api: true,
        })
        .unwrap();

        let main = fs::read_to_string(main_path).unwrap();
        assert!(main.contains(IMPORT_MARKER));
        assert!(main.contains(REGISTRATION_MARKER));
        assert!(main.contains("register_aggregate::<ProductAggregate>()"));
        assert!(fs::read_to_string(routes_path)
            .unwrap()
            .contains("crate::domain::product::api::config(cfg);"));
        fs::remove_dir_all(destination).unwrap();
    }

    #[test]
    fn rejects_invalid_names_and_non_arc_directories() {
        assert!(parse_name("3products").is_err());
        assert!(parse_name("type").is_err());
        assert!(create_resource(&NewResource {
            name: "Product".to_string(),
            root: temp_root(),
            api: false,
        })
        .is_err());
    }
}
