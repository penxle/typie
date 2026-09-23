import Apollo
import ApolloWebSocket
import Foundation
import GraphQL
import InMemoryLogging
import Logging
import Testing

@testable import Core

private struct Harness {
  let factory: FakeSocketFactory
  let tickets: TicketSource
  let reconnects: Counter
  let logs: InMemoryLogHandler
  let connection: SubscriptionConnection
  let apollo: ApolloClient
}

private func makeHarness(
  factory: FakeSocketFactory = FakeSocketFactory(),
  tickets: TicketSource = TicketSource(),
  pingInterval: Duration = .seconds(60),
  responseDeadline: Duration = .seconds(60)
) -> Harness {
  let store = ApolloStore()
  let reconnects = Counter()
  let logs = InMemoryLogHandler()
  var logger = Logger(label: "co.typie.subscription", factory: { _ in logs })
  logger.logLevel = .trace
  let connection = SubscriptionConnection(
    endpointURL: URL(string: "wss://api.example.test/graphql")!, store: store,
    makeTask: { factory.make($0) }, fetchTicket: { try await tickets.next() },
    onReconnect: { reconnects.increment() },
    timing: SubscriptionConnection.Timing(
      pingInterval: pingInterval, responseDeadline: responseDeadline),
    logger: logger)
  return Harness(
    factory: factory, tickets: tickets, reconnects: reconnects, logs: logs,
    connection: connection,
    apollo: ApolloClient(networkTransport: connection.transport, store: store))
}

private func open(_ harness: Harness) async throws -> Task<Void, any Error> {
  _ = try await harness.connection.prepare()
  let stream = try harness.apollo.subscribe(
    subscription: Ping_Subscription(), cachePolicy: .networkOnly)
  return Task { for try await _ in stream {} }
}

