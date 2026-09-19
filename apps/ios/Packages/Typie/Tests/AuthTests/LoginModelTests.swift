import Core
import Design
import Testing

@testable import Auth

@MainActor
@Suite struct LoginModelTests {
  nonisolated private static let emailRequired = "이메일을 입력해주세요."
  nonisolated private static let emailInvalid = "올바른 이메일 형식을 입력해주세요."
  nonisolated private static let passwordRequired = "비밀번호를 입력해주세요."
  nonisolated fileprivate static let validEmail = "Me@Example.COM"
  nonisolated fileprivate static let validPassword = "  a b  "

  @Test func emptySubmissionReportsFieldErrorsWithoutLogin() async {
    let context = TestContext()

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.login.calls.isEmpty)
    #expect(context.model.email.errors == [Self.emailRequired])
    #expect(context.model.password.errors == [Self.passwordRequired])
    #expect(context.success.count == 0)
  }

  @Test func malformedEmailReportsFormatErrorWithoutLogin() async {
    let context = TestContext()
    context.model.email.value = "invalid-email"
    context.model.password.value = Self.validPassword

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.login.calls.isEmpty)
    #expect(context.model.email.errors == [Self.emailInvalid])
    #expect(context.model.password.errors.isEmpty)
    #expect(context.success.count == 0)
  }

  @Test func successSendsRawInputAndReportsSuccess() async {
    let context = TestContext()
    context.fill()

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(
      context.login.calls == [
        LoginStub.Call(email: Self.validEmail, password: Self.validPassword)
      ])
    #expect(context.success.count == 1)
    #expect(context.model.email.errors.isEmpty)
    #expect(context.model.password.errors.isEmpty)
  }

  @Test func invalidCredentialsReportsFailureAndKeepsInput() async {
    let context = TestContext()
    context.fill()
    context.login.error = EmailLoginError.invalidCredentials

    let failure = await context.model.submit()

    #expect(failure == .invalidCredentials)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func passwordNotSetReportsFailureAndKeepsInput() async {
    let context = TestContext()
    context.fill()
    context.login.error = EmailLoginError.passwordNotSet

    let failure = await context.model.submit()

    #expect(failure == .passwordNotSet)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func unclassifiedErrorReportsUnknownFailureAndKeepsInput() async {
    let context = TestContext()
    context.fill()
    context.login.error = HTTPError.status(500)

    let failure = await context.model.submit()

    #expect(failure == .unknown)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func cancellationReportsNoFailure() async {
    let context = TestContext()
    context.fill()
    context.login.error = CancellationError()

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
  }

  @Test func repeatedFailureReportsFailureEachTime() async {
    let context = TestContext()
    context.fill()
    context.login.error = EmailLoginError.invalidCredentials

    let first = await context.model.submit()
    let second = await context.model.submit()

    #expect(first == .invalidCredentials)
    #expect(second == .invalidCredentials)
    #expect(context.login.calls.count == 2)
  }

  @Test func formValidatesOnBlurAndFocusesEmailOnRequest() {
    let context = TestContext()
    #expect(context.model.form.autoFocusFirstField == false)
    #expect(context.model.form.validatesOnBlur)

    context.model.focusEmail()
    #expect(context.model.email.focusRequest == 1)
    #expect(context.model.password.focusRequest == 0)
  }

  @Test func secondSubmitIsIgnoredWhileFirstIsInFlight() async {
    let context = TestContext()
    context.fill()
    context.login.hold()

    let first = Task { await context.model.submit() }
    await context.login.waitForEntry()

    #expect(context.model.isSubmitting)

    let ignored = await context.model.submit()

    #expect(ignored == nil)
    #expect(context.login.calls.count == 1)

    context.login.release()
    _ = await first.value

    #expect(context.login.calls.count == 1)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 1)
  }
}

@MainActor
private final class TestContext {
  let login = LoginStub()
  let success = SuccessCounter()
  let model: LoginModel

  init() {
    let success = success
    model = LoginModel(login: login.login, onSuccess: { success.record() })
  }

  func fill() {
    model.email.value = LoginModelTests.validEmail
    model.password.value = LoginModelTests.validPassword
  }
}

@MainActor
private final class LoginStub {
  struct Call: Equatable {
    let email: String
    let password: String
  }

  private(set) var calls: [Call] = []
  var error: (any Error)?

  private var gated = false
  private var gate: CheckedContinuation<Void, Never>?
  private var entry: CheckedContinuation<Void, Never>?

  var login: @Sendable (String, String) async throws -> Void {
    { [self] email, password in try await run(email: email, password: password) }
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

  private func run(email: String, password: String) async throws {
    calls.append(Call(email: email, password: password))
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
