public struct TFormRule: Sendable {
  private let check: @Sendable (String) -> String?

  public init(_ validate: @escaping @Sendable (String) -> String?) {
    check = validate
  }

  func validate(_ value: String) -> String? {
    check(value)
  }

  public static func required(_ message: String) -> TFormRule {
    TFormRule { value in value.allSatisfy(\.isWhitespace) ? message : nil }
  }

  nonisolated(unsafe) private static let emailPattern =
    /^[A-Za-z0-9+_.-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$/

  public static func email(_ message: String) -> TFormRule {
    TFormRule { value in
      if value.allSatisfy(\.isWhitespace) { return nil }
      return value.wholeMatch(of: emailPattern) == nil ? message : nil
    }
  }
}
