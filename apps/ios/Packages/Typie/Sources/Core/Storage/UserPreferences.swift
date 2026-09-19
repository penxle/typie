import Foundation

public final class UserPreferences: @unchecked Sendable {
  private static let siteIdKey = "site_id"
  private static let recentSearchesKey = "recent_searches"

  private let defaults: UserDefaults
  private let lock = NSLock()
  private let userId: String
  private var siteIdValue: String?
  private var recentSearchesValue: [String]

  public init(userId: String, defaults: UserDefaults = .standard) {
    self.defaults = defaults
    self.userId = userId
    siteIdValue = defaults.string(forKey: Self.scopedKey(Self.siteIdKey, userId))
    recentSearchesValue =
      defaults.stringArray(forKey: Self.scopedKey(Self.recentSearchesKey, userId)) ?? []
  }

  public var siteId: String? {
    get { lock.withLock { siteIdValue } }
    set {
      lock.withLock {
        guard siteIdValue != newValue else { return }
        siteIdValue = newValue
        let key = Self.scopedKey(Self.siteIdKey, userId)
        if let newValue {
          defaults.set(newValue, forKey: key)
        } else {
          defaults.removeObject(forKey: key)
        }
      }
    }
  }

  public var recentSearches: [String] {
    get { lock.withLock { recentSearchesValue } }
    set {
      lock.withLock {
        guard recentSearchesValue != newValue else { return }
        recentSearchesValue = newValue
        let key = Self.scopedKey(Self.recentSearchesKey, userId)
        if newValue.isEmpty {
          defaults.removeObject(forKey: key)
        } else {
          defaults.set(newValue, forKey: key)
        }
      }
    }
  }

  private static func scopedKey(_ base: String, _ userId: String) -> String {
    "\(base)@\(userId)"
  }
}
