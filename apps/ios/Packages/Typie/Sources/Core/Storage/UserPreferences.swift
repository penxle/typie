import Foundation

public final class UserPreferences: @unchecked Sendable {
  private static let siteIdKey = "site_id"
  private static let recentSearchesKey = "recent_searches"
  private static let expandedFoldersKey = "expanded_folders"

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
    set { lock.withLock { store(&siteIdValue, newValue, key: Self.siteIdKey) } }
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

  public func expandedFolders(siteId: String) -> [String] {
    lock.withLock {
      defaults.stringArray(forKey: Self.scopedKey("\(Self.expandedFoldersKey):\(siteId)", userId))
        ?? []
    }
  }

  public func setExpandedFolders(_ ids: [String], siteId: String) {
    lock.withLock {
      let key = Self.scopedKey("\(Self.expandedFoldersKey):\(siteId)", userId)
      if ids.isEmpty {
        defaults.removeObject(forKey: key)
      } else {
        defaults.set(ids, forKey: key)
      }
    }
  }

  private func store(_ slot: inout String?, _ newValue: String?, key: String) {
    guard slot != newValue else { return }
    slot = newValue
    let scoped = Self.scopedKey(key, userId)
    if let newValue {
      defaults.set(newValue, forKey: scoped)
    } else {
      defaults.removeObject(forKey: scoped)
    }
  }

  private static func scopedKey(_ base: String, _ userId: String) -> String {
    "\(base)@\(userId)"
  }
}
