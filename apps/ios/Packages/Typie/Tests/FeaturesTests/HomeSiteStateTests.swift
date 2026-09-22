import ApolloTestSupport
import Core
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@Suite struct HomeSiteStateTests {
  private func state(pinned: [Mock<GraphQLMocks.Entity>], roots: [Mock<GraphQLMocks.Entity>])
    async -> HomeSiteState
  {
    let data = await HomeScreen_Query.Data.from(
      Mock<GraphQLMocks.Query>(me: nil, site: homeSiteMock(pinned: pinned, roots: roots)))
    return HomeSiteState(data.site.fragments.homeScreen_site)
  }

  @Test func keepsAllPinnedButShowsFiveOnHome() async {
    let documents = (1...7).map { entityMock(id: "p\($0)", kind: .document, title: "p\($0)") }
    let pinned = documents[0..<2] + [dividerEntityMock(id: "x9")] + documents[2...]
    let state = await state(pinned: Array(pinned), roots: [])
    #expect(state.pinned.map(\.entityId) == documents.map { $0.id! })
    #expect(state.homePinned.map(\.entityId) == ["p1", "p2", "p3", "p4", "p5"])
    #expect(state.name == "스페이스 A")
  }

  @Test func mapsRootsInOrderIncludingDividers() async {
    let state = await state(
      pinned: [],
      roots: [
        folderEntityMock(id: "f1", name: "소설", childCount: 2), dividerEntityMock(id: "x1"),
        entityMock(id: "d1", kind: .document, title: "할 일"),
      ])
    #expect(state.roots.map(\.id) == ["f1", "x1", "d1"])
  }

  @Test func mapsRecentDocumentsInOrder() async {
    let data = await HomeScreen_Query.Data.from(
      Mock<GraphQLMocks.Query>(
        me: nil,
        site: homeSiteMock(recent: [
          recentDocumentMock(id: "r1", title: "첫째", viewedAt: "2026-09-22T09:00:00.000Z"),
          recentDocumentMock(id: "r2", title: "둘째", viewedAt: "2026-09-21T09:00:00.000Z"),
        ])))
    let state = HomeSiteState(data.site.fragments.homeScreen_site)
    #expect(state.recent.map(\.entityId) == ["r1", "r2"])
  }

  @Test func placeholderHasThreeRowsInBothSections() {
    #expect(HomeSiteState.placeholder.homePinned.count == 3)
    #expect(HomeSiteState.placeholder.recent.count == 3)
    #expect(HomeSiteState.placeholder.roots.count == 3)
    #expect(HomeSiteState.placeholder.roots.allSatisfy { $0.item != nil })
  }
}
