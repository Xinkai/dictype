#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

cd "${SCRIPT_DIR}/../.."

echo "::group build..."
cmake -S . -B cmake-build-debug-llvm -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DBUILD_TESTING=ON
cmake --build cmake-build-debug-llvm
echo "::endgroup::"

echo "::group::clang-format..."
cmake --build cmake-build-debug-llvm --target format
echo "::endgroup::"
