#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .agent-tmp
if [[ -z ${ARCHRESCUE_TEST_DESTINATION:-} ]]; then
  echo 'BLOCKED: set ARCHRESCUE_TEST_DESTINATION to an existing disposable directory on a separate filesystem.' >&2
  exit 2
fi
python3 scripts/scratch.py check --budget-bytes "$((2 * 1024 ** 3))"
if [[ -z ${CARGO_TARGET_DIR:-} ]]; then
  export CARGO_TARGET_DIR="${ARCHRESCUE_SCRATCH_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/archrescue}/cargo-target"
fi
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
