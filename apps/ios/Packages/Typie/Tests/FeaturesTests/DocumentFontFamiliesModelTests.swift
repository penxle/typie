import ApolloTestSupport
import Editor
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import GraphQLMocks
import Observation
import Testing

@testable import Features

@MainActor
@Suite(.container) struct DocumentFontFamiliesModelTests {
  private func makeModel() -> (DocumentFontFamiliesModel, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    return (Container.shared.documentFontFamiliesModel(), client)
  }

  private func font(_ name: String, weight: UInt16) throws -> EditorFontFamily.Font {
    EditorFontFamily.Font(
      weight: weight, url: try #require(URL(string: "https://fonts.example.test/\(name)")),
      hash: "hash-\(name)")
  }

  @Test func asksForEverySource() {
    let (_, client) = makeModel()
    #expect(client.watchCount(of: DocumentScreen_FontFamilies_Query.self) == 1)
    #expect(
      DocumentScreen_FontFamilies_Query.operationDocument.definition?.queryDocument.contains(
        "documentFontFamilies(sources: [DEFAULT, USER, FALLBACK])") == true)
  }

  @Test func mapsEveryFamilyWithItsSourceAndFonts() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(
        await fontFamiliesData([
          familyMock(
            "family-a", source: .case(.default),
            fonts: [fontMock("a-400", weight: 400), fontMock("a-700", weight: 700)]),
          familyMock("family-b", source: .case(.user), fonts: [fontMock("b-400", weight: 400)]),
          familyMock(
            "family-c", source: .case(.fallback), fonts: [fontMock("c-400", weight: 400)]),
        ])), for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families != nil }
    #expect(
      model.families == [
        EditorFontFamily(
          name: "family-a", source: .default,
          fonts: [try font("a-400", weight: 400), try font("a-700", weight: 700)]),
        EditorFontFamily(
          name: "family-b", source: .user, fonts: [try font("b-400", weight: 400)]),
        EditorFontFamily(
          name: "family-c", source: .fallback, fonts: [try font("c-400", weight: 400)]),
      ])
  }

  @Test func leavesOutFontsItCannotLoad() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(
        await fontFamiliesData([
          familyMock(
            "family-a", source: .case(.user),
            fonts: [
              fontMock("a-100", weight: 100, hash: ""),
              fontMock("a-200", weight: 200, url: ""),
              fontMock("a-300", weight: 70_000),
              fontMock("a-400", weight: 400),
            ])
        ])), for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families != nil }
    #expect(
      model.families == [
        EditorFontFamily(name: "family-a", source: .user, fonts: [try font("a-400", weight: 400)])
      ])
  }

  @Test func leavesOutFamiliesFromAnUnknownSource() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(
        await fontFamiliesData([
          familyMock(
            "family-a", source: .unknown("NEW"), fonts: [fontMock("a-400", weight: 400)]),
          familyMock(
            "family-b", source: .case(.default), fonts: [fontMock("b-400", weight: 400)]),
        ])), for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families != nil }
    #expect(model.families?.map(\.name) == ["family-b"])
  }

  @Test func keepsTheLastListWhenAReloadFails() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(await fontFamiliesData([defaultFamilyMock()])),
      for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families != nil }
    client.push(.failure(StubError(name: "reload")), for: DocumentScreen_FontFamilies_Query.self)
    await drainMainActor()
    #expect(model.families?.map(\.name) == ["family-a"])
    #expect(model.loadFailed == false)
  }

  @Test func failureWithoutDataSetsLoadFailed() async throws {
    let (model, client) = makeModel()
    client.push(.failure(StubError(name: "load")), for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.loadFailed }
    #expect(model.families == nil)
  }

  @Test func failureWithoutDataSetsLoadFailedAndDataRecovers() async throws {
    let (model, client) = makeModel()
    client.push(.failure(StubError(name: "load")), for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.loadFailed }
    client.push(
      .success(await fontFamiliesData([defaultFamilyMock()])),
      for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families != nil }
    #expect(model.families?.map(\.name) == ["family-a"])
    #expect(model.loadFailed == false)
  }

  @Test func announcesOnlyAChangedList() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(await fontFamiliesData([defaultFamilyMock()])),
      for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families != nil }
    let recorder = CallRecorder()
    withObservationTracking {
      _ = model.families
      _ = model.loadFailed
    } onChange: {
      recorder.record("families")
    }
    client.push(
      .success(await fontFamiliesData([defaultFamilyMock()])),
      for: DocumentScreen_FontFamilies_Query.self)
    await drainMainActor()
    #expect(recorder.calls.isEmpty)
    client.push(
      .success(
        await fontFamiliesData([
          defaultFamilyMock(),
          familyMock("family-b", source: .case(.user), fonts: [fontMock("b-400", weight: 400)]),
        ])), for: DocumentScreen_FontFamilies_Query.self)
    try await waitOnMain { model.families?.count == 2 }
    #expect(recorder.calls == ["families"])
  }
}

private func fontFamiliesData(_ families: [Mock<GraphQLMocks.DocumentFontFamily>]) async
  -> DocumentScreen_FontFamilies_Query.Data
{
  await DocumentScreen_FontFamilies_Query.Data.from(
    Mock<GraphQLMocks.Query>(
      me: Mock<GraphQLMocks.User>(documentFontFamilies: families, id: "user-1")))
}

private func familyMock(
  _ name: String, source: GraphQLEnum<GraphQL.FontFamilySource>,
  fonts: [Mock<GraphQLMocks.DocumentFont>]
) -> Mock<GraphQLMocks.DocumentFontFamily> {
  Mock<GraphQLMocks.DocumentFontFamily>(
    familyName: name, fonts: fonts, id: GraphQL.ID("id-\(name)"), source: source)
}

private func defaultFamilyMock() -> Mock<GraphQLMocks.DocumentFontFamily> {
  familyMock("family-a", source: .case(.default), fonts: [fontMock("a-400", weight: 400)])
}

private func fontMock(_ name: String, weight: Int, url: String? = nil, hash: String? = nil)
  -> Mock<GraphQLMocks.DocumentFont>
{
  Mock<GraphQLMocks.DocumentFont>(
    hash: hash ?? "hash-\(name)", id: GraphQL.ID("id-\(name)"),
    url: url ?? "https://fonts.example.test/\(name)", weight: weight)
}
