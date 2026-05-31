# Creating a Dynamic Page with Admin-Editable Content

This guide explains how to add a new page with content that can be edited through the admin panel.

---

## Overview

To create a dynamic page, you need:

1. **Database** - Migration + Model for storing content
2. **Backend** - Routes + Controllers for public view and admin editing
3. **Frontend** - Templates for display and admin form

---

## Step 1: Database Migration

Create a migration for the new content table:

```bash
diesel migration generate create_pages
```

Edit `migrations/<timestamp>_create_pages/up.sql`:

```sql
CREATE TABLE pages (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert default page
INSERT INTO pages (slug, title, content) VALUES ('about', 'About Us', '<p>Welcome to our site.</p>');
```

Edit `migrations/<timestamp>_create_pages/down.sql`:

```sql
DROP TABLE pages;
```

Run migration:

```bash
diesel migration run
```

---

## Step 2: Update Schema

After running the migration, Diesel updates `src/schema.rs` automatically. Verify it includes:

```rust
diesel::table! {
    pages (id) {
        id -> Integer,
        slug -> Text,
        title -> Text,
        content -> Text,
        created_at -> Nullable<Text>,
        updated_at -> Nullable<Text>,
    }
}
```

---

## Step 3: Create Model

Add `src/models/page.rs`:

```rust
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::pages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Page {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::pages)]
pub struct NewPage<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub content: &'a str,
}
```

Register in `src/models/mod.rs`:

```rust
pub mod page;
pub mod user;
```

---

## Step 4: Create Public Controller

Add `src/http/controllers/page_controller.rs`:

```rust
use actix_web::{get, web, HttpResponse, Responder};
use diesel::{QueryDsl, RunQueryDsl, ExpressionMethods};
use crate::helpers::database::get_connection;
use crate::helpers::template::load_template;
use crate::models::page::Page;
use crate::schema::pages::dsl::*;

#[get("/page/{page_slug}")]
pub async fn show(path: web::Path<String>) -> impl Responder {
    let page_slug = path.into_inner();

    let page_result = pages
        .filter(slug.eq(&page_slug))
        .first::<Page>(&mut get_connection());

    match page_result {
        Ok(page) => {
            HttpResponse::Ok().body(load_template(
                "pages/show.html",
                vec![
                    ("title", &page.title),
                    ("content", &page.content),
                ],
                None
            ))
        },
        Err(_) => HttpResponse::NotFound().body("Page not found")
    }
}
```

Register in `src/http/controllers/mod.rs`:

```rust
pub mod admin_controller;
pub mod auth_controller;
pub mod home_controller;
pub mod page_controller;
```

---

## Step 5: Create Admin Controller for Pages

Add to `src/http/controllers/admin_controller.rs` or create separate `admin_pages_controller.rs`:

```rust
use actix_web::{get, post, web, HttpResponse, Responder};
use actix_session::Session;
use diesel::{QueryDsl, RunQueryDsl, ExpressionMethods};
use serde::{Deserialize, Serialize};
use crate::helpers::csrf::{get_csrf_token, validate_csrf_token};
use crate::helpers::database::get_connection;
use crate::helpers::session::get_session_user;
use crate::helpers::template::load_template;
use crate::models::page::Page;
use crate::schema::pages::dsl::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct PageForm {
    csrf_token: String,
    title: String,
    content: String,
}

#[get("/pages")]
pub async fn pages_list(session: Session) -> impl Responder {
    let user = get_session_user(&session).unwrap();
    let all_pages = pages
        .load::<Page>(&mut get_connection())
        .expect("Failed to load pages");

    // Convert pages to JSON string for template
    let pages_json = serde_json::to_string(&all_pages).unwrap();

    HttpResponse::Ok().body(load_template(
        "admin/pages/pages-list.html",
        vec![
            ("user_name", &user.name),
            ("pages_json", &pages_json),
        ],
        None
    ))
}

#[get("/pages/{page_id}/edit")]
pub async fn pages_edit(path: web::Path<i32>, session: Session) -> impl Responder {
    let page_id = path.into_inner();
    let user = get_session_user(&session).unwrap();
    let csrf_token = get_csrf_token(&session);

    let page = pages
        .find(page_id)
        .first::<Page>(&mut get_connection())
        .expect("Page not found");

    HttpResponse::Ok().body(load_template(
        "admin/pages/pages-edit.html",
        vec![
            ("user_name", &user.name),
            ("csrf_token", &csrf_token),
            ("page_id", &page.id.to_string()),
            ("page_title", &page.title),
            ("page_content", &page.content),
        ],
        None
    ))
}

#[post("/pages/{page_id}")]
pub async fn pages_update(
    path: web::Path<i32>,
    form: web::Form<PageForm>,
    session: Session
) -> impl Responder {
    let page_id = path.into_inner();

    if !validate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"errors": {"csrf": "Invalid request."}}));
    }

    let result = diesel::update(pages.find(page_id))
        .set((
            title.eq(&form.title),
            content.eq(&form.content),
        ))
        .execute(&mut get_connection());

    match result {
        Ok(_) => HttpResponse::Ok()
            .json(serde_json::json!({"success": true})),
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"errors": {"server": "Failed to update"}}))
    }
}
```

---

## Step 6: Register Routes

Update `src/routes.rs`:

