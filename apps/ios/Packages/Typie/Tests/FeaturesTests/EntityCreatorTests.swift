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
@Suite(.container) struct EntityCreatorTests {
  private func make() -> (EntityCreator, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    return (Container.shared.entityCreator(), client)
  }

  private func created(document id: String) async
    -> EntityContainer_CreateDocument_Mutation.Data
  {
    await EntityContainer_CreateDocument_Mutation.Data.from(
      Mock<GraphQLMocks.Mutation>(
        createDocument: Mock<GraphQLMocks.Document>(
          entity: Mock<GraphQLMocks.Entity>(id: GraphQL.ID(id)), id: "\(id)-doc")))
  }

  private func created(folder id: String) async -> EntityContainer_CreateFolder_Mutation.Data {
    await EntityContainer_CreateFolder_Mutation.Data.from(
      Mock<GraphQLMocks.Mutation>(
        createFolder: Mock<GraphQLMocks.Folder>(
          entity: Mock<GraphQLMocks.Entity>(id: GraphQL.ID(id)), id: "\(id)-folder")))
  }

  @Test func createsDocumentAtRootAsV2() async {
    let (creator, client) = make()
    client.stub(
      .success(await created(document: "e1")), for: EntityContainer_CreateDocument_Mutation.self)
    let id = await creator.createDocument(siteId: "site-1", parentEntityId: nil)
    #expect(id == "e1")
    let input = client.performed(EntityContainer_CreateDocument_Mutation.self)[0].input
    #expect(input.siteId == "site-1")
    #expect(input.parentEntityId == nil)
    #expect(input.v2 == .some(true))
  }

  @Test func createsFolderInsideParentWithDefaultName() async {
    let (creator, client) = make()
    client.stub(
      .success(await created(folder: "e2")), for: EntityContainer_CreateFolder_Mutation.self)
    let id = await creator.createFolder(siteId: "site-1", parentEntityId: "f1")
    #expect(id == "e2")
    let input = client.performed(EntityContainer_CreateFolder_Mutation.self)[0].input
    #expect(input.name == "새 폴더")
    #expect(input.parentEntityId == .some("f1"))
  }

  @Test func failureReturnsNilAndResetsFlag() async {
    let (creator, client) = make()
    client.stub(
      .failure(StubError(name: "create")), for: EntityContainer_CreateDocument_Mutation.self)
    let id = await creator.createDocument(siteId: "site-1", parentEntityId: nil)
    #expect(id == nil)
    #expect(creator.isCreating == false)
  }

  @Test func rejectsConcurrentCreation() async {
    let (creator, client) = make()
    let gate = client.hold()
    client.stub(
      .success(await created(document: "e1")), for: EntityContainer_CreateDocument_Mutation.self)
    let first = Task { await creator.createDocument(siteId: "site-1", parentEntityId: nil) }
    await drainMainActor()
    let second = await creator.createDocument(siteId: "site-1", parentEntityId: nil)
    #expect(second == nil)
    await gate.open()
    #expect(await first.value == "e1")
  }
}
