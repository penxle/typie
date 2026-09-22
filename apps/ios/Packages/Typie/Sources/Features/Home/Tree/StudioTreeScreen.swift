#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct StudioTreeScreen: View {
    @Environment(\.theme) private var theme

    private let store: HomeStore
    private let tree: HomeTreeStore
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeStore, tree: HomeTreeStore, onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.tree = tree
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let site = store.siteOrPlaceholder
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          if store.loadFailed, !store.hasData {
            RetryPrompt { store.refetch() }
          } else if site.roots.isEmpty {
            TText("아직 문서가 없어요", style: TTypography.detail, color: theme.colors.textHint)
              .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
          } else {
            HomeTreeView(
              store: tree, nodes: site.roots, depth: 0, onOpenDocument: onOpenDocument,
              onOpenFolder: onOpenFolder)
          }
        }
        .redacted(reason: store.isPlaceholder ? .placeholder : [])
        .allowsHitTesting(!store.isPlaceholder)
        .accessibilityHidden(store.isPlaceholder)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .padding(.bottom, HomeBody.bottomClearance)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
      }
      .canvasBackground()
    }
  }

#endif
