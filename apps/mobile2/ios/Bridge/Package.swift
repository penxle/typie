// swift-tools-version: 6.2
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
  name: "Bridge",
  platforms: [.iOS("15.6")],
  products: [
    // Products define the executables and libraries a package produces, making them visible to other packages.
    .library(
      name: "Bridge",
      targets: ["Bridge"],
    )
  ],
  dependencies: [
    .package(
      url: "https://github.com/google/GoogleSignIn-iOS.git",
      from: "9.1.0"
    ),
    .package(url: "https://github.com/kakao/kakao-ios-sdk.git", from: "2.27.2"),
    .package(
      url: "https://github.com/naver/naveridlogin-sdk-ios-swift.git",
      from: "5.1.0"
    ),
    .package(
      url: "https://github.com/firebase/firebase-ios-sdk.git",
      from: "12.12.1"
    ),
  ],
  targets: [
    // Targets are the basic building blocks of a package, defining a module or a test suite.
    // Targets can depend on other targets in this package and products from dependencies.
    .binaryTarget(
      name: "EditorFFI",
      path: "../Frameworks/Editor.xcframework"
    ),
    .target(
      name: "Bridge",
      dependencies: [
        "EditorFFI",
        .product(name: "GoogleSignIn", package: "GoogleSignIn-iOS"),
        .product(name: "KakaoSDKUser", package: "kakao-ios-sdk"),
        .product(
          name: "NidThirdPartyLogin",
          package: "naveridlogin-sdk-ios-swift"
        ),
        .product(name: "FirebaseMessaging", package: "firebase-ios-sdk"),
      ],
    ),
    .testTarget(
      name: "BridgeTests",
      dependencies: ["Bridge"],
    ),
  ],
)
