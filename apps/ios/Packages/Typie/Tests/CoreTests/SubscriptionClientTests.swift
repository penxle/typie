import Apollo
import Foundation
import GraphQL
import InMemoryLogging
import Logging
import Testing

@testable import Core

final class ReconnectStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var served = 0

  static func reset() { lock.withLock { served = 0 } }
  static var count: Int { lock.withLock { served } }

  override func startLoading() {
    Self.lock.withLock { Self.served += 1 }
    respond(
      headers: ["Content-Type": "application/json"],
      body: Data(#"{"data":{"__typename":"Query","me":{"__typename":"User","id":"u1"}}}"#.utf8))
  }
}

final class TicketStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var recorded: [(body: String, authorization: String?)] = []

  static func reset() { lock.withLock { recorded = [] } }
  static var requests: [(body: String, authorization: String?)] { lock.withLock { recorded } }

  override func startLoading() {
    let body = Self.body(of: request).flatMap { String(data: $0, encoding: .utf8) } ?? ""
    Self.lock.withLock {
      Self.recorded.append((body, request.value(forHTTPHeaderField: "Authorization")))
    }
    respond(
      headers: ["Content-Type": "application/json"],
      body: Data(#"{"data":{"createWsSession":"ticket-http"}}"#.utf8))
  }
}

private func makeClient(
  factory: FakeSocketFactory, tickets: TicketSource? = TicketSource(),
  configuration: URLSessionConfiguration = HTTPSession.configuration(),
  backoff: Backoff = Backoff(base: .milliseconds(5), cap: .milliseconds(20), jitter: .zero),
  logger: Logger = Logger(label: "co.typie.subscription")
) -> ApolloGraphQLClient {
  ApolloGraphQLClient.make(
    config: makeTestConfig(), deviceHeaders: { [:] }, accessToken: { "access-1" },
    onSessionCookie: { _ in }, configuration: configuration,
    makeSocket: { factory.make($0) },
    fetchTicket: tickets.map { source in { @Sendable in try await source.next() } },
    subscriptionTiming: SubscriptionConnection.Timing(
      pingInterval: .seconds(60), responseDeadline: .seconds(60)),
    backoff: backoff, logger: logger)
}

@Suite(.serialized, .timeLimit(.minutes(1))) struct SubscriptionClientTests {
  @Test func backoffDoublesUpToTheCapPlusJitter() {
    let backoff = Backoff()
    #expect(backoff.delay(attempt: 0, random: 0) == .seconds(1))
    #expect(backoff.delay(attempt: 3, random: 0) == .seconds(8))
    #expect(backoff.delay(attempt: 4, random: 0) == .seconds(10))
    #expect(backoff.delay(attempt: 60, random: 0) == .seconds(10))
    #expect(backoff.delay(attempt: 0, random: 0.5) == .milliseconds(1500))
  }

  @Test func socketsAcceptMessagesOfAnySize() throws {
    let make = ApolloGraphQLClient.webSocketTasks(configuration: .ephemeral)
    let task = try #require(
      make(URLRequest(url: URL(string: "wss://api.example.test/graphql")!))
        as? URLSessionWebSocketTask)
    #expect(task.maximumMessageSize == Int(Int32.max))
    task.cancel()
  }

  @Test func resubscribesWithANewTicketAfterADisconnect() async throws {
    let factory = FakeSocketFactory()
    let client = makeClient(factory: factory)
    let received = CallRecorder()
    let consumer = Task {
      for await data in client.subscribe(Ping_Subscription()) {
        received.record(data.userGoalUpdateStream.id)
      }
    }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    factory.sockets[0].next(id: factory.sockets[0].subscribeIDs[0], data: pingData("u1"))
    try await waitUntil { received.calls == ["u1"] }

    factory.sockets[0].serverClose()
    try await waitUntil { factory.sockets.count == 2 && factory.sockets[1].subscribeIDs.count == 1 }
    factory.sockets[1].next(id: factory.sockets[1].subscribeIDs[0], data: pingData("u2"))
    try await waitUntil { received.calls == ["u1", "u2"] }

    #expect(factory.sockets.map(\.sessionTicket) == ["ticket-1", "ticket-2"])
    consumer.cancel()
  }

  @Test func resubscribesAfterANetworkError() async throws {
    let factory = FakeSocketFactory()
    let client = makeClient(factory: factory)
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }

    factory.sockets[0].serverDrop(URLError(.networkConnectionLost))

    try await waitUntil { factory.sockets.count == 2 && factory.sockets[1].subscribeIDs.count == 1 }
    consumer.cancel()
  }

  @Test func stopsWhenTheServerCompletesWhileConnected() async throws {
    let factory = FakeSocketFactory()
    let client = makeClient(factory: factory)
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    let socket = factory.sockets[0]

    socket.nextErrors(id: socket.subscribeIDs[0], message: "permission_denied")
    socket.complete(id: socket.subscribeIDs[0])
    await consumer.value

    try await Task.sleep(for: .milliseconds(50))
    #expect(factory.sockets.count == 1)
    #expect(socket.subscribeIDs.count == 1)
  }

  @Test func stopsOnAGraphQLErrorMessage() async throws {
    let factory = FakeSocketFactory()
    let client = makeClient(factory: factory)
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    let socket = factory.sockets[0]

    socket.error(id: socket.subscribeIDs[0], message: "invalid")
    await consumer.value

    try await Task.sleep(for: .milliseconds(50))
    #expect(socket.subscribeIDs.count == 1)
  }

  @Test func terminationLogsCarryTheOperationAndItsVariables() async throws {
    let logs = InMemoryLogHandler()
    let factory = FakeSocketFactory()
    let client = makeClient(
      factory: factory, logger: Logger(label: "co.typie.subscription", factory: { _ in logs }))

    let completed = Task {
      for await _ in client.subscribe(Ping_Usage_Subscription(userId: "user-7")) {}
    }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    factory.sockets[0].complete(id: factory.sockets[0].subscribeIDs[0])
    await completed.value

    let failed = Task {
      for await _ in client.subscribe(Ping_Usage_Subscription(userId: "user-8")) {}
    }
    try await waitUntil { factory.sockets[0].subscribeIDs.count == 2 }
    factory.sockets[0].error(id: factory.sockets[0].subscribeIDs[1], message: "invalid")
    await failed.value

    let endings = logs.entries.filter { "\($0.message)".hasPrefix("subscription ") }
    #expect(
      endings.map { "\($0.message)" } == [
        "subscription completed by the server", "subscription stopped by a GraphQL error",
      ])
    #expect(endings.allSatisfy { $0.metadata["operation"] == "Ping_Usage_Subscription" })
    #expect("\(endings[0].metadata["variables"] ?? "")".contains("user-7"))
    #expect("\(endings[1].metadata["variables"] ?? "")".contains("user-8"))
  }

  @Test func anUnrecognizedMessageEndsTheSubscriptionWithAnError() async throws {
    let logs = InMemoryLogHandler()
    let factory = FakeSocketFactory()
    let client = makeClient(
      factory: factory, logger: Logger(label: "co.typie.subscription", factory: { _ in logs }))
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }

    factory.sockets[0].deliver(#"{"type":"bogus"}"#)
    await consumer.value

    try await Task.sleep(for: .milliseconds(50))
    #expect(factory.sockets.count == 1)
    #expect(factory.sockets[0].subscribeIDs.count == 1)
    let ending = try #require(logs.entries.first { "\($0.message)".hasPrefix("subscription ") })
    #expect("\(ending.message)" == "subscription ended with an error")
    #expect(ending.level == .warning)
    #expect(ending.metadata["error"] != nil)
  }

  @Test func retriesAfterTicketFailures() async throws {
    let factory = FakeSocketFactory()
    let tickets = TicketSource(failures: 2)
    let client = makeClient(factory: factory, tickets: tickets)
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }

    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    #expect(tickets.count == 3)
    #expect(factory.sockets[0].sessionTicket == "ticket-3")
    consumer.cancel()
  }

  @Test func backoffResetsAfterAnAcknowledgedRound() async throws {
    let factory = FakeSocketFactory()
    let client = makeClient(
      factory: factory, tickets: TicketSource(failures: 6),
      backoff: Backoff(base: .milliseconds(5), cap: .milliseconds(200), jitter: .zero))
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }

    let dropped = ContinuousClock.now
    factory.sockets[0].serverClose()
    try await waitUntil { factory.sockets.count == 2 && factory.sockets[1].subscribeIDs.count == 1 }

    #expect(ContinuousClock.now - dropped < .milliseconds(100))
    consumer.cancel()
  }

  @Test func backoffResetsForALoopThatJoinsAnAcknowledgedConnection() async throws {
    let factory = FakeSocketFactory()
    let tickets = TicketSource(failures: 6)
    let client = makeClient(
      factory: factory, tickets: tickets,
      backoff: Backoff(base: .milliseconds(5), cap: .milliseconds(200), jitter: .zero))
    let late = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { tickets.count == 6 }
    let early = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 2 }

    let dropped = ContinuousClock.now
    factory.sockets[0].serverClose()
    try await waitUntil { factory.sockets.count == 2 && factory.sockets[1].subscribeIDs.count == 2 }

    #expect(ContinuousClock.now - dropped < .milliseconds(100))
    late.cancel()
    early.cancel()
  }

  @Test func cancellingDuringBackoffStopsTheLoop() async throws {
    let factory = FakeSocketFactory()
    let tickets = TicketSource(failures: 1_000)
    let client = makeClient(factory: factory, tickets: tickets)
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { tickets.count >= 2 }

    consumer.cancel()
    try await Task.sleep(for: .milliseconds(30))
    let settled = tickets.count
    try await Task.sleep(for: .milliseconds(100))

    #expect(tickets.count <= settled + 1)
  }

  @Test func cancellingTheConsumerSendsComplete() async throws {
    let factory = FakeSocketFactory()
    let client = makeClient(factory: factory)
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    let id = factory.sockets[0].subscribeIDs[0]

    consumer.cancel()

    try await waitUntil { factory.sockets[0].completedIDs == [id] }
  }

  @Test func aReconnectRefetchesLiveWatches() async throws {
    ReconnectStub.reset()
    let factory = FakeSocketFactory()
    let client = makeClient(
      factory: factory, configuration: stubbedConfiguration(ReconnectStub.self))
    let watcher = client.watch(Ping_Query()) { _ in }
    try await waitUntil { ReconnectStub.count == 1 }
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    #expect(ReconnectStub.count == 1)

    factory.sockets[0].serverClose()

    try await waitUntil { factory.sockets.count == 2 && factory.sockets[1].subscribeIDs.count == 1 }
    try await waitUntil { ReconnectStub.count == 2 }
    watcher.cancel()
    consumer.cancel()
  }

  @Test func ticketsComeFromCreateWsSessionOverHTTP() async throws {
    TicketStub.reset()
    let factory = FakeSocketFactory()
    let client = makeClient(
      factory: factory, tickets: nil, configuration: stubbedConfiguration(TicketStub.self))
    let consumer = Task { for await _ in client.subscribe(Ping_Subscription()) {} }

    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    #expect(factory.sockets[0].sessionTicket == "ticket-http")
    let request = try #require(TicketStub.requests.first)
    #expect(request.body.contains("SubscriptionConnection_CreateWsSession_Mutation"))
    #expect(request.authorization == "Bearer access-1")
    consumer.cancel()
  }
}
