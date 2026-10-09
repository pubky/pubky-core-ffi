#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly DIST_DIR="${DIST_DIR:-$REPOSITORY_DIR/dist}"
readonly BINDINGS_DIR="$REPOSITORY_DIR/bindings/ios"
readonly MANUAL_OUTPUT="$DIST_DIR/pubky-core-ffi-ios.zip"
readonly SWIFTPM_OUTPUT="$DIST_DIR/PubkyCore.xcframework.zip"
readonly STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pubky-core-ios.XXXXXX")"

cleanup() {
    rm -rf "$STAGING_DIR"
}
trap cleanup EXIT

[[ -f "$BINDINGS_DIR/pubkycore.swift" ]] || {
    echo "Error: Swift bindings have not been generated." >&2
    exit 1
}
[[ -d "$BINDINGS_DIR/PubkyCore.xcframework" ]] || {
    echo "Error: the iOS XCFramework has not been built." >&2
    exit 1
}

create_reproducible_zip() {
    local source_dir="$1"
    local output="$2"

    # ZIP timestamps start in 1980. Normalizing archive metadata avoids changes
    # when packaging the same XCFramework twice. The compiled XCFramework itself
    # is not reproducible across build machines, so releases must upload the exact
    # archive whose checksum is committed to Package.swift.
    find "$source_dir" -depth -exec touch -h -t 198001010000 {} +
    (
        cd "$source_dir"
        find . -mindepth 1 -print | LC_ALL=C sort | zip -X -q "$output" -@
    )
}

mkdir -p "$DIST_DIR" "$STAGING_DIR/manual" "$STAGING_DIR/swiftpm"
rm -f "$MANUAL_OUTPUT" "$SWIFTPM_OUTPUT"

cp "$BINDINGS_DIR/pubkycore.swift" "$STAGING_DIR/manual/"
cp -R "$BINDINGS_DIR/PubkyCore.xcframework" "$STAGING_DIR/manual/"
cp -R "$BINDINGS_DIR/PubkyCore.xcframework" "$STAGING_DIR/swiftpm/"

create_reproducible_zip "$STAGING_DIR/manual" "$MANUAL_OUTPUT"
create_reproducible_zip "$STAGING_DIR/swiftpm" "$SWIFTPM_OUTPUT"

echo "Created $MANUAL_OUTPUT"
echo "Created $SWIFTPM_OUTPUT"
