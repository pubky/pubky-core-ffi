#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <pubky-core-android.aar>" >&2
    exit 1
fi

readonly AAR="$1"
[[ -f "$AAR" ]] || {
    echo "Error: AAR not found: $AAR" >&2
    exit 1
}

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

unzip -q "$AAR" -d "$TMP_DIR/aar"

for abi in armeabi-v7a arm64-v8a x86 x86_64; do
    library="$TMP_DIR/aar/jni/$abi/libpubkycore.so"
    [[ -f "$library" ]] || {
        echo "Error: AAR is missing $abi/libpubkycore.so" >&2
        exit 1
    }
done

readonly CLASSES_JAR="$TMP_DIR/aar/classes.jar"
[[ -f "$CLASSES_JAR" ]] || {
    echo "Error: AAR is missing classes.jar" >&2
    exit 1
}

jar tf "$CLASSES_JAR" > "$TMP_DIR/classes.txt"

grep -q '^uniffi/pubkycore/' "$TMP_DIR/classes.txt" || {
    echo "Error: AAR is missing the UniFFI Kotlin bindings." >&2
    exit 1
}
grep -q '^org/rustls/platformverifier/CertificateVerifier.class$' "$TMP_DIR/classes.txt" || {
    echo "Error: AAR is missing rustls-platform-verifier JVM support." >&2
    exit 1
}

"$SCRIPT_DIR/verify_android_page_size.sh" "$TMP_DIR/aar/jni"
echo "Android AAR contains all ABIs, required JVM classes, and 16 KB-compatible 64-bit libraries."
