public enum SearchPrompt {
  public static func text(spaceName: String?) -> String {
    guard let spaceName else { return "검색" }
    return "\(truncated(spaceName, to: 10))에서 검색..."
  }

  static func truncated(_ text: String, to length: Int) -> String {
    text.count > length ? "\(text.prefix(length))..." : text
  }
}
