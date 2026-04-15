// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "SighFar",
    platforms: [
        .macOS(.v13)
    ],
    targets: [
        .executableTarget(
            name: "SighFar"
        ),
    ],
    swiftLanguageModes: [.v6]
)
