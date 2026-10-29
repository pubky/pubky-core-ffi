#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pubky-swiftpm-build.XXXXXX")"
trap 'rm -rf "$TEMP_DIR"' EXIT

mkdir -p "$TEMP_DIR/bindings/ios"
cp "$REPOSITORY_DIR/Package.swift" "$TEMP_DIR/Package.swift"
cp "$REPOSITORY_DIR/bindings/ios/pubkycore.swift" "$TEMP_DIR/bindings/ios/"
cp "$REPOSITORY_DIR/bindings/ios/pubkycoreFFI.h" "$TEMP_DIR/bindings/ios/"
cp "$REPOSITORY_DIR/bindings/ios/module.modulemap" "$TEMP_DIR/bindings/ios/"
cp -R "$REPOSITORY_DIR/bindings/ios/PubkyCore.xcframework" "$TEMP_DIR/"

# Exercise the real manifest with this build's local binary. Release checksums
# remain local-release checks and are never compared against CI rebuilds.
perl -0pi -e 's/url: binaryURL,\s*checksum: checksum/path: "PubkyCore.xcframework"/' "$TEMP_DIR/Package.swift"
cd "$TEMP_DIR"
xcodebuild -scheme pubky-core-ffi -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath "$TEMP_DIR/DerivedData" CODE_SIGNING_ALLOWED=NO build
