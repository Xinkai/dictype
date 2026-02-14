#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

cd "${SCRIPT_DIR}/../.."

echo "::group::check Cargo.lock..."
cargo check --workspace --all-targets --locked
echo "::endgroup::"

echo "::group::clippy check..."
cargo clippy --workspace --all-targets --all-features
echo "::endgroup::"

echo "::group::format check..."
cargo fmt --check
echo "::endgroup::"
