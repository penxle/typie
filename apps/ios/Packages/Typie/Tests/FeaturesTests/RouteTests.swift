import Testing

@testable import Features

@Suite struct RouteTests {
  @Test func documentsWithDifferentIdsAreDistinct() {
    #expect(Route.document(entityId: "a") != Route.document(entityId: "b"))
  }

  @Test func goalRoutesAreDistinct() {
    #expect(Route.userGoal != Route.home)
  }

  @Test func homeListRoutesAreDistinct() {
    #expect(Route.pinnedEntities != Route.siteEntities)
    #expect(Route.siteEntities != Route.home)
  }

  @Test func tabsMapToTheirRootRoutes() {
    #expect(MainTab.allCases == [.home, .notes, .prism, .square])
    #expect(MainTab.home.route == .home)
    #expect(MainTab.notes.route == .notes)
    #expect(MainTab.prism.route == .prism)
    #expect(MainTab.square.route == .square)
  }
}
