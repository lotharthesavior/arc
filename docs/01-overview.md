# Arc - Project Overview

## Introduction

**Arc** is a web application starter/framework built with Rust and the Actix Web framework. It provides a solid, production-ready foundation for building traditional server-rendered web applications.

**Philosophy:** "Spend time with your ideas on top of a solid foundation" - The project aims to reduce boilerplate setup and provide a complete MVC structure with authentication, database integration, and frontend tooling pre-configured.

## Key Features

- **Authentication System**: Complete login/logout flow with Argon2 password hashing
- **Session Management**: Cookie-based sessions for user state
- **Admin Dashboard**: Protected admin area with dashboard, settings, and profile pages
- **Database ORM**: Diesel ORM with SQLite for type-safe queries
- **Migration System**: Versioned database migrations
- **Seeder Pattern**: Database population with initial data
- **Template Engine**: Tera templates for server-side rendering
- **Modern Frontend**: Tailwind CSS, Alpine.js, and HTMX integration
- **Asset Bundling**: Vite for fast asset compilation with hashing
- **Development Mode**: Hot-reload with cargo-watch and Vite watch mode
- **Test Suite**: Comprehensive tests for models, controllers, and middleware

## Technology Stack

### Backend

- **Framework**: Actix Web 4.x
- **Database**: SQLite with Diesel ORM 2.2.6
- **Template Engine**: Tera 1.20.0
- **Password Hashing**: Argon2 0.5.3
- **Session Management**: actix-session 0.10.1
- **Async Runtime**: Tokio 1.42.0

### Frontend

- **CSS Framework**: Tailwind CSS 3.4.17
- **JavaScript Framework**: Alpine.js 3.14.8
- **Dynamic Updates**: HTMX 2.0.4
- **Notifications**: Toastify.js 1.12.0
- **Build Tool**: Vite 6.0.7

## Quick Start

```bash
# Run database migrations
cargo run migrate

# Seed database with test user
cargo run seed

# Start development server with hot-reload
cargo run develop

# Run production server
cargo run serve
```

## Default Test User

After running the seed command, a test user is available:

- **Email**: jekyll@example.com
- **Password**: password

## System Requirements

- Rust (latest stable)
- Node.js and npm
- SQLite3 development libraries
- `cargo-watch` for development mode

### Ubuntu/Debian Dependencies

```bash
apt install build-essential libssl-dev libsqlite3-dev
cargo install cargo-watch
```

## Directory Structure

```
arc/
├── src/                    # Main source code
│   ├── console/           # CLI commands
│   ├── database/          # Seeders
│   ├── helpers/           # Utility functions
│   ├── http/              # Controllers & middlewares
│   ├── models/            # Data models
│   ├── resources/         # Frontend assets & views
│   ├── services/          # Business logic
│   ├── main.rs            # Application entry point
│   ├── routes.rs          # Route definitions
│   └── schema.rs          # Diesel ORM schema
├── migrations/            # Database migrations
├── database/              # SQLite database files
├── dist/                  # Compiled frontend assets
└── docs/                  # Documentation
```

## License

This project is licensed under the MIT License.
