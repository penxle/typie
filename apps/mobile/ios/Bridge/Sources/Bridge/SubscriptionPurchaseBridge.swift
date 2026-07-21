import Foundation
import StoreKit

@objcMembers public class PurchaseProductPayload: NSObject {
    public let productId: String
    public let name: String
    public let price: String

    init(productId: String, name: String, price: String) {
        self.productId = productId
        self.name = name
        self.price = price
    }
}

@objcMembers public class PurchaseEventPayload: NSObject {
    public let type: String
    public let productId: String
    public let subscriptionId: String
    public let transactionId: String

    init(
        type: String,
        productId: String,
        subscriptionId: String,
        transactionId: String,
    ) {
        self.type = type
        self.productId = productId
        self.subscriptionId = subscriptionId
        self.transactionId = transactionId
    }
}

@MainActor @objcMembers public final class SubscriptionPurchaseBridge: NSObject {
    private var purchaseObserver: ((PurchaseEventPayload?) -> Void)?
    private var updatesTask: Task<Void, Never>?
    private var pendingTransactions: [UInt64: Transaction] = [:]

    override public init() {
        super.init()
    }

    deinit {
        updatesTask?.cancel()
    }

    public func startObservingPurchases(
        completion: @escaping (PurchaseEventPayload?) -> Void
    ) {
        purchaseObserver = completion

        updatesTask?.cancel()
        updatesTask = Task { [weak self] in
            for await result in Transaction.updates {
                guard let self else { break }
                self.handleTransactionUpdate(result)
            }
        }
    }

    public func queryProducts(
        productIds: [String],
        completion: @escaping ([PurchaseProductPayload]?, NSError?) -> Void
    ) {
        Task {
            do {
                let products = try await Product.products(for: productIds)
                let payloads = products.map { product in
                    PurchaseProductPayload(
                        productId: product.id,
                        name: product.displayName,
                        price: product.displayPrice,
                    )
                }
                completion(payloads, nil)
            } catch {
                completion(nil, error as NSError)
            }
        }
    }

    public func purchaseProduct(
        productId: String,
        accountId: String?,
        completion: @escaping (Bool, NSError?) -> Void
    ) {
        Task {
            do {
                let products = try await Product.products(for: [productId])
                guard let product = products.first else {
                    completion(
                        false,
                        NSError(
                            domain: "co.typie.ios.bridge",
                            code: -2,
                            userInfo: [NSLocalizedDescriptionKey: "Product not found"]
                        ))
                    return
                }

                var options: Set<Product.PurchaseOption> = []
                if let accountId, let uuid = UUID(uuidString: accountId) {
                    options.insert(.appAccountToken(uuid))
                }

                let result = try await product.purchase(options: options)
                switch result {
                case .success(let verification):
                    switch verification {
                    case .verified(let transaction):
                        pendingTransactions[transaction.originalID] = transaction
                        let payload = PurchaseEventPayload(
                            type: "purchased",
                            productId: transaction.productID,
                            subscriptionId: String(transaction.originalID),
                            transactionId: String(transaction.id),
                        )
                        purchaseObserver?(payload)
                        completion(true, nil)
                    case .unverified:
                        completion(false, nil)
                    }
                case .pending:
                    completion(false, nil)
                case .userCancelled:
                    completion(false, nil)
                @unknown default:
                    completion(false, nil)
                }
            } catch {
                completion(false, error as NSError)
            }
        }
    }

    public func finishTransaction(
        transactionId: String,
        completion: @escaping (Bool) -> Void
    ) {
        guard let id = UInt64(transactionId),
            let transaction = pendingTransactions.removeValue(forKey: id)
        else {
            completion(false)
            return
        }

        Task {
            await transaction.finish()
            completion(true)
        }
    }

    private func handleTransactionUpdate(_ result: VerificationResult<Transaction>) {
        switch result {
        case .verified(let transaction):
            pendingTransactions[transaction.originalID] = transaction
            let payload = PurchaseEventPayload(
                type: "restored",
                productId: transaction.productID,
                subscriptionId: String(transaction.originalID),
                transactionId: String(transaction.id),
            )
            purchaseObserver?(payload)
        case .unverified:
            break
        }
    }
}
