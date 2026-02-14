#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

cd "${SCRIPT_DIR}/../.."

rm -rf cmake-build-ci

echo "::group build..."
cmake -S . -B cmake-build-ci -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DBUILD_TESTING=ON
cmake --build cmake-build-ci
echo "::endgroup::"

echo "::group::clang-format check..."
cmake --build cmake-build-ci --target format-check
echo "::endgroup::"
