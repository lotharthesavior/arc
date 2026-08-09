# `arc-auth-db`

`arc-auth-db` implements `IdentityStore` with conventional SQLite `users`, `roles`, and
`user_roles` tables. These tables are plugin-owned and are separate from event-sourced application
aggregates and projections.

## Install

```sh
arc plugin add auth-db
make setup
```

Most browser applications use the `auth-db-session` bundle; API applications can use
`auth-db-jwt`.

## Configuration

| Variable | Meaning |
| --- | --- |
| `DATABASE_DRIVER` | Must currently be `sqlite`; other drivers fail setup clearly. |
| `DATABASE_URL` | SQLite file used by the identity store. |
| `ARC_SETUP_ADMIN_NAME` | Noninteractive first-administrator name. |
| `ARC_SETUP_ADMIN_EMAIL` | Noninteractive first-administrator email. |
| `ARC_SETUP_ADMIN_PASSWORD` | Noninteractive first-administrator password; minimum 12 characters. |

If the database has no users, interactive `make setup` prompts for these values and assigns the
first identity the `admin` role. Setup is idempotent and does not reset existing credentials.
Passwords are Argon2 hashes. Emails are normalized to lowercase and must be unique. The final
active administrator cannot be deactivated or stripped of the `admin` role.

Role names are presently stored data, not declarations loaded from a manifest. Assigning a role
name that has no matching `roles` row does not create that role.
