import ApolloWebSocket
import Foundation

protocol PingableWebSocketTask: WebSocketTask, AnyObject {
  func sendPing(pongReceiveHandler: @escaping @Sendable ((any Error)?) -> Void)
}

extension URLSessionWebSocketTask: PingableWebSocketTask {}

final class ObservedWebSocketSession: WebSocketURLSession {
  private let makeTask: @Sendable (URLRequest) -> any PingableWebSocketTask
  private let onResume: @Sendable (any PingableWebSocketTask) -> Void

  init(
    makeTask: @escaping @Sendable (URLRequest) -> any PingableWebSocketTask,
    onResume: @escaping @Sendable (any PingableWebSocketTask) -> Void
  ) {
    self.makeTask = makeTask
    self.onResume = onResume
  }

  func webSocketTask(with request: URLRequest) -> any WebSocketTask {
    ObservedWebSocketTask(base: makeTask(request), onResume: onResume)
  }
}

private final class ObservedWebSocketTask: WebSocketTask {
  let base: any PingableWebSocketTask
  let onResume: @Sendable (any PingableWebSocketTask) -> Void

  init(
    base: any PingableWebSocketTask,
    onResume: @escaping @Sendable (any PingableWebSocketTask) -> Void
  ) {
    self.base = base
    self.onResume = onResume
  }

  func resume() {
    onResume(base)
    base.resume()
  }

  func send(_ message: URLSessionWebSocketTask.Message) async throws {
    try await base.send(message)
  }

  func receive() async throws -> URLSessionWebSocketTask.Message {
    try await base.receive()
  }

  func cancel(with closeCode: URLSessionWebSocketTask.CloseCode, reason: Data?) {
    base.cancel(with: closeCode, reason: reason)
  }
}
