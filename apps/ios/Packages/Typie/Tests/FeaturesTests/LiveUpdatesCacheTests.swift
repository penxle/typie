import Apollo
import ApolloTestSupport
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

struct LiveUpdatesCacheTests {
  @Test func documentEventRefreshesAncestorFolderCharacterCount() async throws {
    let store = ApolloStore()
    let folder = FolderContents_Query(entityId: "f1")
    let contents = await folderContentsData(id: "f1", characterCount: 100)
    try await store.withinReadWriteTransaction { try await $0.write(data: contents, for: folder) }

    let document = entityMock(id: "d1", kind: .document, title: "1장")
    document.ancestors = [
      Mock<GraphQLMocks.Entity>(
        id: "f1", node: Mock<GraphQLMocks.Folder>(characterCount: 250, id: "f1-folder"))
    ]
    document.children = []
    let event = await LiveUpdates_SiteUpdateStream_Subscription.Data.from(
      Mock<GraphQLMocks.Subscription>(siteUpdateStream: document))
    try await store.withinReadWriteTransaction {
      try await $0.write(
        data: event, for: LiveUpdates_SiteUpdateStream_Subscription(siteId: "site-1"))
    }

    let read = try await store.withinReadTransaction { try await $0.read(query: folder) }
    #expect(read.entity.node.asFolder?.characterCount == 250)
  }
}
