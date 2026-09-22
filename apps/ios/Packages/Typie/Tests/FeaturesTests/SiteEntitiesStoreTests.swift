import ApolloTestSupport
import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@MainActor
@Suite(.container) struct SiteEntitiesStoreTests {
  private func makeStore(siteId: String? = "site-1") -> (SiteEntitiesStore, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: siteId)
    Container.shared.userPreferences.register { preferences.userPreferences }
    return (Container.shared.siteEntitiesStore(), client)
  }

  @Test func startsAsPlaceholderAndWatchesActiveSite() async throws {
    let (store, client) = makeStore()
    #expect(store.isPlaceholder)
    #expect(store.site == nil)
    try await waitOnMain { client.watchCount(of: SiteEntities_Query.self) == 1 }
    #expect(client.watched(SiteEntities_Query.self).map(\.siteId) == ["site-1"])
  }

  @Test func withoutActiveSiteDoesNotQuery() async {
    let (store, client) = makeStore(siteId: nil)
    await drainMainActor()
    #expect(client.watchCount(of: SiteEntities_Query.self) == 0)
    #expect(store.isPlaceholder)
  }

  @Test func deliversSiteStateInOrder() async throws {
    let (store, client) = makeStore()
    client.push(
      .success(
        await siteEntitiesData(
          name: "합성 스페이스", folderCount: 2, documentCount: 3,
          entities: [
            folderEntityMock(id: "f1", name: "소설", childCount: 0), dividerEntityMock(id: "x1"),
            entityMock(id: "d1", kind: .document, title: "할 일"),
          ])),
      for: SiteEntities_Query.self)
    try await waitOnMain { store.hasData }
    #expect(store.site?.name == "합성 스페이스")
    #expect(store.site?.summary == "폴더 2개 · 문서 3개")
    #expect(store.site?.items.map(\.id) == ["f1", "x1", "d1"])
    #expect(store.isPlaceholder == false)
    #expect(store.loadFailed == false)
  }

  @Test func emptySiteSummary() async throws {
    let (store, client) = makeStore()
    client.push(.success(await siteEntitiesData()), for: SiteEntities_Query.self)
    try await waitOnMain { store.hasData }
    #expect(store.site?.items.isEmpty == true)
    #expect(store.site?.summary == "비어 있는 스페이스")
  }

  @Test func failureWithoutDataSetsLoadFailed() async throws {
    let (store, client) = makeStore()
    client.push(.failure(StubError(name: "load")), for: SiteEntities_Query.self)
    try await waitOnMain { store.loadFailed }
    #expect(store.hasData == false)
    #expect(store.site == nil)
    #expect(store.isPlaceholder == false)
  }
}
