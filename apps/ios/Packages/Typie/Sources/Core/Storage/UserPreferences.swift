import Foundation

public final class UserPreferences: @unchecked Sendable {
  private static let siteIdKey = "site_id"
  private static let recentSearchesKey = "recent_searches"
  private static let expandedFoldersKey = "expanded_folders"
  private static let homeSectionOrderKey = "home_section_order"
  private static let hiddenHomeSectionsKey = "home_hidden_sections"
  private static let recentDocumentsSortKey = "recent_documents_sort"

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

  public func homeSectionOrder() -> [String] {
    lock.withLock {
      defaults.stringArray(forKey: Self.scopedKey(Self.homeSectionOrderKey, userId)) ?? []
    }
  }

  public func setHomeSectionOrder(_ ids: [String]) {
    lock.withLock { storeArray(ids, key: Self.homeSectionOrderKey) }
  }

  public func hiddenHomeSections() -> [String] {
    lock.withLock {
      defaults.stringArray(forKey: Self.scopedKey(Self.hiddenHomeSectionsKey, userId)) ?? []
    }
  }

  public func setHiddenHomeSections(_ ids: [String]) {
    lock.withLock { storeArray(ids, key: Self.hiddenHomeSectionsKey) }
  }

  public func recentDocumentsSort() -> String? {
    lock.withLock { defaults.string(forKey: Self.scopedKey(Self.recentDocumentsSortKey, userId)) }
  }

  public func setRecentDocumentsSort(_ value: String?) {
    lock.withLock {
      let scoped = Self.scopedKey(Self.recentDocumentsSortKey, userId)
      if let value {
        defaults.set(value, forKey: scoped)
      } else {
        defaults.removeObject(forKey: scoped)
      }
    }
  }

  private func storeArray(_ values: [String], key: String) {
    let scoped = Self.scopedKey(key, userId)
    if values.isEmpty {
      defaults.removeObject(forKey: scoped)
    } else {
      defaults.set(values, forKey: scoped)
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
