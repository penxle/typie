import ApolloTestSupport
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Core
@testable import Features

@MainActor
@Suite(.container) struct LiveUpdatesTests {
  private struct Setup {
    let updates: LiveUpdates
    let client: FakeGraphQLClient
    let authState: AuthStateStore
    let preferences: TestPreferences
  }

  private func register(siteId: String?) async -> (
    FakeGraphQLClient, AuthStateStore, TestPreferences
  ) {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: siteId)
    Container.shared.userPreferences.register { preferences.userPreferences }
    let authState = AuthStateStore()
    Container.shared.authState.register { authState }
    await authState.publish(
      .authenticated(AuthTokens(sessionToken: "s", accessToken: "a", userId: "user-1")))
    return (client, authState, preferences)
  }

  private func start(siteId: String? = "site-1") async -> Setup {
    let (client, authState, preferences) = await register(siteId: siteId)
    let updates = Container.shared.liveUpdates()
    updates.start()
    return Setup(updates: updates, client: client, authState: authState, preferences: preferences)
  }

  private func allSubscribed(_ client: FakeGraphQLClient) -> Bool {
    client.activeSubscriptions(of: LiveUpdates_SiteUpdateStream_Subscription.self) == 1
      && client.activeSubscriptions(
        of: LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self) == 1
      && client.activeSubscriptions(of: LiveUpdates_UserGoalUpdateStream_Subscription.self) == 1
      && client.activeSubscriptions(of: LiveUpdates_UserUsageUpdateStream_Subscription.self) == 1
  }

  @Test func subscribesToTheFourShellStreams() async throws {
    let setup = await start()
    try await waitOnMain { allSubscribed(setup.client) }

    #expect(
      setup.client.subscribed(LiveUpdates_SiteUpdateStream_Subscription.self).map(\.siteId) == [
        "site-1"
      ])
    #expect(
      setup.client.subscribed(LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self)
        .map(\.siteId) == ["site-1"])
    #expect(
      setup.client.subscribed(LiveUpdates_UserUsageUpdateStream_Subscription.self).map(\.userId)
        == ["user-1"])
  }

  @Test func startingTwiceSubscribesOnce() async throws {
    let setup = await start()
    setup.updates.start()
    try await waitOnMain { allSubscribed(setup.client) }
    try await Task.sleep(for: .milliseconds(30))

    #expect(setup.client.subscribed(LiveUpdates_SiteUpdateStream_Subscription.self).count == 1)
  }

  @Test func switchingSitesResubscribesOnlySiteStreams() async throws {
    let setup = await start()
    try await waitOnMain { allSubscribed(setup.client) }

    Container.shared.activeSite().select("site-2")

    try await waitOnMain {
      setup.client.subscribed(LiveUpdates_SiteUpdateStream_Subscription.self).map(\.siteId)
        == ["site-1", "site-2"]
    }
    try await waitOnMain { allSubscribed(setup.client) }
    #expect(
      setup.client.subscribed(LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self)
        .map(\.siteId) == ["site-1", "site-2"])
    #expect(setup.client.subscribed(LiveUpdates_UserGoalUpdateStream_Subscription.self).count == 1)
    #expect(setup.client.subscribed(LiveUpdates_UserUsageUpdateStream_Subscription.self).count == 1)
  }

  @Test func aNewUserResubscribesUsage() async throws {
    let setup = await start()
    try await waitOnMain { allSubscribed(setup.client) }

    await setup.authState.publish(
      .authenticated(AuthTokens(sessionToken: "s2", accessToken: "a2", userId: "user-2")))

    try await waitOnMain {
      setup.client.subscribed(LiveUpdates_UserUsageUpdateStream_Subscription.self).map(\.userId)
        == ["user-1", "user-2"]
        && setup.client.activeSubscriptions(of: LiveUpdates_UserUsageUpdateStream_Subscription.self)
          == 1
    }
    #expect(setup.client.subscribed(LiveUpdates_SiteUpdateStream_Subscription.self).count == 1)
    #expect(setup.client.subscribed(LiveUpdates_UserGoalUpdateStream_Subscription.self).count == 1)
  }

  @Test func aSameUserRenewalKeepsEveryStream() async throws {
    let setup = await start()
    try await waitOnMain { allSubscribed(setup.client) }

    await setup.authState.publish(
      .authenticated(AuthTokens(sessionToken: "s2", accessToken: "a2", userId: "user-1")))
    await drainMainActor()

    #expect(allSubscribed(setup.client))
    #expect(setup.client.subscribed(LiveUpdates_SiteUpdateStream_Subscription.self).count == 1)
    #expect(
      setup.client.subscribed(LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self).count
        == 1)
    #expect(setup.client.subscribed(LiveUpdates_UserGoalUpdateStream_Subscription.self).count == 1)
    #expect(setup.client.subscribed(LiveUpdates_UserUsageUpdateStream_Subscription.self).count == 1)
  }

  @Test func resettingTheSessionCancelsEveryStream() async throws {
    let (client, _, preferences) = await register(siteId: "site-1")
    Container.shared.liveUpdates().start()
    try await waitOnMain { allSubscribed(client) }

    Container.shared.manager.reset(scope: .session)

    try await waitOnMain {
      client.activeSubscriptions(of: LiveUpdates_SiteUpdateStream_Subscription.self) == 0
        && client.activeSubscriptions(
          of: LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self) == 0
        && client.activeSubscriptions(of: LiveUpdates_UserGoalUpdateStream_Subscription.self) == 0
        && client.activeSubscriptions(of: LiveUpdates_UserUsageUpdateStream_Subscription.self) == 0
    }
    withExtendedLifetime(preferences) {}
  }

  @Test func recentDocumentEventsRefetchOnlyMatchingWatches() async throws {
    let setup = await start()
    let client = setup.client
    let watchers = [
      client.watch(HomeScreen_Query(siteId: "site-1")) { _ in },
      client.watch(HomeScreen_Query(siteId: "site-9")) { _ in },
      client.watch(RecentDocuments_Query(siteId: "site-1", sort: .case(.viewedAt))) { _ in },
      client.watch(RecentDocuments_Query(siteId: "site-1", sort: .case(.updatedAt))) { _ in },
      client.watch(RecentDocuments_Query(siteId: "site-9", sort: .case(.updatedAt))) { _ in },
    ]
    try await waitOnMain { allSubscribed(client) }

    let updated = await LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.Data.from(
      Mock<GraphQLMocks.Subscription>(siteRecentDocumentsUpdateStream: .case(.updatedAt)))
    client.emit(updated, for: LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self)

    try await waitOnMain { client.refetched(HomeScreen_Query.self).count == 1 }
    #expect(client.refetched(HomeScreen_Query.self).map(\.siteId) == ["site-1"])
    #expect(client.refetched(RecentDocuments_Query.self).map(\.siteId) == ["site-1"])
    #expect(client.refetched(RecentDocuments_Query.self).map(\.sort) == [.case(.updatedAt)])

    let viewed = await LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.Data.from(
      Mock<GraphQLMocks.Subscription>(siteRecentDocumentsUpdateStream: .case(.viewedAt)))
    client.emit(viewed, for: LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self)

    try await waitOnMain { client.refetched(HomeScreen_Query.self).count == 2 }
    #expect(client.refetched(HomeScreen_Query.self).map(\.siteId) == ["site-1", "site-1"])
    #expect(client.refetched(RecentDocuments_Query.self).map(\.siteId) == ["site-1", "site-1"])
    #expect(
      client.refetched(RecentDocuments_Query.self).map(\.sort) == [
        .case(.updatedAt), .case(.viewedAt),
      ])
    withExtendedLifetime(watchers) {}
  }

  @Test func signingOutCancelsEveryStream() async throws {
    let setup = await start()
    try await waitOnMain { allSubscribed(setup.client) }

    await setup.authState.publish(.unauthenticated)

    try await waitOnMain {
      setup.client.activeSubscriptions(of: LiveUpdates_SiteUpdateStream_Subscription.self) == 0
        && setup.client.activeSubscriptions(
          of: LiveUpdates_SiteRecentDocumentsUpdateStream_Subscription.self) == 0
        && setup.client.activeSubscriptions(of: LiveUpdates_UserGoalUpdateStream_Subscription.self)
          == 0
        && setup.client.activeSubscriptions(of: LiveUpdates_UserUsageUpdateStream_Subscription.self)
          == 0
    }
  }
}
