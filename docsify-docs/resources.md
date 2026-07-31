# Build an Event-Sourced Resource

This tutorial turns a fresh generated app into a small Product API:

```text
POST /api/products
GET  /api/products
GET  /api/products/{id}
```

A Product write follows Arc's supported path:

```text
HTTP request → ProductCommand → ProductAggregate → ProductCreated event
             → event store → ProductProjector → products_view
```

The command side is authoritative. The projection is the query model.

## 1. Create the app

```bash
arc new catalog
cd catalog
make setup
```

The generated `AppAggregate` is a placeholder. This tutorial replaces it with `ProductAggregate`.

## 2. Define the Product aggregate

Create `src/product/mod.rs`:

```rust
use arc_core::aggregate::{Aggregate, Command};
use arc_core::event::{Event, NewEvent};
use async_trait::async_trait;
use serde_json::json;
use thiserror::Error;

pub mod projector;

#[derive(Debug)]
pub enum ProductCommand {
    Create { id: String, name: String },
}

impl Command for ProductCommand {
    fn aggregate_id(&self) -> &str {
        match self {
            Self::Create { id, .. } => id,
        }
    }
}

#[derive(Default)]
pub struct ProductAggregate {
    id: Option<String>,
    name: Option<String>,
    version: i64,
}

#[derive(Debug, Error)]
pub enum ProductError {
    #[error("product already exists")]
    AlreadyExists,
}

#[async_trait]
impl Aggregate for ProductAggregate {
    type Command = ProductCommand;
    type Event = ();
    type Error = ProductError;

    fn aggregate_type() -> &'static str {
        "Product"
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn handle(
        &self,
        command: Self::Command,
    ) -> Result<Vec<Event>, Self::Error> {
        match command {
            ProductCommand::Create { id, name } => {
                if self.id.is_some() {
                    return Err(ProductError::AlreadyExists);
                }

                Ok(vec![Event::new(NewEvent {
                    aggregate_type: Self::aggregate_type(),
                    aggregate_id: id.clone(),
                    sequence: self.version + 1,
                    event_type: "ProductCreated",
                    payload: json!({ "id": id, "name": name }),
                })])
            }
        }
    }

    fn apply(&mut self, event: &Event) {
        if event.event_type == "ProductCreated" {
            self.id = event.payload["id"].as_str().map(str::to_owned);
            self.name = event.payload["name"].as_str().map(str::to_owned);
            self.version = event.sequence;
        }
    }
}
```

`handle` validates intent and produces events. `apply` reconstructs state from stored events. Neither function writes to a database.

## 3. Build the read model

Create `src/product/projector.rs`:

```rust
use arc_core::event::Event;
use arc_core::projection::{
    ProjectionError, ProjectionResult, Projector,
};
use arc_core::read_model_store::{ReadModelStore, Upsert};
use async_trait::async_trait;
use serde_json::json;

pub const PRODUCTS_VIEW: &str = "products_view";

pub struct ProductProjector;

#[async_trait]
impl Projector for ProductProjector {
    fn name(&self) -> &str {
        "ProductProjector"
    }

    fn handles(&self) -> Vec<String> {
        vec!["ProductCreated".to_string()]
    }

    async fn apply(
        &self,
        event: &Event,
        store: &dyn ReadModelStore,
    ) -> ProjectionResult<()> {
        let row = json!({
            "id": event.aggregate_id.clone(),
            "name": event.payload["name"].clone(),
            "version": event.sequence,
        });

        store
            .upsert(Upsert::new(
                PRODUCTS_VIEW,
                &event.aggregate_id,
                row,
            ))
            .await
            .map_err(|error| ProjectionError::handle_failed(
                self.name(),
                &event.event_type,
                event.event_id.to_string(),
                error.to_string(),
            ))
    }
}
```

Every projected row must contain an integer `version`. Arc uses it to make replayed upserts idempotent.

## 4. Add the read-model migration

Create:

```text
migrations/00000000000001_products_view/
├── up.sql
└── down.sql
```

`up.sql`:

```sql
CREATE TABLE products_view (
    id TEXT PRIMARY KEY NOT NULL,
    version BIGINT NOT NULL,
    data TEXT NOT NULL
);
```

