import ApolloTestSupport
import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import GraphQLMocks
import Observation
import Testing

@testable import Features

@MainActor
@Suite(.container) struct HomeTreeStoreTests {
  private func makeStore(siteId: String? = "site-1", expanded: [String] = [])
    -> (HomeTreeStore, FakeGraphQLClient, TestPreferences)
  {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: siteId)
    if let siteId { preferences.userPreferences.setExpandedFolders(expanded, siteId: siteId) }
    Container.shared.userPreferences.register { preferences.userPreferences }
    return (Container.shared.homeTreeStore(), client, preferences)
  }

  private nonisolated func documents(_ ids: [String]) -> sending [Mock<GraphQLMocks.Entity>] {
    ids.map { entityMock(id: $0, kind: .document, title: $0) }
  }

  @Test func collapsedFolderHasNoQuery() async {
    let (store, client, _) = makeStore()
    await drainMainActor()
    #expect(store.isExpanded("f1") == false)
    #expect(store.children(of: "f1").isEmpty)
    #expect(client.watchCount(of: HomeTree_Children_Query.self) == 0)
  }

  @Test func expandingStartsChildrenQueryAndPersists() async throws {
    let (store, client, preferences) = makeStore()
    await drainMainActor()
    store.toggle("f1")
    #expect(store.isExpanded("f1"))
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 1 }
    #expect(client.watched(HomeTree_Children_Query.self).map(\.entityId) == ["f1"])
    #expect(store.isLoading("f1"))
    client.push(
      .success(await childrenData(parentId: "f1", children: documents(["d1", "d2"]))),
      for: HomeTree_Children_Query.self)
    try await waitOnMain { !store.children(of: "f1").isEmpty }
    #expect(store.children(of: "f1").map(\.id) == ["d1", "d2"])
    #expect(store.node("f1")?.item?.title == "부모")
    #expect(preferences.userPreferences.expandedFolders(siteId: "site-1") == ["f1"])
  }

  @Test func collapsingKeepsQueryAndClearsPersistedId() async throws {
    let (store, client, preferences) = makeStore()
    await drainMainActor()
    store.toggle("f1")
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 1 }
    store.toggle("f1")
    #expect(store.isExpanded("f1") == false)
    #expect(preferences.userPreferences.expandedFolders(siteId: "site-1") == [])
    store.toggle("f1")
    await drainMainActor()
    #expect(client.watchCount(of: HomeTree_Children_Query.self) == 1)
  }

  @Test func restoresExpandedFoldersForActiveSite() async throws {
    let (store, client, _) = makeStore(expanded: ["f2", "f1"])
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 2 }
    #expect(store.expanded == ["f1", "f2"])
    #expect(Set(client.watched(HomeTree_Children_Query.self).map(\.entityId)) == ["f1", "f2"])
  }

  @Test func siteChangeResetsExpansion() async throws {
    let (store, client, preferences) = makeStore(expanded: ["f1"])
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 1 }
    preferences.userPreferences.setExpandedFolders(["g1"], siteId: "site-2")
    Container.shared.activeSite().select("site-2")
    try await waitOnMain { store.expanded == ["g1"] }
    #expect(store.isExpanded("f1") == false)
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 2 }
    #expect(client.watched(HomeTree_Children_Query.self).map(\.entityId) == ["f1", "g1"])
  }

  @Test func failedChildrenStayEmptyAndRetapRefetches() async throws {
    let (store, client, _) = makeStore()
    await drainMainActor()
    store.toggle("f1")
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 1 }
    client.push(.failure(StubError(name: "children")), for: HomeTree_Children_Query.self)
    try await waitOnMain { store.failed("f1") }
    #expect(store.isExpanded("f1"))
    #expect(store.children(of: "f1").isEmpty)
    store.toggle("f1")
    #expect(store.isExpanded("f1"))
    try await waitOnMain { client.refetchCount(of: HomeTree_Children_Query.self) == 1 }
  }

  @Test func ensureLoadedStartsQueryWithoutExpanding() async throws {
    let (store, client, _) = makeStore()
    await drainMainActor()
    store.ensureLoaded("f1")
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 1 }
    #expect(store.isExpanded("f1") == false)
  }

  @Test func isLoadedFalseUntilChildrenArrive() async throws {
    let (store, client, _) = makeStore()
    await drainMainActor()
    #expect(store.isLoaded("f1") == false)
    store.ensureLoaded("f1")
    #expect(store.isLoaded("f1") == false)
    try await waitOnMain { client.watchCount(of: HomeTree_Children_Query.self) == 1 }
    client.push(
      .success(await childrenData(parentId: "f1", children: [])),
      for: HomeTree_Children_Query.self)
    try await waitOnMain { store.isLoaded("f1") }
    #expect(store.children(of: "f1").isEmpty)
  }

  @Test func ensureLoadedNotifiesObservers() async {
    let (store, _, _) = makeStore()
    await drainMainActor()
    let recorder = CallRecorder()
    withObservationTracking {
      _ = store.children(of: "f1")
    } onChange: {
      recorder.record("children")
    }
    store.ensureLoaded("f1")
    #expect(recorder.calls == ["children"])
  }
}
