#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status.

echo "Starting Python build process..."

# Define output directories
BASE_DIR="./bindings/python"
PACKAGE_DIR="$BASE_DIR/pubkycore"

# Create output directories
mkdir -p "$BASE_DIR"
mkdir -p "$PACKAGE_DIR"

# Remove previous build
echo "Removing previous build..."
# shellcheck disable=SC2115
rm -rf "$PACKAGE_DIR"/*

# Cargo Build
echo "Building Rust libraries..."
cargo build && cd pubky && cargo build && cd pubky && cargo build && cd ../ && cd pubky-common && cargo build && cd ../ && cd pubky-homeserver && cargo build && cd ../../

# Modify Cargo.toml to ensure correct crate type
echo "Updating Cargo.toml..."
sed -i '' 's/crate_type = .*/crate_type = ["cdylib"]/' Cargo.toml

# Build release
echo "Building release version..."
cargo build --release

# Generate Python bindings
echo "Generating Python bindings..."
LIBRARY_PATH="./target/release/libpubkycore.dylib"

# Check if the library file exists
if [ ! -f "$LIBRARY_PATH" ]; then
    echo "Error: Library file not found at $LIBRARY_PATH"
    echo "Available files in target/release:"
    ls -l ./target/release/
    exit 1
fi

# Generate the Python bindings
cargo run --bin uniffi-bindgen generate \
    --library "$LIBRARY_PATH" \
    --language python \
    --out-dir "$PACKAGE_DIR"

# Create __init__.py
touch "$PACKAGE_DIR/__init__.py"

# Create setup.py
cat > "$BASE_DIR/setup.py" << EOL
from setuptools import setup, find_packages

setup(
    name="pubkycore",
    version="0.1.0",
    packages=find_packages(),
    package_data={
        "pubkycore": ["*.so", "*.dylib", "*.dll"],
    },
    install_requires=[],
    author="Pubky",
    author_email="",
    description="Python bindings for the Pubky Mobile SDK",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.6",
)
EOL

# Create README.md
cat > "$BASE_DIR/README.md" << EOL
# Pubky Mobile Python Bindings

Python bindings for the Pubky Mobile SDK.

## Installation

\`\`\`bash
pip install .
\`\`\`

## Usage

\`\`\`python
from pubkycore import *

# Generate a new keypair
result = generate_secret_key()
if result[0] == "success":
    print(f"Generated key: {result[1]}")
else:
    print(f"Error: {result[1]}")
\`\`\`
EOL

# Copy necessary library files
echo "Copying library files..."
case "$(uname)" in
    "Darwin")
        cp "$LIBRARY_PATH" "$PACKAGE_DIR/"
        ;;
    "Linux")
        cp "./target/release/libpubkycore.so" "$PACKAGE_DIR/"
        ;;
    "MINGW"*|"MSYS"*|"CYGWIN"*)
        cp "./target/release/pubkycore.dll" "$PACKAGE_DIR/"
        ;;
esac

echo "Python build process completed successfully!"
echo "To install the package, cd into $BASE_DIR and run: pip install ."