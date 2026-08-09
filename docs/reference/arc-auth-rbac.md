# `arc-auth-rbac`

`arc-auth-rbac` provides `SimpleRbac` and the `RequireRoles` Actix guard. The policy permits an
active identity when no roles are required or when any required role matches exactly.

## Generate guarded routes

```sh
arc generate resource Report --api --ui \
  --api-auth jwt --ui-auth session --roles admin,auditor
make migrate
```

The generated route accepts an active identity with either `admin` or `auditor`. API requests must
first pass JWT authentication; browser requests must first pass session authentication.

`RequireRoles` resolves identities from the browser session or from the actor ID installed by JWT
middleware. A failed role check returns `403 Forbidden`.

The constants `ADMIN` and `USER` are convenience role names, not a complete role catalog. There is
currently no JSON/YAML role-permission manifest, permission inheritance, deny rule, or runtime
action registry. Use an application-specific `AuthorizationPolicy` when the built-in any-role
semantics are insufficient.
