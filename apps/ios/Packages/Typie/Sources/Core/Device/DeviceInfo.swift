public struct DeviceInfo: Sendable, Equatable {
  public let id: String
  public let model: String
  public let systemName: String

  public init(id: String, model: String, systemName: String) {
    self.id = id
    self.model = model
    self.systemName = systemName
  }

  public var headers: [String: String] {
    [
      "X-Device-Id": id,
      "X-Device-Name": model,
      "X-Device-Platform": systemName.uppercased(),
    ]
  }
}
