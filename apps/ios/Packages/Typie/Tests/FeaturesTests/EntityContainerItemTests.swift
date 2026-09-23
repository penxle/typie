import ApolloTestSupport
import Core
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@Suite struct EntityContainerItemTests {
  private func rows(_ mocks: [Mock<GraphQLMocks.Entity>]) async -> [EntityRow_entity] {
    let data = await siteEntitiesData(entities: mocks)
    return data.site.entities.map { $0.fragments.entityRow_entity }
  }

  @Test func mapsDocumentWithSubtitleAndExcerpt() async {
    let mock = entityMock(id: "d1", kind: .document, title: "합성 문서")
    mock.node = Mock<GraphQLMocks.Document>(
      excerpt: "첫 문단", id: "d1-doc", subtitle: "부제", title: "합성 문서",
      updatedAt: "2026-09-20T15:00:00.000Z")
    let row = await rows([mock])[0]
    guard case .entity(.document(let item))? = EntityContainerItem.make(row) else {
      Issue.record("document expected")
      return
    }
    #expect(item.entityId == "d1")
    #expect(item.title == "합성 문서")
    #expect(item.subtitle == "부제")
    #expect(item.excerpt == "첫 문단")
    #expect(item.updatedAt == parseDateTime("2026-09-20T15:00:00.000Z"))
  }

  @Test func mapsFolderWithCounts() async {
    let row = await rows([folderEntityMock(id: "f1", name: "소설", childCount: 0)])[0]
    guard case .entity(.folder(let item))? = EntityContainerItem.make(row) else {
      Issue.record("folder expected")
      return
    }
    #expect(item.entityId == "f1")
    #expect(item.folderCount == 0)
    #expect(item.documentCount == 0)
    #expect(item.summary == "빈 폴더")
  }

  @Test func mapsDividerById() async {
    let row = await rows([dividerEntityMock(id: "x1")])[0]
    #expect(EntityContainerItem.make(row) == .divider(id: "x1"))
    #expect(EntityContainerItem.make(row)?.id == "x1")
    #expect(EntityContainerItem.make(row)?.entity == nil)
  }

  @Test func placeholderRowsAreThreeDocuments() {
    #expect(EntityContainerItem.placeholderRows.count == 3)
    #expect(EntityContainerItem.placeholderRows.allSatisfy { $0.entity != nil })
    #expect(Set(EntityContainerItem.placeholderRows.map(\.id)).count == 3)
  }
}
