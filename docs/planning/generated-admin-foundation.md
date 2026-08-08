# Composable Authentication Plan

**Status:** Implemented
**Decision date:** 2026-08-08

## Decision

Arc authentication is optional and composable. Cargo capability packages implement the common
`ArcPlugin` interface and register through `ArcAppBuilder`. Generated resources are public unless
their generator invocation explicitly opts into authentication.

Keep only these packages:

- `arc-auth-core`: identity store and authorization contracts.
- `arc-auth-db`: conventional `users`, `roles`, and `user_roles` tables; Argon2 credentials;
  plugin-owned Diesel migrations; first-admin setup.
- `arc-auth-session`: browser sign-in/out, profile/password editing, and user-role administration.
- `arc-auth-jwt`: API session issuance and JWT resource middleware.
- `arc-auth-rbac`: simple any-matching-role policy and resource middleware.

Identity and authorization are infrastructure state, not event-sourced domain resources. Other Arc
resources retain normal event-sourced writes.

## CLI Contract

~~~text
arc plugin add auth-db-session
arc plugin add auth-db-jwt
arc generate resource Product --ui --ui-auth session --roles admin
arc generate resource Product --api --api-auth jwt --roles admin,user
~~~

`--ui` and `--api` alone create public surfaces. Auth flags fail clearly when their capability is
not installed. Roles require an authenticated resource and `arc-auth-rbac`.

First setup uses `ARC_SETUP_ADMIN_NAME`, `ARC_SETUP_ADMIN_EMAIL`, and
`ARC_SETUP_ADMIN_PASSWORD`; reruns do not reset an existing user.

## Done When

- Plugins share one registration/setup/migration lifecycle.
- No auth routes, middleware, user code, or user migration exists without an installed capability.
- DB identity, browser session, API JWT, RBAC, profile editing, and role assignment work together.
- Public and protected generated resources compile and focused tests pass.
