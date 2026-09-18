import Observation

protocol ActiveSitePublishing: Sendable {
  func publish(_ siteId: String?) async
}

struct NoopActiveSitePublisher: ActiveSitePublishing {
  func publish(_ siteId: String?) async {}
}

@MainActor @Observable public final class ActiveSiteStore {
  public private(set) var siteId: String?

  @ObservationIgnored private let preferences: any UserScopedPreferences

  init(preferences: any UserScopedPreferences) {
    self.preferences = preferences
  }

  public func select(_ siteId: String) {
    preferences.siteId = siteId
    self.siteId = siteId
  }
}

extension ActiveSiteStore: ActiveSitePublishing {
  func publish(_ siteId: String?) async {
    self.siteId = siteId
  }
}
