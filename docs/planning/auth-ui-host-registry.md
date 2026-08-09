# Auth UI Host Registry Plan

## Objective

Eliminate layout and theme duplication between generated Arc applications and browser-auth
capabilities. The application must own exactly one UI host, Tera registry, admin layout, public
layout, theme, navigation renderer, and shell-action renderer. Capability packages contribute only
namespaced leaf templates and typed UI metadata through `ArcAppBuilder`.

The completed iteration must also provide a resource-grade Users administration flow, a visible
CSRF-protected logout action, and reusable form/field primitives.

## Locked Decisions

1. `arc-auth-session` owns cookie/session behavior, sign-in/sign-out protocol, session identity,
   `RequireSession`, and idle timeout. It has no Tera dependency and ships no profile, users, layout,
   or theme templates.
2. A new optional `arc-auth-admin` package owns Profile, password, and Users browser experiences.
3. Browser UI contributions without a registered host renderer fail at application startup. There
   is no embedded fallback layout. API-only applications remain valid by omitting browser UI
   packages.
4. Users administration ships as a full resource-grade flow in the first iteration: collection,
   filtering and clear-filter action, pagination, detail, create, edit, activation/deactivation,
   and role management.
5. The host renders standard forms from typed `FormSpec`/`FieldSpec` metadata. Complex capability
   pages may use namespaced leaf templates importing host-owned form macros.
6. Navigation visibility is presentation only. Session and RBAC middleware remain authoritative.
7. Existing customized layouts are never silently overwritten during upgrades.

## Current Problem

- `crates/arc-cli/templates/src/ui.rs` builds the generated application's Tera registry.
- `crates/arc-auth-session/src/lib.rs` builds a second static Tera registry.
- `crates/arc-auth-session/templates/admin_layout.html` duplicates the generated admin shell.
- `crates/arc-cli/src/plugin.rs` textually inserts capability navigation into the host layout.
- Consequently Profile and Users can lose generated resource links, logout is absent from the
  common shell, and theme changes must be duplicated.

## Target Package Boundaries

### `arc-web`

Own the framework UI contracts and immutable runtime registry:

```rust
pub struct ArcAppBuilder {
    // existing fields
    ui_host: Option<UiHost>,
    ui_contributions: Vec<UiContribution>,
}

pub struct UiHost {
    pub owner: &'static str,
    pub templates: TemplateBundle,
    pub admin_layout: TemplateName,
    pub public_layout: TemplateName,
}

pub struct UiContribution {
    pub owner: &'static str,
    pub templates: TemplateBundle,
    pub navigation: Vec<AdminNavItem>,
    pub actions: Vec<AdminAction>,
}

pub struct UiRegistry {
    tera: tera::Tera,
    admin_layout: TemplateName,
    public_layout: TemplateName,
    navigation: Vec<AdminNavItem>,
    actions: Vec<AdminAction>,
}
```

Required builder methods:

```rust
register_ui_host(UiHost)
register_ui(UiContribution)
```

At server construction, finalize the builder into one `UiRegistry`, validate it, and inject it as
`web::Data<UiRegistry>` for all application and capability handlers.

### Generated application

Own:

- `layouts/admin.html`
- `layouts/public.html`
- shared form/UI component templates
- application CSS/assets
- dashboard and resource leaf templates
- the sole `UiHost` registration

The admin layout loops over filtered `admin_navigation` and `admin_actions`; it contains no
capability-specific links or forms.

### `arc-auth-session`

Retain:

- sign-in and sign-out protocol routes
- session identity cache
- `RequireSession`
- idle-timeout integration
- session context usable by the UI registry

Remove:

- Tera dependency and static template registry
- Profile and Users handlers
- `layout.html`, `admin_layout.html`, `profile.html`, and `users.html`
- all assumptions about host CSS classes

Sign-in UI belongs to `arc-auth-admin`; the session package exposes the authentication operations
needed by that package.

### New `arc-auth-admin`

Depend on `arc-auth-core`, `arc-auth-session`, and `arc-web`. Contribute:

- sign-in leaf template and handlers using session authentication operations
- Profile and password handlers/templates
- Users resource-grade handlers/templates
- Profile and Users navigation items
- CSRF-protected Sign out shell action
- session/RBAC route protection

Do not ship layouts, CSS, or asset paths.

## Template Contract

```rust
pub struct TemplateBundle {
    pub templates: &'static [TemplateDef],
}

pub struct TemplateDef {
    pub name: TemplateName,
    pub source: &'static str,
}

pub struct TemplateName(&'static str);
```

Rules:

- The host reserves `layouts/*` and `components/*`.
- Capabilities use `capabilities/{owner}/*`.
- Template names are globally unique.
- Duplicate names, invalid templates, and missing parents are typed startup errors.
- All templates are parsed once before the HTTP server starts.
- No request mutates or reparses the registry.
- Capability leaf templates extend the canonical host layout and may import canonical host
  components.

