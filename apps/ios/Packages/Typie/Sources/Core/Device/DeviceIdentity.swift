import Foundation

public struct DeviceIdentity: @unchecked Sendable {
  private static let key = "device_id"
  private let defaults: UserDefaults

  public init(defaults: UserDefaults = .standard) {
    self.defaults = defaults
  }

  public func id() -> String {
    if let stored = defaults.string(forKey: Self.key) { return stored }
    let generated = UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
    defaults.set(generated, forKey: Self.key)
    return generated
  }
}
