import Apollo
import ApolloAPI
import ApolloWebSocket
import Foundation
import Logging
import Synchronization

final class SubscriptionConnection: Sendable {
  struct Timing: Sendable {
    var pingInterval: Duration = .seconds(30)
    var responseDeadline: Duration = .seconds(10)
  }

  private enum Phase {
    case idle
    case connecting
    case connected
  }

  private enum Expectation: String {
    case acknowledgement
    case pong
  }

  private enum PrepareStep {
    case ready(Int)
    case waiting(Task<Void, any Error>)
  }

  private struct State {
    var phase = Phase.idle
    var generation = 0
    var ticketInstalled = false
    var ticketFetch: Task<Void, any Error>?
    var delegateInstall: Task<Void, Never>?
    var epoch = 0
    var acknowledgements = 0
    var foreground = false
    var task: (any PingableWebSocketTask)?
    var deadline: Task<Void, Never>?
    var deadlineToken = 0
    var pinger: Task<Void, Never>?
  }

  let transport: WebSocketTransport

  private let fetchTicket: @Sendable () async throws -> String
  private let onReconnect: @Sendable () -> Void
  private let timing: Timing
  private let logger: Logger
  private let state = Mutex(State())

  init(
    endpointURL: URL,
    store: ApolloStore,
    makeTask: @escaping @Sendable (URLRequest) -> any PingableWebSocketTask,
    fetchTicket: @escaping @Sendable () async throws -> String,
    onReconnect: @escaping @Sendable () -> Void,
    timing: Timing = Timing(),
    logger: Logger = Logger(label: "co.typie.subscription")
  ) {
    let relay = ResumeRelay()
    transport = WebSocketTransport(
      urlSession: ObservedWebSocketSession(makeTask: makeTask, onResume: { relay.resumed($0) }),
      store: store, endpointURL: endpointURL,
      configuration: WebSocketTransport.Configuration(reconnectionInterval: -1, pingInterval: nil))
    self.fetchTicket = fetchTicket
    self.onReconnect = onReconnect
    self.timing = timing
    self.logger = logger
    relay.bind(self)
  }

  var epoch: Int { state.withLock { $0.epoch } }
  var acknowledgements: Int { state.withLock { $0.acknowledgements } }

  func prepare() async throws -> Int {
    await installDelegate()
    while true {
      let step = state.withLock { s -> PrepareStep in
        if s.phase != .idle || s.ticketInstalled { return .ready(s.epoch) }
        if let fetch = s.ticketFetch { return .waiting(fetch) }
        let fetch = Task { try await self.installTicket() }
        s.ticketFetch = fetch
        return .waiting(fetch)
      }
      switch step {
      case .ready(let epoch): return epoch
      case .waiting(let fetch): try await fetch.value
      }
    }
  }

  func appDidBecomeForeground() {
    let (phase, task) = state.withLock { s -> (Phase, (any PingableWebSocketTask)?) in
      s.foreground = true
      return (s.phase, s.task)
    }
    guard let task else { return }
    switch phase {
    case .idle:
      return
    case .connecting:
      arm(.acknowledgement)
    case .connected:
      logger.notice("probe")
      ping(task, probe: true)
      startPinger(task)
    }
  }

  func appDidEnterBackground() {
    let stale = state.withLock { s -> [Task<Void, Never>] in
      s.foreground = false
      let pinger = s.pinger
      s.pinger = nil
      return [Self.disarm(&s), pinger].compactMap { $0 }
    }
    stale.forEach { $0.cancel() }
  }

  @MainActor func follow(_ lifecycle: AppLifecycle) {
    keepObserving(while: self) { [weak self, lifecycle] in
      if lifecycle.state == .foreground {
        self?.appDidBecomeForeground()
      } else {
        self?.appDidEnterBackground()
      }
    }
  }

  func reset() {
    let (task, stale) = state.withLock { s -> ((any PingableWebSocketTask)?, [Task<Void, Never>]) in
      s.generation += 1
      s.acknowledgements = 0
      s.ticketInstalled = false
      let pinger = s.pinger
      s.pinger = nil
      let deadline = Self.disarm(&s)
      return (s.task, [deadline, pinger].compactMap { $0 })
    }
    stale.forEach { $0.cancel() }
    logger.notice("reset")
    task?.cancel(with: .goingAway, reason: nil)
  }

  fileprivate func attemptStarted(_ task: any PingableWebSocketTask) {
    let (fresh, foreground, stale) = state.withLock { s -> (Bool, Bool, Task<Void, Never>?) in
      let fresh = s.ticketInstalled
      s.phase = .connecting
      s.ticketInstalled = false
      s.task = task
      let pinger = s.pinger
      s.pinger = nil
      return (fresh, s.foreground, pinger)
    }
    stale?.cancel()
    guard fresh else {
      logger.notice("attempt without a fresh ticket")
      task.cancel(with: .goingAway, reason: nil)
      return
    }
    logger.notice("connection attempt")
    if foreground { arm(.acknowledgement) }
  }

