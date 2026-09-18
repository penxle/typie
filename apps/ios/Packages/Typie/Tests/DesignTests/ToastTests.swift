import Testing

@testable import Design

@MainActor
@Suite struct ToastTests {
  nonisolated private static let invalidCredentials = "이메일 또는 비밀번호가 올바르지 않아요."
  nonisolated private static let passwordNotSet = "비밀번호가 설정되지 않았어요."

  @Test func startsEmpty() {
    #expect(TToastCenter(sleep: SleepGate().sleep).current == nil)
  }

  @Test func durationGrowsWithMessageLength() {
    #expect(TToastCenter.duration(for: "") == .milliseconds(2000))
    #expect(TToastCenter.duration(for: String(repeating: "가", count: 18)) == .milliseconds(2000))
    #expect(TToastCenter.duration(for: String(repeating: "가", count: 21)) == .milliseconds(2036))
    #expect(TToastCenter.duration(for: String(repeating: "가", count: 118)) == .milliseconds(3200))
    #expect(TToastCenter.duration(for: String(repeating: "가", count: 400)) == .milliseconds(3200))
  }

  @Test func errorPublishesToastAndStartsTimer() async {
    let gate = SleepGate()
    let center = TToastCenter(sleep: gate.sleep)

    center.error(Self.invalidCredentials)
    await settle()

    #expect(center.current?.message == Self.invalidCredentials)
    #expect(center.current?.kind == .error)
    #expect(gate.requests == [.milliseconds(2048)])
  }

  @Test func secondToastReplacesFirstAndOutlivesItsTimer() async throws {
    let gate = SleepGate()
    let center = TToastCenter(sleep: gate.sleep)

    center.error(Self.invalidCredentials)
    await settle()
    let first = try #require(center.current)

    center.error(Self.passwordNotSet)
    await settle()
    let second = try #require(center.current)
    #expect(second.id != first.id)
    #expect(second.message == Self.passwordNotSet)

    gate.expireOldest()
    await settle()

    #expect(center.current?.id == second.id)
    #expect(center.current?.message == Self.passwordNotSet)
    #expect(gate.requests == [.milliseconds(2048), .milliseconds(2000)])

    gate.expireOldest()
    await settle()

    #expect(center.current == nil)
  }

  @Test func timerExpiryClearsToast() async {
    let gate = SleepGate()
    let center = TToastCenter(sleep: gate.sleep)

    center.error(Self.invalidCredentials)
    await settle()
    gate.expireOldest()
    await settle()

    #expect(center.current == nil)
  }

  @Test func dismissClearsToastImmediately() async {
    let gate = SleepGate()
    let center = TToastCenter(sleep: gate.sleep)

    center.error(Self.invalidCredentials)
    await settle()
    center.dismiss()

    #expect(center.current == nil)

    gate.expireOldest()
    await settle()
    #expect(center.current == nil)
  }

  private func settle() async {
    for _ in 0..<4 {
      await Task.yield()
    }
  }
}

@MainActor
private final class SleepGate {
  private(set) var requests: [Duration] = []
  private var pending: [CheckedContinuation<Void, Never>] = []

  var sleep: @Sendable (Duration) async throws -> Void {
    { [self] duration in await wait(duration) }
  }

  func expireOldest() {
    guard pending.isEmpty == false else { return }
    pending.removeFirst().resume()
  }

  private func wait(_ duration: Duration) async {
    requests.append(duration)
    await withCheckedContinuation { continuation in
      pending.append(continuation)
    }
  }
}
