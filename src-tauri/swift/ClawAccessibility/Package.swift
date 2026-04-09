// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ClawAccessibility",
    platforms: [.macOS(.v10_15)],
    products: [
        .library(
            name: "ClawAccessibility",
            type: .static,
            targets: ["ClawAccessibility"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/Brendonovich/swift-rs", from: "1.0.5")
    ],
    targets: [
        .target(
            name: "ClawAccessibility",
            dependencies: [
                .product(name: "SwiftRs", package: "swift-rs")
            ],
            linkerSettings: [
                .linkedFramework("ApplicationServices"),
                .linkedFramework("AppKit")
            ]
        )
    ]
)