  private func installDelegate() async {
    let install = state.withLock { s -> Task<Void, Never> in
      if let existing = s.delegateInstall { return existing }
      let task = Task { await self.transport.setDelegate(self) }
      s.delegateInstall = task
      return task
    }
    await install.value
  }

  private func installTicket() async throws {
    let generation = state.withLock { $0.generation }
    defer { state.withLock { $0.ticketFetch = nil } }
    let ticket: String
    do {
      ticket = try await fetchTicket()
    } catch {
      logger.warning("ticket fetch failed", metadata: ["error": "\(error)"])
      throw error
    }
    guard state.withLock({ $0.generation == generation }) else { return }
    await transport.updateConnectingPayload(["session": ticket])
    state.withLock { s in
      if s.generation == generation { s.ticketInstalled = true }
    }
  }

  private func acknowledged() {
    let (count, stale, task, foreground) = state.withLock {
      s -> (Int, Task<Void, Never>?, (any PingableWebSocketTask)?, Bool) in
      s.phase = .connected
      s.acknowledgements += 1
      let stale = Self.disarm(&s)
      return (s.acknowledgements, stale, s.task, s.foreground)
    }
    stale?.cancel()
    if count == 1 {
      logger.notice("connected")
    } else {
      logger.notice("reconnected")
      onReconnect()
    }
    if foreground, let task { startPinger(task) }
  }

  private func disconnected(_ error: (any Error)?) {
    let stale = state.withLock { s -> [Task<Void, Never>] in
      s.phase = .idle
      s.epoch += 1
      s.ticketInstalled = false
      s.task = nil
      let pinger = s.pinger
      s.pinger = nil
      return [Self.disarm(&s), pinger].compactMap { $0 }
    }
    stale.forEach { $0.cancel() }
    if let error {
      logger.notice("disconnected", metadata: ["error": "\(error)"])
    } else {
      logger.notice("disconnected")
    }
  }

  private func startPinger(_ task: any PingableWebSocketTask) {
    let interval = timing.pingInterval
    let pinger = Task { [weak self] in
      while !Task.isCancelled {
        try? await Task.sleep(for: interval)
        guard !Task.isCancelled, let self else { return }
        self.ping(task)
      }
    }
    let previous = state.withLock { s -> Task<Void, Never>? in
      let previous = s.pinger
      s.pinger = pinger
      return previous
    }
    previous?.cancel()
  }

  private func ping(_ task: any PingableWebSocketTask, probe: Bool = false) {
    let token = arm(.pong)
    task.sendPing { [weak self] error in
      guard error == nil else { return }
      self?.pongReceived(token: token, probe: probe)
    }
  }

  private func pongReceived(token: Int, probe: Bool) {
    let (answered, stale) = state.withLock { s -> (Bool, Task<Void, Never>?) in
      guard s.deadlineToken == token else { return (false, nil) }
      return (true, Self.disarm(&s))
    }
    stale?.cancel()
    if answered && probe { logger.notice("probe answered") }
  }

  @discardableResult
  private func arm(_ expectation: Expectation) -> Int {
    let (token, previous) = state.withLock { s -> (Int, Task<Void, Never>?) in
      let previous = Self.disarm(&s)
      return (s.deadlineToken, previous)
    }
    previous?.cancel()
    let deadline = timing.responseDeadline
    let timer = Task { [weak self] in
      try? await Task.sleep(for: deadline)
      self?.deadlineExpired(expectation, token: token)
    }
    let orphan = state.withLock { s -> Task<Void, Never>? in
      guard s.deadlineToken == token else { return timer }
      s.deadline = timer
      return nil
    }
    orphan?.cancel()
    return token
  }

  private func deadlineExpired(_ expectation: Expectation, token: Int) {
    let task = state.withLock { s -> (any PingableWebSocketTask)? in
      guard s.deadlineToken == token else { return nil }
      _ = Self.disarm(&s)
      return s.task
    }
    guard let task else { return }
    logger.warning("\(expectation.rawValue) deadline exceeded")
    task.cancel(with: .goingAway, reason: nil)
  }

  private static func disarm(_ s: inout State) -> Task<Void, Never>? {
    s.deadlineToken += 1
    let deadline = s.deadline
    s.deadline = nil
    return deadline
  }
}

extension SubscriptionConnection: WebSocketTransportDelegate {
  func webSocketTransportDidConnect(_ webSocketTransport: isolated WebSocketTransport) {
    acknowledged()
  }

  func webSocketTransportDidReconnect(_ webSocketTransport: isolated WebSocketTransport) {
    acknowledged()
  }

  func webSocketTransport(
    _ webSocketTransport: isolated WebSocketTransport, didDisconnectWithError error: (any Error)?
  ) {
    disconnected(error)
  }
}

private final class ResumeRelay: Sendable {
  private struct Target {
    weak var connection: SubscriptionConnection?
  }

  private let target = Mutex(Target())

  func bind(_ connection: SubscriptionConnection) {
    target.withLock { $0.connection = connection }
  }

  func resumed(_ task: any PingableWebSocketTask) {
    target.withLock { $0.connection }?.attemptStarted(task)
  }
}
