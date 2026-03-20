// swift-tools-version: 5.9
import PackageDescription
let package = Package(
  name: "_internal_linkage_SwiftPMImport",
  platforms: [
    .iOS("15.6")
  ],
  products: [
    .library(
      name: "_internal_linkage_SwiftPMImport",
      type: .none,
      targets: ["_internal_linkage_SwiftPMImport"]
    )
  ],
  dependencies: [
    .package(
      path: "../Bridge",
    )
  ],
  targets: [
    .target(
      name: "_internal_linkage_SwiftPMImport",
      dependencies: [
        .product(
          name: "Bridge",
          package: "Bridge",
        )
      ]
    )
  ]
)
