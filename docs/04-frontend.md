# Frontend Documentation

## Overview

The frontend uses a traditional server-rendered approach with progressive enhancement via modern JavaScript libraries. Assets are bundled using Vite for optimal performance.

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Tailwind CSS | 3.4.17 | Utility-first CSS framework |
| Alpine.js | 3.14.8 | Lightweight JavaScript framework |
| HTMX | 2.0.4 | Dynamic HTML updates without full page reloads |
| Toastify.js | 1.12.0 | Toast notifications |
| Vite | 6.0.7 | Build tool and bundler |
| PostCSS | Latest | CSS processing with autoprefixer |

## Template Engine

### Tera Templates

Templates are located in `src/resources/views/` and use the Tera templating engine.

**Directory Structure:**
```
src/resources/views/
├── home.html                 # Public landing page
├── signin.html               # Sign-in page wrapper
├── admin/
│   ├── index.html           # Admin layout base
│   ├── signin-form.html     # Sign-in form component
│   └── pages/
│       ├── dashboard.html   # Admin dashboard
│       ├── profile.html     # Profile editing
│       └── settings.html    # Settings page
│   └── parts/
│       ├── side-menu.html   # Sidebar navigation
│       └── top-menu.html    # Top navigation
└── parts/
    ├── html-head.html       # HTML head section
    ├── header.html          # Page header
    ├── hero.html            # Hero section
    ├── footer.html          # Page footer
    ├── notification.html    # Session notifications
    ├── components.html      # Reusable macros
    └── open-graph.html      # Open Graph meta tags
```

### Template Features

#### Variable Interpolation
```html
<h1>{{ name }}</h1>
```

#### Includes
```html
{% include "parts/html-head.html" %}
```

#### Conditionals
```html
{% if user_authenticated == "true" %}
    <a href="/admin">Dashboard</a>
{% else %}
    <a href="/signin">Sign In</a>
{% endif %}
```

#### Macros (components.html)
```html
{% macro button(text, type="button", classes="") %}
<button type="{{ type }}" class="btn {{ classes }}">{{ text }}</button>
{% endmacro %}
```

#### Asset Injection
The `{{ assets | safe }}` variable is automatically populated with CSS and JS links from the Vite manifest.

## Styling

### Tailwind CSS Configuration

```javascript
// tailwind.config.js
module.exports = {
    content: ['./src/**/*.{html,js}'],
    theme: {
        extend: {},
    },
    plugins: [
        require('@tailwindcss/forms'),
    ],
}
```

### Main Stylesheet

```css
/* src/resources/css/styles.css */
@tailwind base;
@tailwind components;
@tailwind utilities;
```

### PostCSS Configuration

```javascript
// postcss.config.js
module.exports = {
    plugins: {
        tailwindcss: {},
        autoprefixer: {},
    },
}
```

## JavaScript

### Main Entry Point

```javascript
// src/resources/js/script.js
import Alpine from 'alpinejs';
import htmx from 'htmx.org';
import Toastify from 'toastify-js';
import 'toastify-js/src/toastify.css';

window.Alpine = Alpine;
window.htmx = htmx;
window.Toastify = Toastify;

Alpine.start();
```

### Alpine.js Usage

Alpine.js provides reactive data binding and event handling.

**Example: Profile Form**
```html
<form x-data="{ loading: false }" @submit.prevent="submitForm">
    <input x-model="name" type="text" name="name">
    <button :disabled="loading" type="submit">
        <span x-show="!loading">Save</span>
        <span x-show="loading">Saving...</span>
    </button>
</form>
```

### HTMX Usage

HTMX enables dynamic content updates without writing JavaScript.

**Example: Form Submission**
```html
<form hx-post="/admin/profile"
      hx-target="#response"
      hx-swap="innerHTML">
    <!-- form fields -->
</form>
```

### Toast Notifications

```javascript
Toastify({
    text: "Profile updated successfully",
    duration: 3000,
    gravity: "top",
    position: "right",
    style: {
        background: "#10B981",
    }
}).showToast();
```

## Build System

### Vite Configuration

