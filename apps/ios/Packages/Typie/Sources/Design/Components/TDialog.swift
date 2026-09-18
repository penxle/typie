import SwiftUI

public struct TDialogItem: Identifiable, Equatable, Sendable {
  public let id: UUID
  public let title: String
  public let message: String
  public let confirmText: String

  public init(title: String, message: String, confirmText: String) {
    id = UUID()
    self.title = title
    self.message = message
    self.confirmText = confirmText
  }
}

extension View {
  public func dialog(_ item: TDialogItem?, onDismiss: @escaping () -> Void) -> some View {
    overlay { TDialogOverlay(item: item, onDismiss: onDismiss) }
  }
}

private struct TDialogOverlay: View {
  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  let item: TDialogItem?
  let onDismiss: () -> Void

  private var animation: Animation {
    reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.25)
  }

  private var cardTransition: AnyTransition {
    reduceMotion ? .opacity : .opacity.combined(with: .scale(scale: 1.08))
  }

  var body: some View {
    ZStack {
      if let item {
        colors.scrim
          .ignoresSafeArea()
          .contentShape(Rectangle())
          .onTapGesture(perform: onDismiss)
          .transition(.opacity)
          .zIndex(0)
        card(item)
          .id(item.id)
          .transition(cardTransition)
          .zIndex(1)
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .allowsHitTesting(item != nil)
    .animation(animation, value: item)
  }

  private func card(_ item: TDialogItem) -> some View {
    let shape = TShapes.rounded(TShapes.lg)
    return VStack(alignment: .leading, spacing: 0) {
      TText(item.title, style: TTypography.title, color: colors.textDefault)
      Spacer().frame(height: 6)
      TText(item.message, style: TTypography.caption, color: colors.textMuted)
      Spacer().frame(height: 20)
      TButton(item.confirmText, variant: .secondary) { onDismiss() }
    }
    .padding(.horizontal, 20)
    .padding(.top, 24)
    .padding(.bottom, 16)
    .frame(width: 300)
    .background(colors.surfaceDefault.shadow(theme.shadows.xl), in: shape)
    .compositingGroup()
    .accessibilityElement(children: .contain)
    .accessibilityAddTraits(.isModal)
    .onAppear { AccessibilityNotification.Announcement("\(item.title). \(item.message)").post() }
  }
}
