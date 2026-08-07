# {{project-name}}

Generated with Arc.

```sh
make setup
make dev
```

Open <http://127.0.0.1:8080/health>. Change `APP_PORT` in `.env` if port 8080 is busy.

## Generate a resource

Create and register an event-sourced aggregate, commands, events, projector, read-model migration,
and focused tests:

```sh
arc generate resource Product --api
make migrate
```

`--api` adds create/list/get/update/delete JSON endpoints. Generation refuses to overwrite an
existing resource. `aggregate` is an alias for `resource`.
