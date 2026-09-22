#if canImport(UIKit)

  import Design
  import SwiftUI

  struct EntityContainerScreen: View {
    @Environment(\.theme) private var theme

    private let title: HeroTitleState
    private let hero: EntityContainerHeroState
    private let items: [EntityContainerItem]
    private let now: Date
    private let isPlaceholder: Bool
    private let showsRetry: Bool
    private let emptyText: String
    private let bottomClearance: CGFloat
    private let onRetry: () -> Void
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityFolderItem) -> Void

    init(
      title: HeroTitleState, hero: EntityContainerHeroState, items: [EntityContainerItem],
      now: Date, isPlaceholder: Bool, showsRetry: Bool, emptyText: String,
      bottomClearance: CGFloat, onRetry: @escaping () -> Void,
      onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityFolderItem) -> Void
    ) {
      self.title = title
      self.hero = hero
      self.items = items
      self.now = now
      self.isPlaceholder = isPlaceholder
      self.showsRetry = showsRetry
      self.emptyText = emptyText
      self.bottomClearance = bottomClearance
      self.onRetry = onRetry
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      HeroScrollView(title: title) {
        EntityContainerHero(icon: hero.icon, title: hero.title)
      } content: {
        VStack(alignment: .leading, spacing: 0) {
          if showsRetry {
            RetryPrompt(retry: onRetry)
          } else if isPlaceholder {
            EntityCardList(
              items: EntityContainerItem.placeholderRows, now: now, onOpenDocument: { _ in },
              onOpenFolder: { _ in }
            )
            .redacted(reason: .placeholder)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
          } else if items.isEmpty {
            EmptyStateBox(text: emptyText)
          } else {
            EntityCardList(
              items: items, now: now, onOpenDocument: onOpenDocument, onOpenFolder: onOpenFolder)
            TText(
              hero.summary, style: TTypography.caption, color: theme.colors.textHint,
              alignment: .center, maxLines: 1
            )
            .frame(maxWidth: .infinity)
            .padding(.top, 16)
            .padding(.bottom, 4)
          }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, bottomClearance)
      }
    }
  }

#endif
