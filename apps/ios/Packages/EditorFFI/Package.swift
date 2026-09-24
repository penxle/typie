// swift-tools-version: 6.2

import PackageDescription

let package = Package(
  name: "EditorFFI",
  platforms: [.iOS(.v18), .macOS(.v15)],
  products: [
    .library(name: "EditorFFI", targets: ["EditorFFI"])
  ],
  targets: [
    .binaryTarget(name: "CEditorFFI", path: "CEditorFFI.xcframework"),
    .target(
      name: "EditorFFI",
      dependencies: ["CEditorFFI"],
      resources: [.copy("Resources/icu.zst")]
    ),
  ]
)
