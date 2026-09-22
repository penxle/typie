#if canImport(UIKit)

  import Design
  import SwiftUI

  struct CreateMenuItem: Identifiable {
    let id = UUID()
    let icon: TIconName
    let title: String
    let action: @MainActor () -> Void
  }

  @MainActor
  @Observable
  final class CreateMenuState {
    var items: [CreateMenuItem] = []
    var isPresented = false
    private(set) var appearedFromNil = true
    var highlightedIndex: Int? {
      didSet { appearedFromNil = oldValue == nil }
    }
  }

  struct CreateMenu: View {
    static let rowHeight: CGFloat = 42
    static let padding: CGFloat = 8

    @Environment(\.theme) private var theme
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var lastHighlightedIndex = 0
    let state: CreateMenuState

    var body: some View {
      ZStack(alignment: .top) {
        TShapes.capsule
          .fill(MenuStyle.highlight(colorScheme))
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
            .accessibilityElement(children: .combine)
            .accessibilityAddTraits(.isButton)
            .animation(
              MenuMotion.visibility(
                presented: state.isPresented,
                stagger: MenuMotion.stagger(index: index, count: state.items.count),
                reduceMotion: reduceMotion)
            ) { content in
              content.opacity(state.isPresented ? 1 : 0)
            }
          }
        }
      }
      .onChange(of: state.highlightedIndex) { _, index in
        if let index { lastHighlightedIndex = index }
      }
      .dynamicTypeSize(.large)
    }
  }

#endif
