# API Reference

## Overview

This document provides a complete reference for all HTTP endpoints in the Arc application. The API uses a combination of HTML page responses and JSON API responses.

## Base URL

```
http://{APP_URL}:{APP_PORT}
```

Default: `http://127.0.0.1:8080`

## Authentication

The application uses cookie-based session authentication. After successful login, a session cookie is set and must be included in subsequent requests to protected routes.

### Session Cookie

- **Name**: `id` (actix-session default)
- **Type**: HTTP-only, encrypted cookie
- **Duration**: Session-based (expires on browser close)

## Public Endpoints

### GET /

**Description**: Home page

**Authentication**: None required

**Response**: HTML page

**Template Variables**:
- `name`: Application name
- `user_authenticated`: "true" or "false"
- `session_message`: Flash message from session

**Example**:
```bash
curl http://localhost:8080/
```

---

### GET /signin

**Description**: Sign-in page

**Authentication**: None required (redirects to /admin if already authenticated)

**Response**: HTML page with sign-in form

**Template Variables**:
- `name`: Application name
- `session_message`: Error/success message from session

**Example**:
```bash
curl http://localhost:8080/signin
```

---

### POST /signin

**Description**: Process sign-in credentials

**Authentication**: None required

**Content-Type**: `application/x-www-form-urlencoded`

**Request Body**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | Yes | User email address |
| `password` | string | Yes | User password |

**Responses**:

| Status | Condition | Redirect |
|--------|-----------|----------|
| 302 Found | Success | `/admin` |
| 302 Found | Invalid credentials | `/signin` (with error message) |
| 302 Found | Empty fields | `/signin` (with error message) |

**Example**:
```bash
curl -X POST http://localhost:8080/signin \
  -d "email=jekyll@example.com" \
  -d "password=password" \
  -c cookies.txt \
  -L
```

---

### GET /signout

**Description**: Sign out current user

**Authentication**: None required (clears session if exists)

**Response**: Redirect to `/` with success message

**Example**:
```bash
curl http://localhost:8080/signout \
  -b cookies.txt \
  -L
```

---

### GET /public/{filename}

**Description**: Serve static files from dist/ directory

**Authentication**: None required

**Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `filename` | path | Path to file relative to dist/ |

**Responses**:

| Status | Condition |
|--------|-----------|
| 200 OK | File found |
| 404 Not Found | File doesn't exist |

**Example**:
```bash
curl http://localhost:8080/public/script-abc123.js
curl http://localhost:8080/public/imgs/logo.png
```

---

## Protected Endpoints

All endpoints under `/admin/*` require authentication. Unauthenticated requests are redirected to `/signin`.

### GET /admin

**Description**: Admin dashboard

**Authentication**: Required

**Response**: HTML page

**Template Variables**:
- `name`: Application name
- `user_name`: Current user's name

**Example**:
```bash
curl http://localhost:8080/admin \
  -b cookies.txt
```

---

### GET /admin/settings

**Description**: Settings page

**Authentication**: Required

**Response**: HTML page

**Template Variables**:
- `name`: Application name
- `user_name`: Current user's name

**Example**:
```bash
curl http://localhost:8080/admin/settings \
  -b cookies.txt
```

---

### GET /admin/profile

**Description**: User profile page

**Authentication**: Required

**Response**: HTML page

**Template Variables**:
- `name`: Application name
- `user_name`: Current user's name
- `user_email`: Current user's email

**Example**:
```bash
curl http://localhost:8080/admin/profile \
  -b cookies.txt
```

---

### POST /admin/profile

**Description**: Update user profile

**Authentication**: Required

**Content-Type**: `application/x-www-form-urlencoded`

**Request Body**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | New display name |
| `email` | string | Yes | New email address |

**Responses**:

**Success (200 OK)**:
```json
{
  "data": {
    "name": "New Name",
    "email": "newemail@example.com"
  }
}
```

**Error (500 Internal Server Error)**:
```json
{
  "errors": {
    "server_error": "Failed to update user"
  }
}
```

**Example**:
```bash
curl -X POST http://localhost:8080/admin/profile \
  -b cookies.txt \
  -d "name=John%20Doe" \
  -d "email=john@example.com"
```

---

### POST /admin/profile-password

**Description**: Change user password

**Authentication**: Required

**Content-Type**: `application/x-www-form-urlencoded`

**Request Body**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `current_email` | string | Yes | Current user email (for verification) |
| `old_password` | string | Yes | Current password |
| `new_password` | string | Yes | New password |

**Responses**:

**Success (200 OK)**:
```json
{
  "success": "Password updated"
}
```

**Error (500 Internal Server Error)**:
```json
{
  "errors": {
    "server_error": "Invalid credentials"
  }
}
```

```json
{
  "errors": {
    "server_error": "Failed to update user"
  }
}
```

**Example**:
```bash
curl -X POST http://localhost:8080/admin/profile-password \
  -b cookies.txt \
  -d "current_email=jekyll@example.com" \
  -d "old_password=password" \
  -d "new_password=newpassword123"
```

---

## Error Responses

### HTML Errors

For page requests, errors result in:
- Redirect with session message for auth errors
- Standard HTTP error pages for server errors

### JSON Errors

For API endpoints (`/admin/profile`, `/admin/profile-password`):

```json
{
  "errors": {
    "field_name": "Error description"
  }
}
```

Common error fields:
- `server_error`: General server-side error

---

## Data Types

### User Object

```json
{
  "id": 1,
  "name": "Jekyll",
  "email": "jekyll@example.com",
  "password": "[hashed]",
  "created_at": "2024-12-16 13:40:59",
  "updated_at": "2024-12-16 13:40:59"
}
```

### UserForm (Profile Update)

```json
{
  "name": "string",
  "email": "string"
}
```

### PasswordForm (Password Change)

```json
{
  "current_email": "string",
  "old_password": "string",
  "new_password": "string"
}
```

### ProfileResponse

```json
{
  "data": {
    "name": "string",
    "email": "string"
  }
}
```

---

## Session Messages

Session messages are stored in the session and displayed once:

### Simple Message
```rust
session.insert("message", "You have been signed out").unwrap();
```

### JSON Message
```rust
session.insert("message", serde_json::json!({
    "error": "Invalid credentials",
    "success": ""
})).unwrap();
```

Retrieved in templates as `session_message` variable.

---

## HTTP Status Codes

| Code | Meaning | Usage |
|------|---------|-------|
| 200 | OK | Successful request |
| 302 | Found | Redirect after form submission |
| 404 | Not Found | Resource doesn't exist |
| 500 | Internal Server Error | Server-side error |

---

## Rate Limiting

Currently, no rate limiting is implemented. This is recommended for production deployments.

---

## CORS

CORS headers are not configured by default. Add CORS middleware for cross-origin API access.

---

## Request/Response Examples

### Complete Authentication Flow

```bash
# 1. Sign in
curl -X POST http://localhost:8080/signin \
  -d "email=jekyll@example.com" \
  -d "password=password" \
  -c cookies.txt \
  -v

# 2. Access protected route
curl http://localhost:8080/admin \
  -b cookies.txt

# 3. Update profile
curl -X POST http://localhost:8080/admin/profile \
  -b cookies.txt \
  -d "name=Updated%20Name" \
  -d "email=updated@example.com"

# 4. Change password
curl -X POST http://localhost:8080/admin/profile-password \
  -b cookies.txt \
  -d "current_email=updated@example.com" \
  -d "old_password=password" \
  -d "new_password=newpassword"

# 5. Sign out
curl http://localhost:8080/signout \
  -b cookies.txt \
  -c cookies.txt \
  -L
```
