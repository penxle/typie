import Foundation

public final class UserScopedDefaults: @unchecked Sendable {
  private static let siteIdKey = "site_id"

  private let defaults: UserDefaults
  private let lock = NSLock()
  private var userId: String?
  private var siteIdValue: String?

  public init(defaults: UserDefaults = .standard) {
    self.defaults = defaults
  }

  public func switchUser(_ userId: String?) {
    lock.withLock {
      guard let userId else {
        self.userId = nil
        siteIdValue = nil
        return
      }
      let key = Self.scopedKey(Self.siteIdKey, userId)
      migrate(base: Self.siteIdKey, scoped: key)
      self.userId = userId
      siteIdValue = defaults.string(forKey: key)
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

  private func migrate(base: String, scoped: String) {
    guard defaults.object(forKey: base) != nil else { return }
    if defaults.object(forKey: scoped) == nil {
      write(defaults.string(forKey: base), forKey: scoped)
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
