#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  public struct EmailLoginScreen: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    @State private var keyboardVisible = false

    private let model: LoginModel
    private let dialog: TDialogCenter

    public init(model: LoginModel, dialog: TDialogCenter) {
      self.model = model
      self.dialog = dialog
    }

    public var body: some View {
      GeometryReader { proxy in
        VStack(spacing: 0) {
          ScrollView {
            VStack(alignment: .leading, spacing: 0) {
              Spacer().frame(height: 8)
              TText("이메일로 시작하기", style: TTypography.hero, color: colors.textDefault)
              Spacer().frame(height: 24)
              TTextField(
                model.email, form: model.form, label: "이메일", placeholder: "me@example.com",
                contentType: .username, keyboardType: .emailAddress)
              Spacer().frame(height: 12)
              TTextField(
                model.password, form: model.form, label: "비밀번호", placeholder: "********",
                contentType: .password, isSecure: true,
                onSubmit: { Task { await submit() } })
              Spacer().frame(height: 12)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .animation(
              reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.22),
              value: [model.email.error, model.password.error])
          }
          .scrollBounceBehavior(.basedOnSize)
          .onTapGesture { model.form.endEditing() }
          TButton(
            "로그인", loading: model.isSubmitting, loadingText: "로그인 중...", height: 56,
            textStyle: TTypography.title
          ) {
            await submit()
          }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, keyboardVisible ? 12 : max(0, 16 - proxy.safeAreaInsets.bottom))
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
      .canvasBackground()
      .observingKeyboard($keyboardVisible)
    }

    private func submit() async {
      if let failure = await model.submit() {
        dialog.present(failure.dialogItem)
      }
    }
  }

  extension LoginFailure {
    fileprivate var dialogItem: TDialogItem {
      switch self {
      case .invalidCredentials:
        TDialogItem(
          title: "잘못된 이메일 또는 비밀번호예요",
          message: "입력한 로그인 정보가 일치하지 않아요. 이메일과 비밀번호를 다시 한번 확인해주세요.",
          confirmText: "확인")
      case .passwordNotSet:
        TDialogItem(
          title: "SNS 계정으로 가입한 이메일이에요",
          message:
            "이 계정에는 아직 비밀번호가 없어요. 가입할 때 사용한 SNS 계정으로 시작하거나, 로그인 후 설정에서 비밀번호를 설정하면 이메일로도 로그인할 수 있어요.",
          confirmText: "확인")
      case .unknown:
        TDialogItem(
          title: "로그인할 수 없어요", message: "오류가 발생했어요. 잠시 후 다시 시도해주세요.",
          confirmText: "확인")
      }
    }
  }

#endif
