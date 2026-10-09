// swift-tools-version: 5.9

import PackageDescription

let tag = "v0.4.0"
let checksum = "9832b160624366f3a8574f853480a5500961459a0f4260a630e239cda55bbb13"
let binaryURL = "https://github.com/pubky/pubky-core-ffi/releases/download/\(tag)/PubkyCore.xcframework.zip"

let package = Package(
    name: "pubky-core-ffi",
    platforms: [
        .iOS(.v13),
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
