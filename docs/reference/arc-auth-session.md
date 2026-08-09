# `arc-auth-session`

`arc-auth-session` supplies the browser authentication protocol. It caches the complete
role-bearing `Identity` under `arc_auth_identity` and the standard projection-backed
`SessionUser` under `user`.

## Install

```sh
arc plugin add auth-db-session
```

The bundle also installs the database provider, browser administration UI, and RBAC guard.

## Public API

- `authenticate`: verifies credentials through `IdentityStore` and caches the identity.
- `identity`: reads the cached identity.
- `cache_identity`: refreshes both session identity representations.
- `sign_out`: removes both identity representations.
- `RequireSession`: redirects unauthenticated browser requests to `/signin`.
- `IdentityStoreData`: Actix extractor alias for the configured identity store.

The session plugin intentionally owns no pages. Install `arc-auth-admin` for sign-in and account
management UI. Admin routes also use Arc's idle-timeout middleware; configure it with
`SESSION_IDLE_TIMEOUT_SECS` (default `900`). Cookie settings are supplied by `arc-web`, including
optional `SESSION_DOMAIN` and `SESSION_SAME_SITE`.
