import ApolloTestSupport
import FactoryKit
import FactoryTesting
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@MainActor
@Suite(.container) struct ProfileModelTests {
  private func makeModel() -> (ProfileModel, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    return (Container.shared.profileModel(), client)
  }

  private func profileData() async -> ProfileScreen_Query.Data {
    await ProfileScreen_Query.Data.from(
      Mock<GraphQLMocks.Query>(
        me: Mock<GraphQLMocks.User>(
          avatar: Mock<GraphQLMocks.Image>(
            height: 64, id: GraphQL.ID("avatar-1"), url: "https://img.example.test/a.png",
            width: 64),
          email: "user@example.test", id: GraphQL.ID("user-1"), name: "n-user")))
  }

  @Test func exposesNameEmailAndAvatar() async throws {
    let (model, client) = makeModel()
    client.push(.success(await profileData()), for: ProfileScreen_Query.self)
    try await waitOnMain { model.profile != nil }
    #expect(model.profile?.name == "n-user")
    #expect(model.profile?.email == "user@example.test")
    #expect(model.profile?.avatar.url == "https://img.example.test/a.png")
    #expect(model.loadFailed == false)
  }

  @Test func failureWithoutDataSetsLoadFailed() async throws {
    let (model, client) = makeModel()
    client.push(.failure(StubError(name: "load")), for: ProfileScreen_Query.self)
    try await waitOnMain { model.loadFailed }
    #expect(model.profile == nil)
  }

  @Test func failureAfterDataKeepsProfile() async throws {
    let (model, client) = makeModel()
    client.push(.success(await profileData()), for: ProfileScreen_Query.self)
    try await waitOnMain { model.profile != nil }
    client.push(.failure(StubError(name: "reload")), for: ProfileScreen_Query.self)
    await drainMainActor()
    #expect(model.loadFailed == false)
    #expect(model.profile?.name == "n-user")
  }
}
