#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct RecentDocumentsScreen: View {
    @Environment(\.theme) private var theme

    private let store: RecentDocumentsStore
    private let onOpenDocument: (String) -> Void

    init(store: RecentDocumentsStore, onOpenDocument: @escaping (String) -> Void) {
      self.store = store
      self.onOpenDocument = onOpenDocument
    }

    var body: some View {
      let groups = store.isPlaceholder ? RecentDocumentsStore.placeholderGroups : store.groups
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          if store.loadFailed, !store.hasData {
            RetryPrompt { store.refetch() }
          } else if groups.isEmpty {
            RecentEmptyBox()
          } else {
            ForEach(Array(groups.enumerated()), id: \.element.id) { index, group in
              TText(group.label, style: TTypography.label, color: theme.colors.textMuted)
                .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
                .padding(.top, index == 0 ? 0 : 8)
              ForEach(group.documents, id: \.item.entityId) { document in
                EntityLineRow(item: document.item) { onOpenDocument(document.item.entityId) }
              }
            }
          }
        }
        .redacted(reason: store.isPlaceholder ? .placeholder : [])
        .allowsHitTesting(!store.isPlaceholder)
        .accessibilityHidden(store.isPlaceholder)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
      }
      .canvasBackground()
    }
  }

  struct RecentEmptyBox: View {
    @Environment(\.theme) private var theme

    var body: some View {
      TText(
        "최근 본 문서가 없어요", style: TTypography.detail, color: theme.colors.textHint,
        alignment: .center
      )
      .padding(.horizontal, 16)
      .frame(maxWidth: .infinity, minHeight: 80)
      .background(theme.colors.surfaceInset, in: TShapes.squircle(TShapes.md))
    }
  }

#endif
