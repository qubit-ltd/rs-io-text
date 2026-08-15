#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
exec env \
    RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-${RS_CI_BUILD_TOOLCHAIN:-1.94.0}}" \
    RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
    "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
