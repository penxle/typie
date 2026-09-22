import Foundation
import SwiftUI

public enum TToastKind: Sendable {
  case success
  case error
}

public struct TToastItem: Identifiable, Equatable, Sendable {
  public let id: UUID
  public let kind: TToastKind
  public let message: String
}

@MainActor
@Observable
public final class TToastCenter {
  public private(set) var current: TToastItem?

  @ObservationIgnored private let sleep: @Sendable (Duration) async throws -> Void
  @ObservationIgnored private var timer: Task<Void, Never>?

  public init(
    sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) }
  ) {
    self.sleep = sleep
  }

  public func success(_ message: String) {
    show(TToastItem(id: UUID(), kind: .success, message: message))
  }

  public func error(_ message: String) {
    show(TToastItem(id: UUID(), kind: .error, message: message))
  }

  public func dismiss() {
    timer?.cancel()
    timer = nil
    current = nil
  }

  public static func duration(for message: String) -> Duration {
    .milliseconds(2000 + min(max(message.count - 18, 0), 100) * 12)
  }

  private func show(_ item: TToastItem) {
    timer?.cancel()
    current = item
    let duration = Self.duration(for: item.message)
    timer = Task { [weak self, sleep] in
      try? await sleep(duration)
      guard !Task.isCancelled else { return }
      self?.expire(item.id)
    }
  }

  private func expire(_ id: UUID) {
    guard current?.id == id else { return }
    timer = nil
    current = nil
  }
}

@MainActor
@Observable
public final class TToastLayout {
  public var bottomInset: CGFloat = 0

  public init() {}
}

public struct TToastOverlay: View {
  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  @ScaledMetric(relativeTo: TTypography.control.textStyle) private var badgeSide: CGFloat = 20

  private let center: TToastCenter
  private let layout: TToastLayout

  public init(center: TToastCenter, layout: TToastLayout = TToastLayout()) {
    self.center = center
    self.layout = layout
  }

  private var transition: AnyTransition {
    if reduceMotion {
      return .asymmetric(
        insertion: .opacity.animation(.easeOut(duration: 0.18)),
        removal: .opacity.animation(.easeOut(duration: 0.15)))
    }
    return .asymmetric(
      insertion: .opacity.combined(with: .scale(scale: 0.94, anchor: .bottom))
        .animation(.easeOut(duration: 0.22)),
      removal: .opacity.combined(with: .scale(scale: 0.96, anchor: .bottom))
        .animation(.easeOut(duration: 0.15)))
  }

  public var body: some View {
    ZStack(alignment: .bottom) {
      if let item = center.current {
        surface(item)
          .id(item.id)
          .transition(transition)
          .onAppear { AccessibilityNotification.Announcement(item.message).post() }
      }
    }
    .padding(.bottom, layout.bottomInset)
    .animation(reduceMotion ? nil : .easeOut(duration: 0.25), value: layout.bottomInset)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
    .animation(.easeOut(duration: 0.22), value: center.current)
  }

  private func surface(_ item: TToastItem) -> some View {
    HStack(spacing: 8) {
      ZStack {
        switch item.kind {
        case .success:
          Circle().fill(colors.successDefault).frame(width: badgeSide, height: badgeSide)
          TIcon(
            LucideIcon.check, size: 12, tint: colors.textOnSuccess,
            relativeTo: TTypography.control)
        case .error:
          Circle().fill(colors.dangerDefault).frame(width: badgeSide, height: badgeSide)
          TIcon(
            TypieIcon.exclamation, size: 12, tint: colors.textOnDanger,
            relativeTo: TTypography.control)
        }
      }
      TText(item.message, style: TTypography.control, color: colors.textDefault)
    }
    .padding(.leading, 14)
    .padding(.trailing, 18)
    .padding(.vertical, 12)
    .modifier(ToastGlass())
    .frame(maxWidth: 600)
    .padding(.horizontal, 16)
  }
}

private struct ToastGlass: ViewModifier {
  func body(content: Content) -> some View {
    if #available(iOS 26, macOS 26, *) {
      content.glassEffect(.regular, in: .capsule)
    } else {
      content.background(.ultraThinMaterial, in: TShapes.capsule)
    }
  }
}
