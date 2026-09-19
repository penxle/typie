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

@MainActor
@Observable
public final class TDialogCenter {
  public private(set) var current: TDialogItem?
  @ObservationIgnored private var onDismiss: (@MainActor () -> Void)?

  public init() {}

  public func present(_ item: TDialogItem, onDismiss: @escaping @MainActor () -> Void = {}) {
    current = item
    self.onDismiss = onDismiss
  }

  public func dismiss() {
    current = nil
    onDismiss = nil
  }

  public func confirm() {
    let action = onDismiss
    dismiss()
    action?()
  }
}

public struct TDialogOverlay: View {
  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  private let center: TDialogCenter

  public init(center: TDialogCenter) {
    self.center = center
  }

  private var animation: Animation {
    reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.25)
  }

  private var cardTransition: AnyTransition {
    reduceMotion ? .opacity : .opacity.combined(with: .scale(scale: 1.08))
  }

  public var body: some View {
    ZStack {
      if let item = center.current {
        colors.scrim
          .ignoresSafeArea()
          .contentShape(Rectangle())
          .onTapGesture { center.confirm() }
          .transition(.opacity)
          .zIndex(0)
        card(item)
          .id(item.id)
          .transition(cardTransition)
          .zIndex(1)
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .ignoresSafeArea(.keyboard)
    .allowsHitTesting(center.current != nil)
    .animation(animation, value: center.current)
  }

  private func card(_ item: TDialogItem) -> some View {
    let shape = TShapes.rounded(TShapes.lg)
    return VStack(alignment: .leading, spacing: 0) {
      TText(item.title, style: TTypography.title, color: colors.textDefault)
      Spacer().frame(height: 6)
      TText(item.message, style: TTypography.caption, color: colors.textMuted)
      Spacer().frame(height: 20)
      TButton(item.confirmText, variant: .secondary) { center.confirm() }
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
