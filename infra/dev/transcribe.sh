#!/bin/sh

set -eu

SCRIPT_DIR="$(dirname -- "$0")"

grpcurl -plaintext -proto "${SCRIPT_DIR}/../../proto/dictype.proto" \
  -d "{\"profile_name\": \"${1}\" }" \
  -unix unix:/var/run/user/1000/dictype/dictyped.socket \
  Dictype.Dictype/Transcribe
