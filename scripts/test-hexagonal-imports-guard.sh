#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
guard_script="$repo_root/scripts/check-hexagonal-imports.sh"

tmp_root="$(mktemp -d)"
cleanup() {
    rm -rf "$tmp_root"
}
trap cleanup EXIT

write_case_src_file() {
    local case_root="$1"
    local rel_path="$2"
    local body="$3"

    mkdir -p "$(dirname "$case_root/$rel_path")"
    cat > "$case_root/$rel_path" <<EOF
$body
EOF
}

run_case_expect_pass() {
    local case_name="$1"

    local case_root="$tmp_root/$case_name"
    mkdir -p "$case_root/crates"

    # Composition root import is allowed.
    write_case_src_file "$case_root" "crates/server/src/main.rs" 'use tardigrade_storage::adapters::InMemoryStorage;'

    if ! bash "$guard_script" "$case_root" >/dev/null; then
        echo "[hex-import-guard-test] expected pass, got failure: $case_name" >&2
        exit 1
    fi
}

run_case_expect_fail() {
    local case_name="$1"

    local case_root="$tmp_root/$case_name"
    mkdir -p "$case_root/crates"

    # Non-composition-root adapter import must be rejected.
    write_case_src_file "$case_root" "crates/api/src/state/api_state.rs" 'use tardigrade_scheduler::adapters::InMemoryScheduler;'

    if bash "$guard_script" "$case_root" >/dev/null 2>&1; then
        echo "[hex-import-guard-test] expected failure, got pass: $case_name" >&2
        exit 1
    fi
}

run_case_expect_pass "allow-server-composition-root"
run_case_expect_fail "forbid-api-adapter-import"

# Source-level tests are no longer in allowlist and must be rejected too.
run_case_expect_fail "forbid-source-level-test-adapter-import"

case_root="$tmp_root/forbid-source-level-test-adapter-import"
mkdir -p "$case_root/crates"
write_case_src_file "$case_root" "crates/server/src/webhook_adapter_tests.rs" 'use tardigrade_storage::adapters::InMemoryStorage;'
if bash "$guard_script" "$case_root" >/dev/null 2>&1; then
    echo "[hex-import-guard-test] expected failure, got pass: forbid-source-level-test-adapter-import" >&2
    exit 1
fi

echo "[hex-import-guard-test] Regression scenarios passed."
