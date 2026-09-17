public enum DeviceHeaders {
  public static func make(deviceID: String, model: String, systemName: String) -> [String: String] {
    [
      "X-Device-Id": deviceID,
      "X-Device-Name": model,
      "X-Device-Platform": systemName.uppercased(),
    ]
  }
}
