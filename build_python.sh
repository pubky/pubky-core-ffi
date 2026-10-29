#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Maturin generates matching UniFFI Python bindings and bundles the native library.
# Rust is only needed by maintainers and source builds.
python3 -m maturin build --release --locked --out dist/python "$@"
