import Apollo
import Foundation
import Testing

@testable import Core
@testable import Home

final class SearchStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responses: [(status: Int, body: String)] = []
  nonisolated(unsafe) private static var served = 0

  static func reset(_ queue: [(status: Int, body: String)]) {
    lock.withLock {
      responses = queue
      served = 0
    }
  }

  static var requestCount: Int { lock.withLock { served } }

  override func startLoading() {
    let response = Self.lock.withLock { () -> (status: Int, body: String) in
      let index = min(Self.served, Self.responses.count - 1)
      Self.served += 1
      return Self.responses[index]
    }
    respond(
      status: response.status, headers: ["Content-Type": "application/json"],
      body: Data(response.body.utf8))
  }
}

private func entity(_ id: String, node: String, ancestors: String = "[]") -> String {
  #"{"__typename":"Entity","id":"\#(id)","type":"DOCUMENT","icon":"","iconColor":"","node":\#(node),"ancestors":\#(ancestors)}"#
}

private func documentHit(
  id: String, title: String? = nil, text: String? = nil, docTitle: String = "doc",
  subtitle: String? = nil, excerpt: String = "", ancestors: String = "[]"
) -> String {
  let nodeSubtitle = subtitle.map { #""\#($0)""# } ?? "null"
  let node =
    #"{"__typename":"Document","id":"d-\#(id)","title":"\#(docTitle)","subtitle":\#(nodeSubtitle),"excerpt":"\#(excerpt)","updatedAt":"2027-01-02T03:04:05.678Z"}"#
  let highlightedTitle = title.map { #""\#($0)""# } ?? "null"
  let highlightedText = text.map { #""\#($0)""# } ?? "null"
  return
    #"{"__typename":"SearchHitDocument","title":\#(highlightedTitle),"subtitle":null,"text":\#(highlightedText),"document":{"__typename":"Document","id":"d-\#(id)","entity":\#(entity(id, node: node, ancestors: ancestors))}}"#
}

private func folderHit(id: String, folders: Int, documents: Int) -> String {
  let node =
    #"{"__typename":"Folder","id":"f-\#(id)","name":"","folderCount":\#(folders),"documentCount":\#(documents)}"#
  return
    #"{"__typename":"SearchHitFolder","name":null,"folder":{"__typename":"Folder","id":"f-\#(id)","entity":\#(entity(id, node: node))}}"#
}

private func ancestors(_ names: [String]) -> String {
  let entries = names.enumerated().map { index, name in
    #"{"__typename":"Entity","id":"a-\#(index)","node":{"__typename":"Folder","id":"f-a-\#(index)","name":"\#(name)"}}"#
  }
  return "[\(entries.joined(separator: ","))]"
}

private func searchBody(_ hits: [String]) -> String {
  #"{"data":{"search":{"__typename":"SearchResult","hits":[\#(hits.joined(separator: ","))]}}}"#
}

private let errorBody = #"{"errors":[{"message":"boom"}]}"#

@MainActor @Suite(.serialized) struct SearchModelTests {
  private struct Context {
    let model: SearchModel
    let preferences: UserScopedDefaults
    let activeSite: ActiveSiteStore
  }

  private func makeContext(
    siteId: String? = "site-1", recent: [String] = [],
    responses: [(status: Int, body: String)], debounce: Duration = .milliseconds(20)
  ) async throws -> Context {
    SearchStub.reset(responses)
    let preferences = UserScopedDefaults(
      defaults: UserDefaults(suiteName: "search-\(UUID().uuidString)")!)
    preferences.switchUser("user-1")
    preferences.recentSearches = recent
    let activeSite = ActiveSiteStore(preferences: preferences)
    if let siteId { await activeSite.publish(siteId) }
    let client = GraphQLClient.make(
      config: try makeTestConfig(), deviceHeaders: { [:] }, accessToken: { nil },
      onSessionCookie: { _ in }, store: ApolloStore(),
      configuration: stubbedConfiguration(SearchStub.self))
    let model = SearchModel(
      client: client, activeSite: activeSite, preferences: preferences, debounce: debounce)
    return Context(model: model, preferences: preferences, activeSite: activeSite)
  }

  @Test func blankTextShowsRecentAndDoesNotQuery() async throws {
    let context = try await makeContext(recent: ["a"], responses: [(200, searchBody([]))])
    #expect(context.model.content == .recent)
    #expect(context.model.recentSearches == ["a"])
    context.model.setText("   ")
    try await Task.sleep(for: .milliseconds(60))
    #expect(context.model.content == .recent)
    #expect(SearchStub.requestCount == 0)
  }

  @Test func debouncesBeforeQuerying() async throws {
    let context = try await makeContext(
      responses: [(200, searchBody([documentHit(id: "1")]))], debounce: .milliseconds(150))
    context.model.setText("q")
    #expect(context.model.content == .pending)
    try await Task.sleep(for: .milliseconds(50))
    #expect(SearchStub.requestCount == 0)
    try await waitOnMain { SearchStub.requestCount == 1 }
    try await waitOnMain { context.model.content != .pending }
    guard case .results(let hits) = context.model.content else {
      Issue.record("expected results")
      return
    }
    #expect(hits.map(\.entityId) == ["1"])
  }

  @Test func debounceCancelsSupersededKeystroke() async throws {
    let context = try await makeContext(
      responses: [(200, searchBody([documentHit(id: "1")]))], debounce: .milliseconds(60))
    context.model.setText("a")
    try await Task.sleep(for: .milliseconds(30))
    context.model.setText("ab")
    try await waitOnMain { SearchStub.requestCount == 1 }
    try await Task.sleep(for: .milliseconds(100))
    #expect(SearchStub.requestCount == 1)
    #expect(!Self.hits(context).isEmpty)
  }

  @Test func blankTextStopsRunningQuery() async throws {
    let context = try await makeContext(
      responses: [(200, searchBody([documentHit(id: "1")]))], debounce: .seconds(10))
    context.model.setText("q")
    context.model.submit()
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.setText("")
    #expect(context.model.content == .recent)
    try await waitOnMain { context.model.hits == nil }
    context.model.setText("q")
    context.model.submit()
    try await waitOnMain { SearchStub.requestCount == 2 }
  }

  @Test func submitQueriesImmediately() async throws {
    let context = try await makeContext(
      responses: [(200, searchBody([documentHit(id: "1")]))], debounce: .seconds(10))
    context.model.setText("q")
    context.model.submit()
    try await waitOnMain { SearchStub.requestCount == 1 }
  }

  @Test func keepsPreviousResultsWhileTyping() async throws {
    let context = try await makeContext(responses: [
      (200, searchBody([documentHit(id: "1")])), (200, searchBody([documentHit(id: "2")])),
    ])
    context.model.setText("a")
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.setText("ab")
    #expect(Self.hits(context).map(\.entityId) == ["1"])
    try await waitOnMain { Self.hits(context).map(\.entityId) == ["2"] }
  }

  @Test func emptyResultIsEmpty() async throws {
    let context = try await makeContext(responses: [(200, searchBody([]))])
    context.model.setText("q")
    try await waitOnMain { context.model.content == .empty }
  }

  @Test func failureWithoutDataIsFailed() async throws {
    let context = try await makeContext(responses: [(200, errorBody)])
    context.model.setText("q")
    try await waitOnMain { context.model.content == .failed }
  }

  @Test func failureWithDataKeepsResults() async throws {
    let context = try await makeContext(responses: [
      (200, searchBody([documentHit(id: "1")])), (200, errorBody),
    ])
    context.model.setText("a")
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.setText("ab")
    try await waitOnMain { SearchStub.requestCount == 2 }
    try await Task.sleep(for: .milliseconds(60))
    #expect(Self.hits(context).map(\.entityId) == ["1"])
  }

  @Test func selectingRecentSetsTextAndQueries() async throws {
    let context = try await makeContext(
      recent: ["old"], responses: [(200, searchBody([documentHit(id: "1")]))],
      debounce: .seconds(10))
    context.model.select(recent: "old")
    #expect(context.model.text == "old")
    try await waitOnMain { SearchStub.requestCount == 1 }
  }

  @Test func removingRecentPersists() async throws {
    let context = try await makeContext(recent: ["a", "b"], responses: [(200, searchBody([]))])
    context.model.remove(recent: "a")
    #expect(context.model.recentSearches == ["b"])
    #expect(context.preferences.recentSearches == ["b"])
  }

  @Test func openingHitRecordsActiveKeyword() async throws {
    let context = try await makeContext(
      recent: ["z"], responses: [(200, searchBody([documentHit(id: "1")]))])
    context.model.setText("q")
    try await waitOnMain { !Self.hits(context).isEmpty }
    context.model.didOpen(Self.hits(context)[0])
    #expect(context.model.recentSearches == ["q", "z"])
    #expect(context.preferences.recentSearches == ["q", "z"])
  }

  @Test func noSiteMeansNoQuery() async throws {
    let context = try await makeContext(siteId: nil, responses: [(200, searchBody([]))])
    context.model.setText("q")
    context.model.submit()
    try await Task.sleep(for: .milliseconds(60))
    #expect(SearchStub.requestCount == 0)
    #expect(context.model.content == .pending)
  }

  @Test func convertsHitFields() async throws {
    let context = try await makeContext(responses: [
      (
        200,
        searchBody([
          documentHit(
            id: "1", title: "<em>hi</em> there", text: "a &amp; b",
            ancestors: ancestors(["2026", "parent"])),
          documentHit(id: "2", docTitle: " ", excerpt: ""),
          folderHit(id: "3", folders: 0, documents: 2),
          documentHit(id: "4", subtitle: "  "),
        ])
      )
    ])
    context.model.setText("q")
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

  private static func hits(_ context: Context) -> [SearchHit] {
    if case .results(let hits) = context.model.content { return hits }
    return []
  }
}
