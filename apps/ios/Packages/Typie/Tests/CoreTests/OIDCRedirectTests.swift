import Testing

@testable import Core

@Suite struct OIDCRedirectTests {
  @Test func readsErrorFromRedirectLocation() {
    #expect(
      OIDCClient.redirectQueryItem("error", in: "typie:///authorize?error=login_required")
        == "login_required")
  }

  @Test func returnsNilWhenLocationHasNoError() {
    #expect(OIDCClient.redirectQueryItem("error", in: "typie:///authorize?code=abc") == nil)
    #expect(OIDCClient.redirectQueryItem("error", in: "typie:///authorize") == nil)
  }

  @Test func returnsNilWhenLocationIsAbsent() {
    #expect(OIDCClient.redirectQueryItem("error", in: nil) == nil)
  }
}
