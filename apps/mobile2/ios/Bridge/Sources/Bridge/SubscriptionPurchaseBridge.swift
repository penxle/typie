import Foundation
@preconcurrency import StoreKit
import UIKit

@objcMembers public class PurchaseProductPayload: NSObject {
  public let productId: String
  public let interval: String
  public let price: String
  public let title: String?
  public let rawPrice: Double
  public let currencyCode: String?

  init(
    productId: String,
    interval: String,
    price: String,
    title: String?,
    rawPrice: Double,
    currencyCode: String?
  ) {
    self.productId = productId
    self.interval = interval
    self.price = price
    self.title = title
    self.rawPrice = rawPrice
    self.currencyCode = currencyCode
  }
}

@objcMembers public class PurchaseEventPayload: NSObject {
  public let type: String
  public let productId: String
  public let verificationData: String
  public let purchaseId: String?

  init(
    type: String,
    productId: String,
    verificationData: String,
    purchaseId: String?
  ) {
    self.type = type
    self.productId = productId
    self.verificationData = verificationData
    self.purchaseId = purchaseId
  }
}

@MainActor @objcMembers public final class SubscriptionPurchaseBridge: NSObject, @preconcurrency SKProductsRequestDelegate, @preconcurrency SKPaymentTransactionObserver {
  private var productsRequest: SKProductsRequest?
  private var productsCompletion: (([PurchaseProductPayload]?, NSError?) -> Void)?
  private var productsById: [String: SKProduct] = [:]
  private var purchaseObserver: ((PurchaseEventPayload?) -> Void)?
  private var isQueueObserverAttached = false

  override public init() {
    super.init()
  }

  deinit {
    if isQueueObserverAttached {
      SKPaymentQueue.default().remove(self)
    }
  }

  public func startObservingPurchases(
    completion: @escaping (PurchaseEventPayload?) -> Void
  ) {
    ensureQueueObserverAttached()
    purchaseObserver = completion
  }

  public func queryProducts(
    completion: @escaping ([PurchaseProductPayload]?, NSError?) -> Void
  ) {
    ensureQueueObserverAttached()
    productsCompletion = completion

    let request = SKProductsRequest(
      productIdentifiers: [
        "pl0fl1map",
        "pl0fl1yap",
      ]
    )
    request.delegate = self
    productsRequest = request
    request.start()
  }

  public func purchaseProduct(
    productId: String,
    accountId: String?,
    completion: @escaping (Bool, NSError?) -> Void
  ) {
    ensureQueueObserverAttached()

    guard SKPaymentQueue.canMakePayments() else {
      completion(
        false,
        NSError(
          domain: "co.typie.ios.bridge",
          code: -1,
          userInfo: [NSLocalizedDescriptionKey: "Payments are not available"],
        )
      )
      return
    }

    guard let product = productsById[productId] else {
      completion(
        false,
        NSError(
          domain: "co.typie.ios.bridge",
          code: -2,
          userInfo: [NSLocalizedDescriptionKey: "Product not loaded"],
        )
      )
      return
    }

    let payment = SKMutablePayment(product: product)
    payment.applicationUsername = accountId
    SKPaymentQueue.default().add(payment)
    completion(true, nil)
  }

  public func openSubscriptionManagement(
    completion: @escaping (Bool) -> Void
  ) {
    guard let url = URL(string: "https://apps.apple.com/account/subscriptions") else {
      completion(false)
      return
    }

    UIApplication.shared.open(url, options: [:], completionHandler: completion)
  }

  nonisolated public func productsRequest(_ request: SKProductsRequest, didReceive response: SKProductsResponse) {
    Task { @MainActor in
      handleProductsResponse(response)
    }
  }

  nonisolated public func request(_ request: SKRequest, didFailWithError error: Error) {
    Task { @MainActor in
      handleProductsRequestFailure(error)
    }
  }

  nonisolated public func paymentQueue(
    _ queue: SKPaymentQueue,
    updatedTransactions transactions: [SKPaymentTransaction]
  ) {
    Task { @MainActor in
      handleUpdatedTransactions(queue, transactions: transactions)
    }
  }

  private func ensureQueueObserverAttached() {
    guard !isQueueObserverAttached else {
      return
    }

    SKPaymentQueue.default().add(self)
    isQueueObserverAttached = true
  }

  private func handleProductsResponse(_ response: SKProductsResponse) {
    productsById = Dictionary(uniqueKeysWithValues: response.products.map { ($0.productIdentifier, $0) })

    let payloads = response.products.compactMap { product -> PurchaseProductPayload? in
      let interval: String
      switch product.productIdentifier {
      case "pl0fl1map":
        interval = "monthly"
      case "pl0fl1yap":
        interval = "yearly"
      default:
        return nil
      }

      return PurchaseProductPayload(
        productId: product.productIdentifier,
        interval: interval,
        price: product.formattedPrice,
        title: product.localizedTitle,
        rawPrice: product.price.doubleValue,
        currencyCode: product.priceLocale.currencyCode
      )
    }

    productsCompletion?(payloads, nil)
    productsCompletion = nil
    productsRequest = nil
  }

  private func handleProductsRequestFailure(_ error: Error) {
    productsCompletion?(nil, error as NSError)
    productsCompletion = nil
    productsRequest = nil
  }

  private func handleUpdatedTransactions(
    _ queue: SKPaymentQueue,
    transactions: [SKPaymentTransaction]
  ) {
    for transaction in transactions {
      switch transaction.transactionState {
      case .purchased:
        emitEvent(type: "purchased", transaction: transaction)
        queue.finishTransaction(transaction)
      case .restored:
        emitEvent(type: "restored", transaction: transaction)
        queue.finishTransaction(transaction)
      case .failed:
        queue.finishTransaction(transaction)
      default:
        break
      }
    }
  }

  private func emitEvent(type: String, transaction: SKPaymentTransaction) {
    let payload = PurchaseEventPayload(
      type: type,
      productId: transaction.payment.productIdentifier,
      verificationData: transaction.transactionIdentifier ?? "",
      purchaseId: transaction.transactionIdentifier
    )
    purchaseObserver?(payload)
  }
}

private extension SKProduct {
  var formattedPrice: String {
    let formatter = NumberFormatter()
    formatter.numberStyle = .currency
    formatter.locale = priceLocale
    return formatter.string(from: price) ?? ""
  }
}
