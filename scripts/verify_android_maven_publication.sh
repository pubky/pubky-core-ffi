#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <maven-repository> <version>" >&2
    exit 1
fi

readonly REPOSITORY="$1"
readonly VERSION="$2"
readonly ARTIFACT_DIR="$REPOSITORY/org/pubky/pubky-core-android/$VERSION"
readonly AAR="$ARTIFACT_DIR/pubky-core-android-$VERSION.aar"
readonly POM="$ARTIFACT_DIR/pubky-core-android-$VERSION.pom"
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

[[ -f "$POM" ]] || {
    echo "Error: Maven POM not found: $POM" >&2
    exit 1
}

grep -q '<groupId>net.java.dev.jna</groupId>' "$POM" || {
    echo "Error: Maven POM does not declare the JNA dependency." >&2
    exit 1
}
grep -q '<artifactId>jna</artifactId>' "$POM" || {
    echo "Error: Maven POM does not declare the JNA dependency." >&2
    exit 1
}

"$SCRIPT_DIR/verify_android_aar.sh" "$AAR"
echo "Maven publication org.pubky:pubky-core-android:$VERSION is complete."
