#!/usr/bin/env bash
set -euo pipefail

text_project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
cd -- "$text_project_root"
cargo run --locked --manifest-path tests/fixtures/readme_quick_start/Cargo.toml
cargo test --locked --manifest-path fuzz/Cargo.toml --lib
