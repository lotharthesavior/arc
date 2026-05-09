# Session Configuration Guide

> **Prepared by**: Security Specialist + Backend Engineer + DevOps Engineer
>
> **Date**: 2026-02-27
>
> **Status**: Active

---

## Overview

This document explains how session cookies are configured in Arc and how to set them up for different environments (development, production, network access).

---

## The Problem

### Symptom

When running Arc on a server with `APP_URL=0.0.0.0` (to accept connections from any network interface) and accessing it from another machine via IP address (e.g., `http://192.168.1.100:8080`), login attempts fail even with correct credentials.

### Root Cause

Session cookies were configured with default settings that:
1. Only worked for exact domain matches
2. Required HTTPS in all cases
3. Didn't account for network IP-based access
4. Had no environment-specific configuration

---

## The Solution

### Environment-Based Configuration

Session cookie behavior is now controlled by environment variables, allowing different configurations for development and production.

### Configuration Variables

Add these to your `.env` file:

```bash
# Environment mode
APP_ENV=development  # or "production"

# Session cookie domain (optional)
# Leave empty for development to allow IP-based access
# Set to your domain in production
SESSION_DOMAIN=

# Session cookie SameSite policy
SESSION_SAME_SITE=Lax  # Options: Lax, Strict, None
```

---

## Configuration Scenarios

### Scenario 1: Local Development (Localhost Only)

**Use Case**: Developing on your local machine, accessing via `http://localhost:8080`

**Configuration**:
```bash
APP_URL="127.0.0.1"
APP_PORT=8080
APP_ENV=development
# SESSION_DOMAIN=  (leave empty or commented out)
SESSION_SAME_SITE=Lax
```

**Cookie Settings**:
- Domain: Not set (works for localhost)
- Secure: `false` (allows HTTP)
- HttpOnly: `true` (security)
- SameSite: `Lax`

**Result**: ✅ Works on `http://localhost:8080`

---

### Scenario 2: Network Development (IP Access)

**Use Case**: Running on a development server, accessed from other machines via IP (e.g., `http://192.168.1.100:8080`)

**Configuration**:
```bash
APP_URL="0.0.0.0"  # Accept connections from any interface
APP_PORT=8080
APP_ENV=development
# SESSION_DOMAIN=  (leave empty - this is the key!)
SESSION_SAME_SITE=Lax
```

**Cookie Settings**:
- Domain: Not set (allows any IP)
- Secure: `false` (allows HTTP)
- HttpOnly: `true`
- SameSite: `Lax`

