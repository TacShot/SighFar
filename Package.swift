// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "SighFar",
    dependencies: [
        .package(url: "https://github.com/apple/swift-crypto.git", from: "3.0.0"),
    ],
    targets: [
        // Core library — all cipher/history/UI logic; tested independently of the entry point.
        // NOTE: No Timer, polling loop, or busy-wait must ever be added here.
        // The terminal loop blocks on readLine() which correctly yields to the OS scheduler.
        .target(
            name: "SighFarCore",
            dependencies: [
                .product(name: "Crypto", package: "swift-crypto"),
            ]
        ),
        // Thin entry-point executable that delegates to SighFarCore.
        .executableTarget(
            name: "SighFar",
            dependencies: ["SighFarCore"]
        ),
        // Unit-test target — uses @testable import to reach internal symbols.
        .testTarget(
            name: "SighFarTests",
            dependencies: [
                "SighFarCore",
                .product(name: "Crypto", package: "swift-crypto"),
            ]
        ),
    ],
    swiftLanguageModes: [.v6]
)