```rust
use crate::http::controllers::{admin_controller, auth_controller, home_controller, page_controller};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(home_controller::home)
        .service(auth_controller::signin)
        .service(auth_controller::signin_post)
        .service(auth_controller::signout)
        // Public pages
        .service(page_controller::show)
        // Admin routes
        .service(
            web::scope("/admin")
                .wrap(AuthMiddleware)
                .service(admin_controller::dashboard)
                .service(admin_controller::settings)
                .service(admin_controller::profile)
                .service(admin_controller::profile_post)
                .service(admin_controller::profile_password_post)
                // Page management
                .service(admin_controller::pages_list)
                .service(admin_controller::pages_edit)
                .service(admin_controller::pages_update)
        )
        .route("/ws", web::get().to(websocket::connection::ws_handler))
        .service(static_file);
}
```

---

## Step 7: Create Templates

### Public Page Template

Create `src/resources/views/pages/show.html`:

```html
{% include "parts/html-head.html" %}

<body data-controller="notification">
    {% include "parts/notification.html" %}
    {% include "parts/header.html" %}

    <main class="mx-auto max-w-4xl px-4 py-12">
        <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-8">{{ title }}</h1>
        <div class="prose dark:prose-invert">
            {{ content | safe }}
        </div>
    </main>

    {% include "parts/footer.html" %}
</body>
</html>
```

### Admin Pages List

Create `src/resources/views/admin/pages/pages-list.html`:

```html
{% extends "admin/index.html" %}

{% block content %}
<div class="space-y-6">
    <div class="flex justify-between items-center">
        <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">Pages</h2>
    </div>

    <div class="bg-white dark:bg-gray-800 shadow rounded-lg">
        <ul class="divide-y divide-gray-200 dark:divide-gray-700">
            <!-- Render pages list here using Tera loop or JavaScript -->
        </ul>
    </div>
</div>
{% endblock %}
```

### Admin Page Edit Form

Create `src/resources/views/admin/pages/pages-edit.html`:

```html
{% extends "admin/index.html" %}

{% block content %}
<div class="space-y-6">
    <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">Edit Page</h2>

    <form
        action="/admin/pages/{{ page_id }}"
        method="post"
        data-controller="page-form"
        data-turbo="false"
        data-action="submit->page-form#handleSubmit"
    >
        <input type="hidden" name="csrf_token" value="{{ csrf_token }}">

        <div class="space-y-6">
            <div>
                <label for="title" class="block text-sm font-medium text-gray-900 dark:text-white">Title</label>
                <input
                    type="text"
                    name="title"
                    id="title"
                    value="{{ page_title }}"
                    class="mt-2 block w-full rounded-md border-0 py-1.5 text-gray-900 dark:text-white dark:bg-gray-800 shadow-sm ring-1 ring-inset ring-gray-300 dark:ring-gray-600"
                    required
                >
            </div>

            <div>
                <label for="content" class="block text-sm font-medium text-gray-900 dark:text-white">Content</label>
                <textarea
                    name="content"
                    id="content"
                    rows="10"
                    class="mt-2 block w-full rounded-md border-0 py-1.5 text-gray-900 dark:text-white dark:bg-gray-800 shadow-sm ring-1 ring-inset ring-gray-300 dark:ring-gray-600"
                >{{ page_content }}</textarea>
            </div>

            <button
                type="submit"
                class="rounded-md bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-500"
                data-page-form-target="submitButton"
            >
                Save
            </button>
        </div>
    </form>
</div>
{% endblock %}
```

---

## Step 8: Create Stimulus Controller

Create `src/resources/js/controllers/page_form_controller.js`:

```javascript
import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
    static targets = ["submitButton"];

    async handleSubmit(event) {
        event.preventDefault();

        const formData = new FormData(this.element);
        const urlEncodedData = new URLSearchParams(formData).toString();

        try {
            const response = await fetch(this.element.action, {
                method: "POST",
                headers: {
                    "Content-Type": "application/x-www-form-urlencoded",
                },
                body: urlEncodedData,
            });

            const data = await response.json();

            if (response.ok) {
                this.notify("Page updated successfully", "success");
            } else {
                this.notify("Failed to update page", "error");
            }
        } catch (e) {
            this.notify("Failed to update page", "error");
            console.error(e);
        }
    }

    notify(message, type) {
        window.dispatchEvent(
            new CustomEvent("notify", { detail: { message, type } })
        );
    }
}
```

Register in `src/resources/js/script.js`:

```javascript
import PageFormController from "./controllers/page_form_controller";
application.register("page-form", PageFormController);
```

---

## Step 9: Build and Test

```bash
# Run migration
diesel migration run

# Build frontend
npm run build

# Run server
cargo run develop
```

Visit:
- Public page: `http://localhost:8081/page/about`
- Admin list: `http://localhost:8081/admin/pages`
- Edit page: `http://localhost:8081/admin/pages/1/edit`

---

## Summary

| Component | File | Purpose |
|-----------|------|---------|
| Migration | `migrations/*/up.sql` | Create `pages` table |
| Model | `src/models/page.rs` | Diesel model for pages |
| Public Controller | `src/http/controllers/page_controller.rs` | Display page to visitors |
| Admin Controller | `src/http/controllers/admin_controller.rs` | List, edit, update pages |
| Routes | `src/routes.rs` | Register endpoints |
| Public Template | `src/resources/views/pages/show.html` | Render page content |
| Admin Templates | `src/resources/views/admin/pages/*.html` | Admin UI for editing |
| JS Controller | `src/resources/js/controllers/page_form_controller.js` | Handle form submission |

---

## Optional Enhancements

- **WYSIWYG Editor**: Add TinyMCE or Trix for rich text editing
- **Slug Editing**: Allow changing the page URL slug
- **Page Creation**: Add "New Page" functionality
- **Page Deletion**: Add delete with confirmation
- **Revisions**: Track content history
- **SEO Fields**: Add meta description, og:image, etc.
