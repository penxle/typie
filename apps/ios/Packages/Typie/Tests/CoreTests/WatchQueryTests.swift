import Apollo
import ApolloTestSupport
import Foundation
import GraphQL
import GraphQLMocks
import Observation
import Testing

@testable import Core

final class WatchQueryStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responses: [(status: Int, body: String)] = []
  nonisolated(unsafe) private static var served = 0
  nonisolated(unsafe) private static var gate: TestGate?

  static func reset(_ queue: [(status: Int, body: String)], gated: Bool = false) {
    lock.withLock {
      responses = queue
      served = 0
      gate = gated ? TestGate() : nil
    }
  }

  static func release() async {
    let gate = lock.withLock { self.gate }
    await gate?.open()
  }

  static var requestCount: Int { lock.withLock { served } }

  override func startLoading() {
    let (gate, response) = Self.lock.withLock { () -> (TestGate?, (status: Int, body: String)) in
      let index = min(Self.served, Self.responses.count - 1)
      Self.served += 1
      return (Self.gate, Self.responses[index])
    }
    Task {
      await gate?.wait()
      respond(
        status: response.status, headers: ["Content-Type": "application/json"],
        body: Data(response.body.utf8))
    }
  }
}

@MainActor @Observable final class InputBox {
  var value: String?
  init(_ value: String?) { self.value = value }
}

private func pingBody(_ name: String) -> String {
  #"{"data":{"__typename":"Query","me":{"__typename":"User","id":"\#(name)"}}}"#
}

@Suite(.serialized) @MainActor struct WatchQueryTests {
  private func makeClient() throws -> ApolloGraphQLClient {
    ApolloGraphQLClient.make(
      config: makeTestConfig(),
      deviceHeaders: { [:] }, accessToken: { nil }, onSessionCookie: { _ in },
      store: ApolloStore(),
      configuration: stubbedConfiguration(WatchQueryStub.self))
  }

  private func prefill(_ client: ApolloGraphQLClient, name: String) async throws {
    WatchQueryStub.reset([(200, pingBody(name))])
    _ = try await client.apollo.fetch(query: Ping_Query(), cachePolicy: .networkOnly)
  }

  @Test func cacheMissStaysUnsettledUntilServer() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("fresh"))], gated: true)
    let query = WatchQuery(client: client, query: Ping_Query())
    try await Task.sleep(for: .milliseconds(30))
    #expect(query.isSettled == false)
    #expect(query.data == nil)
    await WatchQueryStub.release()
    try await waitOnMain { query.isSettled }
    #expect(query.data?.me?.id == "fresh")
    #expect(query.error == nil)
  }

  @Test func cacheHitSettlesBeforeServerAndThenRefreshes() async throws {
    let client = try makeClient()
    try await prefill(client, name: "cached")
    WatchQueryStub.reset([(200, pingBody("fresh"))], gated: true)
    let query = WatchQuery(client: client, query: Ping_Query())
    try await waitOnMain { query.isSettled }
    #expect(query.data?.me?.id == "cached")
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.me?.id == "fresh" }
    #expect(query.error == nil)
  }

  @Test func serverFailureKeepsCachedData() async throws {
    let client = try makeClient()
    try await prefill(client, name: "cached")
    WatchQueryStub.reset([(500, "{}")])
    let query = WatchQuery(client: client, query: Ping_Query())
    try await waitOnMain { query.error != nil }
    #expect(query.data?.me?.id == "cached")
    #expect(query.isSettled)
  }

  @Test func serverFailureWithoutCacheReportsError() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(500, "{}")])
    let query = WatchQuery(client: client, query: Ping_Query())
    try await waitOnMain { query.isSettled }
    #expect(query.data == nil)
    #expect(query.error != nil)
  }

  @Test func graphQLErrorsMapToAPIError() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([
      (
        200,
        #"{"data":null,"errors":[{"message":"m","extensions":{"type":"TypieError","code":"forbidden"}}]}"#
      )
    ])
    let query = WatchQuery(client: client, query: Ping_Query())
    try await waitOnMain { query.isSettled }
    #expect((query.error as? APIError)?.code == "forbidden")
  }

  @Test func inputChangeResetsDataByDefault() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("one")), (200, pingBody("two"))], gated: true)
    let box = InputBox("a")
    let query = WatchQuery(client: client, input: { box.value }) { _ in Ping_Query() }
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.me?.id == "one" }
    WatchQueryStub.reset([(200, pingBody("two"))], gated: true)
    try await client.apollo.store.clearCache()
    box.value = "b"
    try await waitOnMain { query.isSettled == false }
    #expect(query.data == nil)
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.me?.id == "two" }
  }

  @Test func inputChangeKeepsDataWhenAsked() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("one"))])
    let box = InputBox("a")
    let query = WatchQuery(
      client: client, input: { box.value }, query: { _ in Ping_Query() },
      keepsDataOnInputChange: true)
    try await waitOnMain { query.data?.me?.id == "one" }
    WatchQueryStub.reset([(200, pingBody("two"))], gated: true)
    try await client.apollo.store.clearCache()
    box.value = "b"
    try await Task.sleep(for: .milliseconds(30))
    #expect(query.data?.me?.id == "one")
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.me?.id == "two" }
  }

  @Test func nilInputStopsAndClears() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("one"))])
    let box = InputBox("a")
    let query = WatchQuery(client: client, input: { box.value }) { _ in Ping_Query() }
    try await waitOnMain { query.data != nil }
    box.value = nil
    try await waitOnMain { query.data == nil }
    #expect(query.isSettled == false)
  }

  @Test func nilInputBeforeWatcherArrivesDropsIt() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("one"))], gated: true)
    let box = InputBox("a")
    let query = WatchQuery(client: client, input: { box.value }) { _ in Ping_Query() }
    box.value = nil
    await WatchQueryStub.release()
    try await Task.sleep(for: .milliseconds(60))
    #expect(query.data == nil)
    #expect(query.isSettled == false)
    query.refetch()
    try await Task.sleep(for: .milliseconds(60))
    #expect(query.data == nil)
    #expect(WatchQueryStub.requestCount == 1)
  }

  @Test func refetchBeforeFirstResultIsIgnored() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("one")), (200, pingBody("two"))], gated: true)
    let query = WatchQuery(client: client, query: Ping_Query())
    try await Task.sleep(for: .milliseconds(30))
    #expect(query.isSettled == false)
    query.refetch()
    try await Task.sleep(for: .milliseconds(30))
    #expect(WatchQueryStub.requestCount == 1)
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.me?.id == "one" }
    query.refetch()
    try await waitOnMain { query.data?.me?.id == "two" }
    #expect(WatchQueryStub.requestCount == 2)
  }

  @Test func refetchHitsNetworkAndUpdates() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, pingBody("one")), (200, pingBody("two"))])
    let query = WatchQuery(client: client, query: Ping_Query())
    try await waitOnMain { query.data?.me?.id == "one" }
    query.refetch()
    try await waitOnMain { query.data?.me?.id == "two" }
    #expect(WatchQueryStub.requestCount == 2)
  }
}
