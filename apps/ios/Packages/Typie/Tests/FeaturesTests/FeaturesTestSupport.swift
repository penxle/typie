import ApolloTestSupport
import FactoryKit
import Foundation
import GraphQL
import GraphQLMocks

@testable import Core
@testable import Features

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

func goalUserMock(
  target: Int?, history: [(date: String, additions: Int, achieved: Bool)],
  today: (date: String, additions: Int)
) -> Mock<GraphQLMocks.User> {
  Mock<GraphQLMocks.User>(
    goal: target.map {
      Mock<GraphQLMocks.UserGoal>(id: GraphQL.ID("goal-1"), targetCharacterCount: $0)
    },
    goalHistory: history.map {
      Mock<GraphQLMocks.UserGoalHistory>(
        achieved: $0.achieved, additions: $0.additions, date: $0.date,
        targetCharacterCount: target ?? 0)
    },
    id: "user-1",
    todayCharacterCountChange: Mock<GraphQLMocks.CharacterCountChange>(
      additions: today.additions, date: today.date))
}

func entityMock(
  id: String, kind: EntityKind, title: String, updatedAt: String = "2026-09-20T15:00:00.000Z",
  ancestors: [(id: String, name: String)] = [], icon: String = "", iconColor: String = ""
) -> Mock<GraphQLMocks.Entity> {
  let node: any AnyMock =
    switch kind {
    case .document:
      Mock<GraphQLMocks.Document>(
        excerpt: "", id: "\(id)-doc", subtitle: nil, title: title, updatedAt: updatedAt)
    case .folder:
      Mock<GraphQLMocks.Folder>(
        documentCount: 2, folderCount: 1, id: "\(id)-folder", name: title)
    }
  return Mock<GraphQLMocks.Entity>(
    ancestors: ancestors.map {
      Mock<GraphQLMocks.Entity>(
        id: $0.id, node: Mock<GraphQLMocks.Folder>(id: "\($0.id)-folder", name: $0.name))
    },
    icon: icon, iconColor: iconColor, id: id, node: node,
    type: .case(kind == .document ? .document : .folder))
}

func dividerEntityMock(id: String) -> Mock<GraphQLMocks.Entity> {
  Mock<GraphQLMocks.Entity>(
    ancestors: [], icon: "", iconColor: "", id: id, node: Mock<GraphQLMocks.Divider>(),
    type: .case(.divider))
}

func folderEntityMock(
  id: String, name: String, childCount: Int, icon: String = "", iconColor: String = ""
) -> Mock<GraphQLMocks.Entity> {
  Mock<GraphQLMocks.Entity>(
    ancestors: [], icon: icon, iconColor: iconColor, id: id,
    node: Mock<GraphQLMocks.Folder>(
      childCount: childCount, documentCount: 0, folderCount: 0, id: "\(id)-folder", name: name),
    type: .case(.folder))
}

func homeSiteMock(
  id: String = "site-1", name: String = "스페이스 A",
  pinned: [Mock<GraphQLMocks.Entity>] = [], roots: [Mock<GraphQLMocks.Entity>] = []
) -> Mock<GraphQLMocks.Site> {
  Mock<GraphQLMocks.Site>(entities: roots, id: id, name: name, pinnedEntities: pinned)
}

func childrenData(parentId: String, children: [Mock<GraphQLMocks.Entity>]) async
  -> HomeTree_Children_Query.Data
{
  await HomeTree_Children_Query.Data.from(
    Mock<GraphQLMocks.Query>(
      entity: Mock<GraphQLMocks.Entity>(
        ancestors: [], children: children, icon: "", iconColor: "", id: parentId,
        node: Mock<GraphQLMocks.Folder>(
          childCount: children.count, documentCount: 0, folderCount: 0,
          id: "\(parentId)-folder", name: "부모"),
        type: .case(.folder))))
}

func homeData(
  target: Int?, history: [(date: String, additions: Int, achieved: Bool)],
  today: (date: String, additions: Int), site: Mock<GraphQLMocks.Site> = homeSiteMock()
) async -> HomeScreen_Query.Data {
  await HomeScreen_Query.Data.from(
    Mock<GraphQLMocks.Query>(
      me: goalUserMock(target: target, history: history, today: today), site: site))
}

func homeDataWithoutMe() async -> HomeScreen_Query.Data {
  await HomeScreen_Query.Data.from(
    Mock<GraphQLMocks.Query>(me: nil, site: homeSiteMock()))
}

func userGoalScreenData(
  target: Int?, history: [(date: String, additions: Int, achieved: Bool)],
  today: (date: String, additions: Int)
) async -> UserGoalScreen_Query.Data {
  await UserGoalScreen_Query.Data.from(
    Mock<GraphQLMocks.Query>(
      me: goalUserMock(target: target, history: history, today: today)))
}

func updatedUserGoalData(target: Int) async -> UserGoalScreen_UpdateUserGoal_Mutation.Data {
  await UserGoalScreen_UpdateUserGoal_Mutation.Data.from(
    Mock<GraphQLMocks.Mutation>(
      updateUserGoal: goalUserMock(
        target: target, history: [], today: (date: "2026-09-20T15:00:00.000Z", additions: 0))))
}

func deletedUserGoalData() async -> UserGoalScreen_DeleteUserGoal_Mutation.Data {
  await UserGoalScreen_DeleteUserGoal_Mutation.Data.from(
    Mock<GraphQLMocks.Mutation>(
      deleteUserGoal: goalUserMock(
        target: nil, history: [], today: (date: "2026-09-20T15:00:00.000Z", additions: 0))))
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

func userGoalDayData(_ documents: [(id: String, title: String, additions: Int)]) async
  -> UserGoalDay_Query.Data
{
  await UserGoalDay_Query.Data.from(
    Mock<GraphQLMocks.Query>(
      me: Mock<GraphQLMocks.User>(
        dailyDocumentCharacterCountChanges: documents.map {
          Mock<GraphQLMocks.DocumentCharacterCountChange>(
            additions: $0.additions,
            document: Mock<GraphQLMocks.Document>(
              entity: Mock<GraphQLMocks.Entity>(id: GraphQL.ID("\($0.id)-entity")),
              id: GraphQL.ID($0.id), title: $0.title))
        },
        id: "user-1")))
}
