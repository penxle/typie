#if canImport(UIKit)

  import Design
  import SwiftUI

  struct HeroScrollView<Hero: View, Content: View>: View {
    @State private var scrollOffset: CGFloat = 0
    @State private var heroHeight: CGFloat = 0

    private let title: HeroTitleState
    private let hero: () -> Hero
    private let content: () -> Content

    init(
      title: HeroTitleState, @ViewBuilder hero: @escaping () -> Hero,
      @ViewBuilder content: @escaping () -> Content
    ) {
      self.title = title
      self.hero = hero
      self.content = content
    }

    var body: some View {
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          hero()
            .onGeometryChange(for: CGFloat.self) { proxy in
              proxy.size.height
            } action: { height in
              heroHeight = height
              syncTitle()
            }
          content()
        }
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
      }
      .onScrollGeometryChange(for: CGFloat.self) { geometry in
        geometry.contentOffset.y + geometry.contentInsets.top
      } action: { _, offset in
        scrollOffset = offset
        syncTitle()
      }
      .canvasBackground()
    }

    private func syncTitle() {
      let visible = heroHeight > 0 && scrollOffset >= heroHeight
      if title.titleVisible != visible {
        title.titleVisible = visible
      }
    }
  }

#endif
