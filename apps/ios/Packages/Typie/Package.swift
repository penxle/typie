// swift-tools-version: 6.1

import PackageDescription

let package = Package(
  name: "Typie",
  platforms: [.iOS(.v18), .macOS(.v15)],
  products: [
    .library(name: "Core", targets: ["Core"]),
    .library(name: "Design", targets: ["Design"]),
    .library(name: "Auth", targets: ["Auth"]),
    .library(name: "Home", targets: ["Home"]),
  ],
  dependencies: [
    .package(url: "https://github.com/apollographql/apollo-ios.git", from: "2.4.0"),
    .package(url: "https://github.com/Alamofire/Alamofire.git", from: "5.12.2"),
    .package(url: "https://github.com/kean/Nuke.git", from: "13.2.0"),
  ],
  targets: [
    .target(name: "GraphQL", dependencies: [.product(name: "ApolloAPI", package: "apollo-ios")]),
    .target(
      name: "Core",
      dependencies: [
        "GraphQL",
        .product(name: "Apollo", package: "apollo-ios"),
        .product(name: "Alamofire", package: "Alamofire"),
      ],
      exclude: [
        "Auth/EmailLogin.graphql", "Auth/SingleSignOnLogin.graphql",
        "Dev/ServerProbe.graphql", "Entity/EntityRow.graphql", "Home/Search.graphql",
        "Home/SpaceSwitcher.graphql", "Image/TImage.graphql",
      ]
    ),
    .target(
      name: "Design",
      dependencies: [
        .product(name: "Nuke", package: "Nuke"),
        .product(name: "NukeUI", package: "Nuke"),
      ],
      resources: [.process("Resources")]
    ),
    .target(name: "Auth", dependencies: ["Core", "Design"]),
    .target(name: "Home", dependencies: ["Core", "Design"]),
    .testTarget(
      name: "CoreTests",
      dependencies: [
        "Core",
        .product(name: "Alamofire", package: "Alamofire"),
        .product(name: "Apollo", package: "apollo-ios"),
      ]),
    .testTarget(name: "DesignTests", dependencies: ["Design"]),
    .testTarget(name: "AuthTests", dependencies: ["Auth", "Core", "Design"]),
    .testTarget(
      name: "HomeTests",
      dependencies: [
        "Home", "Core", "Design",
        .product(name: "Apollo", package: "apollo-ios"),
      ]),
  ],
  swiftLanguageModes: [.v6]
)
