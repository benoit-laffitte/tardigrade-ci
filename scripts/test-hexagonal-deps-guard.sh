#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
guard_script="$repo_root/scripts/check-hexagonal-deps.sh"

tmp_root="$(mktemp -d)"
cleanup() {
    rm -rf "$tmp_root"
}
trap cleanup EXIT

write_case_file() {
    local case_root="$1"
    local crate="$2"
    local body="$3"

    mkdir -p "$case_root/crates/$crate"
    cat > "$case_root/crates/$crate/Cargo.toml" <<EOF
[package]
name = "tardigrade-$crate"
version = "0.1.0"
edition = "2024"

[dependencies]
$body
EOF
}

run_case_expect_pass() {
    local case_name="$1"
    shift

    local case_root="$tmp_root/$case_name"
    mkdir -p "$case_root/crates"

    while [[ $# -gt 0 ]]; do
        local crate="$1"
        local deps="$2"
        write_case_file "$case_root" "$crate" "$deps"
        shift 2
    done

    if ! bash "$guard_script" "$case_root" >/dev/null; then
        echo "[hex-guard-test] expected pass, got failure: $case_name" >&2
        exit 1
    fi
}

run_case_expect_fail() {
    local case_name="$1"
    shift

    local case_root="$tmp_root/$case_name"
    mkdir -p "$case_root/crates"

    while [[ $# -gt 0 ]]; do
        local crate="$1"
        local deps="$2"
        write_case_file "$case_root" "$crate" "$deps"
        shift 2
    done

    if bash "$guard_script" "$case_root" >/dev/null 2>&1; then
        echo "[hex-guard-test] expected failure, got pass: $case_name" >&2
        exit 1
    fi
}

# Baseline valid topology should pass.
run_case_expect_pass \
    "valid-pragmatic-edges" \
    "core" "" \
    "storage" "tardigrade-core = { path = \"../core\" }" \
    "scheduler" "tardigrade-core = { path = \"../core\" }" \
    "application" "tardigrade-core = { path = \"../core\" }\ntardigrade-storage = { path = \"../storage\" }\ntardigrade-scheduler = { path = \"../scheduler\" }" \
    "api" "tardigrade-application = { path = \"../application\" }\ntardigrade-core = { path = \"../core\" }" \
    "server" "tardigrade-api = { path = \"../api\" }" \
    "worker" "tardigrade-core = { path = \"../core\" }"

# Domain must not depend on adapters.
run_case_expect_fail \
    "forbid-core-to-api" \
    "core" "tardigrade-api = { path = \"../api\" }" \
    "api" ""

# Worker to API is only tolerated for optional benchmark edge.
run_case_expect_fail \
    "forbid-worker-to-api-non-optional" \
    "worker" "tardigrade-api = { path = \"../api\" }" \
    "api" ""

# Optional benchmark edge remains accepted by policy.
run_case_expect_pass \
    "allow-worker-to-api-optional" \
    "worker" "tardigrade-api = { path = \"../api\", optional = true }" \
    "api" ""

echo "[hex-guard-test] Regression scenarios passed."
