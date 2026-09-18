import Foundation

public final class UserScopedDefaults: @unchecked Sendable {
  private static let siteIdKey = "site_id"
  private static let recentSearchesKey = "recent_searches"

  private let defaults: UserDefaults
  private let lock = NSLock()
  private var userId: String?
  private var siteIdValue: String?
  private var recentSearchesValue: [String] = []

  public init(defaults: UserDefaults = .standard) {
    self.defaults = defaults
  }

  public func switchUser(_ userId: String?) {
    lock.withLock {
      guard let userId else {
        self.userId = nil
        siteIdValue = nil
        recentSearchesValue = []
        return
      }
      let siteKey = Self.scopedKey(Self.siteIdKey, userId)
      migrate(base: Self.siteIdKey, scoped: siteKey)
      let searchesKey = Self.scopedKey(Self.recentSearchesKey, userId)
      migrate(base: Self.recentSearchesKey, scoped: searchesKey)
      self.userId = userId
      siteIdValue = defaults.string(forKey: siteKey)
      recentSearchesValue = defaults.stringArray(forKey: searchesKey) ?? []
    }
  }

  public var siteId: String? {
    get { lock.withLock { siteIdValue } }
    set {
      lock.withLock {
        let previous = siteIdValue
        siteIdValue = newValue
        guard previous != newValue, let userId else { return }
        write(newValue, forKey: Self.scopedKey(Self.siteIdKey, userId))
      }
    }
  }

  public var recentSearches: [String] {
    get { lock.withLock { recentSearchesValue } }
    set {
      lock.withLock {
        let previous = recentSearchesValue
        recentSearchesValue = newValue
        guard previous != newValue, let userId else { return }
        let key = Self.scopedKey(Self.recentSearchesKey, userId)
        if newValue.isEmpty {
          defaults.removeObject(forKey: key)
        } else {
          defaults.set(newValue, forKey: key)
        }
      }
    }
  }

  private func migrate(base: String, scoped: String) {
    guard let value = defaults.object(forKey: base) else { return }
    if defaults.object(forKey: scoped) == nil {
      defaults.set(value, forKey: scoped)
    }
    defaults.removeObject(forKey: base)
  }

  private func write(_ value: String?, forKey key: String) {
    if let value {
      defaults.set(value, forKey: key)
    } else {
      defaults.removeObject(forKey: key)
    }
  }

  private static func scopedKey(_ base: String, _ userId: String) -> String {
    "\(base)@\(userId)"
  }
}
