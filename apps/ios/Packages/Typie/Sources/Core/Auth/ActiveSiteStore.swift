import Observation

@MainActor @Observable public final class ActiveSiteStore {
  public private(set) var siteId: String?

  @ObservationIgnored private let preferences: UserPreferences

  init(preferences: UserPreferences) {
    self.preferences = preferences
    siteId = preferences.siteId
  }

  public nonisolated static func resolve(stored: String?, available: [String]) -> String? {
    if let stored, available.contains(stored) { return stored }
    return available.first
  }

  public func select(_ siteId: String) {
    preferences.siteId = siteId
    self.siteId = siteId
  }
}
