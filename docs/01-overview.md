# Arc - Project Overview

## Introduction

**Arc** is a web application starter/framework built with Rust and the Actix Web framework. It now uses a workspace-based, event-sourced architecture for writes and projections while keeping server-rendered HTML as the default web surface.

> Current architecture note: older sections in this document may still describe the original single-crate MVC starter. The live code is organized under `crates/` with `arc-core`, `arc-es-sqlite`, `arc-es-postgres`, `arc-es-nats`, and `arc-app`. Durable event routing in distributed mode is handled by Benthos (Redpanda Connect), not a Rust crate — see `docs/adr/0001-benthos-only-event-routing.md`.

**Philosophy:** "Spend time with your ideas on top of a solid foundation" - The project aims to reduce boilerplate setup and provide a complete MVC structure with authentication, database integration, and frontend tooling pre-configured.

## Key Features

- **Authentication System**: Complete login/logout flow with Argon2 password hashing
- **Session Management**: Cookie-based sessions for user state
- **Admin Dashboard**: Protected admin area with dashboard, settings, and profile pages
- **Event Sourcing**: Commands, aggregates, append-only events, projections, and read models
- **Storage**: SQLite event/read-model stores via Diesel, with Postgres available through `DATABASE_DRIVER=postgres`
- **Distributed Event Lane**: NATS JetStream publisher (`arc-es-nats`) with Benthos pipelines as the durable routing, filtering, projection, and event-handler delivery mechanism
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
- **Database**: SQLite with Diesel ORM 2.2.6 for event and read-model storage
- **Event Bus**: In-process by default, optional NATS JetStream
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
├── crates/
│   ├── arc-core/          # Event sourcing primitives and framework traits
│   ├── arc-es-sqlite/     # SQLite event/read-model/session stores
│   ├── arc-es-nats/       # NATS JetStream EventBus
│   └── arc-app/           # Actix/Tera application
├── config/
│   └── benthos/           # Redpanda Connect routing pipelines
├── migrations/            # Database migrations
├── database/              # SQLite database files
├── dist/                  # Compiled frontend assets
└── docs/                  # Documentation
```

## License

This project is licensed under the MIT License.
