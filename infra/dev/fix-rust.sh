#!/bin/sh

set -eu

cargo clippy --all-targets --all-features --fix --allow-dirty

cargo fmt --all