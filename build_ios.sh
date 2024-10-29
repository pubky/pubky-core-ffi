#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status.

echo "Starting iOS build process..."

# Remove previous builds and ensure clean state
echo "Cleaning previous builds..."
rm -rf bindings/ios/*
rm -rf ios/

# Create necessary directories
echo "Creating build directories..."
mkdir -p bindings/ios/

# Set iOS deployment target
export IPHONEOS_DEPLOYMENT_TARGET=13.4

# Cargo Build
echo "Building Rust libraries..."
cargo build && cd pubky && cargo build && cd pubky && cargo build && cd ../ && cd pubky-common && cargo build && cd ../ && cd pubky-homeserver && cargo build && cd ../../

# Modify Cargo.toml
echo "Updating Cargo.toml..."
sed -i '' 's/crate_type = .*/crate_type = ["cdylib", "staticlib"]/' Cargo.toml

# Build release
echo "Building release version..."
cargo build --release

# Add iOS targets
echo "Adding iOS targets..."
rustup target add aarch64-apple-ios-sim aarch64-apple-ios

# Build for iOS simulator and device
echo "Building for iOS targets..."
cargo build --release --target=aarch64-apple-ios-sim
cargo build --release --target=aarch64-apple-ios

# Generate Swift bindings
echo "Generating Swift bindings..."
# First, ensure any existing generated files are removed
rm -rf ./bindings/ios/pubkycore.swift
rm -rf ./bindings/ios/pubkycoreFFI.h
rm -rf ./bindings/ios/pubkycoreFFI.modulemap
rm -rf ./bindings/ios/Headers
rm -rf ./bindings/ios/ios-arm64
rm -rf ./bindings/ios/ios-arm64-sim

cargo run --bin uniffi-bindgen generate \
    --library ./target/release/libpubkycore.dylib \
    --language swift \
    --out-dir ./bindings/ios \
    || { echo "Failed to generate Swift bindings"; exit 1; }

# Handle modulemap file
echo "Handling modulemap file..."
if [ -f bindings/ios/pubkycoreFFI.modulemap ]; then
    mv bindings/ios/pubkycoreFFI.modulemap bindings/ios/module.modulemap
else
    echo "Warning: modulemap file not found"
fi

# Clean up any existing XCFramework and temporary directories
echo "Cleaning up existing XCFramework..."
rm -rf "bindings/ios/PubkyCore.xcframework"
rm -rf "bindings/ios/Headers"
rm -rf "bindings/ios/ios-arm64"
rm -rf "bindings/ios/ios-arm64-sim"

# Create temporary directories for each architecture
echo "Creating architecture-specific directories..."
mkdir -p "bindings/ios/ios-arm64/Headers"
mkdir -p "bindings/ios/ios-arm64-sim/Headers"

# Copy headers to architecture-specific directories
echo "Copying headers to architecture directories..."
cp bindings/ios/pubkycoreFFI.h "bindings/ios/ios-arm64/Headers/"
cp bindings/ios/module.modulemap "bindings/ios/ios-arm64/Headers/"
cp bindings/ios/pubkycoreFFI.h "bindings/ios/ios-arm64-sim/Headers/"
cp bindings/ios/module.modulemap "bindings/ios/ios-arm64-sim/Headers/"

# Create XCFramework
echo "Creating XCFramework..."
xcodebuild -create-xcframework \
    -library ./target/aarch64-apple-ios-sim/release/libpubkycore.a -headers "bindings/ios/ios-arm64-sim/Headers" \
    -library ./target/aarch64-apple-ios/release/libpubkycore.a -headers "bindings/ios/ios-arm64/Headers" \
    -output "bindings/ios/PubkyCore.xcframework" \
    || { echo "Failed to create XCFramework"; exit 1; }

# Clean up temporary directories
echo "Cleaning up temporary directories..."
rm -rf "bindings/ios/ios-arm64"
rm -rf "bindings/ios/ios-arm64-sim"

echo "iOS build process completed successfully!"