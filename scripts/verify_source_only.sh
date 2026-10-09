#!/usr/bin/env bash

set -euo pipefail

failed=0

while IFS= read -r -d '' path; do
    case "$path" in
        target/* | \
        dist/* | \
        android/.gradle/* | \
        android/.kotlin/* | \
        android/*/build/* | \
        bindings/android/pubkycore.kt | \
        bindings/android/jniLibs/* | \
        bindings/ios/*.xcframework/* | \
        bindings/python/pubkycore/*.so | \
        bindings/python/pubkycore/*.dylib | \
        bindings/python/pubkycore/*.dll)
            echo "Error: generated build output is tracked: $path" >&2
            failed=1
            ;;
    esac
done < <(git ls-files -z)

if (( failed != 0 )); then
    echo "Build outputs belong in release artifacts, not Git." >&2
    exit 1
fi

echo "Repository contains source files only."
