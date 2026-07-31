#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if (($#)); then
    crates=("$@")
else
    crates=(arc-core arc-es-sqlite arc-es-postgres arc-es-nats arc-web arc-web-cli)
fi

for crate in "${crates[@]}"; do
    echo "Packaging ${crate} ${version}..."
    cargo package --allow-dirty --no-verify -p "$crate"

    package_dir="$repo_root/target/package/${crate}-${version}"
    package_archive="$repo_root/target/package/${crate}-${version}.crate"
    if [[ ! -f "$package_archive" ]]; then
        echo "Package archive missing: $package_archive" >&2
        exit 1
    fi

    case "$package_dir" in
        "$repo_root"/target/package/*) rm -rf "$package_dir" ;;
        *)
            echo "Refusing to clean unexpected package path: $package_dir" >&2
            exit 1
            ;;
    esac
    tar -xzf "$package_archive" -C "$repo_root/target/package"

    if [[ ! -d "$package_dir" ]]; then
        echo "Packaged source directory missing: $package_dir" >&2
        exit 1
    fi
done

export CARGO_TARGET_DIR="$repo_root/target/publish-check"

for crate in "${crates[@]}"; do
    package_dir="$repo_root/target/package/${crate}-${version}"
    echo "Compiling packaged ${crate} tests..."
    cargo test --manifest-path "$package_dir/Cargo.toml" --all-features --no-run
done

if [[ ! " ${crates[*]} " =~ " arc-web " ]]; then
    echo "Selected publish-package verification passed."
    exit 0
fi

consumer_dir="$(mktemp -d)"
trap 'rm -rf "$consumer_dir"' EXIT

mkdir -p "$consumer_dir/src"
cat >"$consumer_dir/Cargo.toml" <<EOF
[package]
name = "arc-package-consumer"
version = "0.1.0"
edition = "2021"

[dependencies]
arc-web = { path = "$repo_root/target/package/arc-web-${version}", features = ["nats", "postgres"] }
EOF

cat >"$consumer_dir/src/main.rs" <<'EOF'
use arc_web::ArcApp;

fn main() {
    let _ = ArcApp;
}
EOF

echo "Compiling arc-web from a standalone external consumer..."
cargo check --manifest-path "$consumer_dir/Cargo.toml"

echo "Publish-package verification passed."
