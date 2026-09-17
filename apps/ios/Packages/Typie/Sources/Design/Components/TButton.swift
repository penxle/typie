import SwiftUI

public enum TButtonVariant: Sendable {
  case primary
  case secondary
  case danger
}

public struct TButton: View {
  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }
  @State private var debouncedLoading = false
  @State private var hovered = false

  private let text: String
  private let variant: TButtonVariant
  private let enabled: Bool
  private let loading: Bool
  private let loadingText: String?
  private let height: CGFloat
  private let textStyle: TTextStyle
  private let leadingIcon: TIconName?
  private let trailingIcon: TIconName?
  private let action: () async -> Void

  public init(
    _ text: String,
    variant: TButtonVariant = .primary,
    enabled: Bool = true,
    loading: Bool = false,
    loadingText: String? = nil,
    height: CGFloat = 48,
    textStyle: TTextStyle = TTypography.action,
    leadingIcon: TIconName? = nil,
    trailingIcon: TIconName? = nil,
    action: @escaping () async -> Void
  ) {
    self.text = text
    self.variant = variant
    self.enabled = enabled
    self.loading = loading
    self.loadingText = loadingText
    self.height = height
    self.textStyle = textStyle
    self.leadingIcon = leadingIcon
    self.trailingIcon = trailingIcon
    self.action = action
  }

  private var background: Color {
    switch variant {
    case .primary: colors.textDefault
    case .secondary: colors.surfaceInset
    case .danger: colors.dangerDefault
    }
  }

  private var foreground: Color {
    switch variant {
    case .primary: colors.surfaceCanvas
    case .secondary: colors.textDefault
    case .danger: colors.textOnDanger
    }
  }

  private var interactive: Bool { enabled && !loading }

  private var loadingAnimation: Animation {
    reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.3)
  }

  public var body: some View {
    let displayText = debouncedLoading ? (loadingText ?? text) : text
    Button {
      Task { await action() }
    } label: {
      HStack(spacing: 0) {
        if debouncedLoading {
          TSpinner(color: foreground)
            .transition(.opacity)
          Spacer().frame(width: 10)
        }
        if let leadingIcon {
          TIcon(leadingIcon, size: 16, tint: foreground)
          Spacer().frame(width: 8)
        }
        TText(displayText, style: textStyle, color: foreground)
          .contentTransition(.interpolate)
        if let trailingIcon {
          Spacer().frame(width: 8)
          TIcon(trailingIcon, size: 16, tint: foreground)
        }
      }
    }
    .buttonStyle(
      TPressEffectStyle(
        TButtonLook(
          background: background,
          hover: variant == .secondary ? colors.surfaceHover : Color.black.opacity(0.06),
          active: variant == .secondary ? colors.surfaceActive : Color.black.opacity(0.20),
          hovered: hovered,
          interactive: interactive,
          height: height))
    )
    .disabled(!interactive)
    .opacity(enabled ? 1 : 0.4)
    .animation(.default, value: enabled)
    .onHover { hovered = $0 }
    .task(id: loading) {
      if loading {
        guard (try? await Task.sleep(for: .milliseconds(300))) != nil else { return }
      }
      withAnimation(loadingAnimation) { debouncedLoading = loading }
    }
  }
}

private struct TButtonLook: ButtonStyle {
  let background: Color
  let hover: Color
  let active: Color
  let hovered: Bool
  let interactive: Bool
  let height: CGFloat

  func makeBody(configuration: Configuration) -> some View {
    let shape = TShapes.rounded(TShapes.lg)
    configuration.label
      .pressEffect(configuration.isPressed)
      .frame(maxWidth: .infinity, minHeight: height, maxHeight: height)
      .background(background, in: shape)
      .highlight(
        hovered: hovered, isPressed: configuration.isPressed, enabled: interactive,
        hoverColor: hover, pressedColor: active, in: shape)
  }
}
