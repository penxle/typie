import Foundation
import Observation

@MainActor @Observable public final class DevModeStore {
  private static let key = "dev_mode"

  public private(set) var isEnabled: Bool

  @ObservationIgnored private let defaults: UserDefaults

  init(defaults: UserDefaults) {
    self.defaults = defaults
    isEnabled = defaults.bool(forKey: Self.key)
  }

  public func setEnabled(_ enabled: Bool) {
    defaults.set(enabled, forKey: Self.key)
    isEnabled = enabled
  }
}
