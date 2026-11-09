#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .agent-tmp
if [[ -z ${ARCHRESCUE_TEST_DESTINATION:-} ]]; then
  echo 'BLOCKED: set ARCHRESCUE_TEST_DESTINATION to an existing disposable directory on a separate filesystem.' >&2
  exit 2
fi
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