## Navigation and Shell Actions

```rust
pub struct AdminNavItem {
    pub id: &'static str,
    pub label: &'static str,
    pub href: &'static str,
    pub order: i16,
    pub audience: Audience,
}

pub struct AdminAction {
    pub id: &'static str,
    pub label: &'static str,
    pub href: &'static str,
    pub method: ActionMethod,
    pub audience: Audience,
}

pub enum Audience {
    Authenticated,
    AnyRole(&'static [&'static str]),
}

pub enum ActionMethod {
    Get,
    PostWithCsrf,
}
```

Rules:

- IDs are globally unique and collisions fail startup.
- Sort deterministically by `(order, id)`.
- The renderer filters by the session identity before rendering.
- `PostWithCsrf` is rendered by the host as a form with an injected CSRF token.
- The Sign out action uses `PostWithCsrf`; no copied logout HTML is permitted.
- Hidden navigation never substitutes for route middleware.

## Page Rendering Contract

```rust
pub struct UiPage {
    pub template: TemplateName,
    pub title: String,
    pub context: tera::Context,
    pub status: actix_web::http::StatusCode,
}
```

`UiRegistry::render` accepts the page, request, and session. It injects reserved shell context after
the handler supplies page data:

- `app_name`
- environment
- current identity
- filtered `admin_navigation`
- filtered `admin_actions`
- CSRF token
- canonical layout names when required

Handlers cannot override reserved keys. Rendering failures return a safe framework error response
and emit diagnostic logs; template contract failures should already have been rejected at startup.

## Reusable Form Contract

```rust
pub struct FormSpec {
    pub id: &'static str,
    pub action: String,
    pub method: FormMethod,
    pub fields: Vec<FieldSpec>,
    pub submit_label: String,
}

pub struct FieldSpec {
    pub name: &'static str,
    pub label: String,
    pub kind: FieldKind,
    pub value: FieldValue,
    pub required: bool,
    pub autocomplete: Option<&'static str>,
    pub help: Option<String>,
    pub error: Option<String>,
}

pub enum FieldKind {
    Text,
    Email,
    Password,
    Hidden,
    Select { options: Vec<OptionSpec> },
    MultiSelect { options: Vec<OptionSpec> },
    Checkbox,
}

pub enum FieldValue {
    Empty,
    Text(String),
    Bool(bool),
    Many(Vec<String>),
}
```

Form rules:

- The host owns field/form HTML, CSS classes, accessible labels, `aria-invalid`, described-by error
  wiring, help text, buttons, spacing, and responsive behavior.
- The renderer injects CSRF for every POST form.
- Password values are never retained or rerendered.
- `FormSpec` is presentation metadata, not the write model. Handlers deserialize dedicated typed
  inputs and invoke identity/domain operations.
- Validation errors rebuild the form specification with field-specific or form-level errors.
- Complex pages may import `components/forms.html` rather than use a wholly generic form renderer.

Profile example:

```rust
let form = FormSpec::post("profile", "/admin/profile")
    .field(FieldSpec::text("name", "Name").value(identity.name).required())
    .field(FieldSpec::email("email", "Email").value(identity.email).required())
    .submit("Save profile");
```

## Users UX Contract

Routes:

- `GET /admin/users` — paginated collection with name/email filter
- `GET /admin/users?filter=...` — filtered collection
- `GET /admin/users/new` — create form
- `POST /admin/users/new` — create user
- `GET /admin/users/{id}` — detail page
- `GET /admin/users/{id}/edit` — edit form
- `POST /admin/users/{id}/edit` — update identity fields
- `POST /admin/users/{id}/roles` — update roles
- `POST /admin/users/{id}/activation` — activate/deactivate

Collection behavior:

- Match generated resource page structure and components.
- Show Filter and Apply; when active, show Clear linking to `/admin/users` without query params.
- Paginate deterministically.
- Display status and roles as badges.
- Link each row to detail rather than embedding role forms in the table.
- Put Create user on a dedicated page.

Authorization:

- All Users routes require a session and idle-timeout middleware.
- All Users routes require the `admin` role.
- Profile requires a session but not the `admin` role.
- Prevent an administrator from removing their own final admin role or deactivating the final active
  administrator.

## Build and Failure Lifecycle

1. Register the application UI host.
2. Register plugins and collect UI contributions.
3. If contributions exist without a host, return a typed startup error naming their owners.
4. Reject multiple hosts.
5. Merge and parse templates.
6. Validate canonical layouts, template namespaces, parents, navigation IDs, and action IDs.
7. Sort navigation/actions.
8. Inject one immutable `UiRegistry`.
9. Start the HTTP server.

No missing-host or invalid-template condition may panic in an Actix worker.

## Implementation Phases

### Phase 1 — Framework contracts

Files primarily owned by this phase:

