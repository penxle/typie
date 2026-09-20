import Testing

@testable import Features

@Suite struct HomeSearchStateTests {
  @Test func usesSiteName() {
    #expect(HomeSearchState.searchPrompt(siteName: "abc") == "abc에서 검색...")
  }

  @Test func truncatesLongName() {
    #expect(HomeSearchState.searchPrompt(siteName: "abcdefghijk") == "abcdefghij...에서 검색...")
    #expect(HomeSearchState.searchPrompt(siteName: "abcdefghij") == "abcdefghij에서 검색...")
  }

  @Test func fallsBackWithoutName() {
    #expect(HomeSearchState.searchPrompt(siteName: nil) == "검색")
  }
}
