use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::io::Error;
use tera::{Context, Tera};
use tracing::error;

/// Cached Tera instance - compiled once at startup
static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    // Try workspace path first, fallback to direct path
    let patterns = vec![
        "crates/arc-app/src/resources/views/**/*", // Workspace structure
        "src/resources/views/**/*",                // Direct run
    ];

    for pattern in patterns {
        // A glob matching no files still returns Ok with an empty Tera, so only
        // accept a pattern that actually loaded at least one template.
        match Tera::new(pattern) {
            Ok(t) if t.get_template_names().next().is_some() => return t,
            _ => continue,
        }
    }

    error!("Fatal error: Could not find templates in any expected location");
    ::std::process::exit(1);
});

/// Cached manifest assets - parsed once at startup
static MANIFEST_ASSETS: Lazy<HashMap<String, String>> = Lazy::new(parse_manifest_assets);

/// Renders a Tera template with the given parameters and optional asset list.
/// Automatically injects `session_message` (empty if not provided) and `assets`.
pub fn load_template(
    template: &str,
    params: Vec<(&str, &str)>,
    assets: Option<Vec<&str>>,
) -> String {
    let mut context: Context = Context::new();
    for (key, value) in params.into_iter() {
        context.insert(key, value);
    }

    if !context.contains_key("session_message") {
        context.insert("session_message", "");
    }

    context.insert("assets", &get_assets_string(assets));

    TEMPLATES
        .render(template, &context)
        .expect("Failed to render template")
}

/// Returns the HTML string to add the assets to the template.
/// If the assets are passed, we only add the assets passed, otherwise we add all the assets from
/// the manifest.json file.
fn get_assets_string(assets: Option<Vec<&str>>) -> String {
    let mut assets_string: String = String::new();
    if let Some(assets) = assets {
        for value in assets.into_iter() {
            let asset_type = value.split('.').next_back().unwrap();
            if let Some(asset) = MANIFEST_ASSETS.get(value) {
                if asset_type == "css" {
                    assets_string.push_str(&format!(
                        "<link rel=\"stylesheet\" href=\"/public/{}\">",
                        asset
                    ));
                } else if asset_type == "js" {
                    assets_string.push_str(&format!(
                        "<script src=\"/public/{}\" defer></script>",
                        asset
                    ));
                }
            }
        }
    } else {
        for value in MANIFEST_ASSETS.values() {
            let asset_type = value.split('.').next_back().unwrap();
            if asset_type == "css" {
                assets_string.push_str(&format!(
                    "<link rel=\"stylesheet\" href=\"/public/{}\">",
                    value
                ));
            } else if asset_type == "js" {
                assets_string.push_str(&format!(
                    "<script src=\"/public/{}\" defer></script>",
                    value
                ));
            }
        }
    }

    assets_string
}

/// Parse the assets from the manifest.json file (called once at startup)
fn parse_manifest_assets() -> HashMap<String, String> {
    let mut assets: HashMap<String, String> = HashMap::new();
    let manifest: Result<String, Error> = fs::read_to_string("dist/.vite/manifest.json");
    if let Ok(manifest) = manifest {
        let manifest_json: serde_json::Value =
            serde_json::from_str(&manifest).expect("Failed to parse manifest.json");

        for (key, value) in manifest_json.as_object().unwrap().iter() {
            if let Some(asset) = value.get("file") {
                assets.insert(key.to_string(), asset.as_str().unwrap().parse().unwrap());

                // If the asset is a js file, we might add css files to the assets.
                let asset_type = asset.as_str().unwrap().split('.').next_back().unwrap();
                if asset_type == "js" {
                    if let Some(css_array) = value.get("css").and_then(|v| v.as_array()) {
                        for css_file in css_array {
                            let css_file_name =
                                css_file.as_str().unwrap().split('/').next_back().unwrap();
                            assets.insert(css_file_name.to_string(), css_file_name.to_string());
                        }
                    }
                }
            }
        }
    }

    assets
}
