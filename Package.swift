// swift-tools-version: 5.9

import PackageDescription

let tag = "v0.4.0"
let checksum = "87cb07c94971583b5e8a0ec66a3d27b3d06bd0d29c98317e2c5cf725f21236e9"
let binaryURL = "https://github.com/pubky/pubky-core-ffi/releases/download/\(tag)/PubkyCore.xcframework.zip"

let package = Package(
    name: "pubky-core-ffi",
    platforms: [
        .iOS("13.4"),
    ],
    products: [
        .library(
            name: "PubkyCore",
            targets: ["PubkyCore"]
        ),
    ],
    targets: [
        .target(
            name: "PubkyCore",
            dependencies: ["pubkycoreFFI"],
            path: "bindings/ios",
            exclude: ["module.modulemap", "pubkycoreFFI.h"],
            sources: ["pubkycore.swift"]
        ),
        .binaryTarget(
            name: "pubkycoreFFI",
            url: binaryURL,
            checksum: checksum
        ),
    ]
)
