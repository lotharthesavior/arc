# `arc-auth-jwt`

`arc-auth-jwt` issues revocable bearer tokens for API clients and re-exports Arc's JWT middleware
as `RequireJwt`.

## Install and generate a protected API

```sh
arc plugin add auth-db-jwt
arc generate resource Product --api --api-auth jwt
make migrate
```

API protection is explicit: installing the plugin does not wrap every `/api` route automatically.

## Token endpoint

Send credentials to `POST /api/session`:

```json
{"email":"admin@example.com","password":"your-password"}
```

The response contains `token` and `token_type: "Bearer"`. Send the token as
`Authorization: Bearer <token>`. Issued token identifiers are recorded in the configured
`SessionStore`; JWT middleware rejects records that have been revoked.

## Configuration

| Variable | Meaning |
| --- | --- |
| `JWT_SECRET` | Required signing secret. Generated projects initialize it during setup. |
| `JWT_EXPIRY_HOURS` | Token lifetime in hours; generated default is `24`. |
| `JWT_GRANDFATHER_LEGACY` | Opt-in compatibility for legacy tokens without a session record. |
