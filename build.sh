#!/bin/bash

# Save as build.sh
case "$1" in
  "ios")
    ./build_ios.sh
    ;;
  "android")
    ./build_android.sh
    ;;
  "all")
    ./build_ios.sh && ./build_android.sh
    ;;
  *)
    echo "Usage: $0 {ios|android|all}"
    exit 1
    ;;
esac