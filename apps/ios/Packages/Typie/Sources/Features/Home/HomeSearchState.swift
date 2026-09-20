import Observation

@MainActor @Observable
public final class HomeSearchState {
  public static let transitionDuration: Double = 0.35
  public static let transitionBounce: Double = 0

  public var isActive = false
  public var barHeight: Double = 0
  public var titleVisible = false

  public init() {}

  nonisolated static func searchPrompt(siteName: String?) -> String {
    guard let siteName else { return "검색" }
    return "\(truncated(siteName, to: 10))에서 검색..."
  }

  nonisolated private static func truncated(_ text: String, to length: Int) -> String {
    text.count > length ? "\(text.prefix(length))..." : text
  }
}
