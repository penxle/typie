import Apollo
import Foundation
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

private func probeBody(_ name: String) -> String {
  #"{"data":{"__typename":"Query","randomName":"\#(name)","me":null}}"#
}

@Suite(.serialized) @MainActor struct WatchQueryTests {
  private func makeClient() throws -> GraphQLClient {
    GraphQLClient.make(
      config: try makeTestConfig(),
      deviceHeaders: { [:] }, accessToken: { nil }, onSessionCookie: { _ in },
      store: ApolloStore(),
      configuration: stubbedConfiguration(WatchQueryStub.self))
  }

  private func prefill(_ client: GraphQLClient, name: String) async throws {
    WatchQueryStub.reset([(200, probeBody(name))])
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
  }

  @Test func cacheMissStaysUnsettledUntilServer() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, probeBody("fresh"))], gated: true)
    let query = WatchQuery(client: client, query: ServerProbe_Query())
    try await Task.sleep(for: .milliseconds(30))
    #expect(query.isSettled == false)
    #expect(query.data == nil)
    await WatchQueryStub.release()
    try await waitOnMain { query.isSettled }
    #expect(query.data?.randomName == "fresh")
    #expect(query.error == nil)
  }

  @Test func cacheHitSettlesBeforeServerAndThenRefreshes() async throws {
    let client = try makeClient()
    try await prefill(client, name: "cached")
    WatchQueryStub.reset([(200, probeBody("fresh"))], gated: true)
    let query = WatchQuery(client: client, query: ServerProbe_Query())
    try await waitOnMain { query.isSettled }
    #expect(query.data?.randomName == "cached")
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.randomName == "fresh" }
    #expect(query.error == nil)
  }

  @Test func serverFailureKeepsCachedData() async throws {
    let client = try makeClient()
    try await prefill(client, name: "cached")
    WatchQueryStub.reset([(500, "{}")])
    let query = WatchQuery(client: client, query: ServerProbe_Query())
    try await waitOnMain { query.error != nil }
    #expect(query.data?.randomName == "cached")
    #expect(query.isSettled)
  }

  @Test func serverFailureWithoutCacheReportsError() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(500, "{}")])
    let query = WatchQuery(client: client, query: ServerProbe_Query())
    try await waitOnMain { query.isSettled }
    #expect(query.data == nil)
    #expect(query.error != nil)
  }

  @Test func graphQLErrorsMapToTypieError() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([
      (
        200,
        #"{"data":null,"errors":[{"message":"m","extensions":{"type":"TypieError","code":"forbidden"}}]}"#
      )
    ])
    let query = WatchQuery(client: client, query: ServerProbe_Query())
    try await waitOnMain { query.isSettled }
    #expect((query.error as? TypieError)?.code == "forbidden")
  }

  @Test func inputChangeResetsDataByDefault() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, probeBody("one")), (200, probeBody("two"))], gated: true)
    let box = InputBox("a")
    let query = WatchQuery(client: client, input: { box.value }) { _ in ServerProbe_Query() }
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.randomName == "one" }
    WatchQueryStub.reset([(200, probeBody("two"))], gated: true)
    try await client.apollo.store.clearCache()
    box.value = "b"
    try await waitOnMain { query.isSettled == false }
    #expect(query.data == nil)
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.randomName == "two" }
  }

  @Test func inputChangeKeepsDataWhenAsked() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, probeBody("one"))])
    let box = InputBox("a")
    let query = WatchQuery(
      client: client, input: { box.value }, query: { _ in ServerProbe_Query() },
      keepsDataOnInputChange: true)
    try await waitOnMain { query.data?.randomName == "one" }
    WatchQueryStub.reset([(200, probeBody("two"))], gated: true)
    try await client.apollo.store.clearCache()
    box.value = "b"
    try await Task.sleep(for: .milliseconds(30))
    #expect(query.data?.randomName == "one")
    await WatchQueryStub.release()
    try await waitOnMain { query.data?.randomName == "two" }
  }

  @Test func nilInputStopsAndClears() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, probeBody("one"))])
    let box = InputBox("a")
    let query = WatchQuery(client: client, input: { box.value }) { _ in ServerProbe_Query() }
    try await waitOnMain { query.data != nil }
    box.value = nil
    try await waitOnMain { query.data == nil }
    #expect(query.isSettled == false)
  }

  @Test func nilInputBeforeWatcherArrivesDropsIt() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, probeBody("one"))], gated: true)
    let box = InputBox("a")
    let query = WatchQuery(client: client, input: { box.value }) { _ in ServerProbe_Query() }
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

  @Test func refetchHitsNetworkAndUpdates() async throws {
    let client = try makeClient()
    WatchQueryStub.reset([(200, probeBody("one")), (200, probeBody("two"))])
    let query = WatchQuery(client: client, query: ServerProbe_Query())
    try await waitOnMain { query.data?.randomName == "one" }
    query.refetch()
    try await waitOnMain { query.data?.randomName == "two" }
    #expect(WatchQueryStub.requestCount == 2)
  }
}
