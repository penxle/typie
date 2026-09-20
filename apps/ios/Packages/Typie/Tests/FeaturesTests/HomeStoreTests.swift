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
@Suite(.container) struct HomeStoreTests {
  private static let now = Date(timeIntervalSince1970: 1_790_000_000)
  private static let today = KSTDay(now)

  private func makeStore(siteId: String? = "site-1") -> (HomeStore, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: siteId)
    Container.shared.userPreferences.register { preferences.userPreferences }
    let store = Container.shared.homeStore()
    store.now = { Self.now }
    return (store, client)
  }

  private func iso(_ day: KSTDay) -> String {
    ISO8601DateFormatter().string(from: day.date)
  }

  private func data(
    pinned: sending [Mock<GraphQLMocks.Entity>] = [],
    roots: sending [Mock<GraphQLMocks.Entity>] = [], target: Int? = nil
  ) async -> HomeScreen_Query.Data {
    await homeData(
      target: target, history: [], today: (iso(Self.today), 0),
      site: homeSiteMock(pinned: pinned, roots: roots))
  }

  private nonisolated func documents(_ ids: [String]) -> sending [Mock<GraphQLMocks.Entity>] {
    ids.map { entityMock(id: $0, kind: .document, title: $0) }
  }

  @Test func successExposesGoalAndSite() async throws {
    let (store, client) = makeStore()
    client.push(
      .success(await data(pinned: documents(["p1"]), roots: documents(["d1", "d2"]), target: 300)),
      for: HomeScreen_Query.self)
    try await waitOnMain { store.hasData }
    #expect(store.goal?.hasGoal == true)
    #expect(store.site?.name == "스페이스 A")
    #expect(store.site?.pinned.map(\.entityId) == ["p1"])
    #expect(store.site?.roots.map(\.id) == ["d1", "d2"])
    #expect(store.loadFailed == false)
  }

  @Test func siteIdFollowsActiveSite() async throws {
    let (store, client) = makeStore()
    #expect(store.siteId == "site-1")
    client.push(.success(await data()), for: HomeScreen_Query.self)
    try await waitOnMain { store.hasData }
    Container.shared.activeSite().select("site-2")
    try await waitOnMain { store.hasData == false }
    #expect(store.siteId == "site-2")
    #expect(store.site == nil)
    #expect(store.isPlaceholder)
    #expect(client.watched(HomeScreen_Query.self).map(\.siteId) == ["site-1", "site-2"])
  }

  @Test func withoutActiveSiteStaysPlaceholder() async throws {
    let (store, client) = makeStore(siteId: nil)
    await drainMainActor()
    #expect(store.hasData == false)
    #expect(store.loadFailed == false)
    #expect(store.isPlaceholder)
    #expect(client.watchCount(of: HomeScreen_Query.self) == 0)
  }

  @Test func failureWithoutDataSetsLoadFailed() async throws {
    let (store, client) = makeStore()
    client.push(.failure(StubError(name: "load")), for: HomeScreen_Query.self)
    try await waitOnMain { store.loadFailed }
    #expect(store.hasData == false)
    #expect(store.site == nil)
    #expect(store.isPlaceholder == false)
  }

  @Test func failureAfterSuccessKeepsDataAndStaysSilent() async throws {
    let (store, client) = makeStore()
    client.push(
      .success(await data(pinned: documents(["p1"]), target: 300)), for: HomeScreen_Query.self)
    try await waitOnMain { store.hasData }
    client.push(.failure(StubError(name: "later")), for: HomeScreen_Query.self)
    await drainMainActor()
    #expect(store.hasData == true)
    #expect(store.loadFailed == false)
    #expect(store.site?.pinned.map(\.entityId) == ["p1"])
    #expect(store.goal?.hasGoal == true)
  }

  @Test func failedLoadClearsOnSiteChange() async throws {
    let (store, client) = makeStore()
    client.push(.failure(StubError(name: "load")), for: HomeScreen_Query.self)
    try await waitOnMain { store.loadFailed }
    Container.shared.activeSite().select("site-2")
    try await waitOnMain { store.loadFailed == false }
    #expect(store.isPlaceholder)
  }

  @Test func missingMeIsFailure() async throws {
    let (store, client) = makeStore()
    client.push(.success(await homeDataWithoutMe()), for: HomeScreen_Query.self)
    try await waitOnMain { store.loadFailed }
    #expect(store.hasData == false)
  }

  @Test func placeholderUntilDataArrives() async throws {
    let (store, client) = makeStore()
    #expect(store.isPlaceholder)
    #expect(store.siteOrPlaceholder == HomeSiteState.placeholder)
    #expect(store.goalOrPlaceholder.hasGoal == false)
    client.push(.success(await data(target: 300)), for: HomeScreen_Query.self)
    try await waitOnMain { store.hasData }
    #expect(store.isPlaceholder == false)
    #expect(store.goalOrPlaceholder.hasGoal)
  }

  @Test func previewsPlaceholderOverridesData() async throws {
    let (store, client) = makeStore()
    client.push(.success(await data(pinned: documents(["p1"]))), for: HomeScreen_Query.self)
    try await waitOnMain { store.hasData }
    store.previewsPlaceholder = true
    #expect(store.isPlaceholder)
    #expect(store.siteOrPlaceholder == HomeSiteState.placeholder)
    #expect(store.goalOrPlaceholder == UserGoalState.placeholder)
  }

  @Test func refetchAfterSettledTriggersWatcherRefetch() async throws {
    let (store, client) = makeStore()
    client.push(.success(await data()), for: HomeScreen_Query.self)
    try await waitOnMain { store.isSettled }
    store.refetch()
    try await waitOnMain { client.refetchCount(of: HomeScreen_Query.self) == 1 }
  }
}
