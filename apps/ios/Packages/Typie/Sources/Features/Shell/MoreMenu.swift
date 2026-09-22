#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  struct MoreMenuItem: Identifiable {
    let id = UUID()
    let icon: TIconName
    let title: String
    let action: (@MainActor (UIViewController) -> Void)?

    init(icon: TIconName, title: String, action: (@MainActor (UIViewController) -> Void)? = nil) {
      self.icon = icon
      self.title = title
      self.action = action
    }
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

  enum MenuMotion {
    private static let entranceDelay: TimeInterval = 0.06
    private static let entranceDuration: TimeInterval = 0.25
    private static let exitDuration: TimeInterval = 0.1
    private static let staggerBase: TimeInterval = 0.018
    private static let staggerSpan: TimeInterval = 0.14
    private static let staggerExponent = 1.4

    static func visibility(presented: Bool, stagger: TimeInterval, reduceMotion: Bool) -> Animation
    {
      guard presented else { return .easeOut(duration: exitDuration) }
      return .timingCurve(0.23, 1, 0.32, 1, duration: entranceDuration)
        .delay(entranceDelay + (reduceMotion ? 0 : stagger))
    }

    static func stagger(index: Int, count: Int) -> TimeInterval {
      let steps = max(count - 1, 1)
      let progress = Double(index) / Double(steps)
      return staggerBase * Double(index) + staggerSpan * pow(progress, staggerExponent)
    }
  }

  struct MoreMenu: View {
    static let rowHeight: CGFloat = 42
    private static let headerRowHeight: CGFloat = 59
    private static let dividerGap: CGFloat = 6
    static let headerHeight: CGFloat = headerRowHeight + 1 + dividerGap
    private static let avatarSide: CGFloat = 32
    private static let avatarAssetName = "ProfilePlaceholder"
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
      .dynamicTypeSize(.large)
    }

    private var header: some View {
      VStack(spacing: 0) {
        HStack(spacing: 12) {
          Image(Self.avatarAssetName)
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
            state.appearedFromNil || reduceMotion
              ? nil : .snappy(duration: 0.25, extraBounce: 0.05),
            value: state.highlightedIndex)
        VStack(spacing: 0) {
          ForEach(Array(state.items.enumerated()), id: \.element.id) { index, item in
            HStack(spacing: 12) {
              TIcon(item.icon, size: 18, tint: theme.colors.textDefault)
              TText(item.title, style: TTypography.control, color: theme.colors.textDefault)
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
      MenuMotion.visibility(
        presented: state.isPresented, stagger: stagger, reduceMotion: reduceMotion)
    }

    private func staggerDelay(for index: Int) -> TimeInterval {
      MenuMotion.stagger(index: index, count: state.items.count)
    }

    private var highlightColor: Color {
      MenuStyle.highlight(colorScheme)
    }
  }

  enum MenuStyle {
    static func highlight(_ scheme: ColorScheme) -> Color {
      scheme == .dark ? Color.white.opacity(0.16) : Color.black.opacity(0.08)
    }
  }

#endif
