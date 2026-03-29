#!/usr/bin/env bash
set -euo pipefail

USE_PROXY=false
BASE_URL="http://127.0.0.1:8080"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --with-proxy)
      USE_PROXY=true
      shift
      ;;
    --without-proxy)
      USE_PROXY=false
      shift
      ;;
    --base-url)
      if [[ $# -lt 2 ]]; then
        echo "[dev-smoke] Missing value for --base-url"
        exit 1
      fi
      BASE_URL="$2"
      shift 2
      ;;
    -h|--help)
      cat <<'EOF'
Usage: ./scripts/dev-smoke.sh [--with-proxy|--without-proxy] [--base-url URL]

Default:
  --without-proxy
  --base-url http://127.0.0.1:8080
EOF
      exit 0
      ;;
    *)
      echo "[dev-smoke] Unknown argument: $1"
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
  echo "[dev-smoke] Proxy mode: enabled"
else
  echo "[dev-smoke] Proxy mode: disabled (default)"
fi
echo "[dev-smoke] Base URL: $BASE_URL"

api_get() {
  run_cmd curl -fsS "$BASE_URL$1"
}

api_post() {
  local path="$1"
  local body="${2:-}"
  if [[ -n "$body" ]]; then
    run_cmd curl -fsS -X POST "$BASE_URL$path" -H "content-type: application/json" -d "$body"
  else
    run_cmd curl -fsS -X POST "$BASE_URL$path"
  fi
}

json_get() {
  local expr="$1"
  python3 -c "import json,sys; data=json.load(sys.stdin); print($expr)"
}

echo "[dev-smoke] Checking health endpoints..."
api_get /health >/dev/null
api_get /ready >/dev/null

job_name="smoke-$(date +%s)"
create_payload="{\"name\":\"$job_name\",\"repository_url\":\"https://example.com/smoke.git\",\"pipeline_path\":\"pipelines/smoke.yml\"}"

echo "[dev-smoke] Creating job: $job_name"
create_response="$(api_post /jobs "$create_payload")"
job_id="$(printf '%s' "$create_response" | json_get 'data["job"]["id"]')"

if [[ -z "$job_id" ]]; then
  echo "[dev-smoke] Could not parse job id from response"
  exit 1
fi

echo "[dev-smoke] Running job: $job_id"
run_response="$(api_post "/jobs/$job_id/run")"
build_id="$(printf '%s' "$run_response" | json_get 'data["build"]["id"]')"

if [[ -z "$build_id" ]]; then
  echo "[dev-smoke] Could not parse build id from response"
  exit 1
fi

echo "[dev-smoke] Checking workers endpoint..."
workers_response="$(api_get /workers)"
worker_count="$(printf '%s' "$workers_response" | json_get 'len(data["workers"])')"
echo "[dev-smoke] Workers visible: $worker_count"

echo "[dev-smoke] Waiting for build completion: $build_id"
for _ in $(seq 1 60); do
  builds_response="$(api_get /builds)"
  status="$(printf '%s' "$builds_response" | python3 -c "import json,sys; data=json.load(sys.stdin); bid='$build_id'; print(next((b['status'] for b in data['builds'] if b['id']==bid), 'missing'))" | tr '[:upper:]' '[:lower:]')"
  if [[ "$status" == "success" ]]; then
    echo "[dev-smoke] Build completed successfully"
    echo "[dev-smoke] OK"
    exit 0
  fi
  if [[ "$status" == "failed" || "$status" == "canceled" || "$status" == "missing" ]]; then
    echo "[dev-smoke] Build reached terminal status: $status"
    exit 1
  fi
  sleep 1
done

echo "[dev-smoke] Timeout waiting for build completion"
exit 1