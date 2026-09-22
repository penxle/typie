#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeScreen<Sections: View>: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let title: HeroTitleState
    private let store: HomeStore
    private let sections: () -> Sections

    init(title: HeroTitleState, store: HomeStore, @ViewBuilder sections: @escaping () -> Sections) {
      self.title = title
      self.store = store
      self.sections = sections
    }

    var body: some View {
      HeroScrollView(title: title) {
        TText("홈", style: TTypography.hero, color: colors.textDefault)
          .padding(.horizontal, 16)
          .padding(.top, 12)
          .frame(maxWidth: .infinity, alignment: .leading)
      } content: {
        VStack(alignment: .leading, spacing: 0) {
          if store.loadFailed, !store.hasData {
            RetryPrompt { store.refetch() }
          } else {
            sections()
              .redacted(reason: store.isPlaceholder ? .placeholder : [])
              .allowsHitTesting(!store.isPlaceholder)
              .accessibilityHidden(store.isPlaceholder)
          }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, HomeBody.bottomClearance)
      }
      .onAppear { store.refetch() }
    }
  }

#endif
