#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeScreen<Sections: View, Extra: View>: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    @State private var scrollOffset: CGFloat = 0
    @State private var headingHeight: CGFloat = 0

    private let title: HomeTitleState
    private let store: HomeStore
    private let sections: () -> Sections
    private let extra: Extra

    init(
      title: HomeTitleState, store: HomeStore, @ViewBuilder sections: @escaping () -> Sections,
      @ViewBuilder extra: () -> Extra
    ) {
      self.title = title
      self.store = store
      self.sections = sections
      self.extra = extra()
    }

    var body: some View {
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          heading
          VStack(alignment: .leading, spacing: 0) {
            if store.loadFailed, !store.hasData {
              failure
            } else {
              sections()
                .redacted(reason: store.isPlaceholder ? .placeholder : [])
                .allowsHitTesting(!store.isPlaceholder)
                .accessibilityHidden(store.isPlaceholder)
            }
            extra
          }
          .padding(.horizontal, 16)
          .padding(.bottom, HomeBody.bottomClearance)
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
      .onAppear { store.refetch() }
    }

    private var failure: some View {
      RetryPrompt { store.refetch() }
    }

    private var heading: some View {
      TText("홈", style: TTypography.hero, color: colors.textDefault)
        .padding(.horizontal, 16)
        .padding(.top, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .onGeometryChange(for: CGFloat.self) { proxy in
          proxy.size.height
        } action: { height in
          headingHeight = height
          syncTitle()
        }
    }

    private func syncTitle() {
      let visible = scrollOffset >= headingHeight
      if title.titleVisible != visible {
        title.titleVisible = visible
      }
    }
  }

#endif
