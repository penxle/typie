// swift-tools-version: 6.1

import PackageDescription

let package = Package(
  name: "Typie",
  platforms: [.iOS(.v18), .macOS(.v15)],
  products: [
    .library(name: "Core", targets: ["Core"]),
    .library(name: "Design", targets: ["Design"]),
    .library(name: "Auth", targets: ["Auth"]),
  ],
  dependencies: [
    .package(url: "https://github.com/apollographql/apollo-ios.git", from: "2.4.0"),
    .package(url: "https://github.com/Alamofire/Alamofire.git", from: "5.12.2"),
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
        "Auth/AuthorizeSingleSignOn.graphql", "Auth/LoginWithEmail.graphql",
        "Dev/ServerProbe.graphql",
      ]
    ),
    .target(name: "Design", resources: [.process("Resources")]),
    .target(name: "Auth", dependencies: ["Core", "Design"]),
    .testTarget(
      name: "CoreTests",
      dependencies: [
        "Core",
        .product(name: "Alamofire", package: "Alamofire"),
        .product(name: "Apollo", package: "apollo-ios"),
      ]),
    .testTarget(name: "DesignTests", dependencies: ["Design"]),
    .testTarget(name: "AuthTests", dependencies: ["Auth", "Core", "Design"]),
  ],
  swiftLanguageModes: [.v6]
)
