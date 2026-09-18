import Core
import Design
import Observation

@Observable
@MainActor
public final class LoginModel {
  public private(set) var isSubmitting = false
  public private(set) var dialog: TDialogItem?

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

  public func dismissDialog() {
    dialog = nil
  }

  public func focusEmail() {
    email.requestFocus()
  }

  public func discardPassword() {
    password.value = ""
  }

  public func submit() async {
    guard !isSubmitting else { return }
    isSubmitting = true
    defer { isSubmitting = false }
    guard form.validate() else { return }
    form.endEditing()
    do {
      try await login(email.value, password.value)
      onSuccess()
    } catch EmailLoginError.invalidCredentials {
      fail(
        title: "잘못된 이메일 또는 비밀번호예요", message: "입력한 로그인 정보가 일치하지 않아요. 이메일과 비밀번호를 다시 한번 확인해주세요.")
    } catch EmailLoginError.passwordNotSet {
      fail(
        title: "SNS 계정으로 가입한 이메일이에요",
        message:
          "이 계정에는 아직 비밀번호가 없어요. 가입할 때 사용한 SNS 계정으로 시작하거나, 로그인 후 설정에서 비밀번호를 설정하면 이메일로도 로그인할 수 있어요."
      )
    } catch is CancellationError {
      return
    } catch {
      fail(title: "로그인할 수 없어요", message: "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
    }
  }

  private func fail(title: String, message: String) {
    dialog = TDialogItem(title: title, message: message, confirmText: "확인")
  }
}