- `crates/arc-web/src/lib.rs`
- new `crates/arc-web/src/ui/` modules
- `crates/arc-web/Cargo.toml`

Tasks:

1. Add template, host, contribution, navigation, action, audience, page, and error types.
2. Add builder registration methods and finalize-time validation.
3. Add immutable rendering and reserved-context injection.
4. Add reusable form/field types and host form rendering contract.
5. Add unit tests for missing host, multiple hosts, collisions, ordering, invalid templates, missing
   parents, reserved context, audience filtering, CSRF actions, and password value suppression.

### Phase 2 — Generated host integration

Files primarily owned by this phase:

- `crates/arc-cli/templates/src/ui.rs`
- `crates/arc-cli/templates/src/main.rs`
- `crates/arc-cli/templates/resources/views/layouts/admin.html`
- `crates/arc-cli/templates/resources/views/layouts/public.html`
- `crates/arc-cli/templates/resources/views/components/ui.html`
- new shared form component template if separated
- `crates/arc-cli/src/scaffold.rs`
- `crates/arc-cli/src/resource.rs`

Tasks:

1. Register one `UiHost` from the generated application.
2. Replace per-resource Tera registries with the shared `UiRegistry`.
3. Render typed navigation and shell actions in the admin layout.
4. Register generated resources as template/navigation contributions rather than editing layout
   markers.
5. Add Clear behavior to generated collection filters.
6. Preserve public UI and hashed/static asset behavior.

### Phase 3 — Auth package split

Files primarily owned by this phase:

- `crates/arc-auth-session/`
- new `crates/arc-auth-admin/`
- workspace `Cargo.toml`
- affected publish/release scripts

Tasks:

1. Extract browser pages and Tera dependency from `arc-auth-session`.
2. Expose session authentication operations required by `arc-auth-admin` without exposing password
   internals.
3. Implement sign-in, Profile, password, Users, navigation, and Sign out contributions in
   `arc-auth-admin`.
4. Use host forms/components and namespaced leaf templates only.
5. Delete duplicated auth layouts.

### Phase 4 — Resource-grade Users

Tasks:

1. Implement filtering, clear, pagination, and stable ordering.
2. Implement dedicated create, detail, and edit pages.
3. Implement role changes and activation/deactivation.
4. Add final-admin safety invariants to the identity store/service layer.
5. Render all validation failures through reusable form metadata.

### Phase 5 — CLI installation and migration

Files primarily owned by this phase:

- `crates/arc-cli/src/plugin.rs`
- `crates/arc-cli/src/main.rs`
- `scripts/arc-check.sh`
- scaffold verification scripts

Tasks:

1. Add `auth-admin` and update the `auth-db-session` convenience bundle to install it for UI apps.
2. Stop textual navigation insertion.
3. Provide an explicit, idempotent migration for existing generated applications.
4. Detect customized layouts and report manual migration instructions instead of overwriting them.
5. Make doctor detect legacy package-owned auth layouts, stale navigation links, and missing host
   registration.

### Phase 6 — Verification and release

Required gates:

- `cargo fmt --all -- --check`
- `make test`
- `make lint`
- `make doctor`
- `make e2e`
- clean-room `scripts/check-arc-scaffold.sh`
- packaged-crate compilation for all changed publishable crates

Browser E2E must prove:

1. Direct protected-page access redirects to sign-in.
2. Sign-in returns to the admin shell.
3. Dashboard, generated resource collection/detail/forms, Profile, and Users show the identical
   resource navigation and Sign out action.
4. Sign out is a CSRF-protected POST and clears the session.
5. Profile update and password validation work without raw/unstyled responses.
6. Users filter/apply/clear, pagination, create, detail, edit, roles, activation, and final-admin
   safeguards work.
7. API-only/JWT applications compile and run without a UI host.
8. Missing host, duplicate templates, and duplicate navigation fail before server startup.
9. No browser console/page errors occur.

## Upgrade Compatibility

- Existing applications remain on their current generated UI until they run the explicit migration.
- The migration must be idempotent.
- It may modify generated wiring and untouched generated templates.
- It must not overwrite customized layouts or CSS; produce a precise diff/instruction instead.
- After migration, remove legacy auth layout files and navigation insertion only when ownership can be
  proven.
- `arc-auth-session` should retain a documented compatibility window or provide a clear compile-time
  migration error when its old UI exports are removed.

## Done When

- No capability package ships an admin/public layout or application theme.
- `arc-auth-session` has no Tera dependency.
- `arc-auth-admin` renders all pages through the host registry.
- Generated resources and auth pages share navigation, theme, shell actions, and form primitives.
- Logout is visible on every authenticated admin page.
- Users behaves like a first-class resource administration flow.
- UI contract failures are typed startup errors, not request-time worker panics.
- Existing applications have a safe, documented, idempotent migration path.
- All required unit, integration, clean-scaffold, packaged-crate, and browser E2E gates pass.
