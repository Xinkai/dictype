#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

cd "${SCRIPT_DIR}/../.."

cmake -S . -B cmake-build-debug-llvm -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DBUILD_TESTING=ON
cmake --build cmake-build-debug-llvm

cmake --build cmake-build-debug-llvm --target format