**Result**:
- ✅ Works on `http://localhost:8080`
- ✅ Works on `http://192.168.1.100:8080` (server's IP)
- ✅ Works on `http://10.0.0.50:8080` (any network IP)

**⚠️ Security Note**: Only use this in trusted development networks!

---

### Scenario 3: Production with Domain

**Use Case**: Production deployment with a proper domain name and HTTPS

**Configuration**:
```bash
APP_URL="0.0.0.0"  # Bind to all interfaces (behind reverse proxy)
APP_PORT=8080
APP_ENV=production
SESSION_DOMAIN=.example.com  # Your domain (note the leading dot)
SESSION_SAME_SITE=Lax
```

**Cookie Settings**:
- Domain: `.example.com` (works for all subdomains)
- Secure: `true` (requires HTTPS)
- HttpOnly: `true`
- SameSite: `Lax`

**Result**:
- ✅ Works on `https://example.com`
- ✅ Works on `https://app.example.com`
- ✅ Works on `https://www.example.com`
- ❌ Blocks on `http://` (non-HTTPS)
- ❌ Blocks on IP access (e.g., `https://192.168.1.100:8080`)

**✅ Security**: Properly secured for production

---

### Scenario 4: Production with Strict Security

**Use Case**: Maximum security for sensitive applications

**Configuration**:
```bash
APP_URL="0.0.0.0"
APP_PORT=8080
APP_ENV=production
SESSION_DOMAIN=.example.com
SESSION_SAME_SITE=Strict  # Strictest policy
```

**Cookie Settings**:
- Domain: `.example.com`
- Secure: `true`
- HttpOnly: `true`
- SameSite: `Strict`

**Result**:
- ✅ Works on `https://example.com`
- ⚠️ May block some OAuth flows (redirects from external sites)
- ✅ Maximum CSRF protection

**Use When**: Handling very sensitive data and you control all authentication flows

---

## SameSite Policy Explained

### `Lax` (Recommended Default)

**What it does**: Sends cookies on top-level navigation (clicking links) but not on embedded requests (images, iframes from other sites)

**Use when**:
- General web applications
- OAuth/SAML authentication flows
- Good balance of security and functionality

**Blocks**:
- CSRF attacks via POST from other sites
- Cross-site embedded requests

**Allows**:
- Normal navigation between pages
- Clicking links from email
- OAuth redirects

---

### `Strict`

**What it does**: Only sends cookies for requests originating from your own site

**Use when**:
- Maximum security required
- No external authentication providers
- Internal applications only

**Blocks**:
- All cross-site requests
- OAuth/SAML flows from external sites
- Links from email may not work immediately

**Allows**:
- Requests from your own domain only

**⚠️ Warning**: May break some legitimate flows!

---

### `None`

**What it does**: Sends cookies on all requests, even cross-site

**Use when**:
- Need to embed your app in iframes on other sites
- Third-party integrations require it

**Requirements**:
- **MUST** set `APP_ENV=production` (forces `Secure` flag)
- **MUST** use HTTPS

**⚠️ Warning**: Least secure option, use only when necessary!

---

## How It Works Internally

### Session Middleware Configuration

In `src/commands/serve.rs`, the session middleware is configured dynamically:

```rust
// Determine environment
let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
let is_production = app_env == "production";

// Build session middleware
let mut session_middleware = SessionMiddleware::builder(
    CookieSessionStore::default(),
    secret_key.clone(),
)
.cookie_name("arc_session")
.cookie_http_only(true)  // Always true for security
.cookie_same_site(same_site)
.session_lifecycle(
    PersistentSession::default()
        .session_ttl(Duration::hours(24))
);

// Production: enforce HTTPS
if is_production {
    session_middleware = session_middleware.cookie_secure(true);
} else {
    session_middleware = session_middleware.cookie_secure(false);
}

// Set domain if specified
if let Some(ref domain) = session_domain {
    session_middleware = session_middleware.cookie_domain(Some(domain.clone()));
}
```

### Cookie Attributes Set

| Attribute | Development | Production |
|-----------|-------------|------------|
| `Name` | `arc_session` | `arc_session` |
| `Domain` | Not set (or from env) | From `SESSION_DOMAIN` |
| `Secure` | `false` | `true` |
| `HttpOnly` | `true` | `true` |
| `SameSite` | From `SESSION_SAME_SITE` | From `SESSION_SAME_SITE` |
| `Max-Age` | 24 hours | 24 hours |

---

## Testing Your Configuration

### Test 1: Local Access

```bash
# Start server
cargo run serve

# Test login
curl -X POST http://localhost:8080/signin \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "email=test@example.com&password=password" \
  -c cookies.txt \
  -v
```

**Expected**: You should see `Set-Cookie: arc_session=...` in response headers

---

### Test 2: Network Access (IP)

**Setup**:
```bash
# .env
APP_URL="0.0.0.0"
APP_ENV=development
# SESSION_DOMAIN not set
```

**Test from another machine**:
```bash
# Replace 192.168.1.100 with your server's IP
curl -X POST http://192.168.1.100:8080/signin \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "email=test@example.com&password=password" \
  -c cookies.txt \
  -v
```

**Expected**:
- Response includes `Set-Cookie`
- Cookie has NO `Domain` attribute (allows any host)
- Cookie has NO `Secure` flag (allows HTTP)

---

### Test 3: Browser DevTools

1. Open browser DevTools (F12)
2. Go to Application → Cookies
3. Login to your app
4. Check the `arc_session` cookie

**Verify**:
- `HttpOnly`: ✅ (should be checked)
- `Secure`: Depends on `APP_ENV`
- `SameSite`: Should match `SESSION_SAME_SITE`
- `Domain`: Should match configuration
- `Path`: `/`

---

## Troubleshooting

### Problem: Login fails from network IP

**Symptoms**:
- Works on `localhost`
- Fails on `192.168.x.x` or `10.0.x.x`
- No cookie set in browser

**Solution**:
```bash
# In .env
APP_ENV=development
# SESSION_DOMAIN=  (must be empty or commented out)
```

Restart server and clear browser cookies.

---

### Problem: "Secure cookie warning" in logs

**Symptoms**:
```
Warning: Cookie is set to Secure but connection is not HTTPS
```

**Solution**:
```bash
# For development with HTTP
APP_ENV=development
```

**Or** set up HTTPS with a reverse proxy (nginx, Caddy).

---

### Problem: Cookies not sent on cross-domain requests

**Symptoms**:
- OAuth redirects don't work
- External links to your app lose session

**Solution**:
```bash
# Change SameSite policy
SESSION_SAME_SITE=Lax  # Instead of Strict
```

If using `None`:
```bash
SESSION_SAME_SITE=None
APP_ENV=production  # Forces Secure flag
```

**And** ensure you're using HTTPS.

---

### Problem: "Session not found" after login

**Check**:
1. Is `SECRET_KEY` the same on all app instances?
2. Is cookie being sent? (Check DevTools → Network → Request Headers)
3. Is cookie domain correct?
4. Is cookie expired? (Check `Expires` in DevTools)

**Solution**:
- Ensure `SECRET_KEY` is consistent
- Check `SESSION_DOMAIN` matches your access pattern
- Verify `APP_ENV` is set correctly

---

## Security Best Practices

### Development

✅ **DO**:
- Use `APP_ENV=development`
- Test with real network access before deploying
- Keep `SESSION_DOMAIN` empty for flexibility
- Use `SESSION_SAME_SITE=Lax`

❌ **DON'T**:
- Use development settings in production
- Expose development servers to the public internet
- Use weak `SECRET_KEY` values

---

### Production

✅ **DO**:
- Always set `APP_ENV=production`
- Use HTTPS (required when `APP_ENV=production`)
- Set `SESSION_DOMAIN` to your actual domain
- Use strong `SECRET_KEY` (32+ random characters)
- Use `SESSION_SAME_SITE=Lax` or `Strict`
- Put app behind reverse proxy (nginx, Caddy)
- Set up proper firewall rules

❌ **DON'T**:
- Use `APP_ENV=development` in production
- Use `SESSION_SAME_SITE=None` unless absolutely necessary
- Expose app directly without reverse proxy
- Share `SECRET_KEY` in version control

---

## Migration Guide

### From Previous Version

If you're upgrading from a version without this session configuration:

1. **Update `.env`**:
   ```bash
   # Add these lines
   APP_ENV=development  # or production
   SESSION_SAME_SITE=Lax
   ```

2. **Clear existing sessions**:
   - Users will need to log in again after upgrade
   - Old cookies won't work with new configuration

3. **Test thoroughly**:
   - Test local access
   - Test network access (if using `APP_URL=0.0.0.0`)
   - Test on your production domain (if applicable)

4. **Update documentation**:
   - Inform users of new environment variables
   - Document your chosen configuration

---

## Environment Variable Reference

| Variable | Required | Default | Values | Description |
|----------|----------|---------|--------|-------------|
| `APP_ENV` | No | `development` | `development`, `production` | Environment mode |
| `SESSION_DOMAIN` | No | None | Domain string | Cookie domain (e.g., `.example.com`) |
| `SESSION_SAME_SITE` | No | `Lax` | `Lax`, `Strict`, `None` | SameSite policy |
| `SECRET_KEY` | **Yes** | - | 32+ char string | Session encryption key |

---

## Summary

**Key Points**:

1. **Development Mode** (`APP_ENV=development`):
   - Cookies work over HTTP
   - No domain restriction (works with IPs)
   - Perfect for local development and testing

2. **Production Mode** (`APP_ENV=production`):
   - Cookies require HTTPS (`Secure` flag)
   - Should set `SESSION_DOMAIN`
   - Maximum security

3. **Network Access**:
   - Keep `SESSION_DOMAIN` empty in development
   - Set `APP_URL=0.0.0.0` to accept connections from any IP
   - Set `APP_ENV=development` to allow HTTP

4. **Security**:
   - `HttpOnly` always enabled (prevents XSS cookie theft)
   - `Secure` flag in production (requires HTTPS)
   - `SameSite=Lax` recommended (CSRF protection)

**Default Configuration** (works for most cases):
```bash
APP_ENV=development
SESSION_SAME_SITE=Lax
# SESSION_DOMAIN=  (leave empty)
```

---

## Related Documentation

- [Authentication Guide](03-backend.md#authentication)
- [Security Best Practices](08-problems-and-improvements.md#security-concerns)
- [Production Deployment](roadmap.md#phase-5-performance--monitoring-p2)

---

**Last Updated**: 2026-02-27
**Next Review**: After first production deployment
