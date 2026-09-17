import Testing

@testable import Core

@Suite struct ActiveSiteTests {
  @Test func keepsStoredSiteIdWhenAvailable() {
    #expect(resolveActiveSiteId(stored: "S2", available: ["S1", "S2"]) == "S2")
  }

  @Test func fallsBackToFirstSiteWhenStoredIsInvalid() {
    #expect(resolveActiveSiteId(stored: "GONE", available: ["S1", "S2"]) == "S1")
  }

  @Test func fallsBackToFirstSiteWhenStoredIsNil() {
    #expect(resolveActiveSiteId(stored: nil, available: ["S1", "S2"]) == "S1")
  }

  @Test func returnsNilWhenNoSitesAvailable() {
    #expect(resolveActiveSiteId(stored: "S1", available: []) == nil)
    #expect(resolveActiveSiteId(stored: nil, available: []) == nil)
  }
}
