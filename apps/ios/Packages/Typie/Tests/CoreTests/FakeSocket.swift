import ApolloWebSocket
import Foundation

@testable import Core

final class FakeSocket: PingableWebSocketTask, @unchecked Sendable {
  typealias Message = URLSessionWebSocketTask.Message

  let request: URLRequest

  private let lock = NSLock()
  private var inbox: [Message] = []
  private var waiter: CheckedContinuation<Message, any Error>?
  private var closure: (any Error)?
  private var outgoing: [String] = []
  private var resumes = 0
  private var cancels = 0
  private var pings = 0
  private var heldPongs: [@Sendable ((any Error)?) -> Void] = []
  private var acknowledges = true
  private var answersPings = true

  init(request: URLRequest) {
    self.request = request
  }

  func holdAcknowledgement() { lock.withLock { acknowledges = false } }
  func holdPongs() { lock.withLock { answersPings = false } }

  var sent: [String] { lock.withLock { outgoing } }
  var resumeCount: Int { lock.withLock { resumes } }
  var cancelCount: Int { lock.withLock { cancels } }
  var pingCount: Int { lock.withLock { pings } }

  var subscribeIDs: [String] { ids(ofType: "subscribe") }
  var completedIDs: [String] { ids(ofType: "complete") }

  var sessionTicket: String? {
    sent.compactMap(Self.json).first { $0["type"] as? String == "connection_init" }
      .flatMap { ($0["payload"] as? [String: Any])?["session"] as? String }
  }

  func resume() {
    lock.withLock { resumes += 1 }
  }

  func send(_ message: Message) async throws {
    guard case .string(let text) = message else { return }
    let acknowledge = lock.withLock { () -> Bool in
      outgoing.append(text)
      return acknowledges && Self.json(text)?["type"] as? String == "connection_init"
    }
    if acknowledge { deliver(#"{"type":"connection_ack"}"#) }
  }

  func receive() async throws -> Message {
    try await withCheckedThrowingContinuation { continuation in
      let ready = lock.withLock { () -> Result<Message, any Error>? in
        if !inbox.isEmpty { return .success(inbox.removeFirst()) }
        if let closure { return .failure(closure) }
        waiter = continuation
        return nil
      }
      if let ready { continuation.resume(with: ready) }
    }
  }

  func cancel(with closeCode: URLSessionWebSocketTask.CloseCode, reason: Data?) {
    lock.withLock { cancels += 1 }
    close(URLError(.cancelled))
  }

  func sendPing(pongReceiveHandler: @escaping @Sendable ((any Error)?) -> Void) {
    let answer = lock.withLock { () -> Bool in
      pings += 1
      if !answersPings { heldPongs.append(pongReceiveHandler) }
      return answersPings
    }
    if answer { pongReceiveHandler(nil) }
  }

  func deliver(_ text: String) {
    let waiting = lock.withLock { () -> CheckedContinuation<Message, any Error>? in
      if let waiter {
        self.waiter = nil
        return waiter
      }
      inbox.append(.string(text))
      return nil
    }
    waiting?.resume(returning: .string(text))
  }

  func next(id: String, data: String) {
    deliver(#"{"type":"next","id":"\#(id)","payload":{"data":\#(data)}}"#)
  }

  func nextErrors(id: String, message: String) {
    deliver(#"{"type":"next","id":"\#(id)","payload":{"errors":[{"message":"\#(message)"}]}}"#)
  }

  func complete(id: String) {
    deliver(#"{"type":"complete","id":"\#(id)"}"#)
  }

  func error(id: String, message: String) {
    deliver(#"{"type":"error","id":"\#(id)","payload":[{"message":"\#(message)"}]}"#)
  }

  func serverClose() {
    close(POSIXError(.ENOTCONN))
  }

  func serverDrop(_ error: any Error) {
    close(error)
  }

  private func close(_ reason: any Error) {
    let waiting = lock.withLock { () -> CheckedContinuation<Message, any Error>? in
      if closure == nil { closure = reason }
      let current = waiter
      waiter = nil
      return current
    }
    waiting?.resume(throwing: reason)
  }

  private func ids(ofType type: String) -> [String] {
    sent.compactMap(Self.json).filter { $0["type"] as? String == type }
      .compactMap { $0["id"] as? String }
  }

  static func json(_ text: String) -> [String: Any]? {
    (try? JSONSerialization.jsonObject(with: Data(text.utf8))) as? [String: Any]
  }
}

final class FakeSocketFactory: @unchecked Sendable {
  private let lock = NSLock()
  private var made: [FakeSocket] = []
  private let prepare: @Sendable (FakeSocket) -> Void

  init(prepare: @escaping @Sendable (FakeSocket) -> Void = { _ in }) {
    self.prepare = prepare
  }

  func make(_ request: URLRequest) -> any PingableWebSocketTask {
    let socket = FakeSocket(request: request)
    prepare(socket)
    lock.withLock { made.append(socket) }
    return socket
  }

  var sockets: [FakeSocket] { lock.withLock { made } }
}

final class TransportEvents: WebSocketTransportDelegate, @unchecked Sendable {
  let recorder: CallRecorder
  let probe: @Sendable () -> String?

  init(_ recorder: CallRecorder, probe: @escaping @Sendable () -> String? = { nil }) {
    self.recorder = recorder
    self.probe = probe
  }

  func webSocketTransportDidConnect(_ webSocketTransport: isolated WebSocketTransport) {
    recorder.record("connect")
  }

  func webSocketTransportDidReconnect(_ webSocketTransport: isolated WebSocketTransport) {
    recorder.record("reconnect")
  }

  func webSocketTransport(
    _ webSocketTransport: isolated WebSocketTransport, didDisconnectWithError error: (any Error)?
  ) {
    recorder.record(probe().map { "disconnect:\($0)" } ?? "disconnect")
  }
}

func pingData(_ id: String) -> String {
  #"{"userGoalUpdateStream":{"__typename":"User","id":"\#(id)"}}"#
}
