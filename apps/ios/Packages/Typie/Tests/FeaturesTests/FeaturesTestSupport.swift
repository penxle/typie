import ApolloTestSupport
import FactoryKit
import Foundation
import GraphQL
import GraphQLMocks

@testable import Core

struct WaitTimeout: Error {}

struct StubError: Error, Equatable {
  let name: String
}

@MainActor
func waitOnMain(_ condition: @MainActor () -> Bool) async throws {
  for _ in 0..<1000 {
    if condition() { return }
    try await Task.sleep(for: .milliseconds(2))
  }
  throw WaitTimeout()
}

@MainActor
func registerFake(_ client: FakeGraphQLClient) {
  Container.shared.graphQLClient.register { client }
}

final class TestPreferences {
  let userPreferences: UserPreferences

  private let suiteName: String

  init(siteId: String?, recentSearches: [String]) {
    suiteName = "features-\(UUID().uuidString)"
    userPreferences = UserPreferences(
      userId: "user-1", defaults: UserDefaults(suiteName: suiteName)!)
    userPreferences.siteId = siteId
    userPreferences.recentSearches = recentSearches
  }

  deinit {
    UserDefaults().removePersistentDomain(forName: suiteName)
  }

  var recentSearches: [String] { userPreferences.recentSearches }
}

func makeTestPreferences(siteId: String? = nil, recentSearches: [String] = []) -> TestPreferences {
  TestPreferences(siteId: siteId, recentSearches: recentSearches)
}

func siteSwitcherData(_ ids: [String]) async -> SiteSwitcher_Query.Data {
  await SiteSwitcher_Query.Data.from(
    Mock<GraphQLMocks.Query>(
      me: Mock<GraphQLMocks.User>(
        id: "user-1",
        sites: ids.map {
          Mock<GraphQLMocks.Site>(
            id: GraphQL.ID($0),
            logo: Mock<GraphQLMocks.Image>(
              height: 64, id: GraphQL.ID("img-\($0)"),
              url: "https://img.example.test/\($0).png", width: 64),
            name: "n-\($0)", url: "https://\($0).example.test")
        })))
}

func createdSiteData(_ id: String) async -> SiteSwitcher_CreateSite_Mutation.Data {
  await SiteSwitcher_CreateSite_Mutation.Data.from(
    Mock<GraphQLMocks.Mutation>(createSite: Mock<GraphQLMocks.Site>(id: GraphQL.ID(id))))
}

@MainActor
func drainMainActor() async {
  for _ in 0..<8 { await Task.yield() }
}

actor Gate {
  private var opened = false
  private var waiters: [CheckedContinuation<Void, Never>] = []

  func wait() async {
    if opened { return }
    await withCheckedContinuation { waiters.append($0) }
  }

  func open() {
    opened = true
    let pending = waiters
    waiters = []
    for continuation in pending { continuation.resume() }
  }
}

final class CallRecorder: @unchecked Sendable {
  private let lock = NSLock()
  private var entries: [String] = []

  func record(_ entry: String) {
    lock.withLock { entries.append(entry) }
  }

  var calls: [String] {
    lock.withLock { entries }
  }
}
