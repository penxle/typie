// swift-tools-version: 6.2

import PackageDescription

let package = Package(
  name: "EditorFFI",
  platforms: [.iOS(.v18)],
  products: [
    .library(name: "EditorBindings", targets: ["EditorBindings"])
  ],
  targets: [
    .binaryTarget(name: "EditorFFI", path: "Editor.xcframework"),
    .target(
      name: "EditorBindings",
      dependencies: ["EditorFFI"],
      resources: [.copy("Resources/icu.zst")],
      swiftSettings: [.swiftLanguageMode(.v5)]
    ),
  ]
)
