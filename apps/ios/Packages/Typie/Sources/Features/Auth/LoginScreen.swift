#if canImport(UIKit)

  import Core
  import Design
  import FactoryKit
  import SwiftUI
  import UIKit

  @MainActor
  struct LoginScreen: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var isRevealed = false
    @Namespace private var glass

    private static let options: [LoginOption] = [
      .provider(.google), .provider(.kakao), .provider(.naver), .provider(.apple), .email,
    ]

    private let singleSignOn: SingleSignOnModel
    private let onEmail: () -> Void
    private let presenter: @MainActor () -> UIViewController?
    private let toast = Container.shared.toast()

    init(
      onEmail: @escaping () -> Void, presenter: @escaping @MainActor () -> UIViewController?
    ) {
      self.onEmail = onEmail
      self.presenter = presenter
      singleSignOn = Container.shared.singleSignOnModel {}
    }

    var body: some View {
      GeometryReader { proxy in
        let insets = proxy.safeAreaInsets
        let restShift = (insets.bottom + StartButton.height - insets.top) / 2
        VStack(spacing: 0) {
          TiltedLogo()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .offset(y: isRevealed ? 0 : restShift)
          bottom
        }
        .padding(.horizontal, 16)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
      .background(LoginBackdrop())
      .canvasBackground()
      .ignoresSafeArea(.keyboard)
    }

    @ViewBuilder
    private var bottom: some View {
      if #available(iOS 26, macOS 26, *) {
        GlassEffectContainer(spacing: 12) { bottomContent }
      } else {
        bottomContent
      }
    }

    private var bottomContent: some View {
      VStack(spacing: 8) {
        if isRevealed {
          ForEach(Self.options, id: \.self) { option in
            LoginOptionButton(
              option,
              loading: option.provider != nil && option.provider == singleSignOn.activeProvider,
              glassID: .option(option), in: glass
            ) {
              if let provider = option.provider {
                Task {
                  if await singleSignOn.signIn(with: provider, presenter: presenter) == .failed {
                    toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
                  }
                }
              } else {
                onEmail()
              }
            }
          }
        } else {
          StartButton(glassID: .start, in: glass) {
            withAnimation(reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.45)) {
              isRevealed = true
            }
          }
        }
      }
      .allowsHitTesting(!singleSignOn.isBusy)
    }
  }

  private struct TiltedLogo: View {
    @Environment(\.theme) private var theme
    @Environment(\.colorScheme) private var colorScheme
    private var colors: TColors { theme.colors }

    @State private var viewer = UIOffset.zero

    private let height = TLogo.launchHeight
    private let dispersion: CGFloat = 0.05
    private let tiltShift: CGFloat = 2

    private let sensitivity: CGFloat = 2.4

    private func sensitive(_ value: CGFloat) -> CGFloat {
      tanh(value * sensitivity) / tanh(sensitivity)
    }

    private var tiltX: CGFloat { sensitive(viewer.horizontal) }
    private var tiltY: CGFloat { sensitive(viewer.vertical) }
    private var tiltMagnitude: CGFloat { min(1, hypot(tiltX, tiltY)) }

    private var fringeBlend: BlendMode { colorScheme == .dark ? .screen : .multiply }

    private func placed(_ color: Color, sign: CGFloat) -> some View {
      TLogo(height: height, color: color)
        .scaleEffect(1 + dispersion * tiltMagnitude * sign)
        .offset(x: tiltX * tiltShift * sign, y: tiltY * tiltShift * sign)
    }

    var body: some View {
      ZStack {
        placed(Color(red: 0.98, green: 0.84, blue: 0.10), sign: 1).blendMode(fringeBlend)
        placed(Color(red: 0.16, green: 0.36, blue: 0.96), sign: -1).blendMode(fringeBlend)
        placed(colors.textDefault, sign: 1).mask(placed(.black, sign: -1))
      }
      .compositingGroup()
      .background(ViewerOffsetReader { viewer = $0 }.frame(width: 0, height: 0))
      .rotation3DEffect(.degrees(-tiltY * 12), axis: (x: 1, y: 0, z: 0), perspective: 0.7)
      .rotation3DEffect(
        .degrees(tiltX * 12), axis: (x: 0, y: 1, z: 0), perspective: 0.7
      )
      .offset(x: tiltX * 6, y: tiltY * 6)
    }
  }

  private struct StartButton: View {
    static let height: CGFloat = 52

    @Environment(\.theme) private var theme
    @Environment(\.colorScheme) private var colorScheme
    private var colors: TColors { theme.colors }

    private var lift: Color? {
      colorScheme == .dark ? colors.textDefault.opacity(0.12) : nil
    }

    let glassID: LoginGlassID
    let namespace: Namespace.ID
    let action: () -> Void

    init(glassID: LoginGlassID, in namespace: Namespace.ID, action: @escaping () -> Void) {
      self.glassID = glassID
      self.namespace = namespace
      self.action = action
    }

    private var label: some View {
      TText("시작하기", style: TTypography.control, color: colors.textDefault)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, minHeight: Self.height)
        .contentShape(TShapes.capsule)
    }

    var body: some View {
      Button(action: action) { label }
        .buttonStyle(
          LoginGlassLook(
            glass: .clear, tint: lift, fallback: colors.surfaceInset, glassID: glassID,
            namespace: namespace)
        )
    }
  }

#endif
