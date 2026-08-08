#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
consumer_root="$(mktemp -d)"
consumer_target="$repo_root/target/scaffold-check"
server_pid=""

cleanup() {
    if [[ -n "$server_pid" ]]; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    case "$consumer_root" in
        /tmp/*) rm -rf "$consumer_root" ;;
        *) echo "Refusing to clean unexpected scaffold path: $consumer_root" >&2 ;;
    esac
}
trap cleanup EXIT

export CARGO_TARGET_DIR="$consumer_target"

generate_and_check() {
    local name="$1"
    shift
    local resource_args=(--api)
    if [[ " $* " == *" --ui "* ]]; then
        resource_args+=(--ui)
    fi

    (
        cd "$consumer_root"
        ARC_CLI_TEST_LOCAL_ROOT="$repo_root" \
            cargo run --quiet --manifest-path "$repo_root/crates/arc-cli/Cargo.toml" -- \
            new "$name" --no-git "$@"
    )

    (
        cd "$consumer_root/$name"
        cargo run --quiet --manifest-path "$repo_root/crates/arc-cli/Cargo.toml" -- \
            generate resource Product "${resource_args[@]}"
        cargo run --quiet -- setup
        before="$(sed -n 's/^SECRET_KEY=//p' .env)"
        cargo run --quiet -- setup
        after="$(sed -n 's/^SECRET_KEY=//p' .env)"
        test -n "$before"
        test "$before" = "$after"
        cargo check --quiet
        cargo test --quiet
        cargo fmt --all -- --check
        cargo clippy --quiet --all-targets -- -D warnings
    )
}

generate_and_check arc-scaffold-minimal
generate_and_check arc-scaffold-ui --ui

test ! -e "$consumer_root/arc-scaffold-minimal/src/ui.rs"
test -e "$consumer_root/arc-scaffold-ui/src/ui.rs"
test -e "$consumer_root/arc-scaffold-ui/resources/views/home.html"
test -e "$consumer_root/arc-scaffold-minimal/src/domain/product/aggregate.rs"
test -e "$consumer_root/arc-scaffold-ui/src/domain/product/projector.rs"
test -e "$consumer_root/arc-scaffold-ui/src/domain/product/api.rs"
grep -q 'register_aggregate::<ProductAggregate>()' \
    "$consumer_root/arc-scaffold-minimal/src/main.rs"
grep -q 'register_projector(ProductProjector, PRODUCTS_VIEW)' \
    "$consumer_root/arc-scaffold-ui/src/main.rs"
grep -q 'crate::domain::product::api::config(cfg);' \
    "$consumer_root/arc-scaffold-ui/src/routes.rs"

pushd "$consumer_root/arc-scaffold-minimal" >/dev/null
sed -i 's/^APP_PORT=.*/APP_PORT=39081/' .env
cargo run --quiet -- serve >server.log 2>&1 &
server_pid=$!
for _ in {1..40}; do
    if curl --silent --fail http://127.0.0.1:39081/health >/dev/null; then
        break
    fi
    sleep 0.25
done

test "$(curl --silent --output /dev/null --write-out '%{http_code}' \
    http://127.0.0.1:39081/api/products)" = "401"
token="$(curl --silent --fail \
    --request POST \
    --header 'content-type: application/json' \
    --data '{"email":"admin@example.com","password":"change-me"}' \
    http://127.0.0.1:39081/api/session | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')"
test -n "$token"
auth_header="authorization: Bearer $token"
curl --silent --fail \
    --request POST \
    --header 'content-type: application/json' \
    --header "$auth_header" \
    --data '{"id":"product-1","name":"Notebook"}' \
    http://127.0.0.1:39081/api/products | grep -q 'Notebook'
curl --silent --fail --header "$auth_header" http://127.0.0.1:39081/api/products/product-1 | grep -q 'Notebook'
curl --silent --fail --header "$auth_header" http://127.0.0.1:39081/api/products | grep -q 'product-1'
curl --silent --fail \
    --request PUT \
    --header 'content-type: application/json' \
    --header "$auth_header" \
    --data '{"name":"Field Notebook"}' \
    http://127.0.0.1:39081/api/products/product-1 | grep -q 'Field Notebook'
curl --silent --fail --header "$auth_header" http://127.0.0.1:39081/api/products/product-1 | grep -q 'Field Notebook'
curl --silent --fail \
    --request DELETE \
    --header "$auth_header" \
    http://127.0.0.1:39081/api/products/product-1 >/dev/null
test "$(curl --silent --output /dev/null --write-out '%{http_code}' \
    --header "$auth_header" \
    http://127.0.0.1:39081/api/products/product-1)" = "404"
test "$(sqlite3 database/database.sqlite \
    "SELECT COUNT(*) FROM events WHERE aggregate_type = 'Product' AND aggregate_id = 'product-1';")" = "3"
kill "$server_pid"
wait "$server_pid" 2>/dev/null || true
server_pid=""
popd >/dev/null

echo "Arc scaffold verification passed."
