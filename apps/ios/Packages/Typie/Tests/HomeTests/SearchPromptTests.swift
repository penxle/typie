import Testing

@testable import Home

@Suite struct SearchPromptTests {
  @Test func usesSpaceName() {
    #expect(SearchPrompt.text(spaceName: "abc") == "abc에서 검색...")
  }

  @Test func truncatesLongName() {
    #expect(SearchPrompt.text(spaceName: "abcdefghijk") == "abcdefghij...에서 검색...")
    #expect(SearchPrompt.text(spaceName: "abcdefghij") == "abcdefghij에서 검색...")
  }

  @Test func fallsBackWithoutName() {
    #expect(SearchPrompt.text(spaceName: nil) == "검색")
  }
}
