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
@Suite(.container) struct FolderContentsStoreTests {
  private static let initial = EntityFolderItem(
    entityId: "f1", icon: EntityIconSpec(kind: .folder, name: "book", color: "blue"), path: [],
    title: "소설", folderCount: 1, documentCount: 2)

  private func makeStore(initial: EntityFolderItem? = Self.initial, title: String = "소설")
    -> (FolderContentsStore, FakeGraphQLClient)
  {
    let client = FakeGraphQLClient()
    registerFake(client)
    return (FolderContentsStore(entityId: "f1", initial: initial, title: title), client)
  }

  @Test func heroFromInitialBeforeArrival() async throws {
    let (store, client) = makeStore()
    try await waitOnMain { client.watchCount(of: FolderContents_Query.self) == 1 }
    #expect(client.watched(FolderContents_Query.self).map(\.entityId) == ["f1"])
    #expect(store.isPlaceholder)
    #expect(store.hero.title == "소설")
    #expect(store.hero.icon == Self.initial.icon)
    #expect(store.hero.summary == "폴더 1개 · 문서 2개")
  }

  @Test func heroWithoutInitialUsesTitleOnly() async {
    let (store, _) = makeStore(initial: nil, title: "검색된 폴더")
    await drainMainActor()
    #expect(store.hero.title == "검색된 폴더")
    #expect(store.hero.icon == EntityIconSpec(kind: .folder, name: "", color: ""))
    #expect(store.hero.summary == " ")
  }

  @Test func initialWithZeroCountsKeepsSummaryBlank() async {
    let zero = EntityFolderItem(
      entityId: "f1", icon: EntityIconSpec(kind: .folder, name: "", color: ""), path: [],
      title: "빈 폴더", folderCount: 0, documentCount: 0)
    let (store, _) = makeStore(initial: zero)
    await drainMainActor()
    #expect(store.hero.summary == " ")
  }

  @Test func arrivalFillsHeroAndChildren() async throws {
    let (store, client) = makeStore()
    client.push(
      .success(
        await folderContentsData(
          name: "소설 (서버)", folderCount: 1, documentCount: 3, characterCount: 12345,
          children: [
            entityMock(id: "d1", kind: .document, title: "1장"), dividerEntityMock(id: "x1"),
            folderEntityMock(id: "f2", name: "자료", childCount: 0),
          ], icon: "notebook", iconColor: "red")),
      for: FolderContents_Query.self)
    try await waitOnMain { store.hasData }
    #expect(store.hero.title == "소설 (서버)")
    #expect(store.hero.icon == EntityIconSpec(kind: .folder, name: "notebook", color: "red"))
    #expect(store.hero.summary == "폴더 1개 · 문서 3개 · 총 12,345자")
    #expect(store.items.map(\.id) == ["d1", "x1", "f2"])
    #expect(store.isPlaceholder == false)
  }

  @Test func failureWithoutDataSetsLoadFailed() async throws {
    let (store, client) = makeStore()
    client.push(.failure(StubError(name: "load")), for: FolderContents_Query.self)
    try await waitOnMain { store.loadFailed }
    #expect(store.hasData == false)
    #expect(store.folder == nil)
    #expect(store.isPlaceholder == false)
    #expect(store.hero.title == "소설")
  }
}
