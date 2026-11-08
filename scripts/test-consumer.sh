#!/bin/sh
# Builds a throwaway app that depends on oc-kit by path, fully offline after `cargo vendor`,
# with SQLX_OFFLINE and no DATABASE_URL: the situation of an airgapped consumer.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/app/src"
cat > "$work/app/Cargo.toml" <<TOML
[package]
name = "consumer"
version = "0.1.0"
edition = "2024"

[dependencies]
oc-kit = { path = "$root/crates/oc-kit" }
TOML
printf 'fn main() {\n    let _ = oc_kit::assets::AssetSet::kit().hash();\n}\n' > "$work/app/src/main.rs"
cd "$work/app"
mkdir -p .cargo
cargo vendor --quiet vendor > .cargo/config.toml
unset DATABASE_URL
SQLX_OFFLINE=true cargo build --offline --quiet
echo "consumer: oc-kit builds offline"
