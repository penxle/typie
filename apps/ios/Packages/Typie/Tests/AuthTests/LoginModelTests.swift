import Core
import Design
import Testing

@testable import Auth

@MainActor
@Suite struct LoginModelTests {
  nonisolated private static let emailRequired = "이메일을 입력해주세요."
  nonisolated private static let emailInvalid = "올바른 이메일 형식을 입력해주세요."
  nonisolated private static let passwordRequired = "비밀번호를 입력해주세요."
  nonisolated private static let invalidCredentials =
    "입력한 로그인 정보가 일치하지 않아요. 이메일과 비밀번호를 다시 한번 확인해주세요."
  nonisolated private static let invalidCredentialsTitle = "잘못된 이메일 또는 비밀번호예요"
  nonisolated private static let passwordNotSet =
    "이 계정에는 아직 비밀번호가 없어요. 가입할 때 사용한 SNS 계정으로 시작하거나, 로그인 후 설정에서 비밀번호를 설정하면 이메일로도 로그인할 수 있어요."
  nonisolated private static let passwordNotSetTitle = "SNS 계정으로 가입한 이메일이에요"
  nonisolated private static let unknownError = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
  nonisolated private static let dialogTitle = "로그인할 수 없어요"
  nonisolated private static let confirm = "확인"
  nonisolated fileprivate static let validEmail = "Me@Example.COM"
  nonisolated fileprivate static let validPassword = "  a b  "

  @Test func emptySubmissionReportsFieldErrorsWithoutLogin() async {
    let context = TestContext()

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(context.login.calls.isEmpty)
    #expect(context.model.email.errors == [Self.emailRequired])
    #expect(context.model.password.errors == [Self.passwordRequired])
    #expect(context.model.dialog == nil)
    #expect(context.success.count == 0)
  }

  @Test func malformedEmailReportsFormatErrorWithoutLogin() async {
    let context = TestContext()
    context.model.email.value = "invalid-email"
    context.model.password.value = Self.validPassword

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(context.login.calls.isEmpty)
    #expect(context.model.email.errors == [Self.emailInvalid])
    #expect(context.model.password.errors.isEmpty)
    #expect(context.model.dialog == nil)
    #expect(context.success.count == 0)
  }

  @Test func successSendsRawInputAndReportsSuccess() async {
    let context = TestContext()
    context.fill()

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(
      context.login.calls == [
        LoginStub.Call(email: Self.validEmail, password: Self.validPassword)
      ])
    #expect(context.success.count == 1)
    #expect(context.model.dialog == nil)
    #expect(context.model.email.errors.isEmpty)
    #expect(context.model.password.errors.isEmpty)
  }

  @Test func invalidCredentialsShowsDialogAndKeepsInput() async {
    let context = TestContext()
    context.fill()
    context.login.error = EmailLoginError.invalidCredentials

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(context.model.dialog?.title == Self.invalidCredentialsTitle)
    #expect(context.model.dialog?.message == Self.invalidCredentials)
    #expect(context.model.dialog?.confirmText == Self.confirm)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func passwordNotSetShowsDialogAndKeepsInput() async {
    let context = TestContext()
    context.fill()
    context.login.error = EmailLoginError.passwordNotSet

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(context.model.dialog?.title == Self.passwordNotSetTitle)
    #expect(context.model.dialog?.message == Self.passwordNotSet)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func unclassifiedErrorShowsDefaultDialogAndKeepsInput() async {
    let context = TestContext()
    context.fill()
    context.login.error = HTTPError.status(500)

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(context.model.dialog?.title == Self.dialogTitle)
    #expect(context.model.dialog?.message == Self.unknownError)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func cancellationShowsNoDialog() async {
    let context = TestContext()
    context.fill()
    context.login.error = CancellationError()

    await context.model.submit()

    #expect(context.model.isSubmitting == false)
    #expect(context.model.dialog == nil)
    #expect(context.success.count == 0)
  }

  @Test func dismissClearsDialogAndRepeatedFailureShowsFreshDialog() async {
    let context = TestContext()
    context.fill()
    context.login.error = EmailLoginError.invalidCredentials

    await context.model.submit()
    let first = context.model.dialog
    context.model.dismissDialog()
    #expect(context.model.dialog == nil)

    await context.model.submit()
    #expect(context.model.dialog != nil)
    #expect(context.model.dialog != first)
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

    await context.model.submit()

    #expect(context.login.calls.count == 1)

    context.login.release()
    await first.value

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
