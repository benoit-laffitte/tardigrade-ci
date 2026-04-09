#!/usr/bin/env bash

set -euo pipefail

# This function prints script usage and available options.
usage() {
	cat <<'EOF'
Usage: ./scripts/package-platform-zips.sh [options]

Create one zip archive per platform (mac, windows, linux) with this structure:
  - bin/      : server binaries
  - config/   : runtime configuration files
  - docs/     : product documentation
  - README.md : install and usage instructions
  - LICENSE.txt

Options:
  --platforms <csv>  Platforms to package (default: mac,windows,linux)
  --out-dir <path>   Output directory for zip files (default: dist)
  --no-build         Reuse existing binaries from target/ and skip cargo build
  --help             Show this help

Environment overrides:
  TARGET_MAC         Rust target for macOS (default: aarch64-apple-darwin)
  TARGET_WINDOWS     Rust target for Windows (default: x86_64-pc-windows-gnu)
  TARGET_LINUX       Rust target for Linux (default: x86_64-unknown-linux-gnu)
EOF
}

# This function prints a prefixed log line to make execution easier to follow.
log() {
	echo "[package-zips] $*"
}

# This function verifies required external commands are available.
require_cmd() {
	local cmd="$1"
	if ! command -v "$cmd" >/dev/null 2>&1; then
		echo "Missing required command: $cmd" >&2
		exit 1
	fi
}

# This function validates user platform names.
validate_platform() {
	local platform="$1"
	case "$platform" in
		mac|windows|linux)
			return 0
			;;
		*)
			echo "Unsupported platform: $platform" >&2
			return 1
			;;
	esac
}

# This function writes the packaged README.md with install/use instructions.
write_package_readme() {
	local package_dir="$1"
	local platform="$2"
	cat >"$package_dir/README.md" <<EOF
# Tardigrade CI (${platform})

## Installation

1. Unzip this archive.
2. Configure runtime values in files inside the config/ directory.
3. Run the server binary from the bin/ directory.

## Usage

- Start server (Linux/macOS):
  ./bin/tardigrade-server

- Start server (Windows PowerShell):
  .\\bin\\tardigrade-server.exe

Optional worker binary is included in the same bin/ directory.

For detailed product documentation, see docs/.
EOF
}

# This function copies expected binaries for one platform into bin/.
copy_platform_binaries() {
	local root_dir="$1"
	local platform="$2"
	local target="$3"
	local destination_dir="$4"
	local extension=""
	local binary=""
	local source_path=""

	if [[ "$platform" == "windows" ]]; then
		extension=".exe"
	fi

	for binary in tardigrade-server tardigrade-worker; do
		source_path="$root_dir/target/$target/release/${binary}${extension}"
		if [[ ! -f "$source_path" ]]; then
			echo "Missing binary for $platform: $source_path" >&2
			echo "Hint: run cargo build --release --target $target -p tardigrade-server -p tardigrade-worker" >&2
			exit 1
		fi
		cp "$source_path" "$destination_dir/"
	done
}

# This function builds the release binaries for all selected targets.
build_targets() {
	local root_dir="$1"
	shift
	local targets=("$@")
	local target=""
	local linux_linker=""
	local -a no_proxy_env=(
		env
		-u https_proxy
		-u http_proxy
		-u HTTPS_PROXY
		-u HTTP_PROXY
		-u ALL_PROXY
		-u NO_PROXY
		-u no_proxy
	)

	for target in "${targets[@]}"; do
		log "Building release binaries for target: $target"
		linux_linker=""
		if [[ "$target" == "x86_64-unknown-linux-gnu" ]] && command -v x86_64-unknown-linux-gnu-gcc >/dev/null 2>&1; then
			# On macOS cross-builds, use GNU Linux linker when available to avoid host clang linker flags mismatch.
			linux_linker="x86_64-unknown-linux-gnu-gcc"
		fi
		(
			cd "$root_dir"
			if [[ -n "$linux_linker" ]]; then
				"${no_proxy_env[@]}" CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$linux_linker" cargo build --release --target "$target" -p tardigrade-server -p tardigrade-worker
			else
				"${no_proxy_env[@]}" cargo build --release --target "$target" -p tardigrade-server -p tardigrade-worker
			fi
		)
	done
}

main() {
	local script_dir
	local root_dir
	local out_dir
	local build_enabled="true"
	local platforms_csv="mac,windows,linux"
	local work_dir
	local platform
	local target
	local package_name
	local package_dir
	local zip_name
	local targets_to_build=()
	local platform_list=()

	declare -A platform_target=(
		[mac]="${TARGET_MAC:-aarch64-apple-darwin}"
		[windows]="${TARGET_WINDOWS:-x86_64-pc-windows-gnu}"
		[linux]="${TARGET_LINUX:-x86_64-unknown-linux-gnu}"
	)

	while [[ $# -gt 0 ]]; do
		case "$1" in
			--platforms)
				platforms_csv="${2:-}"
				shift 2
				;;
			--out-dir)
				out_dir="${2:-}"
				shift 2
				;;
			--no-build)
				build_enabled="false"
				shift
				;;
			--help)
				usage
				exit 0
				;;
			*)
				echo "Unknown option: $1" >&2
				usage
				exit 1
				;;
		esac
	done

	script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
	root_dir="$(cd "$script_dir/.." && pwd)"
	out_dir="${out_dir:-$root_dir/dist}"

	IFS=',' read -r -a platform_list <<<"$platforms_csv"
	if [[ ${#platform_list[@]} -eq 0 ]]; then
		echo "No platforms provided" >&2
		exit 1
	fi

	require_cmd cargo
	require_cmd zip
	require_cmd mktemp

	for platform in "${platform_list[@]}"; do
		platform="$(echo "$platform" | xargs)"
		validate_platform "$platform"
		target="${platform_target[$platform]}"
		targets_to_build+=("$target")
	done

	if [[ "$build_enabled" == "true" ]]; then
		build_targets "$root_dir" "${targets_to_build[@]}"
	fi

	mkdir -p "$out_dir"
	work_dir="$(mktemp -d)"
	trap 'rm -rf "${work_dir:-}"' EXIT

	for platform in "${platform_list[@]}"; do
		platform="$(echo "$platform" | xargs)"
		target="${platform_target[$platform]}"
		package_name="tardigrade-ci-${platform}"
		package_dir="$work_dir/$package_name"

		log "Preparing package for $platform ($target)"
		mkdir -p "$package_dir/bin" "$package_dir/config" "$package_dir/docs"

		copy_platform_binaries "$root_dir" "$platform" "$target" "$package_dir/bin"
		cp -R "$root_dir/config/." "$package_dir/config/"
		cp -R "$root_dir/docs/." "$package_dir/docs/"
		cp "$root_dir/LICENSE" "$package_dir/LICENSE.txt"
		write_package_readme "$package_dir" "$platform"

		zip_name="$out_dir/${package_name}.zip"
		rm -f "$zip_name"
		(
			cd "$work_dir"
			zip -rq "$zip_name" "$package_name"
		)
		log "Created $zip_name"
	done

	log "Done"
}

main "$@"
