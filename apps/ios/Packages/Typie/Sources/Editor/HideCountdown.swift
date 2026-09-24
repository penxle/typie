@MainActor
final class HideCountdown {
  var onExpire: (() -> Void)?

  private let delay: Duration
  private let now: @MainActor () -> ContinuousClock.Instant
  private let sleep: @Sendable (Duration) async throws -> Void
  private var last: ContinuousClock.Instant?
  private var timer: Task<Void, Never>?

  init(
    delay: Duration, now: @escaping @MainActor () -> ContinuousClock.Instant,
    sleep: @escaping @Sendable (Duration) async throws -> Void
  ) {
    self.delay = delay
    self.now = now
    self.sleep = sleep
  }

  func start() {
    last = now()
    if timer == nil {
      timer = Task { await self.run() }
    }
  }

  func stop() {
    timer?.cancel()
    timer = nil
    last = nil
  }

  private func run() async {
    while let last {
      let remaining = last + delay - now()
      guard remaining > .zero else { break }
      do { try await sleep(remaining) } catch { return }
      if Task.isCancelled { return }
    }
    guard !Task.isCancelled else { return }
    timer = nil
    last = nil
    onExpire?()
  }
}