```javascript
// vite.config.js
import { resolve } from 'path';
import copy from 'rollup-plugin-copy';
import { defineConfig } from 'vite';

export default defineConfig({
    build: {
        rollupOptions: {
            input: {
                script: resolve(__dirname, 'src/resources/js/script.js'),
                styles: resolve(__dirname, 'src/resources/css/styles.css'),
            },
            output: {
                entryFileNames: '[name]-[hash].js',
                chunkFileNames: '[name]-[hash].js',
                assetFileNames: '[name]-[hash][extname]',
            },
        },
        outDir: 'dist',
        emptyOutDir: true,
        cssCodeSplit: true,
        manifest: true,
    },
    plugins: [
        copy({
            targets: [
                {
                    src: 'src/resources/imgs/**/*',
                    dest: 'dist/imgs',
                },
            ],
            hook: 'writeBundle',
        }),
    ],
});
```

### Build Output

After running `npm run build` or `cargo run develop`:

```
dist/
├── .vite/
│   └── manifest.json        # Asset manifest for cache busting
├── script-[hash].js         # Bundled JavaScript
├── styles-[hash].css        # Bundled CSS
└── imgs/                    # Copied images
    └── arc-logo.png
```

### Asset Manifest

The `manifest.json` maps source files to hashed output files:

```json
{
    "src/resources/js/script.js": {
        "file": "script-abc123.js",
        "css": ["styles-def456.css"]
    },
    "src/resources/css/styles.css": {
        "file": "styles-def456.css"
    }
}
```

## Asset Injection

The template helper (`src/helpers/template.rs`) automatically injects assets:

1. Reads `dist/.vite/manifest.json`
2. Extracts file paths with hashes
3. Generates `<link>` and `<script>` tags
4. Injects into templates via `{{ assets | safe }}`

**Generated HTML:**
```html
<link rel="stylesheet" href="/public/styles-def456.css">
<script src="/public/script-abc123.js" defer></script>
```

## Static File Serving

Static files are served from the `dist/` directory via the `/public/` route:

```rust
#[get("/public/{filename:.*}")]
pub async fn static_file(req: HttpRequest) -> Result<fs::NamedFile, Error>
```

**URL Examples:**
- `/public/script-abc123.js` -> `dist/script-abc123.js`
- `/public/imgs/logo.png` -> `dist/imgs/logo.png`

## Session Notifications

The notification system uses session messages displayed via Alpine.js:

```html
<!-- parts/notification.html -->
<div x-data="{ show: {{ session_message | length > 0 }} }"
     x-show="show"
     x-init="setTimeout(() => show = false, 5000)">
    {{ session_message }}
</div>
```

**Controller Usage:**
```rust
session.insert("message", serde_json::json!({
    "error": "Invalid credentials",
    "success": ""
})).unwrap();
```

## Admin Panel Layout

The admin panel uses a two-column layout:

```
┌─────────────────────────────────────────────────────────┐
│                     Top Menu                             │
├─────────────┬───────────────────────────────────────────┤
│             │                                           │
│   Sidebar   │              Main Content                 │
│    Menu     │                                           │
│             │                                           │
│             │                                           │
└─────────────┴───────────────────────────────────────────┘
```

**Structure:**
- `admin/index.html` - Base layout with sidebar and content area
- `admin/parts/side-menu.html` - Navigation links
- `admin/parts/top-menu.html` - User menu and logout
- `admin/pages/*.html` - Page-specific content

## Development Commands

```bash
# Install Node.js dependencies
npm install

# Build assets once
npm run build

# Watch and rebuild on changes (via cargo run develop)
npx vite build --watch

# Run Tailwind in watch mode (standalone)
npx tailwindcss -i ./src/resources/css/styles.css -o ./dist/styles.css --watch
```

## Adding New Pages

1. Create template in `src/resources/views/`
2. Include necessary partials:
   ```html
   {% include "parts/html-head.html" %}
   ```
3. Add route in `src/routes.rs`
4. Create controller handler
5. Render template with context:
   ```rust
   HttpResponse::Ok().body(load_template(
       "my-page.html",
       vec![("name", &app_name)],
       None
   ))
   ```

## Adding New Components

1. Create partial in `src/resources/views/parts/`
2. Include in parent templates:
   ```html
   {% include "parts/my-component.html" %}
   ```
3. Or add as macro in `parts/components.html`:
   ```html
   {% macro my_component(param) %}
   <div>{{ param }}</div>
   {% endmacro %}
   ```
