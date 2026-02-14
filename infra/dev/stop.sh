#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

grpcurl \
    -plaintext \
    -v \
    -proto "${SCRIPT_DIR}/../../proto/dictype.proto" \
    -unix unix:/var/run/user/1000/dictyped.socket \
    Dictype.Dictype/Stop
