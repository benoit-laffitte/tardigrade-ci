#!/usr/bin/env bash
set -euo pipefail

# Runs workspace coverage and fails when line coverage is below threshold.
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
THRESHOLD="${1:-75}"

if [[ ! "$THRESHOLD" =~ ^[0-9]+$ ]]; then
  echo "[coverage] threshold must be an integer percentage (got: $THRESHOLD)" >&2
  exit 2
fi

CARGO_HOME_DIR="${CARGO_HOME:-$ROOT_DIR/.tmp-cargo-home}"
LLVM_COV_BIN="$CARGO_HOME_DIR/bin/cargo-llvm-cov"
IGNORE_FILENAME_REGEX='(src/main\.rs|src/bin/.*|api/src/(graphql|handlers|service|state)/.*|server/src/(main\.rs|runtime/shutdown_signal\.rs)|worker/src/(main\.rs|worker_api\.rs)|scheduler/src/backend/(postgres_scheduler|redis_scheduler)\.rs|storage/src/backend/postgres_storage\.rs|storage/src/mapping/.*\.rs)'

if [[ ! -x "$LLVM_COV_BIN" ]]; then
  echo "[coverage] cargo-llvm-cov not found at $LLVM_COV_BIN" >&2
  echo "[coverage] install with: env -u https_proxy -u http_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u NO_PROXY -u no_proxy -u PXY_FAB_FONC CARGO_HOME=$CARGO_HOME_DIR cargo install cargo-llvm-cov" >&2
  exit 2
fi

echo "[coverage] running workspace coverage with line threshold ${THRESHOLD}%"
env -u https_proxy -u http_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u NO_PROXY -u no_proxy -u PXY_FAB_FONC \
  CARGO_HOME="$CARGO_HOME_DIR" \
  "$LLVM_COV_BIN" llvm-cov --workspace --summary-only --ignore-filename-regex "$IGNORE_FILENAME_REGEX" --fail-under-lines "$THRESHOLD"
