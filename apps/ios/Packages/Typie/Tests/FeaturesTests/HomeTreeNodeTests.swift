import ApolloTestSupport
import Core
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@Suite struct HomeTreeNodeTests {
  private func roots(_ mocks: [Mock<GraphQLMocks.Entity>]) async -> [HomeTree_entity] {
    let data = await HomeScreen_Query.Data.from(
      Mock<GraphQLMocks.Query>(me: nil, site: homeSiteMock(roots: mocks)))
    return data.site.fragments.homeScreen_site.entities.map { $0.fragments.homeTree_entity }
  }

  @Test func mapsFolderWithChildCount() async {
    let entities = await roots([folderEntityMock(id: "f1", name: "소설", childCount: 3)])
    guard case .folder(let item, let childCount)? = HomeTreeNode.make(entities[0]) else {
      Issue.record("folder expected")
      return
    }
    #expect(item.entityId == "f1")
    #expect(item.title == "소설")
    #expect(item.path.isEmpty)
    #expect(childCount == 3)
  }

  @Test func mapsDocument() async {
    let entities = await roots([entityMock(id: "d1", kind: .document, title: "합성 문서")])
    guard case .document(let item)? = HomeTreeNode.make(entities[0]) else {
      Issue.record("document expected")
      return
    }
    #expect(item.entityId == "d1")
    #expect(HomeTreeNode.make(entities[0])?.id == "d1")
  }

  @Test func mapsDividerWithoutItem() async {
    let entities = await roots([dividerEntityMock(id: "x1")])
    let node = HomeTreeNode.make(entities[0])
    #expect(node == .divider(id: "x1"))
    #expect(node?.item == nil)
  }
}
