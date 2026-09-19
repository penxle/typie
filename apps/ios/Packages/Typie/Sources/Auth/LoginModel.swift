import Core
import Design
import Observation

public enum LoginFailure: Sendable, Equatable {
  case invalidCredentials
  case passwordNotSet
  case unknown
}

@Observable
@MainActor
public final class LoginModel {
  public private(set) var isSubmitting = false

  public let form: TFormState
  public let email: TFieldState
  public let password: TFieldState

  @ObservationIgnored private let login: @Sendable (String, String) async throws -> Void
  @ObservationIgnored private let onSuccess: @MainActor () -> Void

  public init(
    login: @escaping @Sendable (_ email: String, _ password: String) async throws -> Void,
    onSuccess: @escaping @MainActor () -> Void
  ) {
    self.login = login
    self.onSuccess = onSuccess

    let form = TFormState(autoFocusFirstField: false, validatesOnBlur: true)
    email = form.field(
      rules: [.required("이메일을 입력해주세요."), .email("올바른 이메일 형식을 입력해주세요.")])
    password = form.field(rules: [.required("비밀번호를 입력해주세요.")])
    self.form = form
  }

  public func focusEmail() {
    email.requestFocus()
  }

  public func discardPassword() {
    password.value = ""
  }

  public func submit() async -> LoginFailure? {
    guard !isSubmitting else { return nil }
    isSubmitting = true
    defer { isSubmitting = false }
    guard form.validate() else { return nil }
    form.endEditing()
    do {
      try await login(email.value, password.value)
      onSuccess()
      return nil
    } catch EmailLoginError.invalidCredentials {
      return .invalidCredentials
    } catch EmailLoginError.passwordNotSet {
      return .passwordNotSet
    } catch is CancellationError {
      return nil
    } catch {
      return .unknown
    }
  }
}
