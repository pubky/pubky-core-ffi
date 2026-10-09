#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly MANIFEST="$REPOSITORY_DIR/Package.swift"
readonly ARCHIVE="${1:-$REPOSITORY_DIR/dist/PubkyCore.xcframework.zip}"

[[ -f "$ARCHIVE" ]] || {
    echo "Error: SwiftPM archive not found at $ARCHIVE." >&2
    exit 1
}

cargo_version="$(awk -F ' *= *' '/^\[package\]/{package=1; next} package && /^version *=/{gsub(/"/, "", $2); print $2; exit}' "$REPOSITORY_DIR/Cargo.toml")"
manifest_tag="$(awk -F '"' '/^let tag = /{print $2; exit}' "$MANIFEST")"
expected_checksum="$(awk -F '"' '/^let checksum = /{print $2; exit}' "$MANIFEST")"

[[ "$manifest_tag" == "v$cargo_version" ]] || {
    echo "Error: Package.swift tag $manifest_tag does not match Cargo version v$cargo_version." >&2
    exit 1
}

if [[ "${GITHUB_REF_TYPE:-}" == "tag" && "${GITHUB_REF_NAME:-}" != "$manifest_tag" ]]; then
    echo "Error: Git tag ${GITHUB_REF_NAME:-} does not match Package.swift tag $manifest_tag." >&2
    exit 1
fi

actual_checksum="$(swift package compute-checksum "$ARCHIVE")"
[[ "$actual_checksum" == "$expected_checksum" ]] || {
    echo "Error: SwiftPM checksum mismatch." >&2
    echo "Expected: $expected_checksum" >&2
    echo "Actual:   $actual_checksum" >&2
    echo "Run ./scripts/update_swift_package.sh after rebuilding the archive." >&2
    exit 1
}

unzip -tq "$ARCHIVE" >/dev/null
(cd "$REPOSITORY_DIR" && swift package dump-package >/dev/null)

echo "Swift package v$cargo_version matches $ARCHIVE ($actual_checksum)."
