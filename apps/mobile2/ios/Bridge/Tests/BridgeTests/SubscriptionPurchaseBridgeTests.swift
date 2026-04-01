import StoreKit
import XCTest
@testable import Bridge

final class SubscriptionPurchaseBridgeTests: XCTestCase {
  func testRequestFailureDelegateCanBeInvokedFromBackgroundQueue() async {
    let bridge = await MainActor.run { SubscriptionPurchaseBridge() }
    let completionExpectation = expectation(description: "background delegate invocation returned")

    DispatchQueue.global(qos: .default).async {
      _ = bridge.perform(
        #selector(SubscriptionPurchaseBridge.request(_:didFailWithError:)),
        with: SKRequest(),
        with: NSError(domain: "BridgeTests", code: 1),
      )
      completionExpectation.fulfill()
    }

    await fulfillment(of: [completionExpectation], timeout: 1.0)
  }
}
