# Plugin System Plan for Nineties

## Objective

Make Nineties "hookable" so features like dynamic pages can be implemented as decoupled plugins rather than core code.

---

## Evaluation: `the-hook` Library

### What It Does

`the-hook` provides a priority-based filter system:

```rust
use rust_filters::{add_filter, apply_filters};

add_filter("modify_content", 10, |v: String| format!("<p>{}</p>", v));
let result = apply_filters("modify_content", "Hello".to_string());
```

### Pros

- Simple API (`add_filter`, `apply_filters`, `remove_filter`)
- Priority ordering (lower = runs first)
- WordPress-style familiarity
- Your library, so you can extend it

### Cons / Concerns

| Concern | Impact | Mitigation |
|---------|--------|------------|
| **Mutex overhead** | Every `apply_filters` acquires a lock | Acceptable for web apps (not called 1000s/sec per request) |
| **Single type per hook** | Each hook works with one type `T` | Design hooks with specific types or use `serde_json::Value` |
| **No async support** | Filters are synchronous | Fine for transforms; DB/IO should happen outside filters |
| **Global state** | Static storage | Acceptable for plugin system; matches WordPress model |

### Verdict

**Good fit for Nineties.** The mutex overhead is negligible compared to template rendering and DB queries. The simple API makes plugin development easy.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Nineties Core                          │
├─────────────────────────────────────────────────────────────┤
│  Hooks System (the-hook)                                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  • routes:register                                   │   │
│  │  • admin:menu_items                                  │   │
│  │  • template:before_render                            │   │
│  │  • content:transform                                 │   │
│  │  • migrations:register                               │   │
│  └─────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  Plugin Loader                                              │
│  - Discovers plugins in /plugins directory                  │
│  - Calls plugin.register() on startup                       │
└─────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│                       Plugins                               │
├──────────────────┬──────────────────┬───────────────────────┤
│  pages-plugin    │  blog-plugin     │  seo-plugin           │
│  - adds /page/*  │  - adds /blog/*  │  - adds meta tags     │
│  - admin UI      │  - admin UI      │  - sitemap            │
└──────────────────┴──────────────────┴───────────────────────┘
```

---

## Implementation Plan

### Phase 1: Core Hook System

#### 1.1 Add Dependency

```toml
# Cargo.toml
[dependencies]
rust-filters = { git = "https://github.com/lotharthesavior/the-hook" }
```

#### 1.2 Create Hook Points Module

Create `src/hooks/mod.rs`:

```rust
pub mod points;
pub mod types;

use rust_filters::{add_filter, apply_filters};

// Re-export for convenience
pub use rust_filters::{add_filter, apply_filters, remove_filter, remove_all_filters};
```

Create `src/hooks/points.rs`:

```rust
//! Hook point constants

// Route registration
pub const ROUTES_REGISTER: &str = "routes:register";

// Admin menu
pub const ADMIN_MENU_ITEMS: &str = "admin:menu_items";

// Template hooks
pub const TEMPLATE_BEFORE_RENDER: &str = "template:before_render";
pub const TEMPLATE_HEAD: &str = "template:head";
pub const TEMPLATE_FOOTER: &str = "template:footer";

// Content transformation
pub const CONTENT_TRANSFORM: &str = "content:transform";

// Migrations
pub const MIGRATIONS_REGISTER: &str = "migrations:register";

// Startup/shutdown
pub const APP_INIT: &str = "app:init";
pub const APP_SHUTDOWN: &str = "app:shutdown";
```

Create `src/hooks/types.rs`:

```rust
use actix_web::web::ServiceConfig;
use serde::{Deserialize, Serialize};

/// Route registration context
pub struct RouteContext<'a> {
    pub config: &'a mut ServiceConfig,
}

/// Admin menu item
#[derive(Clone, Serialize, Deserialize)]
pub struct AdminMenuItem {
    pub label: String,
    pub path: String,
    pub icon: Option<String>,
    pub priority: i32,
}

/// Template context for hooks
#[derive(Clone)]
pub struct TemplateContext {
    pub template_name: String,
    pub params: Vec<(String, String)>,
    pub extra_head: String,
    pub extra_footer: String,
}

/// Migration info
#[derive(Clone)]
pub struct MigrationInfo {
    pub name: String,
    pub version: i32,
    pub up_sql: String,
    pub down_sql: String,
}
```

#### 1.3 Integrate Hooks into Routes

Update `src/routes.rs`:

```rust
use crate::hooks::{apply_filters, points};
use crate::hooks::types::AdminMenuItem;

pub fn config(cfg: &mut web::ServiceConfig) {
    // Core routes
    cfg
        .service(home_controller::home)
        .service(auth_controller::signin)
        // ... other core routes

    // Let plugins register routes
    let additional_routes: Vec<Box<dyn Fn(&mut web::ServiceConfig)>> =
        apply_filters(points::ROUTES_REGISTER, vec![]);

    for route_fn in additional_routes {
        route_fn(cfg);
    }
}

pub fn get_admin_menu_items() -> Vec<AdminMenuItem> {
    let core_items = vec![
        AdminMenuItem {
            label: "Dashboard".into(),
            path: "/admin".into(),
            icon: Some("home".into()),
            priority: 0,
        },
        AdminMenuItem {
            label: "Settings".into(),
            path: "/admin/settings".into(),
            icon: Some("cog".into()),
            priority: 100,
        },
    ];

    let mut items = apply_filters(points::ADMIN_MENU_ITEMS, core_items);
    items.sort_by_key(|i| i.priority);
    items
}
```

#### 1.4 Integrate Hooks into Templates

Update `src/helpers/template.rs`:

```rust
use crate::hooks::{apply_filters, points};
use crate::hooks::types::TemplateContext;

pub fn load_template(template: &str, params: Vec<(&str, &str)>, assets: Option<Vec<&str>>) -> String {
    let mut context = TemplateContext {
        template_name: template.to_string(),
        params: params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        extra_head: String::new(),
        extra_footer: String::new(),
    };

    // Allow plugins to modify template context
    context = apply_filters(points::TEMPLATE_BEFORE_RENDER, context);

    let mut tera_context = Context::new();
    for (key, value) in &context.params {
        tera_context.insert(key, value);
    }

    // Add plugin-injected head/footer content
    tera_context.insert("plugin_head", &context.extra_head);
    tera_context.insert("plugin_footer", &context.extra_footer);

    // ... rest of template rendering
}
```

---

### Phase 2: Plugin Loader

#### 2.1 Plugin Trait

Create `src/plugins/mod.rs`:

```rust
pub mod loader;

use actix_web::web::ServiceConfig;

/// Trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier
    fn name(&self) -> &'static str;

    /// Plugin version
    fn version(&self) -> &'static str;

    /// Called once at startup to register hooks
    fn register(&self);

    /// Optional: Register routes directly
    fn register_routes(&self, _cfg: &mut ServiceConfig) {}

    /// Optional: Run migrations
    fn migrations(&self) -> Vec<MigrationInfo> {
        vec![]
    }
}

// Plugin registry
use std::sync::RwLock;
use once_cell::sync::Lazy;

static PLUGINS: Lazy<RwLock<Vec<Box<dyn Plugin>>>> = Lazy::new(|| RwLock::new(vec![]));

pub fn register_plugin(plugin: Box<dyn Plugin>) {
    let mut plugins = PLUGINS.write().unwrap();
    plugin.register();
    plugins.push(plugin);
}

pub fn get_plugins() -> Vec<String> {
    PLUGINS.read().unwrap().iter().map(|p| p.name().to_string()).collect()
}
```

#### 2.2 Plugin Loader

Create `src/plugins/loader.rs`:

```rust
use super::{register_plugin, Plugin};

/// Load all enabled plugins
pub fn load_plugins() {
    // Built-in plugins (compiled in)
    #[cfg(feature = "plugin-pages")]
    register_plugin(Box::new(crate::plugins::pages::PagesPlugin));

    #[cfg(feature = "plugin-blog")]
    register_plugin(Box::new(crate::plugins::blog::BlogPlugin));

    // Future: Dynamic plugin loading from /plugins directory
    // load_dynamic_plugins();

    println!("Loaded plugins: {:?}", super::get_plugins());
}
```

#### 2.3 Initialize in Main

Update `src/main.rs`:

```rust
mod plugins;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    // Load plugins before starting server
    plugins::loader::load_plugins();

    HttpServer::new(move || {
        App::new()
            // ... existing setup
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
```

---

### Phase 3: Pages Plugin Example

#### 3.1 Plugin Structure

```
src/plugins/
├── mod.rs
├── loader.rs
└── pages/
    ├── mod.rs
    ├── controllers.rs
    ├── models.rs
    └── migrations.rs
```

#### 3.2 Pages Plugin Implementation

Create `src/plugins/pages/mod.rs`:

```rust
mod controllers;
mod models;

use crate::hooks::{add_filter, points};
use crate::hooks::types::AdminMenuItem;
use crate::plugins::Plugin;
use actix_web::web::ServiceConfig;

pub struct PagesPlugin;

impl Plugin for PagesPlugin {
    fn name(&self) -> &'static str {
        "pages"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn register(&self) {
        // Register admin menu item
        add_filter(points::ADMIN_MENU_ITEMS, 10, |mut items: Vec<AdminMenuItem>| {
            items.push(AdminMenuItem {
                label: "Pages".into(),
                path: "/admin/pages".into(),
                icon: Some("document".into()),
                priority: 20,
            });
            items
        });

        println!("Pages plugin registered");
    }

    fn register_routes(&self, cfg: &mut ServiceConfig) {
        cfg
            .service(controllers::show)
            .service(controllers::admin_list)
            .service(controllers::admin_edit)
            .service(controllers::admin_update);
    }
}
```

Create `src/plugins/pages/controllers.rs`:

```rust
use actix_web::{get, post, web, HttpResponse, Responder};
use actix_session::Session;
// ... controller implementations from creating-dynamic-page.md
```

#### 3.3 Feature Flag

Update `Cargo.toml`:

```toml
[features]
default = ["plugin-pages"]
plugin-pages = []
plugin-blog = []
```

---

### Phase 4: Advanced Hook Types

#### 4.1 Action Hooks (No Return Value)

Extend `the-hook` or create wrapper:

```rust
use std::sync::Mutex;
use once_cell::sync::Lazy;

type ActionCallback = Box<dyn Fn() + Send + Sync>;

static ACTIONS: Lazy<Mutex<HashMap<String, Vec<ActionCallback>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn add_action(hook: &str, callback: impl Fn() + Send + Sync + 'static) {
    let mut actions = ACTIONS.lock().unwrap();
    actions.entry(hook.to_string())
        .or_insert_with(Vec::new)
        .push(Box::new(callback));
}

pub fn do_action(hook: &str) {
    let actions = ACTIONS.lock().unwrap();
    if let Some(callbacks) = actions.get(hook) {
        for callback in callbacks {
            callback();
        }
    }
}
```

#### 4.2 Async Hooks (Future Consideration)

```rust
// For async operations, consider:
pub async fn apply_filters_async<T>(hook: &str, value: T) -> T
where
    T: Send + 'static
{
    // Run sync filters in blocking task
    tokio::task::spawn_blocking(move || {
        apply_filters(hook, value)
    }).await.unwrap()
}
```

---

## Hook Reference

| Hook | Type | Input | Output | Purpose |
|------|------|-------|--------|---------|
| `routes:register` | Filter | `Vec<RouteFn>` | `Vec<RouteFn>` | Add routes |
| `admin:menu_items` | Filter | `Vec<AdminMenuItem>` | `Vec<AdminMenuItem>` | Add admin menu items |
| `template:before_render` | Filter | `TemplateContext` | `TemplateContext` | Modify template data |
| `template:head` | Filter | `String` | `String` | Inject into `<head>` |
| `template:footer` | Filter | `String` | `String` | Inject before `</body>` |
| `content:transform` | Filter | `String` | `String` | Transform content (markdown, etc.) |
| `app:init` | Action | - | - | Run on startup |
| `app:shutdown` | Action | - | - | Run on shutdown |

---

## Performance Considerations

| Operation | Cost | Frequency | Impact |
|-----------|------|-----------|--------|
| `add_filter` | Mutex lock + insert | Once at startup | Negligible |
| `apply_filters` | Mutex lock + N callbacks | Per request | Low (~1-5μs per filter) |
| Template render | ~1-5ms | Per request | Dominant cost |
| DB query | ~5-20ms | Per request | Dominant cost |

**Conclusion**: Hook overhead is 0.01-0.1% of request time. Not a concern.

---

## Migration Path

1. **Phase 1** (Week 1): Add hook system, no plugins yet
2. **Phase 2** (Week 2): Create plugin trait and loader
3. **Phase 3** (Week 3): Extract pages feature as plugin
4. **Phase 4** (Week 4): Document plugin API, create plugin template

---

## Directory Structure (Final)

```
nineties/
├── src/
│   ├── main.rs
│   ├── routes.rs
│   ├── hooks/
│   │   ├── mod.rs
│   │   ├── points.rs
│   │   └── types.rs
│   ├── plugins/
│   │   ├── mod.rs
│   │   ├── loader.rs
│   │   └── pages/
│   │       ├── mod.rs
│   │       ├── controllers.rs
│   │       └── models.rs
│   ├── http/
│   ├── helpers/
│   └── models/
└── plugins/           # Future: external plugins
    └── my-plugin/
        └── Cargo.toml
```

---

## Summary

**`the-hook` is a good choice** for Nineties because:

1. Simple, familiar API (WordPress-style)
2. Negligible performance overhead
3. You own the library (can extend if needed)
4. Fits the synchronous nature of most web operations

The plugin system enables:
- Decoupled features (pages, blog, SEO as plugins)
- Easy enable/disable via feature flags
- Clear extension points for third-party plugins
- Future dynamic plugin loading
