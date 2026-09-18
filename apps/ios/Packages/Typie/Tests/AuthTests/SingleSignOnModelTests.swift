import Core
import Design
import Testing

@testable import Auth

@MainActor
@Suite struct SingleSignOnModelTests {
  nonisolated private static let defaultError = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

  @Test func successCallsLoginOnceAndReportsSuccess() async {
    let context = TestContext()

    await context.model.signIn(with: .kakao)

    #expect(context.login.calls == [.kakao])
    #expect(context.success.count == 1)
    #expect(context.toast.current == nil)
    #expect(context.model.activeProvider == nil)
    #expect(context.model.isBusy == false)
  }

  @Test func userCancellationIsSilent() async {
    let context = TestContext()
    context.login.error = SingleSignOnError.cancelled

    await context.model.signIn(with: .google)

    #expect(context.login.calls == [.google])
    #expect(context.success.count == 0)
    #expect(context.toast.current == nil)
    #expect(context.model.activeProvider == nil)
  }

  @Test func taskCancellationIsSilent() async {
    let context = TestContext()
    context.login.error = CancellationError()

    await context.model.signIn(with: .apple)

    #expect(context.success.count == 0)
    #expect(context.toast.current == nil)
  }

  @Test func otherFailuresShowDefaultToast() async {
    let context = TestContext()
    context.login.error = HTTPError.status(500)

    await context.model.signIn(with: .naver)

    #expect(context.success.count == 0)
    #expect(context.toast.current?.kind == .error)
    #expect(context.toast.current?.message == Self.defaultError)
    #expect(context.model.activeProvider == nil)
  }

  @Test func missingCredentialShowsDefaultToast() async {
    let context = TestContext()
    context.login.error = SingleSignOnError.missingCredential

    await context.model.signIn(with: .google)

    #expect(context.toast.current?.message == Self.defaultError)
  }

  @Test func secondTapIsIgnoredWhileFirstIsInFlight() async {
    let context = TestContext()
    context.login.hold()

    let first = Task { await context.model.signIn(with: .kakao) }
    await context.login.waitForEntry()

    #expect(context.model.activeProvider == .kakao)
    #expect(context.model.isBusy)

    await context.model.signIn(with: .naver)
    #expect(context.login.calls == [.kakao])

    context.login.release()
    await first.value

    #expect(context.login.calls == [.kakao])
    #expect(context.success.count == 1)
    #expect(context.model.activeProvider == nil)
  }
}

@MainActor
private final class TestContext {
  let login = SingleSignOnStub()
  let success = SuccessCounter()
  let toast = TToastCenter(sleep: { _ in try await Task.sleep(for: .seconds(60)) })
  let model: SingleSignOnModel

  init() {
    let success = success
    model = SingleSignOnModel(login: login.login, toast: toast, onSuccess: { success.record() })
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
