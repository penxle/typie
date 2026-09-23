// swift-tools-version: 6.1

import PackageDescription

let package = Package(
  name: "Typie",
  platforms: [.iOS(.v18), .macOS(.v15)],
  products: [
    .library(name: "Core", targets: ["Core"]),
    .library(name: "Design", targets: ["Design"]),
    .library(name: "Features", targets: ["Features"]),
    .library(name: "Platform", targets: ["Platform"]),
  ],
  dependencies: [
    .package(url: "https://github.com/apollographql/apollo-ios.git", from: "2.4.0"),
    .package(url: "https://github.com/Alamofire/Alamofire.git", from: "5.12.2"),
    .package(url: "https://github.com/kean/Nuke.git", from: "13.2.0"),
    .package(url: "https://github.com/google/GoogleSignIn-iOS.git", from: "10.0.0"),
    .package(url: "https://github.com/kakao/kakao-ios-sdk.git", from: "2.29.0"),
    .package(url: "https://github.com/naver/naveridlogin-sdk-ios-swift.git", from: "5.2.1"),
    .package(url: "https://github.com/hmlongco/Factory.git", from: "3.3.0"),
    .package(url: "https://github.com/apple/swift-log.git", from: "1.15.1"),
  ],
  targets: [
    .target(name: "GraphQL", dependencies: [.product(name: "ApolloAPI", package: "apollo-ios")]),
    .target(
      name: "GraphQLMocks",
      dependencies: [
        "GraphQL",
        .product(name: "ApolloTestSupport", package: "apollo-ios"),
      ],
      path: "Sources/GraphQLMocks"
    ),
    .target(
      name: "Core",
      dependencies: [
        "GraphQL",
        .product(name: "Apollo", package: "apollo-ios"),
        .product(name: "ApolloSQLite", package: "apollo-ios"),
        .product(name: "ApolloWebSocket", package: "apollo-ios"),
        .product(name: "Alamofire", package: "Alamofire"),
        .product(name: "FactoryKit", package: "Factory"),
        .product(name: "Logging", package: "swift-log"),
      ],
      exclude: ["GraphQL/SubscriptionConnection.graphql"]
    ),
    .target(
      name: "Design",
      dependencies: [
        .product(name: "Nuke", package: "Nuke"),
        .product(name: "NukeUI", package: "Nuke"),
        .product(name: "FactoryKit", package: "Factory"),
      ],
      resources: [.process("Resources")]
    ),
    .target(
      name: "Features",
      dependencies: [
        "Core", "Design", "GraphQL", .product(name: "FactoryKit", package: "Factory"),
      ],
      exclude: [
        "Auth/EmailLogin.graphql", "Auth/SingleSignOnLogin.graphql",
        "Entity/Container/FolderContents.graphql", "Entity/Container/SiteEntities.graphql",
        "Entity/EntityContainer.graphql", "Entity/EntityRow.graphql",
        "Home/Home.graphql", "Home/Recent/RecentDocuments.graphql", "Home/SiteSwitcher.graphql",
        "Image/Img.graphql", "Profile/Profile.graphql", "Search/Search.graphql",
        "Shell/LiveUpdates.graphql", "UserGoal/UserGoal.graphql",
      ]
    ),
    .target(
      name: "Platform",
      dependencies: [
        "Core",
        .product(
          name: "GoogleSignIn", package: "GoogleSignIn-iOS", condition: .when(platforms: [.iOS])),
        .product(
          name: "KakaoSDKCommon", package: "kakao-ios-sdk", condition: .when(platforms: [.iOS])),
        .product(
          name: "KakaoSDKAuth", package: "kakao-ios-sdk", condition: .when(platforms: [.iOS])),
        .product(
          name: "KakaoSDKUser", package: "kakao-ios-sdk", condition: .when(platforms: [.iOS])),
        .product(
          name: "NidThirdPartyLogin", package: "naveridlogin-sdk-ios-swift",
          condition: .when(platforms: [.iOS])),
        .product(name: "FactoryKit", package: "Factory"),
        .product(name: "Logging", package: "swift-log"),
      ]
    ),
    .testTarget(
      name: "CoreTests",
      dependencies: [
        "Core", "GraphQL", "GraphQLMocks",
        .product(name: "Alamofire", package: "Alamofire"),
        .product(name: "Apollo", package: "apollo-ios"),
        .product(name: "ApolloTestSupport", package: "apollo-ios"),
        .product(name: "ApolloWebSocket", package: "apollo-ios"),
        .product(name: "FactoryTesting", package: "Factory"),
        .product(name: "Logging", package: "swift-log"),
        .product(name: "InMemoryLogging", package: "swift-log"),
      ],
      exclude: ["Ping.graphql"]),
    .testTarget(
      name: "DesignTests",
      dependencies: ["Design", .product(name: "FactoryTesting", package: "Factory")]
    ),
    .testTarget(
      name: "FeaturesTests",
      dependencies: [
        "Features", "Core", "Design", "GraphQL", "GraphQLMocks",
        .product(name: "Apollo", package: "apollo-ios"),
        .product(name: "ApolloTestSupport", package: "apollo-ios"),
        .product(name: "FactoryTesting", package: "Factory"),
      ]),
  ],
  swiftLanguageModes: [.v6]
)
