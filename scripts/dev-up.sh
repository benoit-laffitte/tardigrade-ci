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
Usage: ./scripts/dev-up.sh [--with-proxy|--without-proxy]

Default: runs without proxy environment variables.
EOF
      exit 0
      ;;
    *)
      echo "[dev-up] Unknown argument: $arg"
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
  echo "[dev-up] Proxy mode: enabled"
else
  echo "[dev-up] Proxy mode: disabled (default)"
fi

echo "[dev-up] Starting compose stack (build + detached)..."
run_cmd docker compose up --build -d

echo "[dev-up] Waiting for controller readiness on http://127.0.0.1:8080/ready ..."
for _ in $(seq 1 60); do
  if run_cmd curl -fsS http://127.0.0.1:8080/ready >/dev/null; then
    echo "[dev-up] Stack is ready."
    exit 0
  fi
  sleep 1
done

echo "[dev-up] Controller did not become ready in time. Recent logs:"
docker logs --tail 80 tardigrade-controller || true
exit 1