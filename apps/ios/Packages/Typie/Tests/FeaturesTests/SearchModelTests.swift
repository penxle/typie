import ApolloTestSupport
import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

private func documentHit(
  id: String, title: String? = nil, text: String? = nil, docTitle: String = "doc",
  subtitle: String? = nil, excerpt: String = "", updatedAt: String = "2027-01-02T03:04:05.678Z",
  ancestors: [String] = []
) -> Mock<GraphQLMocks.SearchHitDocument> {
  let node = Mock<GraphQLMocks.Document>(
    excerpt: excerpt, id: GraphQL.ID("d-\(id)"), subtitle: subtitle, title: docTitle,
    updatedAt: updatedAt)
  let entity = Mock<GraphQLMocks.Entity>(
    ancestors: ancestors.enumerated().map { index, name in
      Mock<GraphQLMocks.Entity>(
        id: GraphQL.ID("a-\(index)"),
        node: Mock<GraphQLMocks.Folder>(id: GraphQL.ID("f-a-\(index)"), name: name))
    },
    id: GraphQL.ID(id), node: node, type: .case(.document))
  return Mock<GraphQLMocks.SearchHitDocument>(
    document: Mock<GraphQLMocks.Document>(entity: entity, id: GraphQL.ID("d-\(id)")),
    subtitle: nil, text: text, title: title)
}

private func folderHit(id: String, folders: Int, documents: Int)
  -> Mock<GraphQLMocks.SearchHitFolder>
{
  let node = Mock<GraphQLMocks.Folder>(
    documentCount: documents, folderCount: folders, id: GraphQL.ID("f-\(id)"), name: "")
  let entity = Mock<GraphQLMocks.Entity>(id: GraphQL.ID(id), node: node, type: .case(.folder))
  return Mock<GraphQLMocks.SearchHitFolder>(
    folder: Mock<GraphQLMocks.Folder>(entity: entity, id: GraphQL.ID("f-\(id)")), name: nil)
}

private func searchData(_ hits: sending [any AnyMock]) async
  -> SearchScreen_Search_Query.Data
{
  await SearchScreen_Search_Query.Data.from(
    Mock<GraphQLMocks.Query>(search: Mock<GraphQLMocks.SearchResult>(hits: hits)))
}

@MainActor
@Suite(.container) struct SearchModelTests {
  private struct Context {
    let model: SearchModel
    let client: FakeGraphQLClient
    let preferences: TestPreferences
  }

