#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

if [[ ! -d "$repo_root/crates" ]]; then
    echo "[hex-guard] Invalid repository root: missing crates/ under '$repo_root'." >&2
    exit 1
fi

# Collect workspace crate names from crates/* folders.
declare -A workspace_crates=()
for crate_dir in "$repo_root"/crates/*; do
    [[ -d "$crate_dir" ]] || continue
    workspace_crates["$(basename "$crate_dir")"]=1
done

declare -a violations=()

is_allowed_edge() {
    local from="$1"
    local to="$2"
    local optional="$3"

    case "$from" in
        core)
            # Domain must not depend on adapter/infrastructure crates.
            return 1
            ;;
        storage)
            [[ "$to" == "core" ]] && return 0 || return 1
            ;;
        scheduler)
            [[ "$to" == "core" ]] && return 0 || return 1
            ;;
        plugins)
            return 1
            ;;
        auth)
            return 1
            ;;
        api)
            case "$to" in
                application|core|storage|scheduler|plugins|auth)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
        application)
            case "$to" in
                core|storage|scheduler|plugins|auth)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
        server)
            case "$to" in
                api|application|storage|scheduler)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
        worker)
            case "$to" in
                core)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
        *)
            return 1
            ;;
    esac
}

for cargo_toml in "$repo_root"/crates/*/Cargo.toml; do
    [[ -f "$cargo_toml" ]] || continue

    from_crate="$(basename "$(dirname "$cargo_toml")")"

    while IFS= read -r line; do
        # Parse workspace internal dependencies declared with `tardigrade-<crate> = { ... }`.
        if [[ "$line" =~ ^[[:space:]]*tardigrade-([a-zA-Z0-9_-]+)[[:space:]]*=[[:space:]]*\{([^}]*)\} ]]; then
            to_crate="${BASH_REMATCH[1]}"
            attrs="${BASH_REMATCH[2]}"

            # Only enforce workspace internal edges.
            [[ -n "${workspace_crates[$to_crate]:-}" ]] || continue

            optional="false"
            if [[ "$attrs" =~ optional[[:space:]]*=[[:space:]]*true ]]; then
                optional="true"
            fi

            if ! is_allowed_edge "$from_crate" "$to_crate" "$optional"; then
                violations+=("$from_crate -> $to_crate (optional=$optional) in ${cargo_toml#$repo_root/}")
            fi
        fi
    done < "$cargo_toml"
done

if (( ${#violations[@]} > 0 )); then
    echo "[hex-guard] Forbidden internal dependency edges detected:" >&2
    for violation in "${violations[@]}"; do
        echo "  - $violation" >&2
    done
    echo >&2
    echo "[hex-guard] See ARCHI.md (Convergence hexagonale pragmatique) for allowed edges." >&2
    exit 1
fi

echo "[hex-guard] Dependency policy check passed."
