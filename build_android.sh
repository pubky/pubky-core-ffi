#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

readonly BASE_DIR="./bindings/android"
readonly JNILIBS_DIR="$BASE_DIR/jniLibs"
readonly NDK_VERSION_FILE="$SCRIPT_DIR/.ndk-version"
readonly CARGO_NDK_VERSION="3.5.4"

echo "Starting Android build process..."

if [[ -z "${ANDROID_SDK_ROOT:-}" ]]; then
    if [[ -n "${ANDROID_HOME:-}" ]]; then
        export ANDROID_SDK_ROOT="$ANDROID_HOME"
    else
        echo "Error: ANDROID_SDK_ROOT or ANDROID_HOME must point to the Android SDK." >&2
        exit 1
    fi
fi

if [[ ! -f "$NDK_VERSION_FILE" ]]; then
    echo "Error: pinned NDK version file not found: $NDK_VERSION_FILE" >&2
    exit 1
fi

ANDROID_NDK_VERSION="${ANDROID_NDK_VERSION:-$(tr -d '[:space:]' < "$NDK_VERSION_FILE")}"
readonly ANDROID_NDK_VERSION
readonly NDK_MAJOR_VERSION="${ANDROID_NDK_VERSION%%.*}"

if [[ ! "$NDK_MAJOR_VERSION" =~ ^[0-9]+$ ]] || (( NDK_MAJOR_VERSION < 28 )); then
    echo "Error: Android NDK r28 or newer is required for 16 KB ELF alignment; got '$ANDROID_NDK_VERSION'." >&2
    exit 1
fi

readonly PINNED_NDK="$ANDROID_SDK_ROOT/ndk/$ANDROID_NDK_VERSION"
if [[ ! -d "$PINNED_NDK" ]]; then
    echo "Error: Android NDK $ANDROID_NDK_VERSION is not installed at $PINNED_NDK." >&2
    echo "Install it with: sdkmanager \"ndk;$ANDROID_NDK_VERSION\"" >&2
    exit 1
fi

# cargo-ndk honors these variables. Setting both prevents a developer's older
# default NDK (notably r27 and below) from producing 4 KB-aligned binaries.
export ANDROID_NDK_HOME="$PINNED_NDK"
export NDK_HOME="$PINNED_NDK"

if ! cargo ndk --version 2>/dev/null | grep -Fq "cargo-ndk $CARGO_NDK_VERSION"; then
    echo "Installing cargo-ndk $CARGO_NDK_VERSION..."
    cargo install cargo-ndk --version "$CARGO_NDK_VERSION" --locked
fi

echo "Using Android NDK $ANDROID_NDK_VERSION at $ANDROID_NDK_HOME"

echo "Removing previous Android bindings..."
rm -rf "$BASE_DIR"
mkdir -p "$JNILIBS_DIR"

echo "Building host library for UniFFI binding generation..."
cargo build --release

echo "Adding Android Rust targets..."
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    i686-linux-android \
    x86_64-linux-android

echo "Building Rust libraries for all shipped Android ABIs..."
# Keep a content-derived GNU build ID for native crash symbol matching.
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,--build-id=sha1" \
cargo ndk \
    -o "$JNILIBS_DIR" \
    --manifest-path ./Cargo.toml \
    -t armeabi-v7a \
    -t arm64-v8a \
    -t x86 \
    -t x86_64 \
    build --release

case "$(uname -s)" in
    Darwin) HOST_LIBRARY="./target/release/libpubkycore.dylib" ;;
    Linux) HOST_LIBRARY="./target/release/libpubkycore.so" ;;
    *)
        echo "Error: unsupported host for UniFFI binding generation: $(uname -s)" >&2
        exit 1
        ;;
esac

if [[ ! -f "$HOST_LIBRARY" ]]; then
    echo "Error: host library not found at $HOST_LIBRARY" >&2
    exit 1
fi

echo "Generating Kotlin bindings..."
readonly TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cargo run --release --bin uniffi-bindgen generate \
    --library "$HOST_LIBRARY" \
    --language kotlin \
    --out-dir "$TMP_DIR"

readonly GENERATED_KOTLIN="$(find "$TMP_DIR" -name pubkycore.kt -type f -print -quit)"
if [[ -z "$GENERATED_KOTLIN" ]]; then
    echo "Error: UniFFI did not generate pubkycore.kt." >&2
    exit 1
fi

mv "$GENERATED_KOTLIN" "$BASE_DIR/pubkycore.kt"
perl -pi -e 's/[ \t]+$//' "$BASE_DIR/pubkycore.kt"

echo "Android build process completed successfully!"
