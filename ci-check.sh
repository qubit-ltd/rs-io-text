#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
cargo +"${RS_CI_BUILD_TOOLCHAIN:-1.94.0}" check \
    --manifest-path "$PROJECT_ROOT/tests/fixtures/readme_quick_start/Cargo.toml" \
    --locked
