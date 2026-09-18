import Design
import SwiftUI

struct MoreMenuItem: Identifiable {
  let id = UUID()
  let icon: TIconName
  let title: String
}

@MainActor
@Observable
final class MoreMenuState {
  let items: [MoreMenuItem]
  let profileName: String
  var isPresented = false
  private(set) var appearedFromNil = true
  var highlightedIndex: Int? {
    didSet { appearedFromNil = oldValue == nil }
  }

  init(items: [MoreMenuItem], profileName: String) {
    self.items = items
    self.profileName = profileName
  }
}

struct MoreMenu: View {
  static let rowHeight: CGFloat = 42
  private static let headerRowHeight: CGFloat = 59
  private static let dividerGap: CGFloat = 6
  static let headerHeight: CGFloat = headerRowHeight + 1 + dividerGap
  private static let avatarSide: CGFloat = 32
  private static let entranceDelay: TimeInterval = 0.06
  private static let entranceDuration: TimeInterval = 0.25
  private static let exitDuration: TimeInterval = 0.1
  private static let staggerBase: TimeInterval = 0.018
  private static let staggerSpan: TimeInterval = 0.14
  private static let staggerExponent = 1.4

  @Environment(\.theme) private var theme
  @Environment(\.colorScheme) private var colorScheme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var lastHighlightedIndex = 0
  let state: MoreMenuState

  var body: some View {
    VStack(spacing: 0) {
      header
      rows
    }
  }

  private var header: some View {
    VStack(spacing: 0) {
      HStack(spacing: 12) {
        Image("ProfilePlaceholder")
          .resizable()
          .scaledToFill()
          .frame(width: Self.avatarSide, height: Self.avatarSide)
          .clipShape(TShapes.squircle(Self.avatarSide * 0.3))
        TText(state.profileName, style: TTypography.label, color: theme.colors.textDefault)
        Spacer(minLength: 0)
      }
      .padding(.horizontal, 12)
      .frame(height: Self.headerRowHeight)
      Rectangle()
        .fill(theme.colors.borderHairline)
        .frame(height: 1)
      Spacer(minLength: 0)
        .frame(height: Self.dividerGap)
    }
    .frame(height: Self.headerHeight)
    .animation(visibilityAnimation(stagger: 0)) { content in
      content.opacity(state.isPresented ? 1 : 0)
    }
  }

  private var rows: some View {
    ZStack(alignment: .top) {
      TShapes.capsule
        .fill(highlightColor)
        .frame(height: Self.rowHeight)
        .offset(y: CGFloat(state.highlightedIndex ?? lastHighlightedIndex) * Self.rowHeight)
        .opacity(state.highlightedIndex == nil ? 0 : 1)
        .animation(
          state.appearedFromNil || reduceMotion ? nil : .snappy(duration: 0.25, extraBounce: 0.05),
          value: state.highlightedIndex)
      VStack(spacing: 0) {
        ForEach(Array(state.items.enumerated()), id: \.element.id) { index, item in
          HStack(spacing: 12) {
            TIcon(item.icon, size: 18, tint: theme.colors.textDefault)
            TText(item.title, style: TTypography.action, color: theme.colors.textDefault)
            Spacer(minLength: 0)
          }
          .padding(.horizontal, 12)
          .frame(height: Self.rowHeight)
          .animation(visibilityAnimation(stagger: staggerDelay(for: index))) { content in
            content.opacity(state.isPresented ? 1 : 0)
          }
        }
      }
    }
    .onChange(of: state.highlightedIndex) { _, index in
      if let index { lastHighlightedIndex = index }
    }
  }

  private func visibilityAnimation(stagger: TimeInterval) -> Animation {
    guard state.isPresented else { return .easeOut(duration: Self.exitDuration) }
    return .timingCurve(0.23, 1, 0.32, 1, duration: Self.entranceDuration)
      .delay(Self.entranceDelay + (reduceMotion ? 0 : stagger))
  }

  private func staggerDelay(for index: Int) -> TimeInterval {
    let count = max(state.items.count - 1, 1)
    let progress = Double(index) / Double(count)
    return Self.staggerBase * Double(index) + Self.staggerSpan * pow(progress, Self.staggerExponent)
  }

  private var highlightColor: Color {
    colorScheme == .dark ? Color.white.opacity(0.16) : Color.black.opacity(0.08)
  }
}
