import Testing

@testable import Core

@Suite struct RouteTests {
  @Test func documentsWithDifferentIdsAreDistinct() {
    #expect(Route.document(entityId: "a") != Route.document(entityId: "b"))
  }

  @Test func tabsMapToTheirRootRoutes() {
    #expect(MainTab.allCases == [.home, .studio, .notes])
    #expect(MainTab.home.route == .home)
    #expect(MainTab.studio.route == .studio)
    #expect(MainTab.notes.route == .notes)
  }
}
