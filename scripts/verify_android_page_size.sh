#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly MIN_ALIGNMENT=$((16 * 1024))
readonly JNILIBS_DIR="${1:-bindings/android/jniLibs}"

find_llvm_readelf() {
    if [[ -n "${LLVM_READELF:-}" && -x "$LLVM_READELF" ]]; then
        printf '%s\n' "$LLVM_READELF"
        return
    fi

    local ndk_root="${ANDROID_NDK_HOME:-${NDK_HOME:-}}"
    if [[ -z "$ndk_root" && -n "${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}" ]]; then
        local sdk_root="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
        local ndk_version
        ndk_version="$(tr -d '[:space:]' < "$REPOSITORY_DIR/.ndk-version")"
        ndk_root="$sdk_root/ndk/$ndk_version"
    fi
    if [[ -n "$ndk_root" ]]; then
        local candidate
        candidate="$(find "$ndk_root/toolchains/llvm/prebuilt" -path '*/bin/llvm-readelf' -perm -111 -print -quit 2>/dev/null || true)"
        if [[ -n "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return
        fi
    fi

    if command -v llvm-readelf >/dev/null 2>&1; then
        command -v llvm-readelf
        return
    fi

    echo "Error: llvm-readelf was not found. Set LLVM_READELF or ANDROID_NDK_HOME." >&2
    return 1
}

LLVM_READELF_BIN="$(find_llvm_readelf)"
readonly LLVM_READELF_BIN

if [[ ! -d "$JNILIBS_DIR" ]]; then
    echo "Error: JNI library directory not found: $JNILIBS_DIR" >&2
    exit 1
fi

shopt -s nullglob
libraries=(
    "$JNILIBS_DIR"/arm64-v8a/*.so
    "$JNILIBS_DIR"/x86_64/*.so
)

if (( ${#libraries[@]} == 0 )); then
    echo "Error: no 64-bit Android shared libraries found under $JNILIBS_DIR." >&2
    exit 1
fi

failed=0
for library in "${libraries[@]}"; do
    load_lines="$("$LLVM_READELF_BIN" -lW "$library" | awk '$1 == "LOAD"')"

    if [[ -z "$load_lines" ]]; then
        echo "Error: no ELF LOAD segments found in $library." >&2
        failed=1
        continue
    fi

    echo "$library"
    while IFS= read -r line; do
        echo "  $line"
        alignment="${line##* }"
        if (( alignment < MIN_ALIGNMENT )); then
            printf 'Error: %s contains a LOAD segment aligned to %s; expected 0x4000 or greater.\n' \
                "$library" "$alignment" >&2
            failed=1
        fi
    done <<< "$load_lines"
done

if (( failed != 0 )); then
    exit 1
fi

echo "All 64-bit Android ELF LOAD segments are aligned to 0x4000 or greater."
