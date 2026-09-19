import Core
import Design
import FactoryKit
import GraphQL
import Observation

public enum LoginFailure: Sendable, Equatable {
  case invalidCredentials
  case passwordNotSet
  case unknown
}

@Observable
@MainActor
public final class EmailLoginModel {
  public private(set) var isSubmitting = false

  public let form: TFormState
  public let email: TFieldState
  public let password: TFieldState

  @ObservationIgnored private let client = Container.shared.graphQLClient()
  @ObservationIgnored private let onSuccess: @MainActor () -> Void

  public init(onSuccess: @escaping @MainActor () -> Void) {
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
      _ = try await client.perform(
        EmailLogin_LoginWithEmail_Mutation(
          input: LoginWithEmailInput(email: email.value, password: password.value)))
      onSuccess()
      return nil
    } catch let error as APIError {
      switch error.code {
      case "invalid_credentials": return .invalidCredentials
      case "password_not_set": return .passwordNotSet
      default: return .unknown
      }
    } catch is CancellationError {
      return nil
    } catch {
      return .unknown
    }
  }
}
