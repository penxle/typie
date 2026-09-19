import Core
import Testing

@testable import Auth

@MainActor
@Suite struct SingleSignOnModelTests {
  @Test func successCallsLoginOnceAndReportsSuccess() async {
    let context = TestContext()

    let outcome = await context.model.signIn(with: .kakao)

    #expect(outcome == .succeeded)
    #expect(context.login.calls == [.kakao])
    #expect(context.success.count == 1)
    #expect(context.model.activeProvider == nil)
    #expect(context.model.isBusy == false)
  }

  @Test func userCancellationReportsCancelled() async {
    let context = TestContext()
    context.login.error = SingleSignOnError.cancelled

    let outcome = await context.model.signIn(with: .google)

    #expect(outcome == .cancelled)
    #expect(context.login.calls == [.google])
    #expect(context.success.count == 0)
    #expect(context.model.activeProvider == nil)
  }

  @Test func taskCancellationReportsCancelled() async {
    let context = TestContext()
    context.login.error = CancellationError()

    let outcome = await context.model.signIn(with: .apple)

    #expect(outcome == .cancelled)
    #expect(context.success.count == 0)
  }

  @Test func otherFailuresReportFailed() async {
    let context = TestContext()
    context.login.error = HTTPError.status(500)

    let outcome = await context.model.signIn(with: .naver)

    #expect(outcome == .failed)
    #expect(context.success.count == 0)
    #expect(context.model.activeProvider == nil)
  }

  @Test func missingCredentialReportsFailed() async {
    let context = TestContext()
    context.login.error = SingleSignOnError.missingCredential

    let outcome = await context.model.signIn(with: .google)

    #expect(outcome == .failed)
  }

  @Test func secondTapIsIgnoredWhileFirstIsInFlight() async {
    let context = TestContext()
    context.login.hold()

    let first = Task { await context.model.signIn(with: .kakao) }
    await context.login.waitForEntry()

    #expect(context.model.activeProvider == .kakao)
    #expect(context.model.isBusy)

    let second = await context.model.signIn(with: .naver)
    #expect(second == .cancelled)
    #expect(context.login.calls == [.kakao])

    context.login.release()
    let outcome = await first.value

    #expect(outcome == .succeeded)
    #expect(context.login.calls == [.kakao])
    #expect(context.success.count == 1)
    #expect(context.model.activeProvider == nil)
  }
}

@MainActor
private final class TestContext {
  let login = SingleSignOnStub()
  let success = SuccessCounter()
  let model: SingleSignOnModel

  init() {
    let success = success
    model = SingleSignOnModel(login: login.login, onSuccess: { success.record() })
  }
}

@MainActor
private final class SingleSignOnStub {
  private(set) var calls: [SingleSignOnProvider] = []
  var error: (any Error)?

  private var gated = false
  private var gate: CheckedContinuation<Void, Never>?
  private var entry: CheckedContinuation<Void, Never>?

  var login: @Sendable (SingleSignOnProvider) async throws -> Void {
    { [self] provider in try await run(provider) }
  }

  func hold() {
    gated = true
  }

  func waitForEntry() async {
    guard gate == nil else { return }
    await withCheckedContinuation { entry = $0 }
  }

  func release() {
    gated = false
    gate?.resume()
    gate = nil
  }

  private func run(_ provider: SingleSignOnProvider) async throws {
    calls.append(provider)
    if gated, calls.count == 1 {
      await withCheckedContinuation { continuation in
        gate = continuation
        entry?.resume()
        entry = nil
      }
    }
    if let error {
      throw error
    }
  }
}

@MainActor
private final class SuccessCounter {
  private(set) var count = 0

  func record() {
    count += 1
  }
}
