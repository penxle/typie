import Apollo
import ApolloWebSocket
import Foundation
import GraphQL
import Testing

@testable import Core

private func makeTransport(_ factory: FakeSocketFactory) -> (WebSocketTransport, ApolloClient) {
  let store = ApolloStore()
  let session = ObservedWebSocketSession(makeTask: { factory.make($0) }, onResume: { _ in })
  let transport = WebSocketTransport(
    urlSession: session, store: store,
    endpointURL: URL(string: "wss://api.example.test/graphql")!,
    configuration: WebSocketTransport.Configuration(reconnectionInterval: -1, pingInterval: nil))
  return (transport, ApolloClient(networkTransport: transport, store: store))
}

private func consume(
  _ stream: SubscriptionStream<GraphQLResponse<Ping_Subscription>>, into recorder: CallRecorder
) -> Task<[GraphQLResponse<Ping_Subscription>], any Error> {
  Task {
    var responses: [GraphQLResponse<Ping_Subscription>] = []
    for try await response in stream { responses.append(response) }
    recorder.record("finished")
    return responses
  }
}

@Suite(.timeLimit(.minutes(1))) struct ApolloWebSocketContractTests {
  @Test func unexpectedCloseFinishesTheStreamAfterTheDisconnectCallback() async throws {
    let factory = FakeSocketFactory(prepare: { $0.holdAcknowledgement() })
    let (transport, apollo) = makeTransport(factory)
    let recorder = CallRecorder()
    let stream = try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly)
    let events = TransportEvents(recorder, probe: { "\(stream.state)" })
    await transport.setDelegate(events)

    let consumer = consume(stream, into: recorder)
    try await waitUntil { factory.sockets.first?.sent.count == 1 }
    factory.sockets[0].deliver(#"{"type":"connection_ack"}"#)
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    factory.sockets[0].serverClose()
    _ = try await consumer.value

    #expect(recorder.calls == ["connect", "disconnect:active", "finished"])
    withExtendedLifetime(events) {}
  }

  @Test func eachAttemptResumesOneTaskAndSendsTheLatestPayload() async throws {
    let factory = FakeSocketFactory()
    let (transport, apollo) = makeTransport(factory)

    await transport.updateConnectingPayload(["session": "t1"])
    let first = consume(
      try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly),
      into: CallRecorder())
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    factory.sockets[0].serverClose()
    _ = try await first.value

    await transport.updateConnectingPayload(["session": "t2"])
    let second = consume(
      try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly),
      into: CallRecorder())
    try await waitUntil { factory.sockets.count == 2 && factory.sockets[1].subscribeIDs.count == 1 }

    #expect(factory.sockets.map(\.resumeCount) == [1, 1])
    #expect(factory.sockets.map(\.sessionTicket) == ["t1", "t2"])
    second.cancel()
  }

  @Test func subscribeTimeErrorsArriveAsAResponseThenTheStreamCompletes() async throws {
    let factory = FakeSocketFactory()
    let (_, apollo) = makeTransport(factory)
    let consumer = consume(
      try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly),
      into: CallRecorder())
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    let socket = factory.sockets[0]
    let id = socket.subscribeIDs[0]

    socket.nextErrors(id: id, message: "permission_denied")
    socket.complete(id: id)
    let responses = try await consumer.value

    #expect(responses.count == 1)
    #expect(responses.first?.data == nil)
    #expect(responses.first?.errors?.first?.message == "permission_denied")
  }

  @Test func networkOnlyDoesNotReplayTheCachedEvent() async throws {
    let factory = FakeSocketFactory()
    let (_, apollo) = makeTransport(factory)
    let first = consume(
      try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly),
      into: CallRecorder())
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }
    let socket = factory.sockets[0]
    socket.next(id: socket.subscribeIDs[0], data: pingData("u1"))
    socket.complete(id: socket.subscribeIDs[0])
    _ = try await first.value

    let second = consume(
      try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly),
      into: CallRecorder())
    try await waitUntil { socket.subscribeIDs.count == 2 }
    socket.next(id: socket.subscribeIDs[1], data: pingData("u2"))
    socket.complete(id: socket.subscribeIDs[1])
    let responses = try await second.value

    #expect(responses.map { $0.data?.userGoalUpdateStream.id } == ["u2"])
  }

  @Test func cancellingTheTaskIsTreatedAsADisconnect() async throws {
    let factory = FakeSocketFactory()
    let (transport, apollo) = makeTransport(factory)
    let recorder = CallRecorder()
    let events = TransportEvents(recorder)
    await transport.setDelegate(events)
    let consumer = consume(
      try apollo.subscribe(subscription: Ping_Subscription(), cachePolicy: .networkOnly),
      into: recorder)
    try await waitUntil { factory.sockets.first?.subscribeIDs.count == 1 }

    factory.sockets[0].cancel(with: .goingAway, reason: nil)
    _ = try? await consumer.value

    #expect(recorder.calls.contains("disconnect"))
    #expect(recorder.calls.last == "finished")
    withExtendedLifetime(events) {}
  }
}
