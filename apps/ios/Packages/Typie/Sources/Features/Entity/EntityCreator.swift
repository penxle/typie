import Core
import FactoryKit
import GraphQL
import Observation

@MainActor @Observable
final class EntityCreator {
  private(set) var isCreating = false

  @ObservationIgnored private let client: any GraphQLClient

  init() {
    client = Container.shared.graphQLClient()
  }

  func createDocument(siteId: String, parentEntityId: String?) async -> String? {
    await perform {
      try await client.perform(
        EntityContainer_CreateDocument_Mutation(
          input: CreateDocumentInput(
            parentEntityId: parentEntityId.map { .some($0) } ?? nil, siteId: siteId,
            v2: .some(true)))
      ).createDocument.entity.id
    }
  }

  func createFolder(siteId: String, parentEntityId: String?) async -> String? {
    await perform {
      try await client.perform(
        EntityContainer_CreateFolder_Mutation(
          input: CreateFolderInput(
            name: "새 폴더", parentEntityId: parentEntityId.map { .some($0) } ?? nil,
            siteId: siteId))
      ).createFolder.entity.id
    }
  }

  func createDivider(siteId: String, parentEntityId: String?) async -> String? {
    await perform {
      try await client.perform(
        EntityContainer_CreateDivider_Mutation(
          input: CreateDividerInput(
            parentEntityId: parentEntityId.map { .some($0) } ?? nil, siteId: siteId))
      ).createDivider.entity.id
    }
  }

  private func perform(_ operation: () async throws -> String) async -> String? {
    guard !isCreating else { return nil }
    isCreating = true
    defer { isCreating = false }
    do {
      return try await operation()
    } catch {
      return nil
    }
  }
}
