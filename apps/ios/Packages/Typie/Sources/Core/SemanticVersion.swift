public enum SemanticVersion {
  public static func isOlder(current: String, required: String) -> Bool {
    let currentParts = components(of: current)
    let requiredParts = components(of: required)
    let length = max(currentParts.count, requiredParts.count)
    for index in 0..<length {
      let c = index < currentParts.count ? currentParts[index] : 0
      let r = index < requiredParts.count ? requiredParts[index] : 0
      if c < r { return true }
      if c > r { return false }
    }
    return false
  }

  private static func components(of version: String) -> [Int] {
    version.split(separator: ".", omittingEmptySubsequences: false).map { Int($0) ?? 0 }
  }
}
