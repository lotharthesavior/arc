APP_NAME={{project-name}}
APP_ENV=development
APP_URL=127.0.0.1
APP_PORT=8080
DATABASE_DRIVER=sqlite
DATABASE_URL=database/database.sqlite
EVENT_BUS=inprocess
SECRET_KEY=generate-me
JWT_SECRET=generate-me
JWT_EXPIRY_HOURS=24
RUST_LOG={{crate-name}}=info,arc_web=info,actix_web=info

# Shared arc-web security headers apply automatically. See Arc security-headers.md.
# Optional complete policy overrides (unset by default; do not use empty values):
# SECURITY_CSP="base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'"
# SECURITY_CSP_REPORT_ONLY="default-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'"
# HTTPS deployments only; prefer configuring HSTS at the TLS proxy.
# SECURITY_HSTS="max-age=300"
