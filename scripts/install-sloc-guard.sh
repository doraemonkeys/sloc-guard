#!/usr/bin/env bash
# sloc-guard pre-commit hook installer and runner
# Downloads pre-built binary from GitHub Releases with checksum verification
set -euo pipefail

# Configuration
TOOL_NAME="sloc-guard"
CACHE_DIR="${HOME}/.cache/${TOOL_NAME}"
REPO="doraemonkeys/sloc-guard"

# Detect OS and architecture, return Rust target triple
detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "${os}" in
        Linux)
            case "${arch}" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                arm64)   echo "aarch64-unknown-linux-gnu" ;;
                *)       echo "" ;;
            esac
            ;;
        Darwin)
            case "${arch}" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                aarch64) echo "aarch64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *)       echo "" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            case "${arch}" in
                x86_64) echo "x86_64-pc-windows-msvc" ;;
                *)      echo "" ;;
            esac
            ;;
        *)
            echo ""
            ;;
    esac
}

# Get version from pre-commit rev or resolve latest
get_version() {
    # Pre-commit passes the revision as PRE_COMMIT_CONFIG_REPO_REV when available
    # Fall back to 'latest' if not set
    local version="${PRE_COMMIT_CONFIG_REPO_REV:-latest}"

    # Strip 'v' prefix if present
    echo "${version#v}"
}

# Download with exponential backoff retry
download_with_retry() {
    local url="$1" output="$2"
    local max_retries=3 delay=2

    for ((i=1; i<=max_retries; i++)); do
        if curl -fsSL "$url" -o "$output" 2>/dev/null; then
            return 0
        fi
        if [ $i -lt $max_retries ]; then
            echo "Download failed, retrying in ${delay}s..." >&2
            sleep $delay
            delay=$((delay * 2))
        fi
    done
    return 1
}

# Verify SHA256 checksum
verify_checksum() {
    local file="$1" sums_file="$2"
    local filename expected actual

    filename="$(basename "$file")"
    expected="$(grep "$filename" "$sums_file" 2>/dev/null | cut -d' ' -f1)"

    if [ -z "$expected" ]; then
        echo "Checksum not found for $filename" >&2
        return 1
    fi

    if command -v sha256sum &>/dev/null; then
        actual="$(sha256sum "$file" | cut -d' ' -f1)"
    elif command -v shasum &>/dev/null; then
        actual="$(shasum -a 256 "$file" | cut -d' ' -f1)"
    else
        echo "No sha256sum or shasum available, skipping verification" >&2
        return 0
    fi

    if [ "$expected" = "$actual" ]; then
        return 0
    else
        echo "Checksum mismatch: expected $expected, got $actual" >&2
        return 1
    fi
}

# Resolve 'latest' version from GitHub API
resolve_latest_version() {
    local version
    version="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
        | grep '"tag_name"' | sed -E 's/.*"tag_name": *"v?([^"]+)".*/\1/' || echo "")"
    echo "$version"
}

# Install binary from GitHub Releases
install_binary() {
    local version="$1" target="$2" binary_path="$3"

    mkdir -p "$(dirname "$binary_path")"

    # Determine archive extension
    local archive_ext
    case "$target" in
        *windows*) archive_ext="zip" ;;
        *)         archive_ext="tar.gz" ;;
    esac

    local binary_name="${TOOL_NAME}-${target}.${archive_ext}"
    local base_url="https://github.com/${REPO}/releases/download/v${version}"
    local binary_url="${base_url}/${binary_name}"
    local sums_url="${base_url}/SHA256SUMS"

    # Create temp directory
    local temp_dir
    temp_dir="$(mktemp -d)"
    # shellcheck disable=SC2064
    trap "rm -rf '$temp_dir'" EXIT

    # Download archive
    echo "Downloading ${TOOL_NAME} v${version} for ${target}..." >&2
    if ! download_with_retry "$binary_url" "$temp_dir/$binary_name"; then
        echo "Failed to download binary from $binary_url" >&2
        return 1
    fi

    # Download checksums
    if ! download_with_retry "$sums_url" "$temp_dir/SHA256SUMS"; then
        echo "Failed to download checksums, skipping verification" >&2
    else
        # Verify checksum
        if ! verify_checksum "$temp_dir/$binary_name" "$temp_dir/SHA256SUMS"; then
            return 1
        fi
    fi

    # Extract archive
    local extracted_dir="$temp_dir/extracted"
    mkdir -p "$extracted_dir"

    case "$archive_ext" in
        zip)
            unzip -q "$temp_dir/$binary_name" -d "$extracted_dir"
            ;;
        tar.gz)
            tar -xzf "$temp_dir/$binary_name" -C "$extracted_dir"
            ;;
    esac

    # Find and install binary
    local found_binary
    case "$target" in
        *windows*)
            found_binary="$(find "$extracted_dir" -name "${TOOL_NAME}.exe" -type f | head -1)"
            ;;
        *)
            found_binary="$(find "$extracted_dir" -name "${TOOL_NAME}" -type f | head -1)"
            ;;
    esac

    if [ -z "$found_binary" ] || [ ! -f "$found_binary" ]; then
        echo "Binary not found in archive" >&2
        return 1
    fi

    cp "$found_binary" "$binary_path"
    chmod +x "$binary_path"
    echo "Installed ${TOOL_NAME} v${version}" >&2
}

# Fallback to cargo install
install_via_cargo() {
    local version="$1"
    echo "Falling back to cargo install..." >&2

    if ! command -v cargo &>/dev/null; then
        echo "Error: cargo not found. Please install Rust or download a pre-built binary." >&2
        return 1
    fi

    if [ "$version" = "latest" ]; then
        cargo install --git "https://github.com/${REPO}" --locked
    else
        cargo install --git "https://github.com/${REPO}" --tag "v${version}" --locked
    fi
}

# Main execution
main() {
    local target version binary_path

    target="$(detect_target)"
    version="$(get_version)"

    # Resolve latest version if needed
    if [ "$version" = "latest" ]; then
        version="$(resolve_latest_version)"
        if [ -z "$version" ]; then
            echo "Could not determine latest version" >&2
            exit 1
        fi
    fi

    # Determine binary path in cache
    binary_path="${CACHE_DIR}/${version}/${TOOL_NAME}"
    case "$target" in
        *windows*) binary_path="${binary_path}.exe" ;;
    esac

    # Install if not cached
    if [ ! -x "$binary_path" ]; then
        if [ -z "$target" ]; then
            echo "Unsupported platform: $(uname -s)/$(uname -m)" >&2
            if ! install_via_cargo "$version"; then
                exit 1
            fi
            # After cargo install, binary should be in PATH
            binary_path="${TOOL_NAME}"
        else
            if ! install_binary "$version" "$target" "$binary_path"; then
                echo "Binary installation failed, trying cargo..." >&2
                if ! install_via_cargo "$version"; then
                    exit 1
                fi
                binary_path="${TOOL_NAME}"
            fi
        fi
    fi

    # Execute sloc-guard with --files for pure incremental mode
    # If no files provided, run without --files (let sloc-guard use defaults)
    if [ $# -eq 0 ]; then
        exec "$binary_path" check
    else
        exec "$binary_path" check --files "$@"
    fi
}

main "$@"
