# Authentication plugins

Arc keeps authentication capabilities optional and composable. Generated applications install
them with `arc plugin add`; `arc-web` supplies shared middleware and registration contracts but
does not choose an identity provider or authorization policy for the application.

## Packages

| Package | Responsibility | Reference |
| --- | --- | --- |
| `arc-auth-core` | Identity and authorization contracts | [arc-auth-core](arc-auth-core.md) |
| `arc-auth-db` | SQLite identities, passwords, roles, and first-admin setup | [arc-auth-db](arc-auth-db.md) |
| `arc-auth-session` | Browser session protocol and guard | [arc-auth-session](arc-auth-session.md) |
| `arc-auth-admin` | Sign-in, profile, password, and user-management pages | [arc-auth-admin](arc-auth-admin.md) |
| `arc-auth-jwt` | API token issuance and JWT guard export | [arc-auth-jwt](arc-auth-jwt.md) |
| `arc-auth-rbac` | Any-matching-role authorization guard | [arc-auth-rbac](arc-auth-rbac.md) |

## Installation commands

Run these from a generated application root:

```sh
arc plugin add auth-db-session
arc plugin add auth-db-jwt
```

`auth-db-session` installs `auth-db`, `auth-session`, `auth-admin`, and `auth-rbac`.
`auth-db-jwt` installs `auth-db`, `auth-jwt`, and `auth-rbac`. Installation is idempotent, so the
two bundles can be combined. Individual capability names also work with `arc plugin add`.

After installation, run `make setup`. On an empty identity database this creates the schema and
prompts for the first administrator. In noninteractive environments provide
`ARC_SETUP_ADMIN_NAME`, `ARC_SETUP_ADMIN_EMAIL`, and `ARC_SETUP_ADMIN_PASSWORD`.

## Protect generated resources

Install the relevant plugins before generating a resource:

```sh
arc generate resource Product --api --ui --api-auth jwt --ui-auth session
make migrate
```

When `arc-auth-session` is already installed, generated UI routes default to session protection.
API protection must be requested with `--api-auth jwt`. Use `--roles admin,user` to require any
one of the listed roles; the matching transport authenticator must also be enabled.

An API can be explicitly public with `--api-auth none`. A generated UI is public only when session
auth is not installed; when `arc-auth-session` is present, the generator protects browser resources
automatically. Review public routes carefully because generated write endpoints dispatch with an
anonymous actor.

## Current authorization boundary

Roles are currently database rows assigned to identities. Generated route requirements are written
into Rust route configuration at generation time. Arc does **not yet** load role definitions,
permissions, or action policies from JSON/YAML manifests, and it does not yet expose auth-policy
management commands. The built-in admin user-management pages currently require the literal
`admin` role.

That distinction matters: plugin installation provides authentication and the present role guard;
it does not make every application action dynamically configurable.
