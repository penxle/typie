import ApolloTestSupport
import Core
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@Suite struct EntityRowItemTests {
  private func row(_ mock: Mock<GraphQLMocks.Entity>) async -> (
    EntityRow_entity, EntityRowPath_entity
  ) {
    let hit = Mock<GraphQLMocks.SearchHitDocument>(
      document: Mock<GraphQLMocks.Document>(entity: mock, id: GraphQL.ID("d-\(mock.id ?? "")")),
      subtitle: nil, text: nil, title: nil)
    let data = await SearchScreen_Search_Query.Data.from(
      Mock<GraphQLMocks.Query>(search: Mock<GraphQLMocks.SearchResult>(hits: [hit])))
    let entity = data.search.hits[0].asSearchHitDocument!.fragments.searchResultDocument_hit
      .document.entity
    return (entity.fragments.entityRow_entity, entity.fragments.entityRowPath_entity)
  }

  private func folderRow(_ mock: Mock<GraphQLMocks.Entity>) async -> (
    EntityRow_entity, EntityRowPath_entity
  ) {
    let hit = Mock<GraphQLMocks.SearchHitFolder>(
      folder: Mock<GraphQLMocks.Folder>(entity: mock, id: GraphQL.ID("f-\(mock.id ?? "")")),
      name: nil)
    let data = await SearchScreen_Search_Query.Data.from(
      Mock<GraphQLMocks.Query>(search: Mock<GraphQLMocks.SearchResult>(hits: [hit])))
    let entity = data.search.hits[0].asSearchHitFolder!.fragments.searchResultFolder_hit
      .folder.entity
    return (entity.fragments.entityRow_entity, entity.fragments.entityRowPath_entity)
  }

  @Test func mapsDocumentWithPathAndUpdatedAt() async {
    let (entity, path) = await row(
      entityMock(
        id: "e1", kind: .document, title: "합성 문서", updatedAt: "2026-09-20T15:00:00.000Z",
        ancestors: [(id: "f1", name: "폴더 A"), (id: "f2", name: "")], icon: "i1",
        iconColor: "c1"))
    guard case .document(let item)? = EntityRowItem.make(entity, path: path) else {
      Issue.record("document expected")
      return
    }
    #expect(item.entityId == "e1")
    #expect(item.title == "합성 문서")
    #expect(item.path == ["폴더 A", EntityText.folderName("")])
    #expect(item.icon == EntityIconSpec(kind: .document, name: "i1", color: "c1"))
    #expect(item.updatedAt == parseDateTime("2026-09-20T15:00:00.000Z"))
  }

  @Test func unparsableUpdatedAtHasNoTrailing() async {
    let (entity, path) = await row(
      entityMock(id: "e6", kind: .document, title: "t", updatedAt: "x"))
    let made = EntityRowItem.make(entity, path: path)
    guard case .document(let item)? = made else {
      Issue.record("document expected")
      return
    }
    #expect(item.updatedAt == nil)
    #expect(made?.trailing(now: Date()) == nil)
  }

  @Test func mapsFolderWithSummary() async {
    let (entity, path) = await folderRow(entityMock(id: "e2", kind: .folder, title: "합성 폴더"))
    guard case .folder(let item)? = EntityRowItem.make(entity, path: path) else {
      Issue.record("folder expected")
      return
    }
    #expect(item.title == "합성 폴더")
    #expect(item.summary == EntityText.folderSummary(folders: 1, documents: 2))
    #expect(item.path.isEmpty)
  }

  @Test func emptyDocumentTitleUsesFallback() async {
    let (entity, path) = await row(entityMock(id: "e3", kind: .document, title: ""))
    #expect(EntityRowItem.make(entity, path: path)?.title == EntityText.documentTitle(""))
  }

  @Test func nonDocumentNonFolderNodeIsNotMapped() async {
    let (entity, path) = await row(dividerEntityMock(id: "e7"))
    #expect(EntityRowItem.make(entity, path: path) == nil)
  }

  @Test func trailingIsTimeAgoForDocumentAndSummaryForFolder() async {
    let now = parseDateTime("2026-09-20T15:10:00.000Z")!
    let (document, path) = await row(entityMock(id: "e4", kind: .document, title: "t"))
    #expect(
      EntityRowItem.make(document, path: path)?.trailing(now: now)
        == timeAgo(parseDateTime("2026-09-20T15:00:00.000Z")!, now: now))
    let (folder, folderPath) = await folderRow(
      entityMock(id: "e5", kind: .folder, title: "f"))
    #expect(
      EntityRowItem.make(folder, path: folderPath)?.trailing(now: now)
        == EntityText.folderSummary(folders: 1, documents: 2))
  }
}
