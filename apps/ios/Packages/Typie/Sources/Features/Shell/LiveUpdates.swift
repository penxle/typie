import Core
import FactoryKit
import GraphQL

@MainActor
final class LiveUpdates {
  private let client: any GraphQLClient
  private let authState: AuthStateStore
  private let activeSite: ActiveSiteStore
  private var started = false
  private var siteId: String?
  private var userId: String?
  private var goal: Task<Void, Never>?
  private var usage: Task<Void, Never>?
  private var site: [Task<Void, Never>] = []

  init() {
    client = Container.shared.graphQLClient()
    authState = Container.shared.authState()
    activeSite = Container.shared.activeSite()
  }

  deinit {
    goal?.cancel()
    usage?.cancel()
    site.forEach { $0.cancel() }
  }

  func start() {
    guard !started else { return }
    started = true
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  private func sync() {
    guard case .authenticated(let tokens) = authState.state else {
      stop()
      return
    }
    if goal == nil {
      goal = drain(LiveUpdates_UserGoalUpdateStream_Subscription())
    }
    if tokens.userId != userId {
      userId = tokens.userId
      usage?.cancel()
      usage = drain(LiveUpdates_UserUsageUpdateStream_Subscription(userId: tokens.userId))
    }
    let currentSiteId = activeSite.siteId
    if currentSiteId != siteId {
      siteId = currentSiteId
      site.forEach { $0.cancel() }
      site =
        currentSiteId.map { id in
          [drain(LiveUpdates_SiteUpdateStream_Subscription(siteId: id)), refetchRecent(siteId: id)]
        } ?? []
    }
  }

  private func stop() {
    goal?.cancel()
    goal = nil
    usage?.cancel()
    usage = nil
    userId = nil
    site.forEach { $0.cancel() }
    site = []
    siteId = nil
  }

  private func drain<S: GraphQLSubscription>(_ subscription: S) -> Task<Void, Never> {
    let client = client
    return Task {
      for await _ in client.subscribe(subscription) {}
    }
  }

  private func refetchRecent(siteId: String) -> Task<Void, Never> {
    let client = client
    return Task {
      let stream = client.subscribe(
        LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription(siteId: siteId))
      for await data in stream {
        let sort = data.siteRecentDocumentsUpdateStream
        client.refetchWatches { operation in
          if let home = operation as? HomeScreen_Query { return home.siteId == siteId }
          if let recent = operation as? RecentDocuments_Query {
            return recent.siteId == siteId && recent.sort == sort
          }
          return false
        }
      }
    }
  }
}
