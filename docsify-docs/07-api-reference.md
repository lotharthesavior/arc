# API & HTTP Reference

**Last updated:** 2026-07-27

Base URL: `http://{APP_URL}:{APP_PORT}` (defaults often `127.0.0.1:8080`).

## Public HTML

| Method | Path | Notes |
|--------|------|--------|
| GET | `/` | Home |
| GET | `/signin` | Sign-in form |
| POST | `/signin` | Credentials; session cookie; CSRF required |
| GET | `/signout` | Clears session |

## Admin HTML (session required)

| Method | Path | Notes |
|--------|------|--------|
| GET | `/admin` | Dashboard |
| GET | `/admin/settings` | Settings page |
| GET | `/admin/profile` | Profile form (projection-backed) |
| POST | `/admin/profile` | Update profile via command; CSRF |
| POST | `/admin/profile-password` | Change password via command; CSRF |

Wrapped with `AuthMiddleware` + `IdleTimeoutMiddleware`.

## JSON API v1

Prefer **`/api/v1`**. Protected routes require:

```http
Authorization: Bearer <jwt>
```

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| POST | `/api/v1/register` | Public | Register user (command) |
| POST | `/api/v1/login` | Public | Issue JWT |
| POST | `/api/v1/protected/logout` | JWT | Revoke/logout session |
| GET | `/api/v1/protected/profile` | JWT | Current profile |
| PATCH | `/api/v1/protected/profile` | JWT | Update profile |
| DELETE | `/api/v1/protected/profile` | JWT | Delete user |

Enable JWT flows with `ENABLE_JWT_AUTH=true` and a strong `JWT_SECRET` (≥ 32 chars).  
Expiry: `JWT_EXPIRY_HOURS` (default 24).

### Legacy compatibility

| Method | Path | Notes |
|--------|------|--------|
| POST | `/api/login` | Same login as v1 (compat) |
| GET | `/api/protected/profile` | JWT profile (compat) |

New clients should use `/api/v1/...`.

## Internal (Benthos → Arc)

| Method | Path | Auth |
|--------|------|------|
| POST | `/internal/projections/users/handle` | Bearer `INTERNAL_PROJECTION_TOKEN` |

Applies user events into Arc projection code. **Not** a public client API.

## Static & realtime

| Method | Path | Notes |
|--------|------|--------|
| GET | `/public/{file}` | Built assets from `dist/` |
| GET | `/ws` | WebSocket (Turbo Streams) |

## Diagnostics (e2e only)

Mounted only when `APP_ENV=e2e`:

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/__diag__/health` | Health |
| GET | `/__diag__/events/{aggregate_id}` | List events for aggregate |

## Error & security notes

- Login and global endpoints are rate-limited via env (`RATE_LIMIT_*`, `GLOBAL_RATE_LIMIT_*`).
- HTML POSTs need valid CSRF tokens.
- Production cookies expect HTTPS-friendly settings (`APP_ENV=production`).
- Never expose `__diag__` or internal projection tokens to end users.

Request/response JSON shapes are defined in `api_controller.rs` and e2e specs under `tests/e2e/specs/`.
