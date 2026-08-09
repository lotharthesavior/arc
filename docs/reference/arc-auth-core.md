# `arc-auth-core`

`arc-auth-core` defines provider-neutral authentication and authorization contracts. It owns no
routes, middleware, database schema, or UI.

## Public API

- `Identity`: `id`, `name`, `email`, active state, and assigned role names.
- `Identity::has_role`: exact, case-sensitive role membership check.
- `IdentityStore`: authenticate, retrieve/list users, create users, update profiles/passwords,
  assign roles, and activate/deactivate identities.
- `AuthorizationPolicy`: decides whether an identity satisfies required roles.
- `AuthError`: stable invalid-credential, not-found, duplicate-email, invalid-input, and store
  failure categories.

Implement `IdentityStore` in a provider plugin when identities do not live in `arc-auth-db`.
Implement `AuthorizationPolicy` when the simple role matcher in `arc-auth-rbac` is insufficient.

See the [authentication plugin overview](auth-plugins.md) for installation and current limits.
