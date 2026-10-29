#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly MANIFEST="$REPOSITORY_DIR/Package.swift"
readonly ARCHIVE="${1:-$REPOSITORY_DIR/dist/PubkyCore.xcframework.zip}"

[[ -f "$MANIFEST" ]] || {
    echo "Error: Package.swift does not exist." >&2
    exit 1
}
[[ -f "$ARCHIVE" ]] || {
    echo "Error: SwiftPM archive not found at $ARCHIVE." >&2
    exit 1
}

version="$(awk -F ' *= *' '/^\[package\]/{package=1; next} package && /^version *=/{gsub(/"/, "", $2); print $2; exit}' "$REPOSITORY_DIR/Cargo.toml")"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]] || {
    echo "Error: unsupported Cargo package version: $version" >&2
    exit 1
}

checksum="$(swift package compute-checksum "$ARCHIVE")"

VERSION="$version" CHECKSUM="$checksum" perl -0pi -e '
    s/let tag = "[^"]+"/let tag = "v$ENV{VERSION}"/;
    s/let checksum = "[0-9a-f]+"/let checksum = "$ENV{CHECKSUM}"/;
' "$MANIFEST"

echo "Updated Package.swift for v$version ($checksum)."
