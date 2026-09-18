import Testing

@testable import Core

@Suite struct RecentSearchesTests {
  @Test func ignoresBlankKeyword() {
    #expect(RecentSearches.adding("  ", to: ["a"]) == ["a"])
  }

  @Test func insertsAtFrontAndDeduplicates() {
    #expect(RecentSearches.adding("b", to: ["a", "b", "c"]) == ["b", "a", "c"])
  }

  @Test func capsAtTen() {
    let current = (1...10).map { "k\($0)" }
    let next = RecentSearches.adding("new", to: current)
    #expect(next.count == 10)
    #expect(next.first == "new")
    #expect(next.last == "k9")
  }

  @Test func removesKeyword() {
    #expect(RecentSearches.removing("b", from: ["a", "b", "c"]) == ["a", "c"])
  }
}
