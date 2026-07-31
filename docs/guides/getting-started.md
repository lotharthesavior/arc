# Getting started

This guide creates a self-contained Arc application without cloning the Arc repository.

## 1. Install the generator

Install stable Rust, `make`, and SQLite development libraries, then run:

```sh
cargo install arc-web-cli
```

## 2. Create an application

```sh
arc new my-app
cd my-app
```

The plain command creates a minimal server with a health route, SQLite persistence, app-owned migrations, and a domain skeleton.

Add `--ui` to include a Tera home page and browser assets:

```sh
arc new my-app --ui
```

Arc initializes Git by default without creating a commit. Use `--no-git` to skip that step.

## 3. Set up the application

```sh
make setup
```

The setup command:

- copies `.env.example` to `.env`;
- generates a random 64-byte local secret;
- creates the SQLite directory and database;
- runs the generated application’s migrations.

It is idempotent: running it again preserves the existing `.env` and secret.

## 4. Start Arc

```sh
make dev
```

Open <http://127.0.0.1:8080/health>. A `--ui` application also serves its home page at <http://127.0.0.1:8080/>.

The server stays attached to the terminal while running. Stop it with `Ctrl+C`.

## 5. Verify the installation

In another terminal:

```bash
curl --fail http://127.0.0.1:8080/health
```

Then run the generated project checks:

```sh
make check
make test
```

## Port 8080 is busy

Set a free port in `.env`:

```dotenv
APP_PORT=8081
```

Restart Arc with `make dev` and open <http://127.0.0.1:8081>.

On Linux, inspect port 8080 with:

```bash
ss -ltnp '( sport = :8080 )'
```

The generated entry point prints configuration, database, and bind failures. If setup was interrupted, rerun `make setup`; if only migrations are needed, run `make migrate`.
