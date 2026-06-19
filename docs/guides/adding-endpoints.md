# Guide — Adding new endpoints

This guide shows how to add an HTTP endpoint to Arc's current Actix Web app. It covers the
mechanics that apply to public HTML pages, protected admin pages, JWT API endpoints, and internal
service endpoints.

## Request path

Most endpoint work touches these files:

| Concern | Location |
|---|---|
| Handler function | `crates/arc-app/src/http/controllers/<name>_controller.rs` |
| Route registration | `crates/arc-app/src/routes.rs` |
| Module registration | `crates/arc-app/src/main.rs` |
| Server app data | `crates/arc-app/src/commands/serve.rs` |
| Templates | `crates/arc-app/src/resources/views/` |
| Domain writes | `CommandBus` + aggregate commands |
| Reads | Projection read models through `ReadModelStore` |

Rule of thumb: controllers coordinate HTTP concerns. They should not become the domain model, and
they should not bypass event-sourced writes.

## 1. Choose the endpoint type

Pick the route group first:

| Type | Route shape | Auth model | Response style |
|---|---|---|---|
| Public HTML | `/...` | None or session-aware | Tera-rendered HTML |
| Admin HTML | `/admin/...` | Session auth + idle timeout | Tera or redirect |
| Public API | `/api/v1/...` | Usually none for login/register | JSON |
| Protected API | `/api/v1/protected/...` | JWT middleware | JSON |
| Internal service | `/internal/...` | Dedicated bearer token or equivalent | JSON / 204 |

Do not put browser session routes under `/api`; protected API routes use JWT, while admin/browser
routes use session auth.

## 2. Add a controller

Create or extend a controller under `crates/arc-app/src/http/controllers/`.

Example public API endpoint:

```rust
use actix_web::{get, HttpResponse, Responder};
use serde_json::json;

#[get("/status")]
pub async fn status() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "ok"
    }))
}
```

For JSON request bodies, define a request type next to the handler:

```rust
use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct RenameThingRequest {
    name: String,
}

#[post("/things/{id}/rename")]
pub async fn rename_thing(
    path: web::Path<String>,
    body: web::Json<RenameThingRequest>,
) -> impl Responder {
    let id = path.into_inner();

    HttpResponse::Accepted().json(json!({
        "id": id,
        "name": body.name,
    }))
}
```

## 3. Register the module

Add the controller module in `crates/arc-app/src/main.rs`:

```rust
mod http {
    pub mod controllers {
        pub mod thing_controller;
    }
}
```

If you are adding to an existing controller, skip this step.

## 4. Register the route

Import and attach the handler in `crates/arc-app/src/routes.rs`.

Public API route:

```rust
use crate::http::controllers::thing_controller::rename_thing;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .service(rename_thing)
    );
}
```

Protected JWT API route:

```rust
web::scope("/api/v1")
    .service(
        web::scope("/protected")
            .wrap(JwtMiddleware)
            .service(rename_thing),
    );
```

Admin/session route:

```rust
web::scope("/admin")
    .wrap(AuthMiddleware)
    .wrap(IdleTimeoutMiddleware::from_env())
    .service(admin_controller::dashboard)
    .service(thing_controller::admin_thing_page);
```

Internal service route:

```rust
web::scope("/internal")
    .service(thing_controller::internal_handler);
```

Internal routes must authenticate with a dedicated mechanism. Do not rely on obscurity or Docker
network placement as the only protection.

## 5. Use app data deliberately

Handlers receive shared dependencies through `web::Data<T>`. Existing common dependencies include:

| Dependency | Type |
|---|---|
| User command bus | `web::Data<CommandBus<UserAggregate>>` |
| Read models | `web::Data<dyn ReadModelStore>` |
| Projection engine | `web::Data<ProjectionEngine>` |
| Access logger | `web::Data<dyn AccessLogger>` |
| Session store | `web::Data<dyn SessionStore>` |

Example read-model query:

```rust
use actix_web::{get, web, HttpResponse, Responder};
use arc_core::read_model_store::ReadModelStore;

#[get("/users/{id}")]
pub async fn get_user(
    path: web::Path<String>,
    read_model_store: web::Data<dyn ReadModelStore>,
) -> impl Responder {
    match read_model_store.get("users_view", &path.into_inner()).await {
        Ok(Some(row)) => HttpResponse::Ok().json(row),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(error) => {
            tracing::error!(error = ?error, "read model lookup failed");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

If a new dependency is needed, construct it in `crates/arc-app/src/commands/serve.rs` and add it to
the Actix app with `.app_data(...)`.

## 6. Keep writes event-sourced

For state changes, send a command through `CommandBus` instead of writing directly to tables.

```rust
use crate::domain::user::aggregate::UserAggregate;
use crate::domain::user::commands::UserCommand;
use actix_web::{patch, web, HttpRequest, HttpResponse, Responder};
use arc_core::command_bus::CommandBus;

#[patch("/users/{id}/name")]
pub async fn update_user_name(
    req: HttpRequest,
    path: web::Path<String>,
    command_bus: web::Data<CommandBus<UserAggregate>>,
) -> impl Responder {
    let aggregate_id = path.into_inner();
    let ctx = crate::helpers::audit_context::for_actor(&req, aggregate_id.clone());

    let command = UserCommand::UpdateProfile {
        aggregate_id,
        name: "New name".to_string(),
    };

    match command_bus.dispatch(command, ctx).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(error) => {
            tracing::error!(error = ?error, "command dispatch failed");
            HttpResponse::BadRequest().finish()
        }
    }
}
```

Use the actual aggregate command for your domain. If the command does not exist, add it to the
aggregate rather than mutating a read model from the controller.

## 7. Preserve CSRF on HTML forms

HTML form POSTs must validate and regenerate CSRF tokens. Follow the existing admin/auth controller
patterns instead of accepting raw form posts without CSRF checks.

API clients should use JSON endpoints and JWT auth where appropriate; do not mix browser form CSRF
rules into API endpoints.

## 8. Add focused tests

Controller tests should cover:

- route is mounted at the intended path
- auth behavior (unauthorized, forbidden, or redirect)
- success response shape
- command dispatch or read-model effect
- error response for missing rows or invalid input

Example route smoke test:

```rust
#[actix_web::test]
async fn status_route_returns_ok() {
    let app = actix_web::test::init_service(
        actix_web::App::new().service(status)
    )
    .await;

    let req = actix_web::test::TestRequest::get()
        .uri("/status")
        .to_request();
    let resp = actix_web::test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}
```

For handlers that use app data, build the same `web::Data` dependencies the production route
expects. Use `InMemoryReadModelStore` or the existing test helpers when a full SQLite stack is not
needed.

## Checklist

- [ ] Handler lives in `crates/arc-app/src/http/controllers/`.
- [ ] Module is registered in `crates/arc-app/src/main.rs`.
- [ ] Route is registered in `crates/arc-app/src/routes.rs`.
- [ ] Auth matches the route type: session for admin/browser, JWT for protected API, dedicated token for internal service routes.
- [ ] Writes go through `CommandBus`; reads come from projection read models.
- [ ] HTML form POSTs preserve CSRF behavior.
- [ ] Focused tests cover success, auth, and failure behavior.
