# Configuration

`make setup` copies `.env.example` to `.env`, generates a local secret, creates SQLite, and runs migrations.

## Generated variables

```dotenv
APP_NAME=my-app
APP_ENV=development
APP_URL=127.0.0.1
APP_PORT=8080
DATABASE_DRIVER=sqlite
DATABASE_URL=database/database.sqlite
EVENT_BUS=inprocess
SECRET_KEY=generate-me
RUST_LOG=my_app=info,arc_web=info,actix_web=info
```

`SECRET_KEY=generate-me` appears only in `.env.example`; `make setup` replaces it in `.env`.

## Change the port

```dotenv
APP_PORT=8081
```

Restart with `make dev`.

## Change the bind address

For local access from other devices:

```dotenv
APP_URL=0.0.0.0
```

Do not expose a development server publicly.

## Database ownership

The generated application owns:

- `DATABASE_URL`
- every migration under `migrations/`
- its read-model tables
- its schema evolution

Run pending migrations with:

```bash
make migrate
```

Do not add application-specific migrations to an Arc package.

## Keep secrets local

`.env` is ignored by the generated `.gitignore`. Commit `.env.example`, never `.env`.
