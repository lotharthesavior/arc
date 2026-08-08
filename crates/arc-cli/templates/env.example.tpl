APP_NAME={{project-name}}
APP_ENV=development
APP_URL=127.0.0.1
APP_PORT=8080
DATABASE_DRIVER=sqlite
DATABASE_URL=database/database.sqlite
EVENT_BUS=inprocess
SECRET_KEY=generate-me
ADMIN_EMAIL=admin@example.com
ADMIN_PASSWORD=change-me
# Production: remove ADMIN_PASSWORD and set an Argon2 PHC string here.
ADMIN_PASSWORD_HASH=
SELF_REGISTRATION=false
JWT_SECRET=generate-me
JWT_EXPIRY_HOURS=24
RUST_LOG={{crate-name}}=info,arc_web=info,actix_web=info
