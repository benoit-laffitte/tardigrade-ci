#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

USE_PROXY=false

for arg in "$@"; do
  case "$arg" in
    --with-proxy)
      USE_PROXY=true
      ;;
    --without-proxy)
      USE_PROXY=false
      ;;
    -h|--help)
      cat <<'EOF'
Usage: ./scripts/dev-down.sh [--with-proxy|--without-proxy]

Default: runs without proxy environment variables.
EOF
      exit 0
      ;;
    *)
      echo "[dev-down] Unknown argument: $arg"
      exit 1
      ;;
  esac
done

run_cmd() {
  if [[ "$USE_PROXY" == "true" ]]; then
    "$@"
  else
    env -u https_proxy -u http_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u NO_PROXY -u no_proxy -u PXY_FAB_FONC "$@"
  fi
}

if [[ "$USE_PROXY" == "true" ]]; then
  echo "[dev-down] Proxy mode: enabled"
else
  echo "[dev-down] Proxy mode: disabled (default)"
fi

echo "[dev-down] Stopping compose stack..."
run_cmd docker compose down

echo "[dev-down] Stack stopped."