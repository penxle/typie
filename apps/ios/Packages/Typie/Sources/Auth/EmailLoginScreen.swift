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

    public init(model: LoginModel) {
      self.model = model
    }

    public var body: some View {
      GeometryReader { proxy in
        VStack(spacing: 0) {
          ScrollView {
            VStack(alignment: .leading, spacing: 0) {
              Spacer().frame(height: 8)
              TText("이메일로 시작하기", style: TTypography.display, color: colors.textDefault)
              Spacer().frame(height: 24)
              TTextField(
                model.email, form: model.form, label: "이메일", placeholder: "me@example.com",
                contentType: .username, keyboardType: .emailAddress)
              Spacer().frame(height: 12)
              TTextField(
                model.password, form: model.form, label: "비밀번호", placeholder: "********",
                contentType: .password, isSecure: true,
                onSubmit: { Task { await model.submit() } })
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
            await model.submit()
          }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, keyboardVisible ? 12 : max(0, 16 - proxy.safeAreaInsets.bottom))
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
      .canvasBackground()
      .onReceive(
        NotificationCenter.default.publisher(for: UIResponder.keyboardWillShowNotification)
      ) {
        setKeyboardVisible(true, notification: $0)
      }
      .onReceive(
        NotificationCenter.default.publisher(for: UIResponder.keyboardWillHideNotification)
      ) {
        setKeyboardVisible(false, notification: $0)
      }
      .dialog(model.dialog) { model.dismissDialog() }
    }

    private func setKeyboardVisible(_ value: Bool, notification: Notification) {
      let info = notification.userInfo
      guard info?[UIResponder.keyboardIsLocalUserInfoKey] as? Bool != false else { return }
      guard keyboardVisible != value else { return }
      let duration = info?[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double ?? 0.25
      withAnimation(reduceMotion ? nil : .easeOut(duration: duration)) {
        keyboardVisible = value
      }
    }
  }

#endif
