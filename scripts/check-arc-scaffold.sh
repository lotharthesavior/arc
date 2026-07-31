#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
consumer_root="$(mktemp -d)"
consumer_target="$repo_root/target/scaffold-check"

cleanup() {
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

    (
        cd "$consumer_root"
        ARC_CLI_TEST_LOCAL_ROOT="$repo_root" \
            cargo run --quiet --manifest-path "$repo_root/crates/arc-cli/Cargo.toml" -- \
            new "$name" --no-git "$@"
    )

    (
        cd "$consumer_root/$name"
        cargo run --quiet -- setup
        before="$(sed -n 's/^SECRET_KEY=//p' .env)"
        cargo run --quiet -- setup
        after="$(sed -n 's/^SECRET_KEY=//p' .env)"
        test -n "$before"
        test "$before" = "$after"
        cargo check --quiet
    )
}

generate_and_check arc-scaffold-minimal
generate_and_check arc-scaffold-ui --ui

test ! -e "$consumer_root/arc-scaffold-minimal/src/ui.rs"
test -e "$consumer_root/arc-scaffold-ui/src/ui.rs"
test -e "$consumer_root/arc-scaffold-ui/resources/views/home.html"

echo "Arc scaffold verification passed."
