#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

cd "${SCRIPT_DIR}/../.."

echo "::group::clippy check..."
cargo clippy --workspace --all-targets --all-features
echo "::endgroup::"

echo "::group::format check..."
cargo fmt --check
echo "::endgroup::"
