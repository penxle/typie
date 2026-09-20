import Core
import FactoryKit
import FactoryTesting
import Testing

@testable import Features

@MainActor
@Suite(.container) struct SearchSessionTests {
  private func makeSession() -> SearchSession {
    registerFake(FakeGraphQLClient())
    let preferences = makeTestPreferences(siteId: "site-1")
    Container.shared.userPreferences.register { preferences.userPreferences }
    return SearchSession(model: SearchModel(debounce: .milliseconds(20)))
  }

  @Test func requestFocusIncrements() {
    let session = makeSession()
    let before = session.focusRequest
    session.requestFocus()
    session.requestFocus()
    #expect(session.focusRequest == before + 2)
  }

  @Test func releaseFocusIncrements() {
    let session = makeSession()
    let before = session.blurRequest
    session.releaseFocus()
    #expect(session.blurRequest == before + 1)
  }

  @Test func promptUsesSiteName() {
    #expect(SearchSession.searchPrompt(siteName: "abc") == "abc에서 검색...")
  }

  @Test func promptTruncatesLongName() {
    #expect(SearchSession.searchPrompt(siteName: "abcdefghijk") == "abcdefghij...에서 검색...")
    #expect(SearchSession.searchPrompt(siteName: "abcdefghij") == "abcdefghij에서 검색...")
  }

  @Test func promptFallsBackWithoutName() {
    #expect(SearchSession.searchPrompt(siteName: nil) == "검색")
  }
}
