#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

if [[ ! -d "$repo_root/crates" ]]; then
    echo "[hex-import-guard] Invalid repository root: missing crates/ under '$repo_root'." >&2
    exit 1
fi

# Only inspect Rust production source files under crates/*/src.
if command -v rg >/dev/null 2>&1; then
    mapfile -t matches < <(
        rg --no-heading --line-number --color never \
            'tardigrade_(storage|scheduler)::.*adapters::' \
            "$repo_root/crates" --glob '*/src/**/*.rs' || true
    )
else
    mapfile -t matches < <(
        grep -RInE 'tardigrade_(storage|scheduler)::.*adapters::' "$repo_root/crates" \
            --include='*.rs' | grep '/src/' || true
    )
fi

is_allowed_file() {
    local abs_file="$1"
    local rel_file="${abs_file#$repo_root/}"

    case "$rel_file" in
        # Composition root can choose concrete outbound adapters.
        crates/server/src/main.rs)
            return 0
            ;;
        # Transitional default constructors still instantiate in-memory adapters.
        crates/api/src/state/api_state.rs)
            return 0
            ;;
        # Source-level test modules may wire in-memory adapters explicitly.
        */src/*_tests.rs)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

declare -a violations=()
for match in "${matches[@]}"; do
    file="${match%%:*}"
    rest="${match#*:}"
    line="${rest%%:*}"
    text="${rest#*:}"

    if ! is_allowed_file "$file"; then
        rel_file="${file#$repo_root/}"
        violations+=("$rel_file:$line:$text")
    fi
done

if (( ${#violations[@]} > 0 )); then
    echo "[hex-import-guard] Forbidden adapter imports detected outside allowlist:" >&2
    for violation in "${violations[@]}"; do
        echo "  - $violation" >&2
    done
    echo >&2
    echo "[hex-import-guard] Allowed files: crates/server/src/main.rs, crates/api/src/state/api_state.rs, */src/*_tests.rs" >&2
    exit 1
fi

echo "[hex-import-guard] Adapter import policy check passed."
