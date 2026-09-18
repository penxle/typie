import Core
import Design
import SwiftUI

enum LoginGlassID: Hashable, Sendable {
  case start
  case option(LoginOption)
}

enum LoginOption: Hashable, Sendable {
  case provider(TBrandMark)
  case email

  var text: String {
    switch self {
    case .provider(.google): "구글로 시작하기"
    case .provider(.kakao): "카카오로 시작하기"
    case .provider(.naver): "네이버로 시작하기"
    case .provider(.apple): "애플로 시작하기"
    case .email: "이메일로 시작하기"
    }
  }

  var provider: SingleSignOnProvider? {
    switch self {
    case .provider(.google): .google
    case .provider(.kakao): .kakao
    case .provider(.naver): .naver
    case .provider(.apple): .apple
    case .email: nil
    }
  }
}

struct LoginOptionButton: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let option: LoginOption
  private let loading: Bool
  private let glassID: LoginGlassID
  private let namespace: Namespace.ID
  private let action: () -> Void

  init(
    _ option: LoginOption, loading: Bool = false, glassID: LoginGlassID,
    in namespace: Namespace.ID, action: @escaping () -> Void
  ) {
    self.option = option
    self.loading = loading
    self.glassID = glassID
    self.namespace = namespace
    self.action = action
  }

  private var mark: TBrandMark? {
    if case .provider(let mark) = option { mark } else { nil }
  }

  private var tint: Color? { mark?.background }

  private var foreground: Color { mark?.foreground ?? colors.textDefault }

  private var label: some View {
    HStack(spacing: 8) {
      if loading {
        TSpinner(color: foreground, size: 18, relativeTo: TTypography.control)
      } else {
        switch option {
        case .provider(let mark):
          TBrandMarkView(mark, size: 18, relativeTo: TTypography.control)
        case .email:
          TIcon(LucideIcon.mail, size: 18, tint: foreground, relativeTo: TTypography.control)
        }
      }
      TText(option.text, style: TTypography.control, color: foreground)
    }
    .padding(.vertical, 12)
    .frame(maxWidth: .infinity, minHeight: 48)
    .contentShape(TShapes.capsule)
  }

  var body: some View {
    Button(action: action) { label }
      .buttonStyle(
        LoginGlassLook(
          glass: .regular, tint: tint, fallback: tint ?? colors.surfaceDefault, bordered: true,
          glassID: glassID, namespace: namespace)
      )
  }
}

struct LoginGlassLook: ButtonStyle {
  enum GlassKind {
    case regular
    case clear
  }

  @Environment(\.theme) private var theme

  let glass: GlassKind
  let tint: Color?
  let fallback: Color
  var bordered = false
  let glassID: LoginGlassID
  let namespace: Namespace.ID

  @available(iOS 26, macOS 26, *)
  private var resolvedGlass: Glass {
    let base: Glass = glass == .regular ? .regular : .clear
    return (tint.map { base.tint($0) } ?? base).interactive()
  }

  func makeBody(configuration: Configuration) -> some View {
    if #available(iOS 26, macOS 26, *) {
      configuration.label
        .glassEffect(resolvedGlass, in: .capsule)
        .glassEffectID(glassID, in: namespace)
        .glassEffectTransition(.materialize)
    } else {
      configuration.label
        .background(fallback, in: TShapes.capsule)
        .overlay {
          if bordered {
            TShapes.capsule.strokeBorder(theme.colors.borderHairline, lineWidth: 0.5)
          }
        }
        .scaleEffect(configuration.isPressed ? 0.97 : 1)
        .animation(.smooth(duration: 0.2), value: configuration.isPressed)
    }
  }
}