`down.sql`:

```sql
DROP TABLE products_view;
```

Apply it:

```bash
make migrate
```

Arc's SQLite read-model store expects the standard `id`, `version`, and JSON `data` columns.

## 5. Register the aggregate and projector

In `src/main.rs`, replace:

```rust
use crate::domain::AppAggregate;
mod domain;
```

with:

```rust
use crate::product::projector::{
    ProductProjector, PRODUCTS_VIEW,
};
use crate::product::ProductAggregate;

mod product;
```

Then change the builder in `serve`:

```rust
ArcApp::builder()
    .register_aggregate::<ProductAggregate>()
    .register_projector(ProductProjector, PRODUCTS_VIEW)
    .register_routes(routes::config)
    .serve(host.clone(), port)
    .await
    .with_context(|| {
        format!(
            "could not start on {host}:{port}; \
             the port may be busy (change APP_PORT in .env)"
        )
    })
```

You can now remove the unused generated `src/domain.rs`.

## 6. Add Product handlers

Replace `src/routes.rs` with:

> **Note:** [`web::Data`](project-structure.md#actix-shared-application-data) gives these routes access to the command bus and read-model store that Arc created when the server started.

```rust
use actix_web::{
    get, post, web, HttpResponse, Responder,
};
use arc_core::command_bus::{CommandBus, CommandContext};
use arc_core::read_model_store::ReadModelStore;
use serde::Deserialize;
use serde_json::json;

use crate::product::projector::PRODUCTS_VIEW;
use crate::product::{ProductAggregate, ProductCommand};

#[derive(Deserialize)]
struct CreateProduct {
    id: String,
    name: String,
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(json!({ "status": "healthy" }))
}

#[post("/api/products")]
async fn create_product(
    body: web::Json<CreateProduct>,
    bus: web::Data<CommandBus<ProductAggregate>>,
) -> impl Responder {
    let command = ProductCommand::Create {
        id: body.id.clone(),
        name: body.name.clone(),
    };

    match bus
        .dispatch(command, CommandContext::for_actor("anonymous"))
        .await
    {
        Ok(_) => HttpResponse::Created().json(json!({
            "id": body.id.clone()
        })),
        Err(error) => HttpResponse::BadRequest().json(json!({
            "error": error.to_string()
        })),
    }
}

#[get("/api/products")]
async fn list_products(
    store: web::Data<dyn ReadModelStore>,
) -> impl Responder {
    match store.list(PRODUCTS_VIEW).await {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(error) => HttpResponse::InternalServerError().json(json!({
            "error": error.to_string()
        })),
    }
}

#[get("/api/products/{id}")]
async fn get_product(
    id: web::Path<String>,
    store: web::Data<dyn ReadModelStore>,
) -> impl Responder {
    match store.get(PRODUCTS_VIEW, id.as_str()).await {
        Ok(Some(product)) => HttpResponse::Ok().json(product),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(error) => HttpResponse::InternalServerError().json(json!({
            "error": error.to_string()
        })),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .service(create_product)
        .service(list_products)
        .service(get_product);
}
```

For a real public API, derive the actor from authentication rather than using `"anonymous"`. Authentication is not generated today.

## 7. Run and verify

```bash
make check
make dev
```

In another terminal:

```bash
curl --fail \
  --request POST \
  --header 'content-type: application/json' \
  --data '{"id":"product-1","name":"Notebook"}' \
  http://127.0.0.1:8080/api/products
```

Then query the projection:

```bash
curl --fail http://127.0.0.1:8080/api/products
curl --fail http://127.0.0.1:8080/api/products/product-1
```

Because the default event bus is `inprocess`, the Product projection is updated before the POST returns.

## Extending the resource

To add rename or delete behavior:

1. Add variants to `ProductCommand`.
2. Validate them in `ProductAggregate::handle`.
3. produce past-tense events such as `ProductRenamed`.
4. Apply those events in `ProductAggregate::apply`.
5. Add them to `ProductProjector::handles`.
6. Update or delete the read-model row in the projector.
7. Expose handlers that dispatch the new commands.

Never update `products_view` directly from a write endpoint; it is a projection, not the source of truth.
