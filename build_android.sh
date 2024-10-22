#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status.

echo "Starting Android build process..."

# Define output directories
BASE_DIR="./bindings/android"
JNILIBS_DIR="$BASE_DIR/jniLibs"

# Create output directories
mkdir -p "$BASE_DIR"
mkdir -p "$JNILIBS_DIR"

# Remove previous build
echo "Removing previous build..."
rm -rf bindings/android/

# Cargo Build
echo "Building Rust libraries..."
cargo build && cd pubky && cargo build && cd pubky && cargo build && cd ../ && cd pubky-common && cargo build && cd ../ && cd pubky-homeserver && cargo build && cd ../../

# Modify Cargo.toml
echo "Updating Cargo.toml..."
sed -i '' 's/crate_type = .*/crate_type = ["cdylib"]/' Cargo.toml

# Build release
echo "Building release version..."
cargo build --release

# Install cargo-ndk if not already installed
if ! command -v cargo-ndk &> /dev/null; then
    echo "Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# Add Android targets
echo "Adding Android targets..."
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    i686-linux-android \
    x86_64-linux-android

# Build for all Android architectures
echo "Building for Android architectures..."
cargo ndk \
    -o "$JNILIBS_DIR" \
    --manifest-path ./Cargo.toml \
    -t armeabi-v7a \
    -t arm64-v8a \
    -t x86 \
    -t x86_64 \
    build --release

# Generate Kotlin bindings
echo "Generating Kotlin bindings..."
LIBRARY_PATH="./target/release/libpubkymobile.dylib"

# Check if the library file exists
if [ ! -f "$LIBRARY_PATH" ]; then
    echo "Error: Library file not found at $LIBRARY_PATH"
    echo "Available files in target/release:"
    ls -l ./target/release/
    exit 1
fi

# Create a temporary directory for initial generation
TMP_DIR=$(mktemp -d)

# Generate the bindings to temp directory first
cargo run --bin uniffi-bindgen generate \
    --library "$LIBRARY_PATH" \
    --language kotlin \
    --out-dir "$TMP_DIR"

# Move the Kotlin file from the nested directory to the final location
echo "Moving Kotlin file to final location..."
find "$TMP_DIR" -name "pubkymobile.kt" -exec mv {} "$BASE_DIR/" \;

# Clean up temp directory and any remaining uniffi directories
echo "Cleaning up temporary files..."
rm -rf "$TMP_DIR"
rm -rf "$BASE_DIR/uniffi"

# Verify the file was moved correctly
if [ ! -f "$BASE_DIR/pubkymobile.kt" ]; then
    echo "Error: Kotlin bindings were not moved correctly"
    echo "Contents of $BASE_DIR:"
    ls -la "$BASE_DIR"
    exit 1
fi

echo "Android build process completed successfully!"