  private func makeContext(
    siteId: String? = "site-1", recent: [String] = [], debounce: Duration = .milliseconds(20)
  ) -> Context {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: siteId, recentSearches: recent)
    Container.shared.userPreferences.register { preferences.userPreferences }
    return Context(
      model: SearchModel(debounce: debounce), client: client, preferences: preferences)
  }

  private func push(_ context: Context, _ hits: sending [any AnyMock]) async {
    context.client.push(.success(await searchData(hits)), for: SearchScreen_Search_Query.self)
  }

  private func pushFailure(_ context: Context) {
    context.client.push(.failure(HTTPError.status(500)), for: SearchScreen_Search_Query.self)
  }

  private static func watches(_ context: Context) -> Int {
    context.client.watchCount(of: SearchScreen_Search_Query.self)
  }

  private static func hits(_ context: Context) -> [SearchHit] {
    if case .results(let hits) = context.model.content { return hits }
    return []
  }

  @Test func blankTextShowsRecentAndDoesNotQuery() async throws {
    let context = makeContext(recent: ["a"])
    #expect(context.model.content == .recent)
    #expect(context.model.recentSearches == ["a"])
    context.model.setText("   ")
    try await Task.sleep(for: .milliseconds(60))
    #expect(context.model.content == .recent)
    #expect(Self.watches(context) == 0)
  }

  @Test func debouncesBeforeQuerying() async throws {
    let context = makeContext(debounce: .milliseconds(150))
    context.model.setText("q")
    #expect(context.model.content == .pending)
    try await Task.sleep(for: .milliseconds(50))
    #expect(Self.watches(context) == 0)
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [documentHit(id: "1")])
    try await waitOnMain { context.model.content != .pending }
    #expect(Self.hits(context).map(\.entityId) == ["1"])
  }

  @Test func debounceCancelsSupersededKeystroke() async throws {
    let context = makeContext(debounce: .milliseconds(60))
    context.model.setText("a")
    try await Task.sleep(for: .milliseconds(30))
    context.model.setText("ab")
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [documentHit(id: "1")])
    try await waitOnMain { !Self.hits(context).isEmpty }
    try await Task.sleep(for: .milliseconds(100))
    #expect(Self.watches(context) == 1)
    #expect(Self.hits(context).map(\.entityId) == ["1"])
  }

  @Test func blankTextStopsRunningQuery() async throws {
    let context = makeContext(debounce: .seconds(10))
    context.model.setText("q")
    context.model.submit()
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [documentHit(id: "1")])
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.setText("")
    #expect(context.model.content == .recent)
    try await waitOnMain { context.model.hits == nil }
    context.model.setText("q")
    context.model.submit()
    try await waitOnMain { Self.watches(context) == 2 }
  }

  @Test func submitQueriesImmediately() async throws {
    let context = makeContext(debounce: .seconds(10))
    context.model.setText("q")
    context.model.submit()
    try await waitOnMain { Self.watches(context) == 1 }
  }

  @Test func keepsPreviousResultsWhileTyping() async throws {
    let context = makeContext()
    context.model.setText("a")
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [documentHit(id: "1")])
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.setText("ab")
    #expect(Self.hits(context).map(\.entityId) == ["1"])
    try await waitOnMain { Self.watches(context) == 2 }
    await push(context, [documentHit(id: "2")])
    try await waitOnMain { Self.hits(context).map(\.entityId) == ["2"] }
  }

  @Test func emptyResultIsEmpty() async throws {
    let context = makeContext()
    context.model.setText("q")
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [])
    try await waitOnMain { context.model.content == .empty }
  }

  @Test func failureWithoutDataIsFailed() async throws {
    let context = makeContext()
    context.model.setText("q")
    try await waitOnMain { Self.watches(context) == 1 }
    pushFailure(context)
    try await waitOnMain { context.model.content == .failed }
  }

  @Test(.timeLimit(.minutes(1))) func failureWithDataKeepsResults() async throws {
    let context = makeContext()
    context.model.setText("a")
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [documentHit(id: "1")])
    try await waitOnMain { Self.hits(context).map(\.entityId) == ["1"] }

    let observed = CallRecorder()
    keepObserving(while: context.model) { [weak model = context.model] in
      guard model != nil else { return }
      observed.record(Self.hits(context).map(\.entityId).joined(separator: ","))
    }

    pushFailure(context)
    await drainMainActor()

    await push(context, [documentHit(id: "1"), documentHit(id: "3")])
    try await waitOnMain { Self.hits(context).map(\.entityId) == ["1", "3"] }

    #expect(!observed.calls.contains(""))
  }

  @Test func selectingRecentSetsTextAndQueries() async throws {
    let context = makeContext(recent: ["old"], debounce: .seconds(10))
    context.model.select(recent: "old")
    #expect(context.model.text == "old")
    try await waitOnMain { Self.watches(context) == 1 }
  }

  @Test func removingRecentPersists() {
    let context = makeContext(recent: ["a", "b"])
    context.model.remove(recent: "a")
    #expect(context.model.recentSearches == ["b"])
    #expect(context.preferences.recentSearches == ["b"])
  }

  @Test func openingHitRecordsActiveKeyword() async throws {
    let context = makeContext(recent: ["z"])
    context.model.setText("q")
    try await waitOnMain { Self.watches(context) == 1 }
    await push(context, [documentHit(id: "1")])
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.didOpen(Self.hits(context)[0])
    #expect(context.model.recentSearches == ["q", "z"])
    #expect(context.preferences.recentSearches == ["q", "z"])
  }

  @Test func noSiteMeansNoQuery() async throws {
    let context = makeContext(siteId: nil)
    context.model.setText("q")
    context.model.submit()
    try await Task.sleep(for: .milliseconds(60))
    #expect(Self.watches(context) == 0)
    #expect(context.model.content == .pending)
  }

  @Test func convertsHitFields() async throws {
    let context = makeContext()
    context.model.setText("q")
    try await waitOnMain { Self.watches(context) == 1 }
    await push(
      context,
      [
        documentHit(
          id: "1", title: "<em>hi</em> there", text: "a &amp; b", ancestors: ["2026", "parent"]),
        documentHit(id: "2", docTitle: " ", excerpt: ""),
        folderHit(id: "3", folders: 0, documents: 2),
        documentHit(id: "4", subtitle: "  "),
      ])
    try await waitOnMain { Self.hits(context).count == 4 }
    let hits = Self.hits(context)
    guard case .document(let first) = hits[0], case .document(let second) = hits[1],
      case .folder(let third) = hits[2]
    else {
      Issue.record("unexpected hit kinds")
      return
    }
    #expect(first.title.segments.map(\.isHighlighted) == [true, false])
    #expect(first.preview.plain == "a & b")
    #expect(first.previewLines == 2)
    #expect(first.path == ["2026", "parent"])
    let updatedAt = try #require(first.updatedAt)
    #expect(abs(updatedAt.timeIntervalSince1970 - 1_798_859_045.678) < 0.001)
    #expect(second.title.plain == "(제목 없음)")
    #expect(second.preview.plain == "(내용 없음)")
    #expect(second.previewLines == 1)
    #expect(second.path == [])
    #expect(third.title.plain == "(이름 없음)")
    #expect(third.summary == "문서 2개")
    #expect(third.path == [])
    #expect(third.icon.kind == .folder)
    guard case .document(let fourth) = hits[3] else {
      Issue.record("expected document")
      return
    }
    #expect(fourth.subtitle == nil)
  }
}
