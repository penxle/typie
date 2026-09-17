import Foundation
import SwiftUI

public enum TToastKind: Sendable {
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

public struct TToastView: View {
  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  private let center: TToastCenter

  public init(center: TToastCenter) {
    self.center = center
  }

  private var transition: AnyTransition {
    let base: AnyTransition =
      reduceMotion ? .opacity : .opacity.combined(with: .offset(y: 4))
    return .asymmetric(
      insertion: base.animation(.easeOut(duration: 0.2)),
      removal: base.animation(.easeIn(duration: 0.2)))
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
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
    .animation(.easeOut(duration: 0.2), value: center.current)
  }

  private func surface(_ item: TToastItem) -> some View {
    let shape = TShapes.rounded(TShapes.lg)
    return HStack(spacing: 8) {
      ZStack {
        Circle().fill(colors.dangerDefault).frame(width: 20, height: 20)
        TIcon(TypieIcon.exclamation, size: 12, tint: colors.textOnDanger)
      }
      TText(item.message, style: TTypography.caption, color: colors.textOnInverse)
    }
    .padding(.horizontal, 24)
    .padding(.vertical, 16)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background {
      shape.fill(.ultraThinMaterial)
      shape.fill(colors.surfaceInverse.opacity(0.6))
    }
    .frame(maxWidth: 600)
    .padding(.horizontal, 16)
  }
}
