#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct PinnedEntitiesScreen: View {
    private let store: HomeStore
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeStore, onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          ForEach(store.siteOrPlaceholder.pinned) { item in
            EntityLineRow(item: item) {
              switch item {
              case .document: onOpenDocument(item.entityId)
              case .folder: onOpenFolder(item)
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

#endif
