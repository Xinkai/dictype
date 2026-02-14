#!/bin/sh

set -eu

cargo clippy --workspace --all-targets --all-features --fix --allow-dirty

cargo fmt --all