// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "Typie",
  platforms: [.iOS(.v18), .macOS(.v15)],
  products: [
    .library(name: "Core", targets: ["Core"]),
    .library(name: "Design", targets: ["Design"]),
  ],
  targets: [
    .target(name: "Core"),
    .target(name: "Design", resources: [.process("Resources")]),
    .testTarget(name: "CoreTests", dependencies: ["Core"]),
    .testTarget(name: "DesignTests", dependencies: ["Design"]),
  ],
  swiftLanguageModes: [.v6]
)
