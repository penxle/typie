import Foundation

public enum RecentSearches {
  public static let limit = 10

  public static func adding(_ keyword: String, to current: [String]) -> [String] {
    guard !keyword.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return current }
    return Array(([keyword] + current.filter { $0 != keyword }).prefix(limit))
  }

  public static func removing(_ keyword: String, from current: [String]) -> [String] {
    current.filter { $0 != keyword }
  }
}