@Suite(.timeLimit(.minutes(1))) struct SubscriptionConnectionTests {
  @Test func installsAFreshTicketForEachConnection() async throws {
    let harness = makeHarness()
    let first = try await open(harness)
    try await waitUntil { harness.factory.sockets.first?.subscribeIDs.count == 1 }
    harness.factory.sockets[0].serverClose()
    _ = try? await first.value

    let second = try await open(harness)
    try await waitUntil {
      harness.factory.sockets.count == 2 && harness.factory.sockets[1].subscribeIDs.count == 1
    }

    #expect(harness.factory.sockets.map(\.sessionTicket) == ["ticket-1", "ticket-2"])
    #expect(harness.tickets.count == 2)
    second.cancel()
  }

  @Test func concurrentPreparesShareOneTicketFetch() async throws {
    let gate = TestGate()
    let harness = makeHarness(tickets: TicketSource(gate: gate))
    async let a = harness.connection.prepare()
    async let b = harness.connection.prepare()
    async let c = harness.connection.prepare()
    try await waitUntil { harness.tickets.requests == 1 }
    await gate.open()
    _ = try await (a, b, c)

    #expect(harness.tickets.count == 1)
  }

  @Test func aFailedTicketThrowsAndTheNextPrepareFetchesAgain() async throws {
    let harness = makeHarness(tickets: TicketSource(failures: 1))

    await #expect(throws: HTTPError.network("offline")) {
      _ = try await harness.connection.prepare()
    }
    _ = try await harness.connection.prepare()

    #expect(harness.tickets.count == 2)
  }

  @Test func aDisconnectAdvancesTheEpoch() async throws {
    let harness = makeHarness()
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }
    let before = harness.connection.epoch

    harness.factory.sockets[0].serverClose()

    try await waitUntil { harness.connection.epoch == before + 1 }
    _ = try? await task.value
  }

  @Test func theSecondAcknowledgementSignalsAReconnect() async throws {
    let harness = makeHarness()
    let first = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }
    #expect(harness.reconnects.value == 0)
    harness.factory.sockets[0].serverClose()
    _ = try? await first.value

    let second = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 2 }

    #expect(harness.reconnects.value == 1)
    second.cancel()
  }

  @Test func resetCancelsTheSocketAndRestartsCounting() async throws {
    let harness = makeHarness()
    let first = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }

    harness.connection.reset()
    try await waitUntil { harness.factory.sockets[0].cancelCount == 1 }
    _ = try? await first.value

    let second = try await open(harness)
    try await waitUntil {
      harness.factory.sockets.count == 2 && harness.connection.acknowledgements == 1
    }
    #expect(harness.reconnects.value == 0)
    second.cancel()
  }

  @Test func aTicketFetchedAcrossAResetIsDiscarded() async throws {
    let gate = TestGate()
    let harness = makeHarness(tickets: TicketSource(gate: gate))
    let pending = Task { try await harness.connection.prepare() }
    try await waitUntil { harness.tickets.requests == 1 }

    harness.connection.reset()
    await gate.open()
    _ = try await pending.value

    let task = try await open(harness)
    try await waitUntil { harness.factory.sockets.first?.subscribeIDs.count == 1 }
    #expect(harness.factory.sockets[0].sessionTicket == "ticket-2")
    task.cancel()
  }

  @Test func anAttemptWithoutAFreshTicketIsCancelled() async throws {
    let harness = makeHarness(factory: FakeSocketFactory { $0.holdAcknowledgement() })
    _ = try await harness.connection.prepare()
    harness.connection.reset()

    let stream = try harness.apollo.subscribe(
      subscription: Ping_Subscription(), cachePolicy: .networkOnly)
    let task = Task { for try await _ in stream {} }

    try await waitUntil { (harness.factory.sockets.first?.cancelCount ?? 0) >= 1 }
    await #expect(throws: (any Error).self) { try await task.value }
    #expect(harness.connection.acknowledgements == 0)
    let messages = harness.logs.entries.map { "\($0.message)" }
    #expect(messages.contains("attempt without a fresh ticket"))
    #expect(!messages.contains("connection attempt"))
  }

  @Test func logsNeverContainTheTicket() async throws {
    let harness = makeHarness()
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }
    harness.factory.sockets[0].serverClose()
    _ = try? await task.value

    let entries = harness.logs.entries
    #expect(!entries.isEmpty)
    #expect(entries.allSatisfy { !"\($0.message) \($0.metadata)".contains("ticket-") })
  }

  @Test func aMissingAcknowledgementCancelsTheAttempt() async throws {
    let factory = FakeSocketFactory { $0.holdAcknowledgement() }
    let harness = makeHarness(factory: factory, responseDeadline: .milliseconds(50))
    harness.connection.appDidBecomeForeground()
    let task = try await open(harness)

    try await waitUntil { factory.sockets.first?.cancelCount == 1 }
    await #expect(throws: (any Error).self) { try await task.value }
  }

  @Test func aMissingPongCancelsTheConnection() async throws {
    let factory = FakeSocketFactory { $0.holdPongs() }
    let harness = makeHarness(
      factory: factory, pingInterval: .milliseconds(100), responseDeadline: .milliseconds(50))
    harness.connection.appDidBecomeForeground()
    let task = try await open(harness)

    try await waitUntil { factory.sockets.first?.cancelCount == 1 }
    #expect(factory.sockets[0].pingCount >= 1)
    _ = try? await task.value
  }

  @Test func timelyPongsKeepTheConnection() async throws {
    let harness = makeHarness(pingInterval: .milliseconds(20), responseDeadline: .milliseconds(50))
    harness.connection.appDidBecomeForeground()
    let task = try await open(harness)

    try await waitUntil { (harness.factory.sockets.first?.pingCount ?? 0) >= 3 }
    #expect(harness.factory.sockets[0].cancelCount == 0)
    task.cancel()
  }

  @Test func aProbePingsOnlyWhileConnected() async throws {
    let harness = makeHarness()
    harness.connection.appDidBecomeForeground()
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }
    #expect(harness.factory.sockets[0].pingCount == 0)

    harness.connection.appDidEnterBackground()
    harness.connection.appDidBecomeForeground()

    try await waitUntil { harness.factory.sockets[0].pingCount == 1 }
    task.cancel()
  }

  @Test func onlyAnAnsweredProbeIsLogged() async throws {
    let harness = makeHarness(pingInterval: .milliseconds(20))
    harness.connection.appDidBecomeForeground()
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }
    try await waitUntil { harness.factory.sockets[0].pingCount >= 2 }
    #expect(!harness.logs.entries.contains { "\($0.message)" == "probe answered" })

    harness.connection.appDidEnterBackground()
    harness.connection.appDidBecomeForeground()

    try await waitUntil { harness.logs.entries.contains { "\($0.message)" == "probe answered" } }
    task.cancel()
  }

  @Test func anUnansweredProbeIsNotLoggedAsAnswered() async throws {
    let factory = FakeSocketFactory { $0.holdPongs() }
    let harness = makeHarness(factory: factory)
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }

    harness.connection.appDidBecomeForeground()
    try await waitUntil { factory.sockets[0].pingCount == 1 }
    try await Task.sleep(for: .milliseconds(50))

    #expect(harness.logs.entries.contains { "\($0.message)" == "probe" })
    #expect(!harness.logs.entries.contains { "\($0.message)" == "probe answered" })
    task.cancel()
  }

  @Test func acknowledgementBeforeTheFirstForegroundStillStartsPinging() async throws {
    let harness = makeHarness(pingInterval: .milliseconds(20))
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }
    try await Task.sleep(for: .milliseconds(60))
    #expect(harness.factory.sockets[0].pingCount == 0)

    harness.connection.appDidBecomeForeground()

    try await waitUntil { harness.factory.sockets[0].pingCount >= 3 }
    task.cancel()
  }

  @Test func backgroundDisarmsAPendingPongDeadline() async throws {
    let factory = FakeSocketFactory { $0.holdPongs() }
    let harness = makeHarness(factory: factory, responseDeadline: .milliseconds(200))
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }

    harness.connection.appDidBecomeForeground()
    try await waitUntil { factory.sockets[0].pingCount == 1 }
    harness.connection.appDidEnterBackground()
    try await Task.sleep(for: .milliseconds(300))

    #expect(factory.sockets[0].cancelCount == 0)
    task.cancel()
  }

  @Test func returningDuringAnAttemptRearmsTheAcknowledgementDeadline() async throws {
    let factory = FakeSocketFactory { $0.holdAcknowledgement() }
    let harness = makeHarness(factory: factory, responseDeadline: .milliseconds(200))
    harness.connection.appDidBecomeForeground()
    let task = try await open(harness)
    try await waitUntil { factory.sockets.first?.resumeCount == 1 }

    harness.connection.appDidEnterBackground()
    try await Task.sleep(for: .milliseconds(300))
    #expect(factory.sockets[0].cancelCount == 0)

    harness.connection.appDidBecomeForeground()

    try await waitUntil { factory.sockets[0].cancelCount >= 1 }
    _ = try? await task.value
  }

  @Test func anAttemptStartedInTheBackgroundWaitsForTheForegroundToArmItsDeadline() async throws {
    let factory = FakeSocketFactory { $0.holdAcknowledgement() }
    let harness = makeHarness(factory: factory, responseDeadline: .milliseconds(50))
    harness.connection.appDidBecomeForeground()
    harness.connection.appDidEnterBackground()
    let task = try await open(harness)
    try await waitUntil { factory.sockets.first?.resumeCount == 1 }

    try await Task.sleep(for: .milliseconds(150))
    #expect(factory.sockets[0].cancelCount == 0)

    let returned = ContinuousClock.now
    harness.connection.appDidBecomeForeground()

    try await waitUntil { factory.sockets[0].cancelCount >= 1 }
    #expect(ContinuousClock.now - returned >= .milliseconds(50))
    _ = try? await task.value
  }

  @MainActor
  @Test func followsTheAppLifecycle() async throws {
    let harness = makeHarness()
    let lifecycle = AppLifecycle()
    harness.connection.follow(lifecycle)
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }

    lifecycle.update(foreground: true)
    try await waitUntil { harness.factory.sockets[0].pingCount == 1 }
    lifecycle.update(foreground: false)
    lifecycle.update(foreground: true)
    try await waitUntil { harness.factory.sockets[0].pingCount == 2 }
    task.cancel()
  }

  @MainActor
  @Test func followingTheLifecycleIntoTheBackgroundDisarmsAPendingPong() async throws {
    let factory = FakeSocketFactory { $0.holdPongs() }
    let harness = makeHarness(factory: factory, responseDeadline: .milliseconds(200))
    let lifecycle = AppLifecycle()
    harness.connection.follow(lifecycle)
    let task = try await open(harness)
    try await waitUntil { harness.connection.acknowledgements == 1 }

    lifecycle.update(foreground: true)
    try await waitUntil { factory.sockets[0].pingCount == 1 }
    lifecycle.update(foreground: false)
    try await Task.sleep(for: .milliseconds(300))

    #expect(factory.sockets[0].cancelCount == 0)
    task.cancel()
  }
}
