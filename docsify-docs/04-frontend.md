# Frontend

**Last updated:** 2026-07-27

## Overview

Arc’s UI is **server-rendered HTML** (Tera) with progressive enhancement:

| Layer | Technology |
|-------|------------|
| Templates | Tera under `crates/arc-app/src/resources/views/` |
| CSS | Tailwind CSS 3 + PostCSS |
| Interactivity | **Stimulus** controllers |
| Navigation / partial updates | **Turbo** (Drive + Streams) |
| Toasts | Toastify |
| Bundler | Vite 6 → output in `dist/`, served at `/public/*` |

> Not used as the primary stack: Alpine.js and HTMX. Older docs that list them are obsolete. A few template class names may still say `htmx-indicator` cosmetically; behavior is Stimulus/Turbo.

## Template layout

```
crates/arc-app/src/resources/views/
├── home.html
├── signin.html
├── admin/
│   ├── index.html
│   ├── signin-form.html
│   ├── pages/          # dashboard, profile, settings
│   └── parts/          # side-menu, top-menu
└── parts/              # header, footer, hero, html-head, open-graph, …
```

Helpers in `arc-web` inject CSRF tokens, asset URLs (Vite manifest), and flash/session data into template context.

## JavaScript

Entry: `crates/arc-app/src/resources/js/script.js` — starts Turbo and the Stimulus application.

Controllers (examples under `resources/js/controllers/`):

| Controller | Purpose |
|------------|---------|
| `dark_mode_controller` | Theme toggle |
| `dropdown_controller` | Menus |
| `mobile_menu_controller` | Responsive nav |
| `active_link_controller` | Nav highlighting (`turbo:load`) |
| `notification_controller` | Flash / toast |
| `session_notification_controller` | Session-related UI |
| `profile_form_controller` / `password_form_controller` | Form UX (often `data-turbo="false"` on sensitive posts) |
| `websocket_controller` | `/ws` + Turbo Streams |

## Assets & caching

1. Source CSS/JS under `crates/arc-app/src/resources/`.
2. `npm run build` / `make frontend-build` → hashed files in `dist/`.
3. `routes.rs` `static_file` serves `/public/*` with:
   - **Hashed** `.js`/`.css`: long-lived immutable cache
   - Other files: shorter max-age + revalidate
   - ETag / If-None-Match support

Logo and images: `resources/imgs/` (copied into dist by Vite/rollup-plugin-copy as configured).

## Development

```bash
make dev              # backend watch + frontend pipeline
# or separately:
npm run dev           # Vite
cargo run -p arc -- develop
```

## Adding a page (checklist)

1. Add or extend a Tera template under `resources/views/`.
2. Add a controller action in `arc-app` and register the route in `routes.rs`.
3. Protect with `AuthMiddleware` if admin.
4. For writes: dispatch a domain command; for reads: query the projection.
5. Add Stimulus controller only if client behavior is needed.
6. Rebuild assets if CSS/JS changed.

See also `docs/guides/adding-endpoints.md`.
