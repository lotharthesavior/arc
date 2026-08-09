# `arc-auth-admin`

`arc-auth-admin` contributes host-rendered authentication pages and navigation to an application's
registered `UiHost`. It depends on `arc-auth-session` for protocol behavior and an `IdentityStore`
provider such as `arc-auth-db`.

## Routes

| Method and path | Access | Purpose |
| --- | --- | --- |
| `GET /signin`, `POST /signin` | Public + CSRF on POST | Sign in. |
| `POST /signout` | CSRF | Clear the browser identity. |
| `GET/POST /admin/profile` | Session + idle timeout | View or update the current profile. |
| `POST /admin/profile/password` | Session + idle timeout + CSRF | Change password. |
| `/admin/users/*` | Session + idle timeout + `admin` role | List, create, edit, activate, and assign roles. |

All state-changing HTML routes validate CSRF tokens. User management refuses to remove the final
active administrator through the database provider's invariant.

The plugin contributes Profile and Users navigation plus the Sign out action. The host application
owns the surrounding layout and assets. At present, user-management authorization is hard-coded to
the `admin` role; it is not loaded from a permissions manifest.
